//! Pegamento del orquestador: driver paso a paso planner -> workers -> auditor.
//!
//! Coordina agent/, llm/ y workspace/ sin tocar widgets. Usado por update.

use iced::Task;

use super::events::Message;
use super::state::App;
use crate::agent;
use crate::config::Provider;
use crate::db;
use crate::config;
use crate::llm::{ChatMsg, Role};

/// Sesión de un worker del orquestador (v0.6 + v0.7.1): historial crudo
/// por provider, con presupuesto, caché de lecturas y parada temprana.
pub(crate) struct Driver {
    pub(crate) provider: Provider,
    pub(crate) cfg: config::ProviderConfig,
    pub(crate) ws: std::path::PathBuf,
    pub(crate) system: String,
    pub(crate) raw: Vec<serde_json::Value>,
    pub(crate) step: usize,
    /// Índice de la tarea en orch_tasks, o usize::MAX si es el fix pass
    pub(crate) task_idx: usize,
    /// Tope de pasos de este turno (Limits::max_iters, v0.7 Track B).
    pub(crate) max_steps: usize,
    /// Presupuesto de tokens del turno, 0 = ilimitado (v0.7.1).
    pub(crate) token_budget: u64,
    /// Tokens estimados consumidos (chars/4, aproximado).
    pub(crate) tokens_used: u64,
    /// Aviso del 80% ya mostrado (al llegar se compacta + avisa una vez).
    pub(crate) budget_warned: bool,
    /// Lecturas servidas en este turno (clave -> salida) para no releer.
    pub(crate) read_cache: std::collections::HashMap<String, String>,
    /// Firma del lote anterior (parada temprana si se repite).
    pub(crate) last_calls_sig: Option<String>,
    /// Steps seguidos sin cambios (calls idénticas).
    pub(crate) stale_steps: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct OrchTask {
    pub(crate) desc: String,
    pub(crate) files: Vec<String>,
    pub(crate) done: bool,
    pub(crate) active: bool,
}

/// Semilla del worker (v0.6 + v0.7.1): historial base con ventana +
/// ESPEC/AGENTS UNA vez como mensaje de contexto (no en cada step).
pub(crate) fn worker_seed(
    provider: Provider,
    base_history: &[ChatMsg],
    task: &agent::WTask,
    ws: &std::path::Path,
    history_limit: usize,
) -> (String, Vec<serde_json::Value>) {
    let agents_md = agent::read_agents_md(ws);
    let system = agent::worker_system(ws, task);
    let task_msg = serde_json::json!({"role": "user", "content": format!("TAREA: {}", task.desc)});
    // Ventana: últimos N mensajes de la conversación; lo viejo colapsa.
    let (window, cut) = agent::window_history(base_history, history_limit.max(1));
    let mut raw = match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => {
            agent::history_to_openai(&window)
        }
        Provider::Anthropic => {
            let r = agent::history_to_anthropic(&window);
            if r.is_empty() {
                vec![task_msg.clone()]
            } else {
                r
            }
        }
    };
    // Contexto del proyecto UNA vez, delante de todo (v0.7.1 adelgazado).
    if let Some(ctx) = agent::worker_context_block(ws, agents_md.as_deref()) {
        let mut prefix = String::from("Contexto del proyecto (solo lectura, no repetir en cada paso):\n");
        if cut > 0 {
            prefix.push_str(&format!("[{cut} mensajes previos omitidos por ventana de historial]\n"));
        }
        prefix.push_str(&ctx);
        raw.insert(0, serde_json::json!({"role": "user", "content": prefix}));
    } else if cut > 0 {
        raw.insert(
            0,
            serde_json::json!({"role": "user", "content": format!("[{cut} mensajes previos omitidos por ventana de historial]")}),
        );
    }
    // Anthropic exige alternancia válida: el seed ya la cumple si vino del historial;
    // la tarea se anexa como user (si el último ya es user, se fusiona).
    let last_is_user = raw.last().map(|m| m["role"] == "user").unwrap_or(false);
    if provider == Provider::Anthropic && last_is_user {
        match raw.pop() {
            Some(prev) => {
                let merged = format!(
                    "{}\n\nTAREA: {}",
                    prev["content"].as_str().unwrap_or(""),
                    task.desc
                );
                raw.push(serde_json::json!({"role": "user", "content": merged}));
            }
            // Imposible por el check anterior, pero sin unwrap: anexa normal.
            None => raw.push(task_msg),
        }
    } else {
        raw.push(task_msg);
    }
    (system, raw)
}

