//! Handler del cuestionario v0.8.1 (universal + import).
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;
use crate::questionnaire::{self, levels};

fn questionnaire_ws(state: &App) -> Result<std::path::PathBuf, String> {
    let pid = state.q_project.ok_or_else(|| "Sin proyecto asociado.".to_string())?;
    let raw = state
        .projects
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.path.clone())
        .ok_or_else(|| "El proyecto perdió su workspace.".to_string())?;
    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
        format!("{}/{rest}", std::env::var("ARQHIA_HOME").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string()))
    } else {
        raw
    };
    Ok(std::path::PathBuf::from(expanded))
}

fn rollback_unfinished(state: &mut App) {
    let Some(pid) = state.q_project else {
        return;
    };
    state.q_owns_project = false;
    state.q_project = None;
    // En import la carpeta con código se conserva siempre (solo se retira
    // la fila si quedó vacía; con contenido nunca se toca).
    if let Some(raw) = state
        .projects
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.path.clone())
    {
        let expanded = if let Some(rest) = raw.strip_prefix("~/") {
            format!("{}/{rest}", std::env::var("ARQHIA_HOME").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string()))
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

fn steps_of(state: &App) -> (usize, usize) {
    let a = &state.q_answers;
    let total = levels::total_steps(state.q_level, a.cat, a.sys, a.nombre_locked);
    (state.q_step.min(total - 1), total)
}

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::QNext => {
            let a = &state.q_answers;
            if let Some(err) =
                questionnaire::validate_step(state.q_level, a.cat, a.sys, state.q_step, &state.q_answers)
            {
                state.q_error = err;
            } else {
                state.q_error.clear();
                let (_, total) = steps_of(state);
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
            state.q_import_note.clear();
            state.q_source = questionnaire::QSource::New;
            state.view = if rolled_back { View::Home } else { View::Chat };
            Task::none()
        }
        Message::QLevelPicked(level) => {
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
        Message::QUsoPrevistoChanged(v) => {
            state.q_answers.uso_previsto = v;
            Task::none()
        }
        Message::QPublicoChanged(v) => {
            state.q_answers.publico_objetivo = v;
            Task::none()
        }
        Message::QFuncChanged(v) => {
            state.q_answers.funcionalidades = v;
            Task::none()
        }
        Message::QCatPicked(c) => {
            state.q_answers.set_cat(c);
            state.q_ai_questions.clear();
            state.q_ai_answers.clear();
            Task::none()
        }
        Message::QSysPicked(s) => {
            state.q_answers.set_sys(s);
            state.q_ai_questions.clear();
            state.q_ai_answers.clear();
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
        Message::QPlataformaToggled(p) => {
            state.q_answers.toggle_plataforma(p);
            Task::none()
        }
        Message::QFacturacionPicked(f) => {
            state.q_answers.facturacion = f;
            Task::none()
        }
        Message::QLicenciaPicked(l) => {
            state.q_answers.licencia = l;
            Task::none()
        }
        Message::QApiStylePicked(s) => {
            state.q_answers.api_style = s;
            Task::none()
        }
        Message::QAuthPicked(a) => {
            state.q_answers.auth = a;
            Task::none()
        }
        Message::QTriggerPicked(t) => {
            state.q_answers.trigger = t;
            Task::none()
        }
        Message::QSemverPicked(s) => {
            state.q_answers.semver = s;
            Task::none()
        }
        Message::QArchToggled(a) => {
            state.q_answers.toggle_arch(a);
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
        Message::QEndpointsChanged(v) => {
            state.q_answers.endpoints = v;
            Task::none()
        }
        Message::QEscalaChanged(v) => {
            state.q_answers.escala = v;
            Task::none()
        }
        Message::QApiPublicaChanged(v) => {
            state.q_answers.api_publica = v;
            Task::none()
        }
        Message::QEjemplosChanged(v) => {
            state.q_answers.ejemplos = v;
            Task::none()
        }
        Message::QArranqueChanged(v) => {
            state.q_answers.arranque = v;
            Task::none()
        }
        Message::QCompatChanged(v) => {
            state.q_answers.compat = v;
            Task::none()
        }
        Message::QInputsChanged(v) => {
            state.q_answers.inputs_secretos = v;
            Task::none()
        }
        Message::QIdempotenciaChanged(v) => {
            state.q_answers.idempotencia = v;
            Task::none()
        }
        Message::QDatasetChanged(v) => {
            state.q_answers.dataset = v;
            Task::none()
        }
        Message::QPipelineChanged(v) => {
            state.q_answers.pipeline_desc = v;
            Task::none()
        }
        Message::QModeloEvalChanged(v) => {
            state.q_answers.modelo_eval = v;
            Task::none()
        }
        Message::QSintaxisChanged(v) => {
            state.q_answers.sintaxis = v;
            Task::none()
        }
        Message::QToolchainChanged(v) => {
            state.q_answers.toolchain = v;
            Task::none()
        }
        Message::QSyscallsChanged(v) => {
            state.q_answers.syscalls = v;
            Task::none()
        }
        Message::QHostApiChanged(v) => {
            state.q_answers.host_api = v;
            Task::none()
        }
        Message::QOssRepoChanged(v) => {
            state.q_answers.oss_repo = v;
            Task::none()
        }
        Message::QOssGobiernoChanged(v) => {
            state.q_answers.oss_gobierno = v;
            Task::none()
        }
        Message::QOssContribChanged(v) => {
            state.q_answers.oss_contrib = v;
            Task::none()
        }
        Message::QAiAnswerChanged(i, v) => {
            if i < state.q_ai_answers.len() {
                state.q_ai_answers[i] = v;
                state.q_ai_focus = i;
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
                // Sin provider: fallback estático de huecos en modo import.
                if state.q_source == questionnaire::QSource::Import {
                    let dir = questionnaire_ws(state).ok();
                    let scan = dir
                        .as_deref()
                        .map(questionnaire::import::scan)
                        .unwrap_or_default();
                    let qs = questionnaire::import::gap_fallback(&scan);
                    state.q_ai_answers = vec![String::new(); qs.len()];
                    state.q_ai_questions = qs;
                    state.q_ai_error.clear();
                    return Task::none();
                }
                state.q_ai_error =
                    "Configura una API en Configuración para usar la IA (o pulsa Saltar).".to_string();
                return Task::none();
            }
            let level = state.q_level;
            let a = state.q_answers.clone();
            let source = state.q_source.clone();
            // Gaps del import se calculan aquí (síncrono): el async no puede
            // pedir prestado `state`.
            let import_gaps: Vec<String> = if source == questionnaire::QSource::Import {
                let dir = questionnaire_ws(state).ok();
                let scan = dir
                    .as_deref()
                    .map(questionnaire::import::scan)
                    .unwrap_or_default();
                questionnaire::import::gap_fields(&scan).iter().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            };
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
                    let prompt = if source == questionnaire::QSource::Import {
                        let refs: Vec<&str> = import_gaps.iter().map(String::as_str).collect();
                        questionnaire::ai::import_gap_prompt(&summary, &refs)
                    } else {
                        questionnaire::ai::ai_prompt(level, &summary)
                    };
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
        Message::QAiFocusCycle(forward) => {
            // Tab entre respuestas IA: cicla el foco sin tocar el texto.
            let n = state.q_ai_questions.len();
            if state.view != View::Questionnaire || n == 0 {
                return Task::none();
            }
            let cur = state.q_ai_focus.min(n - 1);
            let next = if forward {
                (cur + 1) % n
            } else {
                (cur + n - 1) % n
            };
            state.q_ai_focus = next;
            iced::widget::text_input::focus(format!("qai-{next}"))
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
            let (.., total) = steps_of(state);
            // Seguridad: el nombre siempre existe (vino de la creación o se preguntó).
            if state.q_answers.nombre.trim().is_empty() {
                state.q_step = 0;
                state.q_error = "Pon un nombre al proyecto.".to_string();
                return Task::none();
            }
            for step in 0..total {
                let a = &state.q_answers;
                if levels::is_ai_step(state.q_level, a.cat, a.sys, a.nombre_locked, step) {
                    continue;
                }
                if let Some(err) =
                    questionnaire::validate_step(state.q_level, a.cat, a.sys, step, &state.q_answers)
                {
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
            let source = state.q_source.clone();
            let (project, specs, context) =
                match questionnaire::templates::generate_project_docs(level, &state.q_answers, &ai, &source) {
                    Ok(d) => d,
                    Err(e) => {
                        state.q_error = e;
                        return Task::none();
                    }
                };
            // Merge sin borrar: layout idempotente, nunca toca `Project/`.
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
                    // v0.8.2: pantalla de carga MVP (sin pasar por el chat).
                    let auto = questionnaire::planning::AutoPlanCtx {
                        ws: ws.clone(),
                        level,
                        answers: state.q_answers.clone(),
                        ai: ai.clone(),
                        source: source.clone(),
                    };
                    let provider = state.config.active;
                    let cfg = state.config.active_config();
                    state.q_owns_project = false;
                    state.q_error.clear();
                    if !cfg.is_configured_for(provider) {
                        // Sin API: serie determinista offline, sin LLM.
                        match questionnaire::planning::write_mvp_docs(&auto, &[]) {
                            Ok(report) => {
                                for f in &report.files {
                                    state.push_log(format!("📋 {f} (offline)"));
                                }
                                clear_questionnaire_state(state);
                                let summary = format!(
                                    "Proyecto documentado (offline, sin API): {} + serie MVP v0.1→v1.0. \
                                    Configura una API para regenerar con IA.",
                                    report.files.join(", ")
                                );
                                enter_chat_after_generation(state, &summary);
                                state.status = "Cuestionario listo (offline): configura una API para la IA.".to_string();
                            }
                            Err(e) => state.q_error = e,
                        }
                        return Task::none();
                    }
                    state.gen_run += 1;
                    let run = state.gen_run;
                    state.gen_active = true;
                    state.gen_phase = "Generando CONTEXT y serie MVP (v0.1 → v1.0)…".to_string();
                    state.gen_progress = 0.2;
                    state.gen_error.clear();
                    state.plan_auto = Some(auto.clone());
                    state.view = View::Generating;
                    state.push_log("Generando serie MVP: ROADMAP → VERSIONS → v0.1…v1.0 → ToDo".to_string());
                    return Task::perform(
                        async move {
                            let prompt = questionnaire::planning::mvp_prompt(
                                level,
                                &auto.answers,
                                &auto.ai,
                                &auto.source,
                            );
                            match crate::agent::simple_chat(
                                provider,
                                "Eres el planificador de ARQHIA. Devuelves solo bloques ---FILE: <ruta>---.",
                                &prompt,
                                &cfg,
                            )
                            .await
                            {
                                Ok(raw) => (run, Ok(questionnaire::planning::parse_mvp_files(&raw))),
                                Err(e) => (run, Err(e)),
                            }
                        },
                        |(run, res)| Message::GenDone(run, res),
                    );
                }
                Err(e) => state.q_error = e,
            }
            Task::none()
        }
        Message::GenDone(run, res) => {
            if !state.gen_active || run != state.gen_run {
                return Task::none();
            }
            state.gen_active = false;
            let Some(auto) = state.plan_auto.take() else {
                state.gen_error = "Se perdió el contexto de generación.".to_string();
                return Task::none();
            };
            let files: Vec<(String, String)> = match res {
                Ok(f) if !f.is_empty() => f,
                Ok(_) => {
                    state.push_log("La IA no devolvió archivos: fallback determinista.".to_string());
                    Vec::new()
                }
                Err(e) => {
                    state.push_log(format!("IA: {} (fallback determinista).", crate::llm::friendly_error(&e)));
                    Vec::new()
                }
            };
            match questionnaire::planning::write_mvp_docs(&auto, &files) {
                Ok(report) => {
                    for f in &report.files {
                        state.push_log(format!("📋 {f}"));
                    }
                    for w in &report.warnings {
                        state.push_log(format!("⚠️ {w}"));
                    }
                    let gaps = if report.gaps.is_empty() {
                        "sin gaps".to_string()
                    } else {
                        format!("gaps: {}", report.gaps.join(", "))
                    };
                    clear_questionnaire_state(state);
                    let summary = format!(
                        "Serie MVP generada: {} ({gaps}). Revisa ROADMAP.md y VERSIONS/v1.0.md.",
                        report.files.join(", ")
                    );
                    enter_chat_after_generation(state, &summary);
                    state.status = "Proyecto listo: serie MVP generada.".to_string();
                }
                Err(e) => {
                    state.gen_error = e;
                    state.plan_auto = Some(auto);
                    state.gen_active = false;
                    return Task::none();
                }
            }
            Task::none()
        }
        Message::GenCancel => {
            if !state.gen_active {
                return Task::none();
            }
            state.gen_run += 1;
            state.gen_active = false;
            state.plan_auto = None;
            state.gen_phase.clear();
            state.gen_progress = 0.0;
            // Vuelve al último paso (IA) para reintentar el Finish.
            let total = levels::total_steps(state.q_level, state.q_answers.cat, state.q_answers.sys, state.q_answers.nombre_locked);
            state.q_step = total.saturating_sub(1);
            state.view = View::Questionnaire;
            state.status = "Generación cancelada: puedes reintentar con Generar documentos.".to_string();
            state.push_log("Generación MVP cancelada.".to_string());
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Limpia el estado del wizard tras una generación completa.
fn clear_questionnaire_state(state: &mut App) {
    state.q_step = 0;
    state.q_error.clear();
    state.q_ai_questions.clear();
    state.q_ai_answers.clear();
    state.q_ai_error.clear();
    state.q_pending_level = None;
    state.q_import_note.clear();
    state.q_source = questionnaire::QSource::New;
    state.q_owns_project = false;
}

/// Abre el chat "Plan inicial" en modo Plan con un resumen corto (la serie
/// MVP ya está escrita en disco; no hay prompt gigante ni turno en curso).
fn enter_chat_after_generation(state: &mut App, summary: &str) {
    let Some(pid) = state.q_project else {
        state.status = "Proyecto documentado en CONTEXT/.".to_string();
        return;
    };
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
            state.view = View::Chat;
            state.push_log(summary.to_string());
            state.push_log("Modo Plan: revisa ROADMAP + VERSIONS/v1.0.md y sigue en Work.".to_string());
        }
        Err(e) => {
            state.status = format!("Proyecto documentado, pero no se pudo abrir el Plan: {e}");
        }
    }
}
