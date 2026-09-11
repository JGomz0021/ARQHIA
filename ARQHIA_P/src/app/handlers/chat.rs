//! Handler de Conversación: envío, streaming, chats y menús.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use futures::SinkExt;
use iced::widget::markdown;

use crate::app::state::clear_turn_state;
use crate::app::View;
use crate::agent;
use crate::config::Provider;
use crate::db::{self, ChatMeta};
use crate::llm::{ChatMsg, Role};
use crate::titles;
use crate::workspace;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::InputChanged(v) => {
            state.input = v;
            Task::none()
        }
        Message::SendPressed => {
            if state.streaming || state.agent_running {
                return Task::none();
            }
            let text = state.input.trim().to_string();
            if text.is_empty() {
                return Task::none();
            }
            state.ensure_active_chat();
            let chat_id = match state.active_chat {
                Some(id) => id,
                None => {
                    state.status = "No hay chat activo.".to_string();
                    return Task::none();
                }
            };
            let provider = state.config.active;
            let cfg = state.config.active_config();
            if !cfg.is_configured_for(provider) {
                state.status = if provider.requires_key() {
                    "Configura API key y modelo en Configuración antes de chatear.".to_string()
                } else {
                    "Configura el modelo local en Configuración antes de chatear.".to_string()
                };
                state.view = View::Config;
                return Task::none();
            }
            state.messages.push(ChatMsg {
                role: Role::User,
                content: text.clone(),
            });
            let _ = db::save_msg(chat_id, "user", &text);
            // Auto-título: primeros 30 chars del primer mensaje de usuario
            let user_count = state
                .messages
                .iter()
                .filter(|m| m.role == Role::User)
                .count();
            if user_count == 1 {
                let title = titles::title_for(&text);
                let _ = db::rename_chat(chat_id, &title);
                if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat_id) {
                    c.title = title;
                }
            }
            start_turn(state, provider, cfg, chat_id, text)
        }
        Message::RetryLast => {
            // Reintenta el último turno de usuario sin volver a guardarlo en DB
            // (la respuesta fallida nunca se persistió: solo se guarda en Done).
            if state.streaming || state.agent_running {
                return Task::none();
            }
            let Some(chat_id) = state.active_chat else {
                state.status = "No hay chat activo.".to_string();
                return Task::none();
            };
            let Some(text) = state
                .messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map(|m| m.content.clone())
            else {
                state.status = "No hay nada que reintentar.".to_string();
                return Task::none();
            };
            // Restos en memoria de una respuesta fallida a medias.
            while state
                .messages
                .last()
                .map(|m| m.role == Role::Assistant)
                .unwrap_or(false)
            {
                state.messages.pop();
                state.md.pop();
            }
            let provider = state.config.active;
            let cfg = state.config.active_config();
            if !cfg.is_configured_for(provider) {
                state.status =
                    "Configura API key y modelo en Configuración antes de chatear.".to_string();
                state.view = View::Config;
                return Task::none();
            }
            state.status.clear();
            start_turn(state, provider, cfg, chat_id, text)
        }
        Message::StreamChunk(chunk) => {
            if let Some(last) = state.messages.last_mut()
                && last.role == Role::Assistant {
                    last.content.push_str(&chunk);
                }
            state.reparse_last_md();
            Task::none()
        }
        Message::StreamUsage(input, output, cached, reported_cost) => {
            let usage = crate::llm::Usage { input, output, cached, cost: reported_cost };
            let n = state.messages.len();
            state.msg_usage.resize(n, None);
            if n > 0 && state.messages[n - 1].role == Role::Assistant {
                state.msg_usage[n - 1] = Some(usage);
            }
            state.context_tokens = input as u64;
            let model = state.config.active_config().model;
            let pid = crate::pricing::provider_id_for(
                state.config.active,
                &state.config.active_config().base_url,
            );
            // Preferencia: coste real del proveedor > catálogo models.dev > tabla local.
            let cost = reported_cost
                .or_else(|| state.pricing.cost_in(pid.as_deref(), &model, usage))
                .or_else(|| crate::llm::estimate_cost_usd(state.config.active, &model, usage));
            state.session_in += input as u64;
            state.session_out += output as u64;
            if let Some(c) = cost {
                state.session_cost += c;
            }
            state.push_log(format!(
                "tokens in {} · out {} · cache {} · coste {}",
                input,
                output,
                cached,
                crate::llm::format_cost(cost)
            ));
            Task::none()
        }
        Message::StreamDone => {
            state.streaming = false;
            state.reparse_last_md();
            if let (Some(chat_id), Some(last)) = (state.active_chat, state.messages.last())
                && last.role == Role::Assistant && !last.content.trim().is_empty() {
                    let _ = db::save_msg(chat_id, "assistant", &last.content);
                }
            Task::none()
        }
        Message::StreamError(e) => {
            state.streaming = false;
            if let Some(last) = state.messages.last()
                && last.role == Role::Assistant && last.content.trim().is_empty() {
                    state.messages.pop();
                    state.md.pop();
                }
            // Re-sincroniza por si algún flujo dejó md/uso desalineado.
            while state.md.len() > state.messages.len() {
                state.md.pop();
            }
            while state.msg_usage.len() > state.messages.len() {
                state.msg_usage.pop();
            }
            state.push_log(format!("Error del proveedor: {e}"));
            state.status = format!("Error: {}", crate::llm::friendly_error(&e));
            Task::none()
        }
        Message::ToggleLogExpand => {
            state.log_expanded = !state.log_expanded;
            Task::none()
        }
        Message::NewChat => {
            match db::create_chat("Nuevo chat") {
                Ok(id) => {
                    state.chats.push(ChatMeta {
                        id,
                        title: "Nuevo chat".to_string(),
                        project_id: None,
                        archived: false,
                        mode: db::Mode::Chat,
                    });
                    state.active_chat = Some(id);
                    state.messages.clear();
                    state.md.clear();
                    state.input.clear();
                    state.status.clear();
                    state.pending_delete = None;
                    state.pending_project = None;
                    clear_turn_state(state);
                }
                Err(e) => state.status = format!("No se pudo crear el chat: {e}"),
            }
            Task::none()
        }
        Message::NewChatInProject(pid) => {
            if state.projects.iter().all(|p| p.id != pid) {
                return Task::none();
            }
            match db::create_chat("Nuevo chat") {
                Ok(id) => {
                    let _ = db::move_chat(id, Some(pid));
                    state.chats.push(ChatMeta {
                        id,
                        title: "Nuevo chat".to_string(),
                        project_id: Some(pid),
                        archived: false,
                        mode: db::Mode::Chat,
                    });
                    state.active_chat = Some(id);
                    state.messages.clear();
                    state.md.clear();
                    state.input.clear();
                    state.status.clear();
                    state.pending_delete = None;
                    state.pending_project = None;
                    clear_turn_state(state);
                }
                Err(e) => state.status = format!("No se pudo crear el chat: {e}"),
            }
            Task::none()
        }
        Message::SelectChat(id) => {
            if state.streaming || state.agent_running {
                return Task::none();
            }
            if state.active_chat != Some(id) {
                clear_turn_state(state);
            }
            state.active_chat = Some(id);
            state.messages = db::load_chat_history(id, 500)
                .unwrap_or_default()
                .into_iter()
                .map(|(role, content)| ChatMsg {
                    role: Role::from_str(&role),
                    content,
                })
                .collect();
            state.reparse_md();
            state.status.clear();
            state.pending_delete = None;
            state.pending_project = None;
            Task::none()
        }
        Message::DeleteChat(id) => {
            state.pending_delete = Some(id);
            Task::none()
        }
        Message::ConfirmDeleteChat => {
            if let Some(id) = state.pending_delete {
                if state.streaming && state.active_chat == Some(id) {
                    state.status = "Espera a que termine la respuesta.".to_string();
                    return Task::none();
                }
                match db::delete_chat(id) {
                    Ok(()) => {
                        state.chats.retain(|c| c.id != id);
                        if state.active_chat == Some(id) {
                            state.active_chat = state
                                .chats
                                .iter()
                                .find(|c| !c.archived)
                                .map(|c| c.id);
                            clear_turn_state(state);
                            state.messages = state
                                .active_chat
                                .and_then(|nid| db::load_chat_history(nid, 500).ok())
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(role, content)| ChatMsg {
                                    role: Role::from_str(&role),
                                    content,
                                })
                                .collect();
                            state.reparse_md();
                        }
                        // Sin chats: queda vacío, el próximo se crea al conversar
                        if state.active_chat.is_none() {
                            state.messages.clear();
                            state.md.clear();
                        }
                    }
                    Err(e) => state.status = format!("No se pudo borrar: {e}"),
                }
                state.pending_delete = None;
            }
            Task::none()
        }
        Message::CancelDelete => {
            state.pending_delete = None;
            Task::none()
        }
        Message::ToggleChatMenu(id) => {
            state.chat_menu = if state.chat_menu == Some(id) {
                None
            } else {
                Some(id)
            };
            if state.chat_menu.is_none() {
                state.move_for = None;
            }
            Task::none()
        }
        Message::ToggleMovePick(id) => {
            state.move_for = if state.move_for == Some(id) {
                None
            } else {
                Some(id)
            };
            Task::none()
        }
        Message::AssignChatProject { chat, project } => {
            if project.is_some()
                && state.projects.iter().all(|p| Some(p.id) != project)
            {
                return Task::none();
            }
            match db::move_chat(chat, project) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat) {
                        c.project_id = project;
                    }
                    state.chat_menu = None;
                    state.move_for = None;
                    state.status.clear();
                }
                Err(e) => state.status = format!("No se pudo mover: {e}"),
            }
            Task::none()
        }
        Message::ArchiveChat(id) => {
            match db::set_archived(id, true) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == id) {
                        c.archived = true;
                    }
                    state.chat_menu = None;
                    state.move_for = None;
                    if state.active_chat == Some(id) {
                        state.active_chat = state
                            .chats
                            .iter()
                            .find(|c| !c.archived)
                            .map(|c| c.id);
                        match state.active_chat {
                            Some(nid) => {
                                state.messages = db::load_chat_history(nid, 500)
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(|(role, content)| ChatMsg {
                                        role: Role::from_str(&role),
                                        content,
                                    })
                                    .collect();
                                state.reparse_md();
                            }
                            None => {
                                state.messages.clear();
                                state.md.clear();
                            }
                        }
                        clear_turn_state(state);
                    }
                }
                Err(e) => state.status = format!("No se pudo archivar: {e}"),
            }
            Task::none()
        }
        Message::UnarchiveChat(id) => {
            match db::set_archived(id, false) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == id) {
                        c.archived = false;
                    }
                    state.chat_menu = None;
                    state.move_for = None;
                }
                Err(e) => state.status = format!("No se pudo restaurar: {e}"),
            }
            Task::none()
        }
        Message::ToggleArchived => {
            state.show_archived = !state.show_archived;
            Task::none()
        }
        Message::NavigateProject(name) => {
            // El selector del header NAVEGA (no mueve: mover es AssignChatProject).
            let pid = if name == "Sin proyecto" {
                None
            } else {
                state.projects.iter().find(|p| p.name == name).map(|p| p.id)
            };
            if name != "Sin proyecto" && pid.is_none() {
                return Task::none();
            }
            super::projects::navigate_project(state, pid)
        }
        Message::QuickSwitchModel(label) => {
            if label == "Sin modelos (Config)" {
                state.view = View::Config;
                return Task::none();
            }
            let name = label.split(" · ").next().unwrap_or("").trim();
            let picked = match name {
                "OpenAI" => Some(Provider::OpenAI),
                "Anthropic" => Some(Provider::Anthropic),
                "OpenRouter" => Some(Provider::OpenRouter),
                "Local (LM Studio)" => Some(Provider::Local),
                _ => None,
            };
            match picked {
                Some(p) => {
                    state.config.active = p;
                    state.edit_provider = p;
                    state.sync_edit_fields();
                    match state.config.save() {
                        Ok(()) => state.status = format!("Modelo: {label}"),
                        Err(e) => state.status = format!("Modelo cambiado pero no se guardó: {e}"),
                    }
                }
                None => state.status = format!("Modelo desconocido: {label}"),
            }
            Task::none()
        }
        Message::QuickReasoningPicked(v) => {
            let val = if v == "auto" { String::new() } else { v };
            let active = state.config.active;
            state.config.active_config_mut().reasoning_effort = val.clone();
            if state.edit_provider == active {
                state.edit_reasoning = val;
            }
            // Sin mensaje de estado: el propio selector muestra el cambio.
            let _ = state.config.save();
            Task::none()
        }
        Message::LinkClicked(url) => {
            // Abre enlaces del markdown en el navegador (fire-and-forget)
            let _ = std::process::Command::new("xdg-open")
                .arg(url.as_str())
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            state.status = format!("Abriendo {url}…");
            Task::none()
        }
        Message::ModePicked(mode) => {
            let Some(chat_id) = state.active_chat else {
                state.status = "Sin chat activo.".to_string();
                return Task::none();
            };
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno para cambiar de modo.".to_string();
                return Task::none();
            }
            match db::set_chat_mode(chat_id, mode) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat_id) {
                        c.mode = mode;
                    }
                    state.show_plan = false;
                    state.status = match mode {
                        db::Mode::Chat => "Modo Chat: conversación directa sin tools.".to_string(),
                        db::Mode::Plan => "Modo Plan: describiré el plan en PLAN.md sin tocar tu código.".to_string(),
                        db::Mode::Work => "Modo Work: orquestador con permisos y límites.".to_string(),
                    };
                }
                Err(e) => state.status = format!("No se pudo cambiar el modo: {e}"),
            }
            Task::none()
        }
        Message::CloseOverlays => {
            // Esc: cierra menús y paneles, sin borrar nada.
            state.chat_menu = None;
            state.move_for = None;
            state.project_menu = None;
            state.pending_delete = None;
            state.pending_project_delete = None;
            state.config_pending_delete = None;
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}

