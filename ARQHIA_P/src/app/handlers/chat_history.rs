//! Historial del chat: clave de uso, creación y scroll.
//!
//! Split de `handlers/chat.rs` (v0.9.5): sin cambio de comportamiento.

use iced::Task;

use crate::app::Message;
use crate::app::state::App;
use crate::app::state::clear_turn_state;
use crate::db::{self, ChatMeta};

/// Clave del contador de uso (v0.9 Track B): nombre del proyecto del chat
/// activo o "sin-proyecto". Solo conteos, sin contenido.
pub(crate) fn project_key(state: &App) -> String {
    state
        .active_chat_meta()
        .and_then(|c| c.project_id)
        .and_then(|pid| state.projects.iter().find(|p| p.id == pid))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "sin-proyecto".to_string())
}

/// Crea un chat vacío y lo activa (dentro de un proyecto o suelto).
/// Extraído de `NewChat`/`NewChatInProject` (idénticos salvo el destino).
pub(crate) fn spawn_chat(state: &mut App, project: Option<i64>) {
    match db::create_chat("Nuevo chat") {
        Ok(id) => {
            if let Some(pid) = project {
                let _ = db::move_chat(id, Some(pid));
            }
            state.chats.push(ChatMeta {
                id,
                title: "Nuevo chat".to_string(),
                project_id: project,
                archived: false,
                mode: db::Mode::Chat,
                session_id: None,
            });
            state.active_chat = Some(id);
            state.messages.clear();
            state.md.clear();
            state.msg_times.clear();
            state.msg_ids.clear();
            state.msg_usage.clear();
            state.input.clear();
            state.status.clear();
            state.pending_delete = None;
            state.pending_project = None;
            clear_turn_state(state);
        }
        Err(e) => state.status = format!("No se pudo crear el chat: {e}"),
    }
}

/// Salta el scroll de mensajes al final (entrar al chat desde abajo).
pub(crate) fn scroll_chat_to_end() -> Task<Message> {
    iced::widget::scrollable::snap_to(
        iced::widget::scrollable::Id::new("chat-msgs"),
        iced::widget::scrollable::RelativeOffset { x: 0.0, y: 1.0 },
    )
}
