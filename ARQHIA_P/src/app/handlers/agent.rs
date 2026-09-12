//! Handler de Orquestador: plan, workers, permisos, auditoría.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;
use iced::widget::markdown;

use crate::app::state::App;
use crate::app::Message;
use crate::app::orchestrator::{
    abort_agent_placeholder, account_tokens, continue_after_worker, finish_orchestrator,
    call_needs_approval, finish_agent_answer, request_next_llm_step, spawn_exec_calls, start_worker,
    start_worker_with_task, OrchTask,
};
use crate::agent;
use crate::db;
use crate::llm::Role;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::AgentAnalyze(turn, res) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            let (provider, cfg, ws) = match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
                (Some(p), Some(c), Some(w)) => (p, c, w),
                _ => {
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    return Task::none();
                }
            };
            let pedido = state
                .o_history
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.clone())
                .unwrap_or_default();
            // Fallback v0.7.3: sin provider/timeout → contexto crudo.
            let brief = match res {
                Ok(b) if !b.trim().is_empty() => b,
                _ => {
                    state.push_log("⚠️ analista no disponible: uso contexto crudo".to_string());
                    crate::workspace::context_block(&ws)
                }
            };
            // Trazabilidad: CONTEXT/ANALYSIS.md (lo consume el planner y el worker).
            let dir = ws.join("CONTEXT");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("ANALYSIS.md"), &brief);
            state.push_log(format!("🧠 brief listo ({} chars)", brief.chars().count()));
            let mode = state.o_mode;
            let context = crate::workspace::context_block(&ws);
            let espec = agent::read_espec_md(&ws);
            let label = match mode {
                db::Mode::Plan => "Plan",
                _ => "Work",
            };
            Task::perform(
                async move {
                    agent::plan_tasks(
                        provider,
                        &pedido,
                        &context,
                        espec.as_deref(),
                        Some(&brief),
                        &cfg,
                        label,
                    )
                    .await
                },
                move |r| match mode {
                    db::Mode::Plan => Message::PlanDone(turn, r),
                    _ => Message::AgentPlan(turn, r),
                },
            )
        }
        Message::AgentPlan(turn, res) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            match res {
                Err(e) => {
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    state.driver = None;
                    state.status = format!("Error del planificador: {}", crate::llm::friendly_error(&e));
                    state.push_log(format!("❌ plan: {e}"));
                }
                Ok(tasks) => {
                    let max_tasks = state.config.limits.clamped().max_tasks;
                    let mut tasks = tasks;
                    if tasks.len() > max_tasks {
                        state.push_log(format!(
                            "🧭 plan recortado a {max_tasks} tareas (límite)"
                        ));
                        tasks.truncate(max_tasks);
                    }
                    state.orch_tasks = tasks
                        .into_iter()
                        .map(|t| OrchTask {
                            desc: t.desc.clone(),
                            files: t.files.clone(),
                            done: false,
                            active: false,
                        })
                        .collect();
                    state.push_log(format!("🧭 plan: {} tareas", state.orch_tasks.len()));
                    return start_worker(state, 0);
                }
            }
            Task::none()
        }
        Message::PlanDone(turn, res) => {
            // Modo Plan (v0.7.1): 1 llamada, sin tools por construcción.
            // Solo escribe CONTEXT/PLAN.md + checklist; el disco queda intacto.
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            match res {
                Err(e) => {
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    state.driver = None;
                    state.status = format!("Error del planificador: {}", crate::llm::friendly_error(&e));
                    state.push_log(format!("❌ plan: {e}"));
                }
                Ok(tasks) => {
                    let max_tasks = state.config.limits.clamped().max_tasks;
                    let mut tasks = tasks;
                    if tasks.len() > max_tasks {
                        state.push_log(format!("🧭 plan recortado a {max_tasks} tareas (límite)"));
                        tasks.truncate(max_tasks);
                    }
                    let user_text = state
                        .o_history
                        .iter()
                        .rev()
                        .find(|m| m.role == Role::User)
                        .map(|m| m.content.clone())
                        .unwrap_or_default();
                    let md = render_plan_md(&tasks, &user_text);
                    match write_plan_md(state, &md) {
                        Ok(path) => {
                            state.plan_md = md;
                            state.show_plan = true;
                            state.orch_tasks = tasks
                                .into_iter()
                                .map(|t| OrchTask {
                                    desc: t.desc.clone(),
                                    files: t.files.clone(),
                                    done: false,
                                    active: false,
                                })
                                .collect();
                            let n = state.orch_tasks.len();
                            state.push_log(format!("📋 PLAN.md con {n} tareas ({path})"));
                            finish_agent_answer(
                                state,
                                format!(
                                    "Plan listo en `CONTEXT/PLAN.md`: {n} tareas; brief en `CONTEXT/ANALYSIS.md` (solo CONTEXT/, sin tocar código). Revísalo abajo y pulsa **Ejecutar plan** para pasarlo a Work."
                                ),
                            );
                        }
                        Err(e) => {
                            abort_agent_placeholder(state);
                            state.status = format!("No se pudo escribir PLAN.md: {e}");
                            state.push_log(format!("❌ PLAN.md: {e}"));
                        }
                    }
                    state.agent_running = false;
                    state.driver = None;
                }
            }
            Task::none()
        }
        Message::ExecutePlan => {
            // Botón "Ejecutar plan": pasa a Work y arranca el orquestador.
            if state.agent_running || state.streaming {
                return Task::none();
            }
            if !state.show_plan || state.orch_tasks.is_empty() {
                state.status = "No hay plan pendiente de ejecutar.".to_string();
                return Task::none();
            }
            let (provider, cfg, ws, chat_id) = match (
                state.o_provider,
                state.o_cfg.clone(),
                state.o_ws.clone(),
                state.o_chat,
            ) {
                (Some(p), Some(c), Some(w), chat) => (p, c, w, chat),
                _ => {
                    state.status = "El plan perdió su contexto; pide el plan de nuevo.".to_string();
                    return Task::none();
                }
            };
            if let Some(id) = chat_id {
                let _ = db::set_chat_mode(id, crate::db::Mode::Work);
                if let Some(c) = state.chats.iter_mut().find(|c| c.id == id) {
                    c.mode = crate::db::Mode::Work;
                }
            }
            state.messages.push(crate::llm::ChatMsg {
                role: Role::Assistant,
                content: "orquestando...".to_string(),
            });
            state.md.push(markdown::parse("orquestando...").collect());
            state.msg_usage.push(None);
            state.msg_times.push(String::new());
            state.msg_ids.push(0);
            state.agent_running = true;
            state.show_plan = false;
            state.o_provider = Some(provider);
            state.o_cfg = Some(cfg);
            state.o_ws = Some(ws.clone());
            state.worker_answers.clear();
            state.fix_cycle = 0;
            state.pending_calls.clear();
            state.denied_tools.clear();
            state.driver = None;
            state.agent_gen += 1;
            if let Some(line) = agent::ensure_agents_md(&ws) {
                state.push_log(line);
            }
            // v0.7.2: rama de trabajo + árbol limpio antes de tocar nada.
            crate::app::orchestrator::prepare_git_turn(state);
            state.push_log(format!("▶ ejecutando plan ({} tareas)", state.orch_tasks.len()));
            start_worker(state, 0)
        }
        Message::DismissPlan => {
            state.show_plan = false;
            Task::none()
        }
        Message::AgentLlm(turn, res) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            let mut drv = match state.driver.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            match res {
                Err(e) => {
                    state.driver = None;
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    state.status = format!("Error del agente: {}", crate::llm::friendly_error(&e));
                    state.push_log(format!("❌ {e}"));
                    return Task::none();
                }
                Ok(agent::StepOutcome::Final(answer)) => {
                    state.worker_answers.push(answer);
                    if drv.task_idx != usize::MAX
                        && let Some(t) = state.orch_tasks.get_mut(drv.task_idx) {
                            t.done = true;
                            t.active = false;
                        }
                    state.driver = None;
                    return continue_after_worker(state);
                }
                Ok(agent::StepOutcome::Calls { calls, assistant_msg }) => {
                    drv.raw.push(assistant_msg);
                    drv.step += 1;
                    let max_steps = drv.max_steps;
                    if drv.step > max_steps {
                        state.worker_answers.push(format!(
                            "Llegué al límite de {max_steps} pasos en esta subtarea (Límites en Config)."
                        ));
                        if drv.task_idx != usize::MAX
                            && let Some(t) = state.orch_tasks.get_mut(drv.task_idx) {
                                t.done = true;
                                t.active = false;
                            }
                        state.driver = None;
                        return continue_after_worker(state);
                    }
                    // Parada temprana v0.7.1: el mismo lote 2 veces seguidas
                    // (o steps sin cambios) -> fin del worker, sin quemar tokens.
                    let sig = agent::calls_signature(&calls);
                    if drv.last_calls_sig.as_deref() == Some(&sig) {
                        drv.stale_steps += 1;
                    } else {
                        drv.stale_steps = 0;
                        drv.last_calls_sig = Some(sig);
                    }
                    if drv.stale_steps >= 2 {
                        state.worker_answers.push(
                            "Me estanqué repitiendo las mismas llamadas: paro para no quemar tokens. Reformula el pedido o dime el siguiente paso.".to_string(),
                        );
                        if drv.task_idx != usize::MAX
                            && let Some(t) = state.orch_tasks.get_mut(drv.task_idx) {
                                t.done = true;
                                t.active = false;
                            }
                        state.driver = None;
                        state.push_log("⏹ parada temprana: calls idénticas".to_string());
                        return continue_after_worker(state);
                    }
                    state.driver = Some(drv);
                    // Presupuesto v0.7.1: contabiliza ANTES de ejecutar más.
                    if account_tokens(state) {
                        return stop_worker_for_budget(state);
                    }
                    let mut drv = match state.driver.take() {
                        Some(d) => d,
                        None => return Task::none(),
                    };
                    // Denegadas con memoria: se niegan solas sin modal
                    let mut auto_denied = Vec::new();
                    let mut rest = Vec::new();
                    for c in calls {
                        if state.denied_tools.iter().any(|n| n == &c.name) {
                            auto_denied.push(c);
                        } else {
                            rest.push(c);
                        }
                    }
                    if !auto_denied.is_empty() {
                        state.push_log(format!(
                            "⛔ {} denegada(s) (recordado)",
                            auto_denied.len()
                        ));
                        drv.raw.extend(agent::denial_msgs(drv.provider, &auto_denied));
                    }
                    if rest.is_empty() {
                        state.driver = Some(drv);
                        return request_next_llm_step(state);
                    }
                    // Caché de lecturas v0.7.1: lo ya leído este turno se
                    // sirve sin releer del disco (y sin pedir permiso).
                    let mut fresh = Vec::new();
                    let mut served = 0;
                    for c in rest {
                        let hit = agent::read_cache_key(&c).and_then(|k| {
                            drv.read_cache.get(&k).cloned()
                        });
                        match hit {
                            Some(out) => {
                                let preview: String =
                                    c.args.to_string().chars().take(60).collect();
                                state.push_log(format!("📦 caché: {} {preview}", c.name));
                                drv.raw.push(agent::cached_result_msg(drv.provider, &c, &out));
                                served += 1;
                            }
                            None => fresh.push(c),
                        }
                    }
                    if fresh.is_empty() {
                        debug_assert!(served > 0);
                        state.driver = Some(drv);
                        return request_next_llm_step(state);
                    }
                    let needs_any: bool = fresh.iter().any(|c| call_needs_approval(state, c));
                    state.driver = Some(drv);
                    if !needs_any {
                        // Todo auto: ejecutar ya, el resultado vuelve como AgentExecDone
                        return spawn_exec_calls(state, fresh, false);
                    }
                    // Alguna requiere permiso -> el lote entero espera aprobación
                    state.pending_calls = fresh;
                    state.push_log("🔐 el agente pide permiso (ver panel)".to_string());
                }
            }
            Task::none()
        }
        Message::AgentExecDone(turn, calls, append, logs) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            let mut drv = match state.driver.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            // v0.7.1: guarda lecturas puras en la caché del turno.
            for (c, m) in calls.iter().zip(append.iter()) {
                if let Some(key) = agent::read_cache_key(c)
                    && let Some(text) = tool_output_text(m)
                    && !text.starts_with('❌') {
                        drv.read_cache.insert(key, text);
                    }
            }
            // v0.7.4 Fuentes: captura URLs de fetch_url para el bloque clicable.
            for c in calls.iter() {
                if c.name == "fetch_url"
                    && let Some(u) = c.args.get("url").and_then(|v| v.as_str())
                    && !u.trim().is_empty()
                    && !state.chat_sources.iter().any(|x| x == u.trim()) {
                        state.chat_sources.push(u.trim().to_string());
                    }
            }
            for line in logs {
                state.push_log(line);
            }
            drv.raw.extend(append);
            state.driver = Some(drv);
            // Presupuesto v0.7.1: los outputs pueden ser KBs; contabiliza
            // al recibirlos y para con mensaje si se agotó.
            if account_tokens(state) {
                return stop_worker_for_budget(state);
            }
            // Colapso progresivo: solo los últimos 4 resultados intactos.
            if let Some(d) = state.driver.as_mut() {
                agent::collapse_old_tool_outputs(&mut d.raw, 4);
            }
            request_next_llm_step(state)
        }
        Message::ApproveTools => {
            let calls = std::mem::take(&mut state.pending_calls);
            if calls.is_empty() || state.driver.is_none() {
                return Task::none();
            }
            state.push_log(format!("✅ permitidas {} acciones", calls.len()));
            spawn_exec_calls(state, calls, true)
        }
        Message::DenyTools => {
            let calls = std::mem::take(&mut state.pending_calls);
            if calls.is_empty() {
                return Task::none();
            }
            let mut drv = match state.driver.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            state.push_log("⛔ denegado por el usuario".to_string());
            drv.raw.extend(agent::denial_msgs(drv.provider, &calls));
            state.driver = Some(drv);
            request_next_llm_step(state)
        }
        Message::DenyToolsRemember => {
            let calls = std::mem::take(&mut state.pending_calls);
            if calls.is_empty() {
                return Task::none();
            }
            let mut drv = match state.driver.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            for c in &calls {
                if !state.denied_tools.iter().any(|n| n == &c.name) {
                    state.denied_tools.push(c.name.clone());
                }
            }
            state.push_log("⛔ denegado y no se volverá a preguntar en este turno".to_string());
            drv.raw.extend(agent::denial_msgs(drv.provider, &calls));
            state.driver = Some(drv);
            request_next_llm_step(state)
        }
        Message::StopAgent => {
            if !state.agent_running && !state.streaming {
                return Task::none();
            }
            // Invalida todo lo que siga en vuelo de este turno (agente) y los
            // chunks del stream de chat plano.
            state.agent_gen += 1;
            state.agent_running = false;
            state.driver = None;
            state.pending_calls.clear();
            state.stream_gen += 1;
            state.streaming = false;
            // Conserva lo parcial si ya había texto; si no, marca detenido.
            if let Some(last) = state.messages.last_mut()
                && last.role == Role::Assistant {
                    if last.content.trim().is_empty() {
                        last.content = "_Turno detenido por el usuario._".to_string();
                    } else {
                        last.content.push_str("\n\n_(detenido por el usuario)_");
                    }
                }
            state.reparse_last_md();
            if let Some(chat_id) = state.active_chat
                && let Some(last) = state.messages.last() {
                    let _ = db::save_msg(chat_id, "assistant", &last.content);
                    state.resync_msg_meta(chat_id);
                }
            state.push_log("⏹ turno detenido por el usuario".to_string());
            state.status.clear();
            Task::none()
        }
        Message::AgentAudit(turn, res) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            match res {
                Err(e) => {
                    // Auditor caído: finaliza con lo que haya, sin commit.
                    state.git_verify_ok = false;
                    state.push_log(format!("⚠️ auditor no disponible: {e}"));
                    finish_orchestrator(state, " (auditor no disponible)".to_string())
                }
                Ok((temp, logs, clean)) => {
                    for line in logs {
                        state.push_log(line);
                    }
                    state.git_verify_ok = clean;
                    let ws = match state.o_ws.clone() {
                        Some(w) => w,
                        None => return finish_orchestrator(state, String::new()),
                    };
                    let dir = ws.join("CONTEXT");
                    let _ = std::fs::create_dir_all(&dir);
                    let file = dir.join("TEMP.md");
                    let _ = std::fs::write(&file, &temp);
                    if agent::temp_has_issues(&temp) {
                        let issues_n = temp
                            .lines()
                            .filter(|l| l.trim_start().starts_with('-'))
                            .count();
                        // v0.7.3: bucle sin tope salvo max_fix_cycles (0 = ilimitado).
                        let max = state.config.limits.clamped().max_fix_cycles;
                        if matches!(fix_decision(true, state.fix_cycle, max), FixDecision::CapReached) {
                            state.push_log(format!(
                                "⏹ tope de ciclos ({max}) con issues: sin commit"
                            ));
                            return finish_orchestrator(state, String::new());
                        }
                        state.fix_cycle += 1;
                        state.push_log(format!(
                            "↻ ciclo {}: {} issues",
                            state.fix_cycle, issues_n
                        ));
                        let preview: String = temp
                            .lines()
                            .filter(|l| l.starts_with('-') || l.starts_with('#'))
                            .take(4)
                            .collect::<Vec<_>>()
                            .join(" / ");
                        state.push_log(format!("📝 TEMP.md con issues: {}", preview));
                        let fix_desc = format!(
                            "Corrige estos issues del auditor (ciclo {}, sin cambiar nada más):\n{}",
                            state.fix_cycle,
                            temp.chars().take(1200).collect::<String>()
                        );
                        state.orch_tasks.push(OrchTask {
                            desc: format!("Fixes del auditor (ciclo {})", state.fix_cycle),
                            files: Vec::new(),
                            done: false,
                            active: false,
                        });
                        let idx = state.orch_tasks.len() - 1;
                        let task = agent::WTask {
                            desc: fix_desc,
                            files: Vec::new(),
                        };
                        return start_worker_with_task(state, idx, task);
                    }
                    state.push_log(format!("✅ estable tras {} ciclos", state.fix_cycle));
                    finish_orchestrator(state, String::new())
                }
            }
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}

