//! Stream del chat: turnos Plan/Work/Chat + analista + streaming.
//!
//! Split de `handlers/chat.rs` (v0.9.5): sin cambio de comportamiento.

use iced::Task;

use crate::app::Message;
use crate::app::state::App;
use futures::SinkExt;
use iced::widget::markdown;

use crate::agent;
use crate::config::Provider;
use crate::db::{self};
use crate::llm::{ChatMsg, Role};

/// Despacha un turno ya aceptado (el mensaje de usuario ya está en
/// `state.messages` y en DB). Reutilizado por `SendPressed` y `RetryLast`.
/// El modo manda, no el workspace:
/// Chat = conversación directa sin tools; Plan = 1 llamada sin tools ->
/// CONTEXT/PLAN.md aprobable; Work = orquestador con permisos/límites.
pub(crate) fn start_turn(
    state: &mut App,
    provider: Provider,
    cfg: crate::config::ProviderConfig,
    chat_id: i64,
    text: String,
) -> Task<Message> {
    let mode = state.active_mode();
    let ws = state.active_workspace();
    if mode == db::Mode::Plan {
        let Some(ws) = ws else {
            state.status = "Plan necesita un workspace asignado al proyecto.".to_string();
            return Task::none();
        };
        if state.agent_running {
            return Task::none();
        }
        let log = format!("🧭 plan en {}", ws.display());
        return begin_analysis_turn(
            state,
            provider,
            cfg,
            chat_id,
            text,
            ws,
            db::Mode::Plan,
            "planificando...",
            log,
            false,
        );
    }
    if mode == db::Mode::Work {
        let Some(ws) = ws else {
            state.status =
                "Work sin workspace: respondo directo (asigna uno para ejecutar).".to_string();
            // Cae al chat directo de abajo.
            return send_plain_chat(state, provider, cfg, chat_id);
        };
        if state.agent_running {
            return Task::none();
        }
        let log = format!("🤖 turno en {}", ws.display());
        return begin_analysis_turn(
            state,
            provider,
            cfg,
            chat_id,
            text,
            ws,
            db::Mode::Work,
            "orquestando...",
            log,
            true,
        );
    }
    // Chat (default): conversación directa sin tools, haya o no workspace.
    send_plain_chat(state, provider, cfg, chat_id)
}

/// Arranca un turno Plan/Work (v0.7.3): placeholder visible, estado del
/// orquestador reseteado y 1 llamada al analista que desemboca en
/// `AgentAnalyze`. `with_git` añade AGENTS.md + rama de trabajo (Work).
#[allow(clippy::too_many_arguments)]
pub(crate) fn begin_analysis_turn(
    state: &mut App,
    provider: Provider,
    cfg: crate::config::ProviderConfig,
    chat_id: i64,
    text: String,
    ws: std::path::PathBuf,
    mode: db::Mode,
    placeholder: &str,
    log_line: String,
    with_git: bool,
) -> Task<Message> {
    let base_history = state.messages.clone();
    state.history_push(
        ChatMsg {
            role: Role::Assistant,
            content: placeholder.to_string(),
        },
        markdown::parse(placeholder).collect(),
        None,
        String::new(),
        0,
    );
    state.input.clear();
    state.status.clear();
    state.agent_running = true;
    state.o_provider = Some(provider);
    state.o_cfg = Some(cfg.clone());
    state.o_ws = Some(ws.clone());
    state.o_chat = Some(chat_id);
    state.o_history = base_history;
    state.o_mode = mode;
    state.orch_tasks.clear();
    state.worker_answers.clear();
    state.fix_cycle = 0;
    state.pending_calls.clear();
    state.denied_tools.clear();
    state.show_plan = false;
    state.plan_md.clear();
    state.driver = None;
    state.agent_gen += 1;
    let turn = state.agent_gen;
    if with_git {
        if let Some(line) = agent::ensure_agents_md(&ws) {
            state.push_log(line);
        }
        // v0.7.2: rama de trabajo + árbol limpio antes de tocar nada.
        crate::app::orchestrator::prepare_git_turn(state);
    }
    state.push_log(log_line);
    // v0.7.3: el analista produce el brief antes de planificar.
    // v0.9.1: fase visible del orquestador BETA (1/5 analista).
    crate::app::orchestrator::orch_phase(state, 1, 5, "analista");
    state.push_log("🔎 analizando contexto…".to_string());
    let ws2 = ws.clone();
    let pedido = text.clone();
    Task::perform(
        async move { agent::analyze_workspace(provider, &cfg, &ws2, &pedido).await },
        move |res| Message::AgentAnalyze(turn, res),
    )
}

