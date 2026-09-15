//! Handler de Conversación: envío, streaming, chats y menús.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::Message;
use crate::app::state::App;
use futures::SinkExt;

use crate::agent;
use crate::app::View;
use crate::app::state::clear_turn_state;
use crate::config::Provider;
use crate::db::{self, ChatMeta};
use crate::llm::{ChatMsg, Role};
use crate::titles;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::InputChanged(v) => {
            state.input = v.clone();
            // Al escribir `/` se sugieren skills con su contenido (ver
            // composer): la lista vive en estado para que la vista sea pura.
            if v.trim_start().starts_with('/') {
                state.skill_suggest = crate::skills::suggest(&v);
            } else {
                state.skill_suggest.clear();
            }
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
            state.skill_suggest.clear();
            // `/skill nombre [texto]` (o prefijo único como `/rev`) se
            // resuelve antes del dispatch por modo. Sin match = error
            // amable con candidatas, sin turno.
            let mut skill_ctx: Option<String> = None;
            if text.starts_with('/') {
                match crate::skills::resolve_slash(&text) {
                    Ok((name, rest)) => match crate::skills::load(&name) {
                        Ok(skill) => {
                            let origin = skill.origin.badge();
                            state.push_log(format!("skill {name} ({origin}) inyectada"));
                            skill_ctx = Some(crate::skills::context_block(&skill, &rest));
                        }
                        Err(e) => {
                            state.status = format!("No se pudo cargar «{name}»: {e}");
                            return Task::none();
                        }
                    },
                    Err((prefix, cands)) => {
                        state.status = if prefix.is_empty() {
                            let names = cands.join(", ");
                            format!(
                                "Uso: /skill nombre [texto]. Instaladas: {}.",
                                if names.is_empty() {
                                    "(ninguna)".to_string()
                                } else {
                                    names
                                }
                            )
                        } else if cands.is_empty() {
                            let names: Vec<String> = crate::skills::list()
                                .iter()
                                .map(|d| d.name.clone())
                                .collect();
                            format!(
                                "Skill «{prefix}» no encontrada. Instaladas: {}.",
                                if names.is_empty() {
                                    "(ninguna)".to_string()
                                } else {
                                    names.join(", ")
                                }
                            )
                        } else {
                            format!(
                                "«{prefix}» coincide con varias: {}. Completa el nombre.",
                                cands.join(", ")
                            )
                        };
                        return Task::none();
                    }
                }
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
            // Valida el turno ANTES de persistir: un mensaje inválido (Plan
            // sin workspace) no entra ni en DB ni en contexto. El input se
            // conserva para que el usuario corrija sin reescribir.
            if state.active_mode() == db::Mode::Plan && state.active_workspace().is_none() {
                state.status = "Plan necesita un workspace asignado al proyecto.".to_string();
                return Task::none();
            }
            // Undo v0.7.4: snapshot de 1 paso antes de enviar (la vista
            // manda; las filas a borrar se calculan por diferencia en DB).
            state.undo = Some(crate::app::state::UndoSnapshot {
                chat_id,
                messages: state.messages.clone(),
                msg_times: state.msg_times.clone(),
                msg_ids: state.msg_ids.clone(),
                truncated_tail: Vec::new(),
                deleted_chat: None,
                deleted_messages: Vec::new(),
            });
            state.history_push(
                ChatMsg {
                    role: Role::User,
                    content: text.clone(),
                },
                Vec::new(),
                None,
                String::new(),
                0,
            );
            let _ = db::save_msg(chat_id, "user", &text);
            // Refresca ids/timestamps desde DB (el insert deja id real).
            state.resync_msg_meta(chat_id);
            // Auto-título: fallback inmediato + propuesta IA async (v0.7.4).
            let user_count = state
                .messages
                .iter()
                .filter(|m| m.role == Role::User)
                .count();
            let mut title_task = Task::none();
            if user_count == 1 {
                let title = titles::title_for(&text);
                let _ = db::rename_chat(chat_id, &title);
                if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat_id) {
                    c.title = title;
                }
                // 1 llamada corta sin tools; al llegar, ChatTitleFetched.
                state.title_gen += 1;
                let generation = state.title_gen;
                let first = text.clone();
                let cfg_title = cfg.clone();
                title_task = Task::perform(
                    async move {
                        agent::ai_title(provider, &cfg_title, &first)
                            .await
                            .unwrap_or_default()
                    },
                    move |t| Message::ChatTitleFetched(generation, chat_id, t),
                );
            }
            // La skill se inyecta UNA vez como mensaje de contexto ANTES de
            // arrancar el turno (así viaja al LLM) y el turno continúa
            // normal (Chat/Plan/Work según el modo).
            if let Some(ctx) = skill_ctx {
                let body = format!("Contexto de skill (solo lectura, no repetir):\n{ctx}");
                state.history_push(
                    ChatMsg {
                        role: Role::User,
                        content: body.clone(),
                    },
                    Vec::new(),
                    None,
                    String::new(),
                    0,
                );
                let _ = db::save_msg(chat_id, "user", &body);
                state.resync_msg_meta(chat_id);
            }
            let turn = start_turn(state, provider, cfg, chat_id, text);
            Task::batch(vec![title_task, turn])
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
                state.pop_last_message();
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
        Message::StreamChunk(sgen, chunk) => {
            if sgen != state.stream_gen {
                return Task::none(); // stream cancelado, chunk tardío
            }
            if let Some(last) = state.messages.last_mut()
                && last.role == Role::Assistant
            {
                last.content.push_str(&chunk);
            }
            state.reparse_last_md();
            Task::none()
        }
        Message::StreamUsage(sgen, input, output, cached, reported_cost) => {
            if sgen != state.stream_gen {
                return Task::none();
            }
            let usage = crate::llm::Usage {
                input,
                output,
                cached,
                cost: reported_cost,
            };
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
            // Local nunca cuesta: no caer al catálogo (evita coste falso).
            let cost = if state.config.active == Provider::Local {
                Some(0.0)
            } else {
                reported_cost
                    .or_else(|| state.pricing.cost_in(pid.as_deref(), &model, usage))
                    .or_else(|| crate::llm::estimate_cost_usd(state.config.active, &model, usage))
            };
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
        Message::StreamDone(sgen) => {
            if sgen != state.stream_gen {
                return Task::none();
            }
            state.streaming = false;
            state.reparse_last_md();
            if let (Some(chat_id), Some(last)) =
                (state.o_chat.or(state.active_chat), state.messages.last())
                && last.role == Role::Assistant
                && !last.content.trim().is_empty()
            {
                let _ = db::save_msg(chat_id, "assistant", &last.content);
                state.resync_msg_meta(chat_id);
            }
            // v0.9 Track B: contador de uso local (turno de chat).
            let _ = db::record_turn(&project_key(state));
            Task::none()
        }
        Message::StreamError(sgen, e) => {
            if sgen != state.stream_gen {
                return Task::none();
            }
            state.streaming = false;
            if let Some(last) = state.messages.last()
                && last.role == Role::Assistant
                && last.content.trim().is_empty()
            {
                state.pop_last_message();
            }
            // Re-sincroniza por si algún flujo dejó md/uso desalineado.
            while state.md.len() > state.messages.len() {
                state.md.pop();
            }
            while state.msg_usage.len() > state.messages.len() {
                state.msg_usage.pop();
            }
            while state.msg_times.len() > state.messages.len() {
                state.msg_times.pop();
            }
            while state.msg_ids.len() > state.messages.len() {
                state.msg_ids.pop();
            }
            state.push_log(format!("Error del proveedor: {e}"));
            state.status = format!("Error: {}", crate::llm::friendly_error(&e));
            // v0.9 Track B: los 429 alimentan el panel de uso.
            if crate::llm::is_rate_limit_error(&e) {
                let _ = db::record_429(&project_key(state));
            }
            Task::none()
        }
        Message::ToggleLogExpand => {
            state.log_expanded = !state.log_expanded;
            Task::none()
        }
        Message::NewChat => {
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno.".to_string();
                return Task::none();
            }
            spawn_chat(state, None);
            Task::none()
        }
        Message::NewChatInProject(pid) => {
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno.".to_string();
                return Task::none();
            }
            if state.projects.iter().all(|p| p.id != pid) {
                return Task::none();
            }
            spawn_chat(state, Some(pid));
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
            state.reload_active_chat();
            state.status.clear();
            state.pending_delete = None;
            state.pending_project = None;
            // Entrar al chat desde el final (últimos mensajes visibles).
            scroll_chat_to_end()
        }
        Message::DeleteChat(id) => {
            state.pending_delete = Some(id);
            Task::none()
        }
        Message::ConfirmDeleteChat => {
            if let Some(id) = state.pending_delete {
                if (state.streaming || state.agent_running) && state.active_chat == Some(id) {
                    state.status = "Espera a que termine la respuesta.".to_string();
                    return Task::none();
                }
                // Undo v0.7.4: guarda el chat + mensajes antes de borrar.
                let doomed_meta = state.chats.iter().find(|c| c.id == id).cloned();
                let doomed_msgs: Vec<(String, String, String)> =
                    db::load_chat_history_full(id, 1000)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| (m.role, m.content, m.created_at))
                        .collect();
                let snap = crate::app::state::UndoSnapshot {
                    chat_id: id,
                    messages: state.messages.clone(),
                    msg_times: state.msg_times.clone(),
                    msg_ids: state.msg_ids.clone(),
                    truncated_tail: Vec::new(),
                    deleted_chat: doomed_meta,
                    deleted_messages: doomed_msgs,
                };
                match db::delete_chat(id) {
                    Ok(()) => {
                        state.chats.retain(|c| c.id != id);
                        if state.active_chat == Some(id) {
                            state.active_chat =
                                state.chats.iter().find(|c| !c.archived).map(|c| c.id);
                            clear_turn_state(state);
                            state.reload_active_chat();
                        }
                        // clear_turn_state borra el undo: lo restauramos.
                        state.undo = Some(snap);
                        // Sin chats: queda vacío, el próximo se crea al conversar
                        if state.active_chat.is_none() {
                            state.messages.clear();
                            state.md.clear();
                            state.msg_times.clear();
                            state.msg_ids.clear();
                            state.msg_usage.clear();
                        }
                        state.status = "Chat borrado. Ctrl+Z para deshacer.".to_string();
                    }
                    Err(e) => state.status = format!("No se pudo borrar: {e}"),
                }
                state.pending_delete = None;
                // Recupera el snapshot (clear_turn_state lo había limpiado).
                // Se deja tal cual si el borrado falló (undo=None ya).
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
            if project.is_some() && state.projects.iter().all(|p| Some(p.id) != project) {
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
            if (state.streaming || state.agent_running) && state.active_chat == Some(id) {
                state.status = "Espera a que termine el turno antes de archivar.".to_string();
                return Task::none();
            }
            match db::set_archived(id, true) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == id) {
                        c.archived = true;
                    }
                    state.chat_menu = None;
                    state.move_for = None;
                    if state.active_chat == Some(id) {
                        state.active_chat = state.chats.iter().find(|c| !c.archived).map(|c| c.id);
                        match state.active_chat {
                            Some(_) => {
                                state.reload_active_chat();
                            }
                            None => {
                                state.messages.clear();
                                state.md.clear();
                                state.msg_times.clear();
                                state.msg_ids.clear();
                                state.msg_usage.clear();
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
            // v0.7.4: primero prueba perfiles por nombre/etiqueta.
            if let Some(p) = state
                .config
                .model_profiles
                .iter()
                .find(|p| p.label() == label || p.name == label)
                .cloned()
            {
                state.config.apply_profile(&p.id);
                state.edit_provider = p.provider;
                state.sync_edit_fields();
                match state.config.save() {
                    Ok(()) => state.status = format!("Perfil: {}", p.name),
                    Err(e) => state.status = format!("Perfil cambiado pero no se guardó: {e}"),
                }
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
            // v0.9.5: solo http/https; nunca `file:`, `smb:`… vía xdg-open.
            state.status = match open_in_browser(url.as_str()) {
                Ok(()) => format!("Abriendo {url}…"),
                Err(e) => format!("No se pudo abrir el enlace: {e}"),
            };
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
                        db::Mode::Plan => {
                            "Modo Plan: describiré el plan en PLAN.md sin tocar tu código."
                                .to_string()
                        }
                        db::Mode::Work => {
                            "Modo Work: orquestador con permisos y límites.".to_string()
                        }
                    };
                    // Animación del segmento (v0.8.1): ~8 ticks de 60 ms que
                    // mueven el padding 2→6→2, venga por click o por atajo
                    // (mismo Message). La generación jubila ticks viejos.
                    state.mode_anim_gen += 1;
                    let anim_gen = state.mode_anim_gen;
                    state.mode_anim = Some((std::time::Instant::now(), 0));
                    return Task::stream(iced::stream::channel(10, move |mut output| async move {
                        for step in 1..=8u8 {
                            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                            let _ = output.send(Message::ModeAnimTick(anim_gen, step)).await;
                        }
                    }));
                }
                Err(e) => state.status = format!("No se pudo cambiar el modo: {e}"),
            }
            Task::none()
        }
        Message::ModeAnimTick(tick_gen, step) => {
            // Ticks viejos (de un cambio anterior) se ignoran; el paso 8
            // cierra la animación.
            if tick_gen != state.mode_anim_gen {
                return Task::none();
            }
            if step >= 8 {
                state.mode_anim = None;
            } else if let Some(slot) = state.mode_anim.as_mut() {
                slot.1 = step;
            }
            Task::none()
        }
        Message::CloseOverlays => {
            // Doble Esc: detiene el turno en curso (agente o stream).
            let now = std::time::Instant::now();
            let double = state
                .last_esc
                .map(|t| now.duration_since(t) < std::time::Duration::from_millis(600))
                .unwrap_or(false);
            state.last_esc = Some(now);
            if double && (state.agent_running || state.streaming) {
                return super::agent::handle(state, Message::StopAgent);
            }
            // Esc cierra menús y paneles. En Config, además, vuelve al origen
            // (misma acción que el botón Volver).
            if state.view == View::Config {
                state.view = state.config_from.clone();
                state.status.clear();
                return Task::none();
            }
            // v0.9: en el panel STACK, Esc vuelve a la vista origen.
            if state.view == View::Stack {
                state.view = state.stack_from.clone();
                state.status.clear();
                return Task::none();
            }
            state.chat_menu = None;
            state.move_for = None;
            state.project_menu = None;
            state.pending_delete = None;
            state.pending_project_delete = None;
            state.config_pending_delete = None;
            state.profile_menu = None;
            state.editing_profile = None;
            state.msg_menu = None;
            state.pending_truncate = None;
            Task::none()
        }
        Message::UndoChat => {
            // Ctrl+Z v0.7.4: restaura el snapshot de 1 paso.
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno para deshacer.".to_string();
                return Task::none();
            }
            let Some(snap) = state.undo.take() else {
                state.status = "Nada que deshacer.".to_string();
                return Task::none();
            };
            if let Some(active) = state.active_chat
                && active != snap.chat_id
            {
                state.undo = Some(snap);
                state.status = "El undo es del chat anterior: vuelve a ese chat.".to_string();
                return Task::none();
            }
            // Si se borró un chat, lo recrea con sus mensajes.
            if let Some(del) = snap.deleted_chat.clone() {
                if let Ok(new_id) = db::create_chat(&del.title) {
                    let _ = db::move_chat(new_id, del.project_id);
                    let _ = db::set_chat_mode(new_id, del.mode);
                    for (r, c, _) in &snap.deleted_messages {
                        let _ = db::save_msg(new_id, r, c);
                    }
                    state.chats.push(ChatMeta {
                        id: new_id,
                        title: del.title.clone(),
                        project_id: del.project_id,
                        archived: del.archived,
                        mode: del.mode,
                        session_id: del.session_id.clone(),
                    });
                    state.active_chat = Some(new_id);
                    state.reload_active_chat();
                    state.status = "Chat restaurado.".to_string();
                }
                return Task::none();
            }
            // Si fue un "deshacer hasta aquí", reinserta la cola en DB.
            if !snap.truncated_tail.is_empty() {
                for (r, c) in &snap.truncated_tail {
                    let _ = db::save_msg(snap.chat_id, r, c);
                }
                state.messages = snap.messages;
                state.reparse_md();
                state.resync_msg_meta(snap.chat_id);
                state.status = "Recuperado.".to_string();
                return Task::none();
            }
            // Si fue un envío, borra por diferencia lo que se añadió en DB
            // desde el snapshot (1 fila si el stream falló, 2 si respondió).
            // Así nunca se lleva por delante mensajes anteriores.
            if let Ok(n_db) = db::count_messages(snap.chat_id) {
                let extra = n_db.saturating_sub(snap.messages.len());
                if extra > 0 {
                    let _ = db::delete_last_messages(snap.chat_id, extra);
                }
            }
            state.messages = snap.messages;
            state.msg_times = snap.msg_times;
            state.msg_ids = snap.msg_ids;
            state.reparse_md();
            state.resync_msg_meta(snap.chat_id);
            state.status = "Deshecho.".to_string();
            Task::none()
        }
        Message::ChatMsgMenu(idx) => {
            if idx >= state.messages.len() {
                return Task::none();
            }
            state.msg_menu = if state.msg_menu == Some(idx) {
                None
            } else {
                Some(idx)
            };
            if state.msg_menu != Some(idx) {
                state.pending_truncate = None;
            }
            Task::none()
        }
        Message::CopyMsg(idx) => {
            let Some(m) = state.messages.get(idx) else {
                state.status = "Nada que copiar.".to_string();
                return Task::none();
            };
            let text = m.content.clone();
            if text.trim().is_empty() {
                state.status = "Mensaje vacío.".to_string();
                return Task::none();
            }
            state.msg_menu = None;
            state.status = "Copiado al portapapeles.".to_string();
            iced::clipboard::write(text)
        }
        Message::TruncateRequest(idx) => {
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno.".to_string();
                return Task::none();
            }
            if idx >= state.messages.len() {
                return Task::none();
            }
            state.msg_menu = None;
            state.pending_truncate = Some(idx);
            Task::none()
        }
        Message::CancelTruncate => {
            state.pending_truncate = None;
            Task::none()
        }
        Message::ConfirmTruncate => {
            let Some(idx) = state.pending_truncate else {
                return Task::none();
            };
            state.pending_truncate = None;
            state.msg_menu = None;
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno.".to_string();
                return Task::none();
            }
            let Some(chat_id) = state.active_chat else {
                state.status = "Sin chat activo.".to_string();
                return Task::none();
            };
            if idx >= state.messages.len() {
                return Task::none();
            }
            let Some(mid) = state.msg_ids.get(idx).copied().filter(|m| *m > 0) else {
                state.status = "Ese mensaje aún no está guardado.".to_string();
                return Task::none();
            };
            if idx + 1 >= state.messages.len() {
                state.status = "Ya estás al final: no hay nada que borrar.".to_string();
                return Task::none();
            }
            // Snapshot para Ctrl+Z: vista completa + cola para reinsertar.
            let tail: Vec<(String, String)> = state.messages[idx + 1..]
                .iter()
                .map(|m| (m.role.as_str().to_string(), m.content.clone()))
                .collect();
            state.undo = Some(crate::app::state::UndoSnapshot {
                chat_id,
                messages: state.messages.clone(),
                msg_times: state.msg_times.clone(),
                msg_ids: state.msg_ids.clone(),
                truncated_tail: tail,
                deleted_chat: None,
                deleted_messages: Vec::new(),
            });
            match db::delete_messages_after(chat_id, mid) {
                Ok(_) => {
                    state.messages.truncate(idx + 1);
                    state.reparse_md();
                    // Re-alinea la cola conservada con la DB.
                    state.resync_msg_meta(chat_id);
                    state.status = "Deshecho hasta aquí. Ctrl+Z para recuperar.".to_string();
                }
                Err(e) => {
                    state.undo = None;
                    state.status = format!("No se pudo deshacer: {e}");
                }
            }
            Task::none()
        }
        Message::BranchChatFrom(idx) => {
            if state.streaming || state.agent_running {
                state.status = "Espera a que termine el turno.".to_string();
                return Task::none();
            }
            let Some(id) = state.active_chat else {
                state.status = "Sin chat activo.".to_string();
                return Task::none();
            };
            let Some(mid) = state.msg_ids.get(idx).copied().filter(|m| *m > 0) else {
                state.status = "Ese mensaje aún no está guardado.".to_string();
                return Task::none();
            };
            match db::branch_chat(id, mid) {
                Ok(new_id) => {
                    let meta = db::list_chats()
                        .ok()
                        .and_then(|v| v.into_iter().find(|c| c.id == new_id));
                    if let Some(m) = meta {
                        state.chats.push(m);
                    }
                    state.active_chat = Some(new_id);
                    state.reload_active_chat();
                    clear_turn_state(state);
                    state.status = "Rama creada desde ese mensaje.".to_string();
                }
                Err(e) => state.status = format!("No se pudo bifurcar: {e}"),
            }
            Task::none()
        }
        Message::ResetSession => {
            // v0.8: regenera el id de sesión del chat activo (limpia la
            // caché que el provider asociaba a la conversación).
            let Some(chat_id) = state.active_chat else {
                state.status = "Sin chat activo.".to_string();
                return Task::none();
            };
            let fresh = db::new_session_id();
            match db::set_session_id(chat_id, Some(&fresh)) {
                Ok(()) => {
                    if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat_id) {
                        c.session_id = Some(fresh);
                    }
                    state.chat_menu = None;
                    state.move_for = None;
                    state.status = "Sesión reiniciada.".to_string();
                }
                Err(e) => state.status = format!("No se pudo reiniciar: {e}"),
            }
            Task::none()
        }
        Message::ChatTitleFetched(generation, chat_id, title) => {
            if generation != state.title_gen || title.trim().is_empty() {
                return Task::none();
            }
            // Solo si el chat sigue activo y el título sigue siendo el fallback.
            if state.active_chat != Some(chat_id) {
                return Task::none();
            }
            let clean = titles::sanitize_ai_title(&title, "");
            if clean.trim().is_empty() {
                return Task::none();
            }
            let _ = db::rename_chat(chat_id, &clean);
            if let Some(c) = state.chats.iter_mut().find(|c| c.id == chat_id) {
                c.title = clean;
            }
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}

/// Abre una URL http(s) en el navegador del sistema. Rechaza cualquier otro
/// esquema (`file:`, `smb:`, `javascript:`…) para que contenido externo o
/// generado por el LLM no pueda lanzar handlers arbitrarios. v0.9.5.
fn open_in_browser(url: &str) -> Result<(), String> {
    let u = url.trim();
    if !(u.starts_with("https://") || u.starts_with("http://")) {
        return Err("solo se permiten enlaces http/https".to_string());
    }
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(u)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub(crate) use super::chat_history::{project_key, scroll_chat_to_end, spawn_chat};
use super::chat_stream::start_turn;

#[cfg(test)]
mod tests {
    use super::open_in_browser;

    #[test]
    fn link_opener_rejects_non_http_schemes() {
        assert!(open_in_browser("file:///etc/passwd").is_err());
        assert!(open_in_browser("smb://host/share").is_err());
        assert!(open_in_browser("javascript:alert(1)").is_err());
        assert!(open_in_browser("").is_err());
    }
}