/// Para el worker actual por presupuesto agotado (v0.7.1): marca la tarea,
/// deja mensaje claro (cómo subir el límite) y sigue al siguiente worker.
/// Precondición: `account_tokens` devolvió true con el driver almacenado.
fn stop_worker_for_budget(state: &mut App) -> Task<Message> {
    let (budget, used) = state
        .driver
        .as_ref()
        .map(|d| (d.token_budget, d.tokens_used))
        .unwrap_or((0, 0));
    state.worker_answers.push(format!(
        "Llegué al presupuesto de {} tokens este turno ({} usados). Súbelo en Config -> Permisos -> Límites (0 = sin límite) o pídeme que continúe.",
        agent::format_tokens(budget),
        agent::format_tokens(used),
    ));
    if let Some(d) = state.driver.take()
        && d.task_idx != usize::MAX
        && let Some(t) = state.orch_tasks.get_mut(d.task_idx) {
            t.done = true;
            t.active = false;
        }
    state.push_log("⏹ presupuesto de tokens agotado".to_string());
    // v0.7.3: si el presupuesto corta el turno, no se commitea.
    state.git_turn_interrupted = true;
    continue_after_worker(state)
}

/// Texto de un mensaje de resultado de tool (ambos formatos de provider).
fn tool_output_text(msg: &serde_json::Value) -> Option<String> {
    if let Some(s) = msg["content"].as_str() {
        return Some(s.to_string());
    }
    msg["content"]
        .as_array()?
        .iter()
        .find_map(|b| b["content"].as_str())
        .map(|s| s.to_string())
}

