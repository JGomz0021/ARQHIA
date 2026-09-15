//! Pegamento del orquestador: driver paso a paso planner -> workers -> auditor.
//!
//! Coordina agent/, llm/ y workspace/ sin tocar widgets. Usado por update.

use iced::Task;

use super::events::Message;
use super::state::App;
use crate::agent;
use crate::config;
use crate::config::Provider;
use crate::db;
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

/// Planner en espera de permiso Net (v0.9.1): guarda todo lo necesario para
/// reanudar `plan_tasks` cuando el usuario apruebe o deniegue la red.
#[derive(Debug, Clone)]
pub(crate) struct PendingPlanner {
    pub(crate) provider: Provider,
    pub(crate) cfg: config::ProviderConfig,
    pub(crate) pedido: String,
    pub(crate) context: String,
    pub(crate) espec: Option<String>,
    pub(crate) brief: String,
    pub(crate) label: String,
    pub(crate) mode: db::Mode,
    /// Contexto Read ya resuelto (rutas extra); la red se aporta al reanudar.
    pub(crate) extra_read: Option<String>,
    /// URLs a consultar (las cubiertas por permiso o las aprobadas en panel).
    pub(crate) urls: Vec<String>,
    /// Dominios listados para el fetch (el gate ya se decidió antes).
    pub(crate) net_domains: Vec<String>,
}

/// Fase visible del orquestador BETA (v0.9.1): `orquestador: fase X/N — nombre`.
pub(crate) fn orch_phase(state: &mut App, phase: usize, total: usize, name: &str) {
    let who = agent::roles::Role::Orquestador.label().to_lowercase();
    state.push_log(format!("{who}: fase {phase}/{total} — {name}"));
}

/// Checklist compacta del plan en el Log (v0.9.1): una línea
/// `checklist: ☑ hecha · ▶ activa · ☐ pendiente`.
pub(crate) fn log_checklist(state: &mut App) {
    let rows: Vec<(String, bool, bool)> = state
        .orch_tasks
        .iter()
        .map(|t| (t.desc.clone(), t.done, t.active))
        .collect();
    if !rows.is_empty() {
        state.push_log(agent::roles::checklist_line(&rows));
    }
}

/// Ejecuta el planner completo (v0.9.1): fetch Net de `urls` (tope 8 KB por
/// URL) + `plan_tasks` con brief y contexto extra. `net_approved` viene del
/// panel de permisos; sin él solo salen dominios listados (el gate ya filtró).
async fn run_planner(
    pp: PendingPlanner,
    net_approved: bool,
) -> (db::Mode, Result<Vec<agent::WTask>, String>) {
    let policy = agent::tools::ExecPolicy {
        net_domains: pp.net_domains.clone(),
        net_approved,
        ..agent::tools::ExecPolicy::default()
    };
    let net = if pp.urls.is_empty() {
        None
    } else {
        agent::fetch_planner_net_block(&pp.urls, &policy).await
    };
    let mut extra = pp.extra_read.clone().unwrap_or_default();
    if let Some(n) = net {
        if !extra.is_empty() {
            extra.push_str("\n\n");
        }
        extra.push_str(&n);
    }
    let res = agent::plan_tasks(
        pp.provider,
        &pp.pedido,
        &pp.context,
        pp.espec.as_deref(),
        Some(&pp.brief),
        &pp.cfg,
        &pp.label,
        if extra.is_empty() {
            None
        } else {
            Some(extra.as_str())
        },
    )
    .await;
    (pp.mode, res)
}

/// Lanza `plan_tasks` con el contexto ya resuelto (v0.9.1: brief + Net+Read).
/// El mensaje de vuelta respeta el modo (Plan → PlanDone, Work → AgentPlan).
pub(crate) fn spawn_planner(
    state: &mut App,
    pp: PendingPlanner,
    net_approved: bool,
) -> Task<Message> {
    let turn = state.agent_gen;
    Task::perform(
        async move { run_planner(pp, net_approved).await },
        move |(mode, res)| match mode {
            db::Mode::Plan => Message::PlanDone(turn, res),
            _ => Message::AgentPlan(turn, res),
        },
    )
}