/// Despacha un turno ya aceptado (el mensaje de usuario ya está en
/// `state.messages` y en DB). Reutilizado por `SendPressed` y `RetryLast`.
/// El modo manda, no el workspace:
/// Chat = conversación directa sin tools; Plan = 1 llamada sin tools ->
/// CONTEXT/PLAN.md aprobable; Work = orquestador con permisos/límites.
fn start_turn(
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
        let base_history = state.messages.clone();
        state.messages.push(ChatMsg {
            role: Role::Assistant,
            content: "planificando...".to_string(),
        });
        state.md.push(Vec::new()); // usuario
        state.md.push(markdown::parse("planificando...").collect());
        state.input.clear();
        state.status.clear();
        state.agent_running = true;
        state.o_provider = Some(provider);
        state.o_cfg = Some(cfg.clone());
        state.o_ws = Some(ws.clone());
        state.o_chat = Some(chat_id);
        state.o_history = base_history;
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
        state.push_log(format!("🧭 plan en {}", ws.display()));
        let context = workspace::context_block(&ws);
        let espec = agent::read_espec_md(&ws);
        return Task::perform(
            async move {
                agent::plan_tasks(provider, &text, &context, espec.as_deref(), &cfg, "Plan").await
            },
            move |res| Message::PlanDone(turn, res),
        );
    }
    if mode == db::Mode::Work {
        let Some(ws) = ws else {
            state.status =
                "Work sin workspace: respondo directo (asigna uno para ejecutar).".to_string();
            // Cae al chat directo de abajo.
            return send_plain_chat(state, provider, cfg);
        };
        if state.agent_running {
            return Task::none();
        }
        let base_history = state.messages.clone();
        state.messages.push(ChatMsg {
            role: Role::Assistant,
            content: "orquestando...".to_string(),
        });
        state.md.push(Vec::new()); // usuario
        state.md.push(markdown::parse("orquestando...").collect());
        state.input.clear();
        state.status.clear();
        state.agent_running = true;
        state.o_provider = Some(provider);
        state.o_cfg = Some(cfg.clone());
        state.o_ws = Some(ws.clone());
        state.o_chat = Some(chat_id);
        state.o_history = base_history;
        state.orch_tasks.clear();
        state.worker_answers.clear();
        state.fix_cycle = 0;
        state.pending_calls.clear();
        state.denied_tools.clear();
        state.show_plan = false;
        state.driver = None;
        state.agent_gen += 1;
        let turn = state.agent_gen;
        if let Some(line) = agent::ensure_agents_md(&ws) {
            state.push_log(line);
        }
        state.push_log(format!("🤖 turno en {}", ws.display()));
        state.push_log("planificando...".to_string());
        let context = workspace::context_block(&ws);
        let espec = agent::read_espec_md(&ws);
        return Task::perform(
            async move {
                agent::plan_tasks(provider, &text, &context, espec.as_deref(), &cfg, "Work").await
            },
            move |res| Message::AgentPlan(turn, res),
        );
    }
    // Chat (default): conversación directa sin tools, haya o no workspace.
    send_plain_chat(state, provider, cfg)
}

