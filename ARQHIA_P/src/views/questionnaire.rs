//! Cuestionario v0.8.1: universal (Categoría+Tipo, condicional) + import.
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::questionnaire::{self, levels, Categoria, QKind, SysType};

pub(crate) fn view_questionnaire(state: &App) -> Element<'_, Message> {    use iced::widget::{column, container, pick_list, progress_bar, row, text};
    use crate::ui::{components, design};
    use crate::ui::design::{Tone, type_scale};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let level = state.q_level;
    let (cat, sys) = (state.q_answers.cat, state.q_answers.sys);
    let skip_nombre = state.q_answers.nombre_locked;
    let total = levels::total_steps(level, cat, sys, skip_nombre);
    let step = state.q_step.min(total - 1);
    let progress = (step + 1) as f32 / total as f32;

    let body: Element<'_, Message> = if step == 0 {
        let picker: Element<'_, Message> = pick_list(
            levels::Level::ALL.to_vec(),
            Some(level),
            Message::QLevelPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into();
        let mut col = column![
            text("¿Cuánta experiencia tienes con este tipo de proyecto?").size(design::fs(ts, type_scale::HEADLINE)),
            text("Las preguntas se adaptan: diseño y decisiones, sin código si empiezas.")
                .size(design::fs(ts, 12))
                .color(dim),
            picker,
        ]
        .spacing(10);
        if state.q_pending_level.is_some() {
            col = col.push(
                row![
                    components::primary_btn("Confirmar cambio".to_string(), 13)
                        .on_press(Message::QLevelConfirm),
                ]
                .spacing(8),
            );
        }
        col.into()
    } else if levels::is_ai_step(level, cat, sys, skip_nombre, step) {
        view_ai_step(state, ts)
    } else if let Some(q) = levels::step_question(level, cat, sys, skip_nombre, step) {
        view_question_field(state, &q, ts)
    } else {
        text("Paso desconocido.").into()
    };

    let mut nav = row![];
    if step > 0 {
        nav = nav.push(components::head_btn("Atrás".to_string()).on_press(Message::QBack));
    }
    nav = nav.push(components::head_btn("Cancelar".to_string()).on_press(Message::QCancel));
    nav = nav.push(iced::widget::horizontal_space());
    if levels::is_ai_step(level, cat, sys, skip_nombre, step) {
        nav = nav.push(components::primary_btn("Generar documentos".to_string(), 13).on_press(Message::FinishQuestionnaire));
    } else {
        nav = nav.push(components::primary_btn("Siguiente".to_string(), 13).on_press(Message::QNext));
    }

    let rail = row(
        (0..total)
            .map(|s| {
                let past = s < step;
                let current = s == step;
                container(text(format!("{}", s + 1)).size(design::fs(ts, 11)).color(
                    if past || current {
                        design::accent(&app_theme)
                    } else {
                        dim
                    }
                ))
                .padding([1, 7])
                .style(move |t: &Theme| design::badge(
                    t,
                    if past || current { Tone::Accent } else { Tone::Neutral }
                ))
                .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(6);

    let mut col = column![
        components::section_label(app_theme.clone(),
            format!("{} · {level} · {cat} · {sys} · paso {} de {total}", state.q_answers.nombre.trim(), step + 1)
        ),
        rail,
        text(questionnaire::step_title(level, cat, sys, skip_nombre, step)).size(design::fs(ts, 11)).color(dim),
        progress_bar(0.0..=1.0, progress)
            .height(4)
            .style(|t: &Theme| design::progress(t)),
    ]
    .spacing(10)
    .padding(20)
    .max_width(640);
    if !state.q_import_note.is_empty() {
        col = col.push(
            text(&state.q_import_note).size(design::fs(ts, 12)).color(design::accent(&app_theme)),
        );
    }
    col = col.push(body);
    if !state.q_error.is_empty() {
        col = col.push(
            row![
                components::dot(app_theme.clone(), Tone::Err),
                text(&state.q_error)
                    .size(design::fs(ts, 13))
                    .color(design::tone(&app_theme, Tone::Err)),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
    }
    // Aviso GPL visible al elegirla.
    if state.q_answers.licencia == questionnaire::Licencia::Gpl3 {
        col = col.push(
            text("Aviso: GPL-3.0 obliga a liberar los derivados con la misma licencia.")
                .size(design::fs(ts, 12))
                .color(design::tone(&app_theme, Tone::Warn)),
        );
    }
    col = col.push(nav.spacing(8));

    container(container(col).style(|t: &Theme| design::card(t)))
        .width(iced::Fill)
        .height(iced::Fill)
        .center_x(iced::Fill)
        .center_y(iced::Fill)
        .padding(24)
        .into()
}

fn view_ai_list(state: &App, ts: f32) -> Element<'_, Message> {
    use iced::widget::{column, text, text_input};
    use crate::ui::{components, design};
    let mut col = column![].spacing(4);
    for (i, q) in state.q_ai_questions.iter().enumerate() {
        let ans: &str = state.q_ai_answers.get(i).map(String::as_str).unwrap_or("");
        col = col.push(
            column![
                text(format!("{}. {q}", i + 1)).size(design::fs(ts, 13)),
                text_input("Tu respuesta (opcional)...", ans)
                    .id(iced::widget::text_input::Id::new(format!("qai-{i}")))
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(move |v| Message::QAiAnswerChanged(i, v)),
            ]
            .spacing(4),
        );
    }
    if !state.q_ai_loading {
        col = col.push(
            components::quiet_btn("Regenerar".to_string()).on_press(Message::QAiGenerate),
        );
    }
    col.into()
}

fn view_question_field<'a>(state: &'a App, q: &levels::Question, ts: f32) -> Element<'a, Message> {
    use iced::widget::{checkbox, column, pick_list, text, text_input};
    use crate::ui::design;
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let a = &state.q_answers;
    let input: Element<'a, Message> = match q.kind {
        QKind::Line => text_input("Ej. MiApp...", &a.nombre)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::QNombreChanged)
            .on_submit(Message::QNext)
            .into(),
        QKind::Area => {
            let (value, on_in): (&str, fn(String) -> Message) = match q.id {
                "descripcion" => (&a.descripcion, Message::QDescChanged),
                "objetivo" => (&a.objetivo, Message::QObjChanged),
                "uso_previsto" => (&a.uso_previsto, Message::QUsoPrevistoChanged),
                "publico_objetivo" => (&a.publico_objetivo, Message::QPublicoChanged),
                "funcionalidades" => (&a.funcionalidades, Message::QFuncChanged),
                "ui_ux" => (&a.ui_ux, Message::QUiUxChanged),
                "endpoints" => (&a.endpoints, Message::QEndpointsChanged),
                "escala" => (&a.escala, Message::QEscalaChanged),
                "api_publica" => (&a.api_publica, Message::QApiPublicaChanged),
                "ejemplos" => (&a.ejemplos, Message::QEjemplosChanged),
                "arranque" => (&a.arranque, Message::QArranqueChanged),
                "compat" => (&a.compat, Message::QCompatChanged),
                "inputs" => (&a.inputs_secretos, Message::QInputsChanged),
                "idempotencia" => (&a.idempotencia, Message::QIdempotenciaChanged),
                "dataset" => (&a.dataset, Message::QDatasetChanged),
                "pipeline" => (&a.pipeline_desc, Message::QPipelineChanged),
                "modelo" => (&a.modelo_eval, Message::QModeloEvalChanged),
                "sintaxis" => (&a.sintaxis, Message::QSintaxisChanged),
                "toolchain" => (&a.toolchain, Message::QToolchainChanged),
                "syscalls" => (&a.syscalls, Message::QSyscallsChanged),
                "host_api" => (&a.host_api, Message::QHostApiChanged),
                "oss_repo" => (&a.oss_repo, Message::QOssRepoChanged),
                "oss_gobierno" => (&a.oss_gobierno, Message::QOssGobiernoChanged),
                "oss_contrib" => (&a.oss_contrib, Message::QOssContribChanged),
                // Inalcanzable si la matriz y el match están sincronizados.
                // Se mantiene descripcion como fallback visible (antes causó
                // duplicados al faltar uso_previsto/publico_objetivo).
                _ => (&a.descripcion, Message::QDescChanged),
            };
            text_input("Escribe aquí...", value)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(on_in)
                .on_submit(Message::QNext)
                .into()
        }
        QKind::PickCat => pick_list(
            Categoria::ALL.to_vec(),
            Some(a.cat),
            Message::QCatPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickSys => {
            let opts = SysType::for_cat(a.cat).to_vec();
            pick_list(opts, Some(a.sys), Message::QSysPicked)
                .style(|t: &Theme, s| design::field_pick(t, s))
                .width(320)
                .into()
        }
        QKind::PickEstilo => {
            let pick: Element<'a, Message> = pick_list(
                questionnaire::EstiloPreset::ALL.to_vec(),
                Some(a.estilo),
                Message::QEstiloPicked,
            )
            .style(|t: &Theme, s| design::field_pick(t, s))
            .width(280)
            .into();
            column![
                pick,
                text_input("Referencia (opcional, ej. web, app, color...)", &a.estilo_free)
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::QEstiloFreeChanged)
                    .on_submit(Message::QNext),
            ]
            .spacing(8)
            .into()
        }
        QKind::MultiPlataforma => {
            let mut opts = column![].spacing(4);
            for p in questionnaire::Plataforma::ALL {
                let checked = a.plataformas.contains(&p);
                opts = opts.push(
                    checkbox(p.to_string(), checked).on_toggle(move |_| Message::QPlataformaToggled(p)),
                );
            }
            opts.into()
        }
        QKind::MultiStack => {
            let mut opts = column![].spacing(4);
            for s in questionnaire::StackOpt::ALL {
                let checked = a.stacks.contains(&s);
                opts = opts.push(
                    checkbox(s.to_string(), checked).on_toggle(move |_| Message::QStackToggled(s)),
                );
            }
            opts = opts.push(
                text_input("Detalle (opcional, ej. versión, framework...)", &a.stack_free)
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::QStackFreeChanged)
                    .on_submit(Message::QNext),
            );
            opts.into()
        }
        QKind::PickFacturacion => pick_list(
            questionnaire::Facturacion::ALL.to_vec(),
            Some(a.facturacion),
            Message::QFacturacionPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickLicencia => pick_list(
            questionnaire::Licencia::ALL.to_vec(),
            Some(a.licencia),
            Message::QLicenciaPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickApiStyle => pick_list(
            questionnaire::ApiStyle::ALL.to_vec(),
            Some(a.api_style),
            Message::QApiStylePicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickAuth => pick_list(
            questionnaire::AuthKind::ALL.to_vec(),
            Some(a.auth),
            Message::QAuthPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickTrigger => pick_list(
            questionnaire::Trigger::ALL.to_vec(),
            Some(a.trigger),
            Message::QTriggerPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickSemver => pick_list(
            questionnaire::SemverOpt::ALL.to_vec(),
            Some(a.semver),
            Message::QSemverPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::MultiArch => {
            let mut opts = column![].spacing(4);
            for arch in questionnaire::ArchOpt::ALL {
                let checked = a.archs.contains(&arch);
                opts = opts.push(
                    checkbox(arch.to_string(), checked).on_toggle(move |_| Message::QArchToggled(arch)),
                );
            }
            opts.into()
        }
    };
    let mut col = column![
        text(q.title).size(design::fs(ts, crate::ui::design::type_scale::HEADLINE)),
    ]
    .spacing(10);
    if let Some(hint) = q.hint {
        col = col.push(text(hint).size(design::fs(ts, 12)).color(dim));
    }
    col = col.push(input);
    col.into()
}

fn view_ai_step(state: &App, ts: f32) -> Element<'_, Message> {
    use iced::widget::{column, row, text};
    use crate::ui::{components, design};
    use crate::ui::design::{Tone, type_scale};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let provider = state.config.active;
    let has_api = state.config.active_config().is_configured_for(provider);
    let mut col = column![
        text("¿Afinamos con la IA? (opcional)").size(design::fs(ts, type_scale::HEADLINE)),
        text("El provider activo propone 5–10 preguntas adaptadas a lo respondido. \
            Puedes responderlas o saltar este paso: los documentos se generan igual.")
            .size(design::fs(ts, 12))
            .color(dim),
    ]
    .spacing(10);
    if !has_api && state.q_source == questionnaire::QSource::New {
        col = col.push(
            text("Sin API configurada: este paso está deshabilitado. Pulsa Generar documentos para continuar.")
                .size(design::fs(ts, 12))
                .color(dim),
        );
    } else if state.q_ai_questions.is_empty() {
        let btn = if state.q_ai_loading {
            components::head_btn("Generando...".to_string())
        } else {
            components::primary_btn("Generar preguntas adicionales (IA)".to_string(), 13)
                .on_press(Message::QAiGenerate)
        };
        col = col.push(row![btn].spacing(8));
    } else {
        col = col.push(view_ai_list(state, ts));
    }
    if state.q_ai_loading {
        col = col.push(text("Pensando...").size(design::fs(ts, 12)).color(dim));
    }
    if !state.q_ai_error.is_empty() {
        col = col.push(
            row![
                components::dot(app_theme.clone(), Tone::Err),
                text(&state.q_ai_error)
                    .size(design::fs(ts, 13))
                    .color(design::tone(&app_theme, Tone::Err)),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
    }
    col.into()
}

/// Pantalla de carga MVP (v0.8.2): la IA genera CONTEXT + serie v0.1→v1.0
/// sin pasar por el chat. Solo progreso + Detener.
pub(crate) fn view_generating(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, progress_bar, text};
    use crate::ui::{components, design};
    use crate::ui::design::type_scale;
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let mut col = column![
        text("Generando tu proyecto…").size(design::fs(ts, type_scale::HEADLINE)),
        text(&state.gen_phase).size(design::fs(ts, 12)).color(dim),
        progress_bar(0.0..=1.0, state.gen_progress.clamp(0.0, 1.0))
            .height(6)
            .style(|t: &Theme| design::progress(t)),
        text("CONTEXT + ROADMAP + VERSIONS (v0.1 → v1.0 MVP) + ToDo. Sin chat: es un proceso previo al desarrollo.")
            .size(design::fs(ts, 12))
            .color(dim),
    ]
    .spacing(12)
    .padding(24)
    .max_width(560);
    if !state.gen_error.is_empty() {
        col = col.push(
            text(&state.gen_error)
                .size(design::fs(ts, 13))
                .color(design::tone(&app_theme, design::Tone::Err)),
        );
    }
    col = col.push(
        iced::widget::row![
            components::danger_btn("Detener".to_string()).on_press(Message::GenCancel),
        ]
        .spacing(8),
    );
    container(container(col).style(|t: &Theme| design::card(t)))
        .width(iced::Fill)
        .height(iced::Fill)
        .center_x(iced::Fill)
        .center_y(iced::Fill)
        .padding(24)
        .into()
}