/// Reanuda un planner pendiente tras aprobar la red (v0.9.1): descarga las
/// URLs con el lote aprobado y luego planifica.
pub(crate) fn resume_planner_with_net(state: &mut App) -> Task<Message> {
    let Some(pp) = state.pending_planner.take() else {
        return Task::none();
    };
    spawn_planner(state, pp, true)
}

/// Semilla del worker (v0.6 + v0.7.1): historial base con ventana +
/// SPECS/AGENTS UNA vez como mensaje de contexto (no en cada step).
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
        let mut prefix =
            String::from("Contexto del proyecto (solo lectura, no repetir en cada paso):\n");
        if cut > 0 {
            prefix.push_str(&format!(
                "[{cut} mensajes previos omitidos por ventana de historial]\n"
            ));
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
    // MCP v0.9.3: decide por servidor (Install/Read/Net) primero
    if crate::mcp::is_mcp_tool(&call.name) {
        let cat = agent::tools::category_of_mcp_call(&call.name, &state.config.mcp.servers);
        return match cat {
            agent::tools::ToolCat::Read => false,
            agent::tools::ToolCat::Install => !state.config.permissions.auto_install,
            _ => true,
        };
    }
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
        && last.role == Role::Assistant
    {
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
    let usage = crate::llm::Usage {
        input: input_est,
        output: output_est,
        cached: 0,
        cost: None,
    };
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
    // Local nunca cuesta; no caer al catálogo global (coste falso).
    let cost = if state.config.active == Provider::Local {
        Some(0.0)
    } else {
        state
            .pricing
            .cost_in(pid.as_deref(), &model, usage)
            .or_else(|| crate::llm::estimate_cost_usd(state.config.active, &model, usage))
    };
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
    if let Some(chat_id) = state.o_chat.or(state.active_chat)
        && let Some(last) = state.messages.last()
        && !last.content.trim().is_empty()
    {
        let _ = db::save_msg(chat_id, "assistant", &last.content);
        state.resync_msg_meta(chat_id);
    }
}

/// Quita el placeholder del turno si abortó ("orquestando..." o
/// "planificando…" de modo Plan). Pertenece al chat donde ocurrió.
pub(crate) fn abort_agent_placeholder(state: &mut App) {
    if let Some(last) = state.messages.last()
        && last.role == Role::Assistant
        && (last.content == "orquestando..." || last.content == "planificando...")
    {
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
            accept: String::new(),
        },
    )
}

pub(crate) fn start_worker_with_task(
    state: &mut App,
    idx: usize,
    task: agent::WTask,
) -> Task<Message> {
    let (provider, cfg, ws) = match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
        (Some(p), Some(c), Some(w)) => (p, c, w),
        _ => return Task::none(),
    };
    if let Some(t) = state.orch_tasks.get_mut(idx) {
        t.active = true;
    }
    let limits = state.config.limits.clamped();
    let (system_base, raw) =
        worker_seed(provider, &state.o_history, &task, &ws, limits.history_limit);
    // El system declara modo y capacidades (v0.7.1 visibilidad obligatoria).
    // v0.9 Track B: sin `share_local`, el agente nunca propone guardar.
    let share_note = share_note(state.config.stack_consent.share_local);
    let system = format!(
        "{system_base}\n\nModo actual: Work (orquestador con permisos y límites; system adelgazado: el contexto del proyecto ya viajó como mensaje).{share_note}"
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
        Some(d) => (
            d.provider,
            d.cfg.clone(),
            d.system.clone(),
            d.raw.clone(),
            state.agent_gen,
        ),
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
    let policy = agent::tools::policy_for_with_mcp(
        perms.auto_install,
        batch_approved,
        limits.bash_timeout_s,
        limits.max_read_kb as usize * 1024,
        &perms.net_domains,
        &ignores,
        &state.config.git,
        &state.config.mcp.servers,
    );
    // Los calls viajan de vuelta DENTRO del futuro (el mapper es Fn y no
    // puede mover capturas): así AgentExecDone puede poblar la caché.
    Task::perform(
        async move {
            let (append, logs) = agent::exec_calls(
                provider,
                &ws,
                &extra,
                step,
                &calls,
                &policy,
                agent::roles::Role::Worker,
            )
            .await;
            (calls, append, logs)
        },
        move |(calls, append, logs)| Message::AgentExecDone(turn, calls, append, logs),
    )
}