/// PLAN.md aprobable del modo Plan (v0.7.1): checklist, sin ejecutar nada.
fn render_plan_md(tasks: &[agent::WTask], user_text: &str) -> String {
    let pedido: String = user_text.replace('\n', " ").chars().take(200).collect();
    let mut out = format!("# PLAN\n\nPedido: {pedido}\n\n");
    for (i, t) in tasks.iter().enumerate() {
        let files = if t.files.is_empty() {
            String::new()
        } else {
            format!(" (archivos: {})", t.files.join(", "))
        };
        out.push_str(&format!("- [ ] {}. {}{}\n", i + 1, t.desc, files));
    }
    out.push_str("\n> Generado por ARQHIA en modo Plan (analista + 1 llamada; solo escribe CONTEXT/). Pulsa «Ejecutar plan» en el chat para pasarlo a Work.\n");
    out
}

/// Escribe `{ws}/CONTEXT/PLAN.md`. Devuelve ruta legible para el Log.
fn write_plan_md(state: &App, md: &str) -> Result<String, String> {
    let ws = state
        .o_ws
        .clone()
        .ok_or_else(|| "sin workspace".to_string())?;
    let dir = ws.join("CONTEXT");
    std::fs::create_dir_all(&dir).map_err(|e| format!("no se pudo crear CONTEXT/: {e}"))?;
    let file = dir.join("PLAN.md");
    std::fs::write(&file, md).map_err(|e| format!("no se pudo escribir: {e}"))?;
    Ok(file.to_string_lossy().to_string())
}
/// Decisión del bucle de estabilidad (v0.7.3). Pura para poder testear el
/// tope de ciclos sin red ni UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FixDecision {
    /// Auditor limpio: se cierra el turno (commit si git está verde).
    Clean,
    /// Hay issues: correr un worker de fixes y re-auditar.
    Fix,
    /// Tope de ciclos alcanzado con issues: cerrar sin commit.
    CapReached,
}