/// ¿Una llamada concreta necesita aprobación? (v0.7 Track B: Install y
/// Net con dominios se deciden por args, no solo por nombre; v0.7.2: git
/// según autonomía/push).
pub(crate) fn call_needs_approval(state: &App, call: &agent::PendingCall) -> bool {
    let perms = &state.config.permissions;
    match agent::tools::category_of_call(&call.name, &call.args) {
        agent::tools::ToolCat::Read => !perms.auto_read,
        agent::tools::ToolCat::Write => !perms.auto_write,
        agent::tools::ToolCat::Bash => !perms.auto_bash,
        agent::tools::ToolCat::Install => !perms.auto_install,
        agent::tools::ToolCat::Net => {
            if !perms.auto_net {
                return true;
            }
            // auto_net solo cubre dominios listados explícitamente.
            let url = call.args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            !agent::tools::url_domain_listed(url, &perms.net_domains)
        }
        agent::tools::ToolCat::Git => match agent::tools::git_call_kind(&call.args) {
            Some(agent::tools::GitKind::Read) => !perms.auto_read,
            Some(agent::tools::GitKind::Write) => {
                state.config.git.autonomy < crate::config::GitAutonomy::CommitLocal
            }
            // Bloqueado (lo rechaza el executor): siempre al panel.
            _ => true,
        },
        agent::tools::ToolCat::GitPush => !state.config.git.auto_push(),
    }
}

/// Coloca la respuesta final del agente como último mensaje + DB + markdown.
/// Registra uso estimado de tokens (input/output/coste) para ese mensaje.
pub(crate) fn finish_agent_answer(state: &mut App, answer: String) {
    // v0.7.4: si hay fuentes fetch_url, se anexan como bloque clicable.
    // Además se rescatan URLs citadas en el texto (fallback extract_urls).
    for u in agent::extract_urls(&answer) {
        if !state.chat_sources.iter().any(|x| x == &u) && state.chat_sources.len() < 10 {
            state.chat_sources.push(u);
        }
    }
    let mut final_answer = answer.clone();
    if !state.chat_sources.is_empty() {
        let mut block = String::from("\n\nFuentes:");
        for (i, u) in state.chat_sources.iter().enumerate() {
            block.push_str(&format!("\n[{}] {u}", i + 1));
        }
        final_answer.push_str(&block);
    }
    if let Some(last) = state.messages.last_mut()
        && last.role == Role::Assistant {
            last.content = final_answer.clone();
        }
    state.reparse_last_md();
    // Uso estimado del turno de agente (sin `usage` real: es no-streaming).
    let input_est: u32 = state
        .messages
        .iter()
        .map(|m| crate::llm::estimate_tokens_text(&m.content))
        .sum();
    let output_est = crate::llm::estimate_tokens_text(&answer);
    let usage = crate::llm::Usage { input: input_est, output: output_est, cached: 0, cost: None };
    let n = state.messages.len();
    state.msg_usage.resize(n, None);
    if n > 0 && state.messages[n - 1].role == Role::Assistant {
        state.msg_usage[n - 1] = Some(usage);
    }
    // El contexto del turno incluye system + historial + salidas de tools.
    state.context_tokens = state.context_tokens.max(input_est as u64);
    let model = state.config.active_config().model;
    let pid = crate::pricing::provider_id_for(
        state.config.active,
        &state.config.active_config().base_url,
    );
    let cost = state
        .pricing
        .cost_in(pid.as_deref(), &model, usage)
        .or_else(|| crate::llm::estimate_cost_usd(state.config.active, &model, usage));
    state.session_in += input_est as u64;
    state.session_out += output_est as u64;
    if let Some(c) = cost {
        state.session_cost += c;
    }
    state.push_log(format!(
        "tokens in {} · out {} · coste {}",
        input_est,
        output_est,
        crate::llm::format_cost(cost)
    ));
    if let Some(chat_id) = state.active_chat
        && let Some(last) = state.messages.last()
            && !last.content.trim().is_empty() {
                let _ = db::save_msg(chat_id, "assistant", &last.content);
                state.resync_msg_meta(chat_id);
            }
}