/// Tras terminar un worker: siguiente worker o auditoría.
pub(crate) fn continue_after_worker(state: &mut App) -> Task<Message> {
    let next = state.orch_tasks.iter().position(|t| !t.done && !t.active);
    match next {
        Some(idx) => start_worker(state, idx),
        None => {
            // Todos listos -> auditor
            let (provider, cfg, ws) =
                match (state.o_provider, state.o_cfg.clone(), state.o_ws.clone()) {
                    (Some(p), Some(c), Some(w)) => (p, c, w),
                    _ => {
                        return finish_orchestrator(
                            state,
                            " (sin auditor: faltan datos)".to_string(),
                        );
                    }
                };
            orch_phase(state, 4, 5, "auditor");
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
    if state
        .tool_logs
        .last()
        .map(|l| l.starts_with("🪙"))
        .unwrap_or(false)
    {
        let last = state.tool_logs.len() - 1;
        state.tool_logs[last] = line;
    } else {
        state.push_log(line);
    }
}

/// true si el pedido nombra el STACK: es consentimiento puntual para ese
/// turno aunque `use_stack` esté desactivado (el usuario lo pide a mano).
pub(crate) fn mentions_stack(pedido: &str) -> bool {
    pedido.to_lowercase().contains("stack")
}

/// Consulta del planner al STACK (v0.9 Track A): `None` sin consentimiento
/// `use_stack` o sin coincidencias (el turno sigue igual, sin Log de hits).
/// Con hits devuelve el bloque a inyectar al prompt del planner.
pub(crate) fn stack_consult_block(use_stack: bool, pedido: &str) -> Option<String> {
    if !use_stack {
        return None;
    }
    crate::stack::consult(pedido)
}

/// Nota de consentimiento del worker (v0.9 Track B): sin `share_local`,
/// el agente nunca propone guardar snippets.
pub(crate) fn share_note(share_local: bool) -> &'static str {
    if share_local {
        ""
    } else {
        "\nNota de consentimiento: el usuario NO permite guardar en el STACK local; no propongas guardar snippets."
    }
}

/// Decisión pura del bucle auditor → analista → fix (v0.9.5).
/// `temp`: contenido de TEMP.md del auditor; `cycle`: ciclo actual;
/// `max`: tope de `Limits.max_fix_cycles` (0 = ilimitado).
/// El handler solo despacha `Message`; la lógica vive aquí y es testeable.
pub(crate) fn run_fix_cycle(
    temp: &str,
    cycle: usize,
    max: usize,
) -> crate::agent::roles::FixDecision {
    use crate::agent::{roles::fix_decision, temp_has_issues};
    // v0.9.5: tope duro de seguridad cuando la config lo deja "ilimitado"
    // (0): evita un bucle infinito quemando tokens si el auditor nunca queda
    // verde. Alineado con el máximo configurable (20).
    const SAFETY_MAX_FIX_CYCLES: usize = 20;
    let effective = if max == 0 { SAFETY_MAX_FIX_CYCLES } else { max };
    fix_decision(temp_has_issues(temp), cycle, effective)
}

/// Issues contados en un TEMP (líneas `- ...`): puro, para el Log.
pub(crate) fn count_temp_issues(temp: &str) -> usize {
    temp.lines()
        .filter(|l| l.trim_start().starts_with('-'))
        .count()
}

/// Prompt de re-análisis del analista ante ISSUES (v0.9.5): puro.
pub(crate) fn reanalyze_prompt(pedido_orig: &str, temp: &str, cycle: usize) -> String {
    format!(
        "Re-analiza antes de arreglar (ciclo {cycle}). Pedido original: {pedido_orig}\n\n## TEMP del auditor:\n{}",
        temp.chars().take(1500).collect::<String>()
    )
}

/// Descripción del worker de fixes (v0.9.5): pura.
pub(crate) fn fix_task_desc(temp: &str, cycle: usize) -> String {
    format!(
        "Corrige estos issues del auditor (ciclo {}, sin cambiar nada más):\n{}",
        cycle,
        temp.chars().take(1200).collect::<String>()
    )
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
    orch_phase(state, 5, 5, "cierre");
    state.driver = None;
    state.pending_calls.clear();
    state.status.clear();
    // v0.9 Track B: contador de uso local (turno de agente completado).
    let key = state
        .active_chat_meta()
        .and_then(|c| c.project_id)
        .and_then(|pid| state.projects.iter().find(|p| p.id == pid))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "sin-proyecto".to_string());
    let _ = db::record_turn(&key);
    close_git_turn(state)
}

