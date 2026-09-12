//! Cuestionario v0.8: nivel + genéricas + por nivel + IA opcional.
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::questionnaire::{self, levels, ArqPreset, EstiloPreset, Facturacion, Level, QKind, StackOpt, TipoProyecto};

pub(crate) fn view_questionnaire(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, pick_list, progress_bar, row, text};
    use crate::ui::{components, design};
    use crate::ui::design::{Tone, type_scale};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let level = state.q_level;
    let total = levels::total_steps(level);
    let step = state.q_step.min(total - 1);
    let progress = (step + 1) as f32 / total as f32;

    let body: Element<'_, Message> = if step == 0 {
        // Paso 0 — Nivel.
        let picker: Element<'_, Message> = pick_list(
            Level::ALL.to_vec(),
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
    } else if levels::is_ai_step(level, step) {
        view_ai_step(state, ts)
    } else if let Some(q) = levels::step_question(level, step) {
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
    if levels::is_ai_step(level, step) {
        nav = nav.push(components::primary_btn("Generar documentos".to_string(), 13).on_press(Message::FinishQuestionnaire));
    } else {
        nav = nav.push(components::primary_btn("Siguiente".to_string(), 13).on_press(Message::QNext));
    }

    // Step rail: números compactos (pasado/actual con acento) + título actual.
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
            format!("{level} · paso {} de {total}", step + 1)
        ),
        rail,
        text(questionnaire::step_title(level, step)).size(design::fs(ts, 11)).color(dim),
        progress_bar(0.0..=1.0, progress)
            .height(4)
            .style(|t: &Theme| design::progress(t)),
        body,
    ]
    .spacing(10)
    .padding(20)
    .max_width(640);
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
    col = col.push(nav.spacing(8));

    container(container(col).style(|t: &Theme| design::card(t)))
        .width(iced::Fill)
        .height(iced::Fill)
        .center_x(iced::Fill)
        .center_y(iced::Fill)
        .padding(24)
        .into()
}

/// Lista de preguntas IA con sus respuestas + botón Regenerar.
/// Se construye de una vez (un solo `column!`) para no mezclar vidas
/// de préstamos entre pushes incrementales.
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

/// Campo de una pregunta intermedia según su tipo.
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
                "funcionalidades" => (&a.funcionalidades, Message::QFuncChanged),
                "ui_ux" => (&a.ui_ux, Message::QUiUxChanged),
                _ => (&a.descripcion, Message::QDescChanged),
            };
            text_input("Escribe aquí...", value)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(on_in)
                .on_submit(Message::QNext)
                .into()
        }
        QKind::PickTipo => pick_list(
            TipoProyecto::ALL.to_vec(),
            Some(a.tipo),
            Message::QTipoPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickEstilo => {
            let pick: Element<'a, Message> = pick_list(
                EstiloPreset::ALL.to_vec(),
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
            for p in crate::questionnaire::Plataforma::ALL {
                let checked = a.plataformas.contains(&p);
                opts = opts.push(
                    checkbox(p.to_string(), checked).on_toggle(move |_| Message::QPlataformaToggled(p)),
                );
            }
            opts.into()
        }
        QKind::MultiStack => {
            let mut opts = column![].spacing(4);
            for s in StackOpt::ALL {
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
            Facturacion::ALL.to_vec(),
            Some(a.facturacion),
            Message::QFacturacionPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        QKind::PickArq => {
            let pick: Element<'a, Message> = pick_list(
                ArqPreset::ALL.to_vec(),
                Some(a.arq),
                Message::QArqPicked,
            )
            .style(|t: &Theme, s| design::field_pick(t, s))
            .width(280)
            .into();
            column![
                pick,
                text_input("Detalle (módulos, capas, cómo escala...)", &a.arquitectura)
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::QArqChanged)
                    .on_submit(Message::QNext),
            ]
            .spacing(8)
            .into()
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

/// Último paso: preguntas adicionales generadas por IA (saltable).
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
        text("El provider activo propone 3–5 preguntas adaptadas a lo respondido. \
            Puedes responderlas o saltar este paso: los documentos se generan igual.")
            .size(design::fs(ts, 12))
            .color(dim),
    ]
    .spacing(10);
    if !has_api {
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