/// Quita el placeholder del turno si abortó ("orquestando..." o
/// "planificando…" de modo Plan). Pertenece al chat donde ocurrió.
pub(crate) fn abort_agent_placeholder(state: &mut App) {
    if let Some(last) = state.messages.last()
        && last.role == Role::Assistant
        && (last.content == "orquestando..." || last.content == "planificando...") {
            state.pop_last_message();
        }
}

/// Lanza el worker `idx` de orch_tasks (o el fix si idx == len-1 y es fix).
/// Precondición: o_provider/o_cfg/o_ws/o_chat/o_history ya están seteados.
pub(crate) fn start_worker(state: &mut App, idx: usize) -> Task<Message> {
    let (desc, files) = match state.orch_tasks.get(idx) {
        Some(t) => (t.desc.clone(), t.files.clone()),
        None => return Task::none(),
    };
    start_worker_with_task(
        state,
        idx,
        agent::WTask {
            desc,
            files,
        },
    )
}

pub(crate) fn start_worker_with_task(state: &mut App, idx: usize, task: agent::WTask) -> Task<Message> {
    let (provider, cfg, ws) = match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
        (Some(p), Some(c), Some(w)) => (p, c, w),
        _ => return Task::none(),
    };
    if let Some(t) = state.orch_tasks.get_mut(idx) {
        t.active = true;
    }
    let limits = state.config.limits.clamped();
    let (system_base, raw) = worker_seed(provider, &state.o_history, &task, &ws, limits.history_limit);
    // El system declara modo y capacidades (v0.7.1 visibilidad obligatoria).
    let system = format!(
        "{system_base}\n\nModo actual: Work (orquestador con permisos y límites; system adelgazado: el contexto del proyecto ya viajó como mensaje)."
    );
    state.push_log(format!("🔨 worker{}: {}", idx + 1, short_task(&task.desc)));
    let max_steps = limits.max_iters;
    state.driver = Some(Driver {
        provider,
        cfg: cfg.clone(),
        ws: ws.clone(),
        system,
        raw,
        step: 0,
        task_idx: idx,
        max_steps,
        token_budget: limits.max_tokens_turn,
        tokens_used: 0,
        budget_warned: false,
        read_cache: std::collections::HashMap::new(),
        last_calls_sig: None,
        stale_steps: 0,
    });
    request_next_llm_step(state)
}

pub(crate) fn short_task(desc: &str) -> String {
    let s: String = desc.replace('\n', " ");
    crate::ui::design::trunc_end(&s, 90)
}

/// Pide UN paso LLM al driver actual (el resultado vuelve como AgentLlm).
pub(crate) fn request_next_llm_step(state: &mut App) -> Task<Message> {
    let (provider, cfg, system, raw, turn) = match state.driver.as_ref() {
        Some(d) => (d.provider, d.cfg.clone(), d.system.clone(), d.raw.clone(), state.agent_gen),
        None => return Task::none(),
    };
    Task::perform(
        async move { agent::llm_step(provider, &cfg, &system, &raw).await },
        move |res| Message::AgentLlm(turn, res),
    )
}

/// Ejecuta llamadas aprobadas (o auto) y devuelve AgentExecDone.
pub(crate) fn spawn_exec_calls(
    state: &mut App,
    calls: Vec<agent::PendingCall>,
    batch_approved: bool,
) -> Task<Message> {
    let (provider, ws, step, turn) = match state.driver.as_ref() {
        Some(d) => (d.provider, d.ws.clone(), d.step, state.agent_gen),
        None => return Task::none(),
    };
    let limits = state.config.limits.clamped();
    let perms = &state.config.permissions;
    let extra = perms.extra_paths.clone();
    let ignores = state.config.effective_ignores();
    let policy = agent::tools::policy_for(
        perms.auto_install,
        batch_approved,
        limits.bash_timeout_s,
        limits.max_read_kb as usize * 1024,
        &perms.net_domains,
        &ignores,
        &state.config.git,
    );
    // Los calls viajan de vuelta DENTRO del futuro (el mapper es Fn y no
    // puede mover capturas): así AgentExecDone puede poblar la caché.
    Task::perform(
        async move {
            let (append, logs) =
                agent::exec_calls(provider, &ws, &extra, step, &calls, &policy).await;
            (calls, append, logs)
        },
        move |(calls, append, logs)| Message::AgentExecDone(turn, calls, append, logs),
    )
}