/// Prepara git al arrancar un turno Work (v0.7.2): asegura la rama de
/// trabajo y recuerda si el árbol venía limpio (guarda anti-sucio).
pub(crate) fn prepare_git_turn(state: &mut App) {
    state.git_verify_ok = false;
    state.git_turn_interrupted = false;
    state.git_clean_before = true;
    let git = state.config.git.clone();
    let Some(ws) = state.o_ws.clone() else {
        return;
    };
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

/// Cierra el turno con git (v0.7.2, commit async v0.9.5): auto-commit solo
/// si la verificación pasó y el árbol venía limpio; el commit corre en
/// background (`GitCommitDone`) y el push —solo si `CommitAndPush +
/// push_enabled`— se encadena desde ese mensaje. La UI no se congela.
fn close_git_turn(state: &mut App) -> Task<Message> {
    let git = state.config.git.clone();
    if !git.enabled || matches!(git.autonomy, crate::config::GitAutonomy::ReadOnly) {
        return Task::none();
    }
    let Some(ws) = state.o_ws.clone() else {
        return Task::none();
    };
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
    let author = git.author();
    let clean_before = state.git_clean_before;
    state.push_log("🌿 commit en background…".to_string());
    Task::perform(
        async move { crate::git::commit_all_async(&ws, &msg, author, clean_before).await },
        Message::GitCommitDone,
    )
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
        agent::WTask {
            desc: desc.to_string(),
            files: Vec::new(),
            accept: String::new(),
        }
    }

    fn history(n: usize) -> Vec<ChatMsg> {
        (0..n)
            .map(|i| ChatMsg {
                role: if i % 2 == 0 {
                    Role::User
                } else {
                    Role::Assistant
                },
                content: format!("m{i}"),
            })
            .collect()
    }

    #[test]
    fn run_fix_cycle_pure_clean_fix_cap() {
        use crate::agent::roles::FixDecision;
        assert_eq!(
            run_fix_cycle("## Auditoría\n\nSIN ISSUES", 0, 0),
            FixDecision::Clean
        );
        assert_eq!(run_fix_cycle("VERDICT: CLEAN", 5, 2), FixDecision::Clean);
        assert_eq!(
            run_fix_cycle("VERDICT: ISSUES\n- algo roto", 0, 0),
            FixDecision::Fix
        );
        assert_eq!(
            run_fix_cycle("VERDICT: ISSUES\n- algo roto", 5, 0),
            FixDecision::Fix
        );
        assert_eq!(
            run_fix_cycle("VERDICT: ISSUES\n- x", 0, 2),
            FixDecision::Fix
        );
        assert_eq!(
            run_fix_cycle("VERDICT: ISSUES\n- x", 1, 2),
            FixDecision::Fix
        );
        assert_eq!(
            run_fix_cycle("VERDICT: ISSUES\n- x", 2, 2),
            FixDecision::CapReached
        );
        assert_eq!(count_temp_issues("VERDICT: ISSUES\n- a\n- b\n#c"), 2);
        assert_eq!(count_temp_issues("SIN ISSUES"), 0);
        assert!(reanalyze_prompt("haz X", "- roto", 1).contains("ciclo 1"));
        assert!(fix_task_desc("- roto", 2).contains("ciclo 2"));
    }

    #[test]
    fn seed_windows_history_and_context_once() {
        let ws = tmp_ws("window");
        std::fs::write(ws.join("CONTEXT").join("ESPEC.md"), "# Demo").unwrap();
        // Historial largo (30) con ventana 20: 20 mensajes + contexto + tarea.
        let (system, raw) = worker_seed(Provider::OpenAI, &history(30), &task("haz X"), &ws, 20);
        assert!(system.contains("haz X"));
        assert!(
            !system.contains("Demo"),
            "ESPEC no va en el system (va una vez como mensaje)"
        );
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
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0)
    }

    /// v0.9: consentimiento del STACK (Track A consulta, Track B niega).
    #[test]
    fn stack_consent_gates_consult_and_save_note() {
        // Sin `use_stack`: ni consulta ni Log de hits (None siempre).
        assert!(stack_consult_block(false, "auth con jwt y tokens").is_none());
        assert!(stack_consult_block(false, "").is_none());
        // Con permiso pero sin nada que matchee: None (el turno sigue igual).
        assert!(stack_consult_block(true, "zzz-sin-match-qqq").is_none());
        // Sin `share_local`: el worker lleva la nota de no proponer guardar.
        assert!(share_note(false).contains("NO permite guardar"));
        assert_eq!(share_note(true), "");
        // Nombrar el STACK en el pedido vale como consentimiento puntual.
        assert!(mentions_stack("usa el STACK para el health endpoint"));
        assert!(mentions_stack("Stack local, por favor"));
        assert!(!mentions_stack("crea un endpoint de salud"));
    }

    /// v0.7.3: el auto-commit del cierre solo ocurre con el turno verde.
    /// v0.9.5: el commit corre en background (`GitCommitDone`); aquí se
    /// verifica la guarda (rojo = sin commit ni tarea) y el commit real
    /// se prueba vía `commit_all_async`.
    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn commit_only_when_turn_is_green() {
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
        crate::git::commit_all_async(&ws, "init", git.author(), true)
            .await
            .unwrap();
        let before = commit_count(&ws);

        // Turno ROJO: hay cambios pero la verificación no pasó -> sin commit.
        let mut red = App::default();
        red.active_chat = None;
        red.config.git = git.clone();
        red.o_ws = Some(ws.clone());
        red.o_history = vec![ChatMsg {
            role: Role::User,
            content: "haz algo".to_string(),
        }];
        red.worker_answers = vec!["listo".to_string()];
        red.git_clean_before = true;
        red.git_verify_ok = false;
        std::fs::write(ws.join("a.txt"), "cambio").unwrap();
        let _ = finish_orchestrator(&mut red, String::new());
        assert_eq!(commit_count(&ws), before, "en rojo no debe commitear");

        // Turno VERDE: mismo cambio pendiente -> commit en background.
        let mut green = App::default();
        green.active_chat = None;
        green.config.git = git.clone();
        green.o_ws = Some(ws.clone());
        green.o_history = vec![ChatMsg {
            role: Role::User,
            content: "haz algo".to_string(),
        }];
        green.worker_answers = vec!["listo".to_string()];
        green.git_clean_before = true;
        green.git_verify_ok = true;
        let _ = finish_orchestrator(&mut green, String::new());
        assert!(
            green
                .tool_logs
                .iter()
                .any(|l| l.contains("commit en background")),
            "en verde se agenda el commit async"
        );
        // El commit real (async) hace el trabajo sin bloquear.
        let sha = crate::git::commit_all_async(&ws, "ARQHIA: verde", git.author(), true)
            .await
            .unwrap();
        assert!(sha.is_some(), "en verde debe commitear");
        assert_eq!(commit_count(&ws), before + 1, "en verde debe commitear");
        let _ = std::fs::remove_dir_all(&ws);
    }
}
