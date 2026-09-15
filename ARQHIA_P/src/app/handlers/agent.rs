//! Handler de Orquestador: plan, workers, permisos, auditoría.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;
use iced::widget::markdown;

use crate::agent;
use crate::app::Message;
use crate::app::orchestrator::{
    OrchTask, PendingPlanner, abort_agent_placeholder, account_tokens, call_needs_approval,
    continue_after_worker, finish_agent_answer, finish_orchestrator, log_checklist, orch_phase,
    request_next_llm_step, resume_planner_with_net, spawn_exec_calls, spawn_planner, start_worker,
    start_worker_with_task,
};
use crate::app::state::App;
use crate::db;
use crate::llm::Role;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::AgentAnalyze(turn, res) => {
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            let (provider, cfg, ws) =
                match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
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
            orch_phase(state, 2, 5, "planner");
            let mode = state.o_mode;
            let context = crate::workspace::context_block(&ws);
            let espec = agent::read_espec_md(&ws);
            // v0.9: el planner consulta el STACK antes de diseñar tareas.
            // Con `use_stack` siempre; si el pedido lo nombra a mano, esa
            // mención vale como consentimiento puntual del turno.
            let explicit = crate::app::orchestrator::mentions_stack(&pedido);
            let allow = state.config.stack_consent.use_stack || explicit;
            let mut context = context;
            if allow {
                match crate::app::orchestrator::stack_consult_block(allow, &pedido) {
                    Some(block) => {
                        let n = block.lines().next().unwrap_or("STACK local").to_string();
                        state.push_log(format!(
                            "📚 {n}{}",
                            if explicit && !state.config.stack_consent.use_stack {
                                " (pedido explícito)"
                            } else {
                                ""
                            }
                        ));
                        context.push_str(&format!("\n\n{block}"));
                        context.push_str(
                            "\nSi algún snippet sirve, úsalo como base y cítalo en el plan.",
                        );
                    }
                    None => state.push_log("📚 STACK: sin coincidencias".to_string()),
                }
            }
            let label = match mode {
                db::Mode::Plan => "Plan",
                _ => "Work",
            }
            .to_string();
            // v0.9.1: planner Net+Read. Las URLs del pedido necesitan permiso
            // si `planner_net` está OFF o Net no las cubre; las rutas extra
            // se leen con el permiso Read. Sin Write/Bash por rol.
            let perms = state.config.permissions.clone();
            let need = agent::roles::planner_urls_needing_permission(
                &pedido,
                perms.planner_net,
                perms.auto_net,
                &perms.net_domains,
                false,
            );
            let extra_read = if perms.auto_read {
                agent::roles::extra_paths_block(&perms.extra_paths)
            } else {
                None
            };
            let pp = PendingPlanner {
                provider,
                cfg,
                pedido: pedido.clone(),
                context,
                espec,
                brief,
                label,
                mode,
                extra_read,
                urls: need.clone(),
                net_domains: perms.net_domains.clone(),
            };
            if !need.is_empty() {
                // El planner pide permiso Net antes de salir a la red: el
                // lote va al panel habitual (Aprobar = consulta y planifica).
                state.pending_planner = Some(pp);
                state.pending_calls = need
                    .iter()
                    .enumerate()
                    .map(|(i, u)| agent::PendingCall {
                        id: format!("planner-net-{i}"),
                        name: "fetch_url".to_string(),
                        args: serde_json::json!({"url": u}),
                    })
                    .collect();
                state.push_log("el planner pide permiso de Red (ver panel)".to_string());
                return Task::none();
            }
            // URLs cubiertas por permiso: se consultan directo (con aviso).
            let mut pp = pp;
            pp.urls = agent::extract_urls(&pedido);
            if !pp.urls.is_empty() {
                state.push_log(format!(
                    "planner: consultando {} doc(s) externo(s)…",
                    pp.urls.len()
                ));
            }
            spawn_planner(state, pp, false)
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
                    state.status =
                        format!("Error del planificador: {}", crate::llm::friendly_error(&e));
                    state.push_log(format!("❌ plan: {e}"));
                }
                Ok(tasks) => {
                    let max_tasks = state.config.limits.clamped().max_tasks;
                    let mut tasks = tasks;
                    if tasks.len() > max_tasks {
                        state.push_log(format!("🧭 plan recortado a {max_tasks} tareas (límite)"));
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
                    log_checklist(state);
                    orch_phase(state, 3, 5, "workers");
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
                    // v0.9 Track D: un auto-plan fallido no deja contexto colgado.
                    state.plan_auto = None;
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    state.driver = None;
                    state.status =
                        format!("Error del planificador: {}", crate::llm::friendly_error(&e));
                    state.push_log(format!("❌ plan: {e}"));
                }
                Ok(tasks) => {
                    // v0.9 Track D: turno automático del post-cuestionario.
                    // Escribe ROADMAP + VERSIONS + v0.1 + ToDo (pre-autorizado
                    // en este turno) y resume en el chat. Sin panel de plan.
                    if let Some(ctx) = state.plan_auto.take() {
                        let mut descs: Vec<String> = tasks.iter().map(|t| t.desc.clone()).collect();
                        let max = state.config.limits.clamped().max_tasks;
                        if descs.len() > max {
                            descs.truncate(max);
                        }
                        match crate::questionnaire::planning::write_auto_docs(&ctx, &descs) {
                            Ok(report) => {
                                for f in &report.files {
                                    state.push_log(format!("📋 {f}"));
                                }
                                for w in &report.v01_warnings {
                                    state.push_log(format!("⚠️ {w}"));
                                }
                                if !report.v01_valid {
                                    state.push_log(
                                        "⚠️ v0.1 sin plantilla: revísalo a mano.".to_string(),
                                    );
                                }
                                let gaps_md = if report.gaps.is_empty() {
                                    "Sin gaps pendientes.".to_string()
                                } else {
                                    format!("Gaps: {}.", report.gaps.join(", "))
                                };
                                finish_agent_answer(
                                    state,
                                    format!(
                                        "Contexto generado: {} (+ ToDo.md con {} tareas). {}",
                                        report.files.join(", "),
                                        descs.len(),
                                        gaps_md
                                    ),
                                );
                            }
                            Err(e) => {
                                abort_agent_placeholder(state);
                                state.status = format!("No se pudo escribir el contexto: {e}");
                                state.push_log(format!("❌ contexto auto: {e}"));
                            }
                        }
                        state.agent_running = false;
                        state.driver = None;
                        let _ = db::record_turn(&super::chat::project_key(state));
                        return Task::none();
                    }
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
                            log_checklist(state);
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
            state.history_push(
                crate::llm::ChatMsg {
                    role: Role::Assistant,
                    content: "orquestando...".to_string(),
                },
                markdown::parse("orquestando...").collect(),
                None,
                String::new(),
                0,
            );
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
            state.push_log(format!(
                "▶ ejecutando plan ({} tareas)",
                state.orch_tasks.len()
            ));
            log_checklist(state);
            orch_phase(state, 3, 5, "workers");
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
                    // v0.9 Track B: los 429 alimentan el panel de uso.
                    if crate::llm::is_rate_limit_error(&e) {
                        let _ = db::record_429(&super::chat::project_key(state));
                    }
                    return Task::none();
                }
                Ok(agent::StepOutcome::Final(answer)) => {
                    state.worker_answers.push(answer);
                    if drv.task_idx != usize::MAX
                        && let Some(t) = state.orch_tasks.get_mut(drv.task_idx)
                    {
                        t.done = true;
                        t.active = false;
                    }
                    state.driver = None;
                    return continue_after_worker(state);
                }
                Ok(agent::StepOutcome::Calls {
                    calls,
                    assistant_msg,
                }) => {
                    drv.raw.push(assistant_msg);
                    drv.step += 1;
                    let max_steps = drv.max_steps;
                    if drv.step > max_steps {
                        state.worker_answers.push(format!(
                            "Llegué al límite de {max_steps} pasos en esta subtarea (Límites en Config)."
                        ));
                        if drv.task_idx != usize::MAX
                            && let Some(t) = state.orch_tasks.get_mut(drv.task_idx)
                        {
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
                            && let Some(t) = state.orch_tasks.get_mut(drv.task_idx)
                        {
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
                        state.push_log(format!("⛔ {} denegada(s) (recordado)", auto_denied.len()));
                        drv.raw
                            .extend(agent::denial_msgs(drv.provider, &auto_denied));
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
                        let hit =
                            agent::read_cache_key(&c).and_then(|k| drv.read_cache.get(&k).cloned());
                        match hit {
                            Some(out) => {
                                let preview: String = c.args.to_string().chars().take(60).collect();
                                state.push_log(format!("📦 caché: {} {preview}", c.name));
                                drv.raw
                                    .push(agent::cached_result_msg(drv.provider, &c, &out));
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
                    && !text.starts_with('❌')
                {
                    drv.read_cache.insert(key, text);
                }
            }
            // v0.7.4 Fuentes: captura URLs de fetch_url para el bloque clicable.
            for c in calls.iter() {
                if c.name == "fetch_url"
                    && let Some(u) = c.args.get("url").and_then(|v| v.as_str())
                    && !u.trim().is_empty()
                    && !state.chat_sources.iter().any(|x| x == u.trim())
                {
                    state.chat_sources.push(u.trim().to_string());
                }
            }
            for line in logs {
                state.push_log(line);
            }
            drv.raw.extend(append);
            state.driver = Some(drv);
            // v0.9 Track B: tool calls al contador de uso del proyecto
            // (total + desglose por categoría).
            let ukey = super::chat::project_key(state);
            let _ = db::record_tool_calls(&ukey, calls.len());
            for c in calls.iter() {
                let label = match agent::tools::category_of_call(&c.name, &c.args) {
                    agent::tools::ToolCat::Read => "read",
                    agent::tools::ToolCat::Write => "write",
                    agent::tools::ToolCat::Bash => "bash",
                    agent::tools::ToolCat::Install => "install",
                    agent::tools::ToolCat::Net => "net",
                    agent::tools::ToolCat::Git => "git",
                    agent::tools::ToolCat::GitPush => "git-push",
                };
                let _ = db::record_tool_call_cat(&ukey, label);
            }
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
            // v0.9.1: el planner también pide Red por este panel.
            if state.pending_planner.is_some() {
                let n = state.pending_calls.len();
                state.pending_calls.clear();
                state.push_log(format!("planner: red aprobada ({n} doc(s))"));
                return resume_planner_with_net(state);
            }
            let calls = std::mem::take(&mut state.pending_calls);
            if calls.is_empty() || state.driver.is_none() {
                return Task::none();
            }
            state.push_log(format!("✅ permitidas {} acciones", calls.len()));
            spawn_exec_calls(state, calls, true)
        }
        Message::DenyTools => {
            // v0.9.1: planner sin docs externos (planifica igual, sin red).
            if let Some(mut pp) = state.pending_planner.take() {
                state.pending_calls.clear();
                state.push_log("planner: red denegada, planifico sin docs externos".to_string());
                pp.urls.clear();
                return spawn_planner(state, pp, false);
            }
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
            // v0.9.1: como Deny para el planner (el "recordar" es de workers).
            if let Some(mut pp) = state.pending_planner.take() {
                state.pending_calls.clear();
                state.push_log("planner: red denegada, planifico sin docs externos".to_string());
                pp.urls.clear();
                return spawn_planner(state, pp, false);
            }
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
            // v0.9 Track D: un plan automático interrumpido no escribe docs.
            state.plan_auto = None;
            // v0.9.1: un planner/re-análisis pendiente tampoco continúa.
            state.pending_planner = None;
            state.pending_fix = None;
            // Invalida todo lo que siga en vuelo de este turno (agente) y los
            // chunks del stream de chat plano.
            state.agent_gen += 1;
            state.agent_running = false;
            state.driver = None;
            state.pending_calls.clear();
            state.stream_gen += 1;
            state.streaming = false;
            // v0.9.5: corta de verdad la petición de red en vuelo.
            if let Ok(mut slot) = state.stream_abort.lock()
                && let Some(h) = slot.take()
            {
                h.abort();
            }
            // Conserva lo parcial si ya había texto; si no, marca detenido.
            if let Some(last) = state.messages.last_mut()
                && last.role == Role::Assistant
            {
                if last.content.trim().is_empty() {
                    last.content = "_Turno detenido por el usuario._".to_string();
                } else {
                    last.content.push_str("\n\n_(detenido por el usuario)_");
                }
            }
            state.reparse_last_md();
            if let Some(chat_id) = state.o_chat.or(state.active_chat)
                && let Some(last) = state.messages.last()
                && last.role == Role::Assistant
                && !last.content.trim().is_empty()
            {
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
                    use crate::app::orchestrator::{
                        count_temp_issues, reanalyze_prompt, run_fix_cycle,
                    };
                    if agent::temp_has_issues(&temp) {
                        let issues_n = count_temp_issues(&temp);
                        // v0.7.3: bucle sin tope salvo max_fix_cycles (0 = ilimitado).
                        let max = state.config.limits.clamped().max_fix_cycles;
                        if matches!(
                            run_fix_cycle(&temp, state.fix_cycle, max),
                            FixDecision::CapReached
                        ) {
                            state.push_log(format!(
                                "⏹ tope de ciclos ({max}) con issues: sin commit"
                            ));
                            return finish_orchestrator(state, String::new());
                        }
                        state.fix_cycle += 1;
                        state.push_log(format!("↻ ciclo {}: {} issues", state.fix_cycle, issues_n));
                        let preview: String = temp
                            .lines()
                            .filter(|l| l.starts_with('-') || l.starts_with('#'))
                            .take(4)
                            .collect::<Vec<_>>()
                            .join(" / ");
                        state.push_log(format!("📝 TEMP.md con issues: {}", preview));
                        // v0.9.2: el auditor sugiere skill de dominio (sin
                        // auto-ejecutar): QA si falló la puerta, review si no.
                        let skill = agent::roles::suggest_skill_for_issues(&temp);
                        state.push_log(format!(
                            "Prueba /skill {skill} (sugerencia; tú decides si ejecutarla)"
                        ));
                        // v0.9.1: re-análisis — el analista revisa el TEMP del
                        // auditor antes de lanzar el worker de fixes.
                        let (provider, cfg, ws2) =
                            match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
                                (Some(p), Some(c), Some(w)) => (p, c, w),
                                _ => return finish_orchestrator(state, String::new()),
                            };
                        state.pending_fix = Some(temp.clone());
                        orch_phase(
                            state,
                            2,
                            5,
                            &format!("re-análisis (ciclo {})", state.fix_cycle),
                        );
                        state.push_log("analista revisando TEMP…".to_string());
                        let pedido_orig = state
                            .o_history
                            .iter()
                            .rev()
                            .find(|m| m.role == Role::User)
                            .map(|m| m.content.clone())
                            .unwrap_or_default();
                        let cycle = state.fix_cycle;
                        let re_pedido = reanalyze_prompt(&pedido_orig, &temp, cycle);
                        let turn = state.agent_gen;
                        return Task::perform(
                            async move {
                                agent::analyze_workspace(provider, &cfg, &ws2, &re_pedido).await
                            },
                            move |res| Message::AgentReanalyze(turn, res),
                        );
                    }
                    state.push_log(format!("✅ estable tras {} ciclos", state.fix_cycle));
                    finish_orchestrator(state, String::new())
                }
            }
        }
        Message::AgentReanalyze(turn, res) => {
            // v0.9.1: el analista revisó el TEMP; se actualiza ANALYSIS.md y
            // se lanza el worker de fixes con el brief revisado.
            if turn != state.agent_gen {
                return Task::none(); // turno cancelado, resultado tardío
            }
            let ws = match state.o_ws.clone() {
                Some(w) => w,
                None => {
                    abort_agent_placeholder(state);
                    state.agent_running = false;
                    return Task::none();
                }
            };
            let Some(temp) = state.pending_fix.take() else {
                return Task::none(); // re-análisis tardío, ya se resolvió
            };
            let brief = match res {
                Ok(b) if !b.trim().is_empty() => b,
                _ => {
                    state.push_log("⚠️ re-análisis no disponible: uso TEMP crudo".to_string());
                    temp.clone()
                }
            };
            let dir = ws.join("CONTEXT");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("ANALYSIS.md"), &brief);
            state.push_log(format!(
                "re-análisis listo ({} chars, ciclo {})",
                brief.chars().count(),
                state.fix_cycle
            ));
            orch_phase(state, 3, 5, "workers");
            let fix_desc = crate::app::orchestrator::fix_task_desc(&temp, state.fix_cycle);
            state.orch_tasks.push(OrchTask {
                desc: format!("Fixes del auditor (ciclo {})", state.fix_cycle),
                files: Vec::new(),
                done: false,
                active: false,
            });
            log_checklist(state);
            let idx = state.orch_tasks.len() - 1;
            let task = agent::WTask {
                desc: fix_desc,
                files: Vec::new(),
                accept: "el auditor queda en VERDICT: CLEAN".to_string(),
            };
            start_worker_with_task(state, idx, task)
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
        && let Some(t) = state.orch_tasks.get_mut(d.task_idx)
    {
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

/// PLAN.md aprobable del modo Plan (v0.7.1 + criterios v0.9.1): checklist
/// con criterio de aceptación por tarea, sin ejecutar nada.
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
        if !t.accept.trim().is_empty() {
            out.push_str(&format!("  - Aceptación: {}\n", t.accept.trim()));
        }
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
/// Decisión del bucle de estabilidad (v0.7.3, vive en `agent::roles`
/// desde v0.9.1). Re-export para compatibilidad con tests existentes.
pub(crate) use crate::agent::roles::FixDecision;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::roles::fix_decision;

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

    /// v0.9.5: TEMP.md rotativo — el auditor sobrescribe, no anexa: tras
    /// dos auditorías solo queda la última (historia al journal, no al TEMP).
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn audit_temp_is_rotative_not_appended() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-temp_rotative");
        let mut app = App::default();
        app.config.git.enabled = false; // cierre sin commit en el test
        let ws = std::env::temp_dir().join("arqhIA-temp-rotative-ws");
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(ws.join("CONTEXT")).unwrap();
        app.o_ws = Some(ws.clone());
        app.o_history = vec![crate::llm::ChatMsg {
            role: crate::llm::Role::User,
            content: "haz algo".to_string(),
        }];
        app.worker_answers = vec!["listo".to_string()];
        let turn = app.agent_gen;
        let temp1 = "## Auditoría ARQHIA\n\nrevisión vieja\n\nVERDICT: CLEAN".to_string();
        let temp2 = "## Auditoría ARQHIA\n\nrevisión nueva\n\nVERDICT: CLEAN".to_string();
        let _ = handle(
            &mut app,
            Message::AgentAudit(turn, Ok((temp1, vec![], true))),
        );
        let _ = handle(
            &mut app,
            Message::AgentAudit(turn, Ok((temp2.clone(), vec![], true))),
        );
        let on_disk = std::fs::read_to_string(ws.join("CONTEXT").join("TEMP.md")).unwrap();
        assert_eq!(on_disk, temp2, "TEMP.md solo guarda la última auditoría");
        assert!(
            !on_disk.contains("revisión vieja"),
            "nada de historia anexada"
        );
        let _ = std::fs::remove_dir_all(&ws);
    }
}