/// Tras terminar un worker: siguiente worker o auditoría.
pub(crate) fn continue_after_worker(state: &mut App) -> Task<Message> {
    let next = state
        .orch_tasks
        .iter()
        .position(|t| !t.done && !t.active);
    match next {
        Some(idx) => start_worker(state, idx),
        None => {
            // Todos listos -> auditor
            let (provider, cfg, ws) = match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
                (Some(p), Some(c), Some(w)) => (p, c, w),
                _ => {
                    return finish_orchestrator(state, " (sin auditor: faltan datos)".to_string());
                }
            };
            let turn = state.agent_gen;
            let cycle = state.fix_cycle;
            Task::perform(
                async move { agent::audit_workspace(provider, &cfg, &ws, cycle).await },
                move |res| Message::AgentAudit(turn, res),
            )
        }
    }
}

/// Contabiliza tokens del turno (v0.7.1) y refresca el badge del Log.
/// Estimación absoluta `system + raw` (chars/4, aproximada ±30%).
/// Al 80% del presupuesto: compacta outputs viejos y avisa una sola vez.
/// Devuelve true si se agotó el presupuesto (el llamador debe parar).
pub(crate) fn account_tokens(state: &mut App) -> bool {
    let (used, budget, warned) = match state.driver.as_ref() {
        Some(d) => {
            let used = agent::estimate_tokens(&d.system) + agent::estimate_raw_tokens(&d.raw);
            (used, d.token_budget, d.budget_warned)
        }
        None => return false,
    };
    if let Some(d) = state.driver.as_mut() {
        d.tokens_used = used;
    }
    // Contexto real del turno incluyendo tools/lecturas (system + raw).
    state.context_tokens = state.context_tokens.max(used);
    refresh_token_badge(state);
    if budget == 0 {
        return false;
    }
    if used >= budget {
        return true;
    }
    if !warned && used * 5 >= budget * 4 {
        if let Some(d) = state.driver.as_mut() {
            agent::collapse_old_tool_outputs(&mut d.raw, 2);
            d.budget_warned = true;
        }
        state.push_log(format!(
            "🪙 al 80% del presupuesto ({} de {}): contexto viejo compactado",
            agent::format_tokens(used),
            agent::format_tokens(budget),
        ));
        refresh_token_badge(state);
    }
    false
}

/// Badge discreto en el Log (v0.7.1): una sola línea `🪙 …` que se
/// reescribe, no una por step. Sin panel.
pub(crate) fn refresh_token_badge(state: &mut App) {
    let (used, budget) = match state.driver.as_ref() {
        Some(d) => (d.tokens_used, d.token_budget),
        None => return,
    };
    let line = if budget > 0 {
        format!(
            "🪙 {} tokens este turno (límite {})",
            agent::format_tokens(used),
            agent::format_tokens(budget),
        )
    } else {
        format!("🪙 {} tokens este turno", agent::format_tokens(used))
    };
    if state.tool_logs.last().map(|l| l.starts_with("🪙")).unwrap_or(false) {
        let last = state.tool_logs.len() - 1;
        state.tool_logs[last] = line;
    } else {
        state.push_log(line);
    }
}