/// Chat directo (v0.7.1, modo Chat o Work-sin-workspace): streaming sin
/// tools, con ventana de historial configurable. Extraído para reusar en
/// las ramas de modo. Precondición: el mensaje del usuario ya está en
/// `state.messages` y guardado en DB.
pub(crate) fn send_plain_chat(
    state: &mut App,
    provider: Provider,
    cfg: crate::config::ProviderConfig,
    chat_id: i64,
) -> Task<Message> {
    // System prompt de ARQHIA + modo declarado (no se guarda en la DB:
    // se antepone solo al historial enviado al modelo).
    let mode = state.active_mode();
    let mode_line = match mode {
        db::Mode::Chat => "Modo actual: Chat (conversación directa, sin herramientas).",
        db::Mode::Plan => "Modo actual: Plan (solo planificar, sin ejecutar herramientas).",
        db::Mode::Work => "Modo actual: Work (sin workspace: respondo directo, sin herramientas).",
    };
    let system = format!(
        "{}\n\n{mode_line}",
        crate::llm::system_identity(&format!("{} ({})", cfg.model, provider), false),
    );
    // Ventana v0.7.1: últimos N mensajes; lo viejo colapsa con marcador.
    let limit = state.config.limits.clamped().history_limit;
    let (window, cut) = agent::window_history(&state.messages, limit);
    let mut history_for_llm = vec![ChatMsg {
        role: Role::System,
        content: system,
    }];
    if cut > 0 {
        history_for_llm.push(ChatMsg {
            role: Role::System,
            content: format!("[{cut} mensajes previos omitidos por ventana de historial]"),
        });
    }
    history_for_llm.extend(window);
    // v0.8: sesión estable del chat (una vez por chat; persiste en DB).
    let session: Option<String> = db::ensure_session_id(chat_id).ok();
    if let (Some(s), Some(meta)) = (
        session.clone(),
        state.chats.iter_mut().find(|c| c.id == chat_id),
    ) {
        meta.session_id = Some(s);
    }
    state.history_push(
        ChatMsg {
            role: Role::Assistant,
            content: String::new(),
        },
        Vec::new(),
        None,
        String::new(),
        0,
    ); // assistant: se re-parsea por chunk
    state.input.clear();
    state.status.clear();
    state.streaming = true;
    // v0.9.5: recuerda el chat del turno para persistir la respuesta en él
    // aunque el usuario navegue (las navegaciones ya están guardadas).
    state.o_chat = Some(chat_id);
    // Nueva generación: los chunks de streams anteriores se ignoran.
    state.stream_gen += 1;
    let sgen = state.stream_gen;
    // v0.9.5: registro del handle para poder abortar la petición en "Detener".
    let abort_slot = state.stream_abort.clone();

    Task::stream(iced::stream::channel(100, move |mut output| async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let fut = crate::llm::chat_stream(provider, history_for_llm, cfg, session, move |chunk| {
            let _ = tx.send(chunk);
        });
        let handle = tokio::spawn(fut);
        if let Ok(mut slot) = abort_slot.lock() {
            *slot = Some(handle.abort_handle());
        }
        while let Some(chunk) = rx.recv().await {
            let _ = output.send(Message::StreamChunk(sgen, chunk)).await;
        }
        match handle.await {
            Ok(Ok(usage)) => {
                let _ = output
                    .send(Message::StreamUsage(
                        sgen,
                        usage.input,
                        usage.output,
                        usage.cached,
                        usage.cost,
                    ))
                    .await;
                let _ = output.send(Message::StreamDone(sgen)).await;
            }
            Ok(Err(e)) => {
                let _ = output.send(Message::StreamError(sgen, e)).await;
            }
            Err(e) => {
                let _ = output
                    .send(Message::StreamError(sgen, format!("Tarea cancelada: {e}")))
                    .await;
            }
        }
        if let Ok(mut slot) = abort_slot.lock() {
            *slot = None;
        }
    }))
}
