//! Handler del cuestionario v0.8 (genérico + por nivel + IA opcional).
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;
use crate::questionnaire::{self, levels};

/// Resuelve el workspace del proyecto del cuestionario (expande `~`).
fn questionnaire_ws(state: &App) -> Result<std::path::PathBuf, String> {
    let pid = state.q_project.ok_or_else(|| "Sin proyecto asociado.".to_string())?;
    let raw = state
        .projects
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.path.clone())
        .ok_or_else(|| "El proyecto perdió su workspace.".to_string())?;
    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
        format!("{}/{rest}", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    } else {
        raw
    };
    Ok(std::path::PathBuf::from(expanded))
}

/// Deshace la creación del proyecto si el cuestionario se cancela sin
/// terminar (v0.8.1): borra la fila de la DB + el estado, y la carpeta solo
/// si quedó vacía (recién creada por el flujo). Una carpeta con contenido
/// del usuario nunca se toca.
fn rollback_unfinished(state: &mut App) {
    let Some(pid) = state.q_project else {
        return;
    };
    state.q_owns_project = false;
    state.q_project = None;
    if let Some(raw) = state
        .projects
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.path.clone())
    {
        let expanded = if let Some(rest) = raw.strip_prefix("~/") {
            format!("{}/{rest}", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        } else {
            raw
        };
        let dir = std::path::PathBuf::from(expanded);
        let empty = dir.is_dir()
            && std::fs::read_dir(&dir).map(|mut it| it.next().is_none()).unwrap_or(false);
        if empty {
            let _ = std::fs::remove_dir(&dir);
        }
    }
    let _ = crate::db::delete_project(pid);
    state.projects.retain(|p| p.id != pid);
    if state.pending_project == Some(pid) {
        state.pending_project = None;
    }
    // No debería haber chats (el cuestionario no conversa), pero si los
    // hubiera quedan sueltos, no borrados (delete_project los desasigna).
    if let Some(active) = state.active_chat
        && state.chats.iter().any(|c| c.id == active && c.project_id == Some(pid))
    {
        state.active_chat = None;
        state.messages.clear();
        state.md.clear();
        state.msg_times.clear();
        state.msg_ids.clear();
        state.msg_usage.clear();
    }
    state.push_log("Cuestionario cancelado: proyecto descartado.".to_string());
}

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::QNext => {
            let total = levels::total_steps(state.q_level);
            if let Some(err) = questionnaire::validate_step(state.q_level, state.q_step, &state.q_answers) {
                state.q_error = err;
            } else {
                state.q_error.clear();
                if state.q_step + 1 < total {
                    state.q_step += 1;
                }
            }
            Task::none()
        }
        Message::QBack => {
            state.q_error.clear();
            state.q_ai_error.clear();
            state.q_pending_level = None;
            state.q_step = state.q_step.saturating_sub(1);
            Task::none()
        }
        Message::QCancel => {
            // Si el proyecto nació en este cuestionario y no terminó, se
            // deshace la creación (sin proyectos fantasma). Si era previo,
            // se conserva y se vuelve al Chat.
            let rolled_back = state.q_owns_project && state.q_project.is_some();
            if rolled_back {
                rollback_unfinished(state);
            }
            state.q_step = 0;
            state.q_error.clear();
            state.q_pending_level = None;
            state.q_ai_questions.clear();
            state.q_ai_answers.clear();
            state.q_ai_loading = false;
            state.q_ai_error.clear();
            state.view = if rolled_back { View::Home } else { View::Chat };
            Task::none()
        }
        Message::QLevelPicked(level) => {
            // Cambiar de nivel con respuestas de nivel las descarta: pide
            // confirmación inline (segunda pulsación confirma).
            if level != state.q_level && state.q_answers.has_level_content() {
                if state.q_pending_level == Some(level) {
                    state.q_answers.clear_level();
                    state.q_level = level;
                    state.q_pending_level = None;
                    state.q_error.clear();
                    state.q_ai_questions.clear();
                    state.q_ai_answers.clear();
                } else {
                    state.q_pending_level = Some(level);
                    state.q_error = format!(
                        "Cambiar a {level} borra las respuestas de nivel. \
                        Pulsa {level} otra vez para confirmar."
                    );
                }
            } else {
                state.q_level = level;
                state.q_pending_level = None;
                state.q_error.clear();
            }
            Task::none()
        }
        Message::QLevelConfirm => {
            if let Some(level) = state.q_pending_level {
                state.q_answers.clear_level();
                state.q_level = level;
                state.q_pending_level = None;
                state.q_error.clear();
                state.q_ai_questions.clear();
                state.q_ai_answers.clear();
            }
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
        Message::QObjChanged(v) => {
            state.q_answers.objetivo = v;
            Task::none()
        }
        Message::QFuncChanged(v) => {
            state.q_answers.funcionalidades = v;
            Task::none()
        }
        Message::QEstiloPicked(e) => {
            state.q_answers.estilo = e;
            Task::none()
        }
        Message::QEstiloFreeChanged(v) => {
            state.q_answers.estilo_free = v;
            Task::none()
        }
        Message::QUiUxChanged(v) => {
            state.q_answers.ui_ux = v;
            Task::none()
        }
        Message::QTipoPicked(t) => {
            state.q_answers.tipo = t;
            Task::none()
        }
        Message::QPlataformaToggled(p) => {
            state.q_answers.toggle_plataforma(p);
            Task::none()
        }
        Message::QFacturacionPicked(f) => {
            state.q_answers.facturacion = f;
            Task::none()
        }
        Message::QStackToggled(s) => {
            state.q_answers.toggle_stack(s);
            Task::none()
        }
        Message::QStackFreeChanged(v) => {
            state.q_answers.stack_free = v;
            Task::none()
        }
        Message::QArqPicked(a) => {
            state.q_answers.arq = a;
            Task::none()
        }
        Message::QArqChanged(v) => {
            state.q_answers.arquitectura = v;
            Task::none()
        }
        Message::QAiAnswerChanged(i, v) => {
            if i < state.q_ai_answers.len() {
                state.q_ai_answers[i] = v;
            }
            Task::none()
        }
        Message::QAiGenerate => {
            if state.q_ai_loading {
                return Task::none();
            }
            let provider = state.config.active;
            let cfg = state.config.active_config();
            if !cfg.is_configured_for(provider) {
                state.q_ai_error =
                    "Configura una API en Configuración para usar la IA (o pulsa Saltar).".to_string();
                return Task::none();
            }
            let level = state.q_level;
            let a = state.q_answers.clone();
            state.q_ai_loading = true;
            state.q_ai_error.clear();
            Task::perform(
                async move {
                    let summary = questionnaire::ai::answers_summary(
                        a.nombre.trim(),
                        a.descripcion.trim(),
                        a.objetivo.trim(),
                        a.funcionalidades.trim(),
                        &a.level_extras(level),
                    );
                    let prompt = questionnaire::ai::ai_prompt(level, &summary);
                    match crate::agent::simple_chat(
                        provider,
                        "Eres el ayudante de definición de proyectos de ARQHIA.",
                        &prompt,
                        &cfg,
                    )
                    .await
                    {
                        Ok(raw) => Ok(questionnaire::ai::parse_ai_questions(&raw)),
                        Err(e) => Err(e),
                    }
                },
                Message::QAiGenerated,
            )
        }
        Message::QAiGenerated(res) => {
            state.q_ai_loading = false;
            match res {
                Ok(qs) if !qs.is_empty() => {
                    state.q_ai_error.clear();
                    state.q_ai_answers = vec![String::new(); qs.len()];
                    state.q_ai_questions = qs;
                }
                Ok(_) => {
                    state.q_ai_error = "La IA no devolvió preguntas. Reintenta o pulsa Saltar.".to_string();
                }
                Err(e) => {
                    state.q_ai_error = format!("IA: {}", crate::llm::friendly_error(&e));
                }
            }
            Task::none()
        }
        Message::FinishQuestionnaire => {
            // Valida genéricas + nivel (la IA es opcional y nunca bloquea).
            let total = levels::total_steps(state.q_level);
            for step in 0..total {
                if levels::is_ai_step(state.q_level, step) {
                    continue;
                }
                if let Some(err) = questionnaire::validate_step(state.q_level, step, &state.q_answers) {
                    state.q_step = step;
                    state.q_error = err;
                    return Task::none();
                }
            }
            let ws = match questionnaire_ws(state) {
                Ok(w) => w,
                Err(e) => {
                    state.q_error = e;
                    return Task::none();
                }
            };
            let level = state.q_level;
            let ai: Vec<(String, String)> = state
                .q_ai_questions
                .iter()
                .zip(state.q_ai_answers.iter())
                .map(|(q, r)| (q.clone(), r.clone()))
                .collect();
            let (project, specs, context) =
                match questionnaire::templates::generate_project_docs(level, &state.q_answers, &ai) {
                    Ok(d) => d,
                    Err(e) => {
                        state.q_error = e;
                        return Task::none();
                    }
                };
            // Estructura + documentos + migración legacy (todo idempotente).
            if let Err(e) = crate::workspace::ensure_project_layout(&ws) {
                state.q_error = e;
                return Task::none();
            }
            match crate::workspace::save_project_docs(&ws, &project, &specs, &context) {
                Ok(files) => {
                    for f in &files {
                        state.push_log(format!("📋 {f}"));
                    }
                    if let Some(line) = crate::workspace::migrate_espec(&ws) {
                        state.push_log(line);
                    }
                    // El proyecto ya es legítimo: no hay rollback.
                    state.q_owns_project = false;
                    // Limpia el wizard para la próxima vez.
                    state.q_step = 0;
                    state.q_error.clear();
                    state.q_ai_questions.clear();
                    state.q_ai_answers.clear();
                    state.q_ai_error.clear();
                    state.q_pending_level = None;
                    // Entra en modo Plan: chat nuevo en Plan con el prompt
                    // inicial precargado (versiones, dependencias, contexto,
                    // ToDo). El usuario revisa y pulsa Enviar.
                    enter_plan_after_questionnaire(state);
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

/// Tras un cuestionario completado: abre un chat en modo Plan dentro del
/// proyecto con el prompt de planificación precargado en la entrada.
/// No dispara el turno solo: el usuario revisa y pulsa Enviar.
fn enter_plan_after_questionnaire(state: &mut App) {
    let Some(pid) = state.q_project else {
        state.status = "Proyecto documentado en CONTEXT/.".to_string();
        return;
    };
    let prompt = "A partir de CONTEXT/PROJECT.md y CONTEXT/SPECS.md, genera la \
        planificación inicial: ROADMAP.md, VERSIONS.md y VERSIONS/v0.1.md en \
        CONTEXT/, las dependencias detectadas y la lista de tareas en ToDo.md. \
        Todavía sin tocar código."
        .to_string();
    match crate::db::create_chat("Plan inicial") {
        Ok(new_id) => {
            let _ = crate::db::move_chat(new_id, Some(pid));
            let _ = crate::db::set_chat_mode(new_id, crate::db::Mode::Plan);
            state.chats.push(crate::db::ChatMeta {
                id: new_id,
                title: "Plan inicial".to_string(),
                project_id: Some(pid),
                archived: false,
                mode: crate::db::Mode::Plan,
                session_id: None,
            });
            state.active_chat = Some(new_id);
            state.messages.clear();
            state.md.clear();
            state.msg_times.clear();
            state.msg_ids.clear();
            state.msg_usage.clear();
            crate::app::state::clear_turn_state(state);
            state.pending_project = None;
            state.input = prompt;
            state.status =
                "Cuestionario listo: revisa el prompt y pulsa Enviar para generar el plan.".to_string();
            state.push_log("Modo Plan: genera ROADMAP + VERSIONS + ToDo.".to_string());
        }
        Err(e) => {
            state.status = format!("Proyecto documentado, pero no se pudo abrir el Plan: {e}");
        }
    }
}