/// Chat directo (v0.7.1, modo Chat o Work-sin-workspace): streaming sin
/// tools, con ventana de historial configurable. Extraído para reusar en
/// las ramas de modo. Precondición: el mensaje del usuario ya está en
/// `state.messages` y guardado en DB.
fn send_plain_chat(
    state: &mut App,
    provider: Provider,
    cfg: crate::config::ProviderConfig,
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
    state.messages.push(ChatMsg {
        role: Role::Assistant,
        content: String::new(),
    });
    state.md.push(Vec::new()); // usuario: sin markdown
    state.md.push(Vec::new()); // assistant: se re-parsea por chunk
    state.input.clear();
    state.status.clear();
    state.streaming = true;

    Task::stream(iced::stream::channel(100, move |mut output| async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let fut =
            crate::llm::chat_stream(provider, history_for_llm, cfg, move |chunk| {
                let _ = tx.send(chunk);
            });
        let handle = tokio::spawn(fut);
        while let Some(chunk) = rx.recv().await {
            let _ = output.send(Message::StreamChunk(chunk)).await;
        }
        match handle.await {
            Ok(Ok(usage)) => {
                let _ = output
                    .send(Message::StreamUsage(
                        usage.input,
                        usage.output,
                        usage.cached,
                        usage.cost,
                    ))
                    .await;
                let _ = output.send(Message::StreamDone).await;
            }
            Ok(Err(e)) => {
                let _ = output.send(Message::StreamError(e)).await;
            }
            Err(e) => {
                let _ = output
                    .send(Message::StreamError(format!("Tarea cancelada: {e}")))
                    .await;
            }
        }
    }))
}