/// Respuesta final del turno: resumen de workers + veredicto.
pub(crate) fn finish_orchestrator(state: &mut App, extra: String) -> Task<Message> {
    let mut answer = if state.worker_answers.is_empty() {
        "Terminé sin producir salidas.".to_string()
    } else if state.worker_answers.len() == 1 {
        state.worker_answers[0].clone()
    } else {
        state
            .worker_answers
            .iter()
            .enumerate()
            .map(|(i, a)| format!("**Tarea {}:** {}", i + 1, a))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    answer.push_str(&extra);
    finish_agent_answer(state, answer);
    state.agent_running = false;
    state.driver = None;
    state.pending_calls.clear();
    state.status.clear();
    close_git_turn(state)
}

/// Prepara git al arrancar un turno Work (v0.7.2): asegura la rama de
/// trabajo y recuerda si el árbol venía limpio (guarda anti-sucio).
pub(crate) fn prepare_git_turn(state: &mut App) {
    state.git_verify_ok = false;
    state.git_turn_interrupted = false;
    state.git_clean_before = true;
    let git = state.config.git.clone();
    let Some(ws) = state.o_ws.clone() else { return; };
    if !git.enabled {
        return;
    }
    match crate::git::ensure_work_branch(&ws, &git) {
        Ok(()) => {
            let branch = crate::git::current_branch(&ws).unwrap_or_else(|| git.work_branch.clone());
            state.git_clean_before = crate::git::is_clean(&ws);
            state.push_log(format!("🌿 git: rama {branch}"));
        }
        Err(e) => state.push_log(format!("⚠️ git: {e}")),
    }
}

/// Cierra el turno con git (v0.7.2): auto-commit solo si la verificación
/// pasó y el árbol venía limpio; push solo si `CommitAndPush + push_enabled`.
fn close_git_turn(state: &mut App) -> Task<Message> {
    let git = state.config.git.clone();
    if !git.enabled || matches!(git.autonomy, crate::config::GitAutonomy::ReadOnly) {
        return Task::none();
    }
    let Some(ws) = state.o_ws.clone() else { return Task::none(); };
    if !crate::git::is_repo(&ws) {
        return Task::none();
    }
    if state.git_turn_interrupted {
        state.push_log("⏹ sin commit: el turno se cortó (presupuesto/parada)".to_string());
        return Task::none();
    }
    if !state.git_verify_ok {
        state.push_log("⏹ sin commit: la verificación (check/test/clippy) no pasó".to_string());
        return Task::none();
    }
    if !state.git_clean_before {
        state.push_log("⏹ sin commit: el árbol ya venía sucio antes del turno".to_string());
        return Task::none();
    }
    let msg = format!("ARQHIA: {}", last_user_summary(state));
    match crate::git::commit_all(&ws, &msg, git.author(), state.git_clean_before) {
        Ok(Some(sha)) => state.push_log(format!("🌿 commit {sha}: {msg}")),
        Ok(None) => {}
        Err(e) => state.push_log(format!("⚠️ commit falló: {e}")),
    }
    if git.auto_push() {
        let remote = git.remote.clone();
        let branch = git.push_target().to_string();
        state.push_log(format!("⬆ push {remote}/{branch}…"));
        return Task::perform(
            async move { crate::git::push(&ws, &remote, &branch).await },
            Message::GitPushDone,
        );
    }
    Task::none()
}

/// Resumen corto del último pedido del usuario para el mensaje de commit.
fn last_user_summary(state: &App) -> String {
    let text = state
        .o_history
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.content.clone())
        .unwrap_or_else(|| "turno de trabajo".to_string());
    crate::ui::design::trunc_end(&text.replace('\n', " "), 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMsg, Role};

    fn tmp_ws(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("arqhia-seed-test-{name}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(p.join("CONTEXT")).unwrap();
        p
    }

    fn task(desc: &str) -> agent::WTask {
        agent::WTask { desc: desc.to_string(), files: Vec::new() }
    }

    fn history(n: usize) -> Vec<ChatMsg> {
        (0..n)
            .map(|i| ChatMsg { role: if i % 2 == 0 { Role::User } else { Role::Assistant }, content: format!("m{i}") })
            .collect()
    }

    #[test]
    fn seed_windows_history_and_context_once() {
        let ws = tmp_ws("window");
        std::fs::write(ws.join("CONTEXT").join("ESPEC.md"), "# Demo").unwrap();
        // Historial largo (30) con ventana 20: 20 mensajes + contexto + tarea.
        let (system, raw) = worker_seed(Provider::OpenAI, &history(30), &task("haz X"), &ws, 20);
        assert!(system.contains("haz X"));
        assert!(!system.contains("Demo"), "ESPEC no va en el system (va una vez como mensaje)");
        let first = raw[0]["content"].as_str().unwrap_or("");
        assert!(first.contains("Demo"), "contexto primero: {first}");
        assert!(first.contains("10 mensajes previos omitidos"), "{first}");
        // Sin ESPEC ni recorte: sin marcador.
        let ws2 = tmp_ws("window-clean");
        let (sys2, raw2) = worker_seed(Provider::OpenAI, &history(5), &task("haz Y"), &ws2, 20);
        assert!(sys2.contains("haz Y"));
        assert!(raw2.iter().all(|m| !m.to_string().contains("omitidos")));
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_dir_all(&ws2);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn token_badge_rewrites_single_line() {
        let mut app = App::default();
        app.driver = Some(Driver {
            provider: Provider::OpenAI,
            cfg: crate::config::ProviderConfig::new(Provider::OpenAI),
            ws: std::env::temp_dir(),
            system: "sys".to_string(),
            raw: vec![serde_json::json!({"role": "user", "content": "hola"})],
            step: 0,
            task_idx: 0,
            max_steps: 10,
            token_budget: 0,
            tokens_used: 0,
            budget_warned: false,
            read_cache: Default::default(),
            last_calls_sig: None,
            stale_steps: 0,
        });
        assert!(!account_tokens(&mut app), "sin presupuesto nunca para");
        let badges = app.tool_logs.iter().filter(|l| l.starts_with("🪙")).count();
        assert_eq!(badges, 1, "una sola línea de badge");
        assert!(!account_tokens(&mut app));
        let badges2 = app.tool_logs.iter().filter(|l| l.starts_with("🪙")).count();
        assert_eq!(badges2, 1, "el badge se reescribe, no se duplica");
        // Con presupuesto agotado sí para.
        if let Some(d) = app.driver.as_mut() {
            d.token_budget = 1;
        }
        assert!(account_tokens(&mut app));
    }

    fn commit_count(ws: &std::path::Path) -> usize {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(ws)
            .args(["rev-list", "--count", "HEAD"])
            .output()
            .expect("git rev-list");
        String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
    }

    /// v0.7.3: el auto-commit del cierre solo ocurre con el turno verde.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn commit_only_when_turn_is_green() {
        let ws = std::env::temp_dir().join("arqhia-orch-commit-test");
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        let git = crate::config::GitConfig {
            author_name: "Test".to_string(),
            author_email: "t@test.dev".to_string(),
            ..crate::config::GitConfig::default()
        };
        crate::git::ensure_work_branch(&ws, &git).unwrap();
        std::fs::write(ws.join("a.txt"), "1").unwrap();
        crate::git::commit_all(&ws, "init", git.author(), true).unwrap();
        let before = commit_count(&ws);

        // Turno ROJO: hay cambios pero la verificación no pasó -> sin commit.
        let mut red = App::default();
        red.active_chat = None;
        red.config.git = git.clone();
        red.o_ws = Some(ws.clone());
        red.o_history = vec![ChatMsg { role: Role::User, content: "haz algo".to_string() }];
        red.worker_answers = vec!["listo".to_string()];
        red.git_clean_before = true;
        red.git_verify_ok = false;
        std::fs::write(ws.join("a.txt"), "cambio").unwrap();
        let _ = finish_orchestrator(&mut red, String::new());
        assert_eq!(commit_count(&ws), before, "en rojo no debe commitear");

        // Turno VERDE: mismo cambio pendiente -> commit.
        let mut green = App::default();
        green.active_chat = None;
        green.config.git = git.clone();
        green.o_ws = Some(ws.clone());
        green.o_history = vec![ChatMsg { role: Role::User, content: "haz algo".to_string() }];
        green.worker_answers = vec!["listo".to_string()];
        green.git_clean_before = true;
        green.git_verify_ok = true;
        let _ = finish_orchestrator(&mut green, String::new());
        assert_eq!(commit_count(&ws), before + 1, "en verde debe commitear");
        let _ = std::fs::remove_dir_all(&ws);
    }
}