/// `max == 0` ⇒ ilimitado. Con issues y ciclo ya en el tope ⇒ `CapReached`.
pub(crate) fn fix_decision(has_issues: bool, cycle: usize, max: usize) -> FixDecision {
    if !has_issues {
        FixDecision::Clean
    } else if max != 0 && cycle >= max {
        FixDecision::CapReached
    } else {
        FixDecision::Fix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stability_loop_walks_until_green_or_cap() {
        // Sin issues: cierra.
        assert_eq!(fix_decision(false, 0, 0), FixDecision::Clean);
        assert_eq!(fix_decision(false, 3, 2), FixDecision::Clean);
        // Ilimitado (0): siempre hay otro ciclo, sin importar el contador.
        assert_eq!(fix_decision(true, 0, 0), FixDecision::Fix);
        assert_eq!(fix_decision(true, 5, 0), FixDecision::Fix);
        // Tope 2: ciclos 0 y 1 corren fix; al 2 se topa con issues.
        assert_eq!(fix_decision(true, 0, 2), FixDecision::Fix);
        assert_eq!(fix_decision(true, 1, 2), FixDecision::Fix);
        assert_eq!(fix_decision(true, 2, 2), FixDecision::CapReached);
        assert_eq!(fix_decision(true, 9, 2), FixDecision::CapReached);
    }
}
