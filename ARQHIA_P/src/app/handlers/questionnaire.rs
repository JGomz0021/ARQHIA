//! Handler de Cuestionario de definición del proyecto.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;
use crate::questionnaire;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::QNext => {
            if let Some(err) = questionnaire::validate(state.q_step, &state.q_answers) {
                state.q_error = err;
            } else {
                state.q_error.clear();
                if state.q_step + 1 < questionnaire::TOTAL_STEPS {
                    state.q_step += 1;
                }
            }
            Task::none()
        }
        Message::QBack => {
            state.q_error.clear();
            state.q_step = state.q_step.saturating_sub(1);
            Task::none()
        }
        Message::QCancel => {
            state.view = View::Chat;
            state.q_error.clear();
            Task::none()
        }
        Message::QNombreChanged(v) => {
            state.q_answers.nombre = v;
            Task::none()
        }
        Message::QDescChanged(v) => {
            state.q_answers.descripcion = v;
            Task::none()
        }
        Message::QUbicChanged(v) => {
            state.q_answers.ubicacion = v;
            Task::none()
        }
        Message::QObjChanged(v) => {
            state.q_answers.objetivo = v;
            Task::none()
        }
        Message::QPublicoPicked(p) => {
            state.q_answers.publico = p;
            Task::none()
        }
        Message::QInterfazPicked(i) => {
            state.q_answers.interfaz = i;
            Task::none()
        }
        Message::FinishQuestionnaire => {
            for step in 0..questionnaire::TOTAL_STEPS {
                if let Some(err) = questionnaire::validate(step, &state.q_answers) {
                    state.q_step = step;
                    state.q_error = err;
                    return Task::none();
                }
            }
            let pid = match state.q_project {
                Some(id) => id,
                None => {
                    state.q_error = "Sin proyecto asociado.".to_string();
                    return Task::none();
                }
            };
            let ws = match state
                .projects
                .iter()
                .find(|p| p.id == pid)
                .and_then(|p| p.path.as_deref())
            {
                Some(raw) => {
                    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
                        format!(
                            "{}/{rest}",
                            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                        )
                    } else {
                        raw.to_string()
                    };
                    std::path::PathBuf::from(expanded)
                }
                None => {
                    state.q_error = "El proyecto perdió su workspace.".to_string();
                    return Task::none();
                }
            };
            let content = match questionnaire::generate_spec(&state.q_answers, &ws) {
                Ok(c) => c,
                Err(e) => {
                    state.q_error = e;
                    return Task::none();
                }
            };
            match questionnaire::save_spec(&ws, &content) {
                Ok(file) => {
                    state.status = format!("ESPEC.md generado en {}", file.display());
                    state.push_log(format!("📋 ESPEC.md -> {}", file.display()));
                    state.view = View::Chat;
                }
                Err(e) => state.q_error = e,
            }
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}