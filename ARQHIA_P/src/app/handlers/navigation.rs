//! Handler de Navegación entre vistas.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::GoChat => {
            state.view = View::Chat;
            state.status.clear();
            state.creating = false;
            Task::none()
        }
        Message::GoHome => {
            state.view = View::Home;
            state.status.clear();
            state.creating = false;
            Task::none()
        }
        Message::ExitApp => iced::exit(),
        // v0.8: el aviso de onboarding se oculta en memoria ("después").
        // Desaparece del todo al guardar la primera API (condición de vista).
        Message::DismissOnboarding => {
            state.onboarding_dismissed = true;
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}