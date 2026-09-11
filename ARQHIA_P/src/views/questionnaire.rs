//! Cuestionario: definición guiada del proyecto.
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::questionnaire::{self, Interfaz, Publico};

pub(crate) fn view_questionnaire(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, pick_list, progress_bar, row, text, text_input};
    use crate::ui::{components, design};
    use crate::ui::design::{Tone, type_scale};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let step = state.q_step.min(questionnaire::TOTAL_STEPS - 1);
    let progress = (step + 1) as f32 / questionnaire::TOTAL_STEPS as f32;

    let (question, explanation): (&str, Option<&str>) = match step {
        0 => ("Cómo se llama tu proyecto?", Some("Un nombre corto y reconocible.")),
        1 => ("Qué hace, en una o dos frases?", None),
        2 => (
            "Dónde vive en tu equipo?",
            Some("Vacío = usa el workspace del proyecto."),
        ),
        3 => ("Qué problema resuelve?", None),
        4 => ("Quién lo va a usar?", None),
        _ => ("Cómo se interactúa con él?", Some("Escritorio, web, terminal...")),
    };

    let answer: Element<'_, Message> = match step {
        0 => text_input("Ej. MiApp...", &state.q_answers.nombre)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::QNombreChanged)
            .on_submit(Message::QNext)
            .into(),
        1 => text_input("Qué es y qué hace...", &state.q_answers.descripcion)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::QDescChanged)
            .on_submit(Message::QNext)
            .into(),
        2 => text_input("/ruta/absoluta o vacío...", &state.q_answers.ubicacion)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::QUbicChanged)
            .on_submit(Message::QNext)
            .into(),
        3 => text_input("Objetivo principal...", &state.q_answers.objetivo)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::QObjChanged)
            .on_submit(Message::QNext)
            .into(),
        4 => pick_list(
            Publico::ALL.to_vec(),
            Some(state.q_answers.publico),
            Message::QPublicoPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
        _ => pick_list(
            Interfaz::ALL.to_vec(),
            Some(state.q_answers.interfaz),
            Message::QInterfazPicked,
        )
        .style(|t: &Theme, s| design::field_pick(t, s))
        .width(280)
        .into(),
    };

    let mut nav = row![];
    if step > 0 {
        nav = nav.push(components::head_btn("Atrás".to_string()).on_press(Message::QBack));
    }
    nav = nav.push(components::head_btn("Cancelar".to_string()).on_press(Message::QCancel));
    nav = nav.push(iced::widget::horizontal_space());
    if step + 1 == questionnaire::TOTAL_STEPS {
        nav = nav.push(components::primary_btn("Generar ESPEC".to_string(), 13).on_press(Message::FinishQuestionnaire));
    } else {
        nav = nav.push(components::primary_btn("Siguiente".to_string(), 13).on_press(Message::QNext));
    }

    // Step rail: compact numbers (past/current accented) + current title.
    // Numbers only so the rail never overflows the card at small widths.
    let rail = row(
        (0..questionnaire::TOTAL_STEPS)
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
            format!(
                "Definición del proyecto · paso {} de {}",
                step + 1,
                questionnaire::TOTAL_STEPS
            )
        ),
        rail,
        text(questionnaire::step_title(step)).size(design::fs(ts, 11)).color(dim),
        progress_bar(0.0..=1.0, progress)
            .height(4)
            .style(|t: &Theme| design::progress(t)),
        text(question).size(design::fs(ts, type_scale::HEADLINE)),
    ]
    .spacing(10)
    .padding(20)
    .max_width(640);
    if let Some(exp) = explanation {
        col = col.push(text(exp).size(design::fs(ts, 12)).color(dim));
    }
    col = col.push(answer);
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
