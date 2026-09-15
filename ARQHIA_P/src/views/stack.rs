//! Panel del STACK local (v0.9 Track A).
//!
//! Render puro sobre App. Buscador + lista + preview (15 líneas +
//! metadatos) + guardar/valorar/reportar + copiar al workspace.

use iced::{Element, Theme};

use crate::app::{App, Message};

pub(crate) fn view_stack(state: &App) -> Element<'_, Message> {
    use crate::ui::design::type_scale;
    use crate::ui::{components, design};
    use iced::widget::{column, container, row, scrollable, text, text_input};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let compact = state.config.appearance.compact();

    let header = row![
        column![
            text("STACK local").size(design::fs(ts, type_scale::HEADLINE)),
            text("Snippets reutilizables con etiquetas, rating y metadatos.")
                .size(design::fs(ts, 12))
                .color(dim),
        ]
        .spacing(0),
        iced::widget::horizontal_space(),
        components::head_btn("Volver".to_string()).on_press(Message::StackBack),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(8);

    let search_card = stack_card(
        &app_theme,
        compact,
        "Buscar",
        column![
            text_input(
                "buscar en título, código o etiquetas...",
                &state.stack_query
            )
            .size(design::fs(ts, 13))
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::StackQueryChanged)
            .on_submit(Message::StackSearch),
            text_input(
                "etiquetas (separadas por comas o espacios)...",
                &state.stack_tags
            )
            .size(design::fs(ts, 13))
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::StackTagsChanged)
            .on_submit(Message::StackSearch),
            row![
                components::primary_btn("Buscar".to_string(), 13).on_press(Message::StackSearch),
                text(if state.stack_searched {
                    format!("{} resultado(s)", state.stack_results.len())
                } else {
                    "Sin buscar todavía".to_string()
                })
                .size(design::fs(ts, 12))
                .color(dim),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
        ]
        .spacing(6),
    );

    let mut results = column![components::section_label(
        app_theme.clone(),
        format!("Resultados ({})", state.stack_results.len())
    )]
    .spacing(4);
    for hit in &state.stack_results {
        let id = hit.id;
        let selected = state.stack_selected.as_ref().map(|s| s.id) == Some(id);
        let label = format!(
            "#{} {} [{}] ★{:.1}",
            hit.id,
            hit.title,
            if hit.tags.is_empty() {
                hit.lang.clone()
            } else {
                hit.tags.clone()
            },
            hit.rating
        );
        results = results.push(
            iced::widget::button(text(label).size(design::fs(ts, 13)))
                .width(iced::Fill)
                .padding([6, 10])
                .style(move |t: &Theme, s| design::nav(t, s, selected))
                .on_press(Message::StackSelect(id)),
        );
    }

    let preview: Element<'_, Message> = match &state.stack_selected {
        None => text("Elige un resultado para verlo.")
            .size(design::fs(ts, 12))
            .color(dim)
            .into(),
        Some(full) => {
            let lines: Vec<&str> = full.code.lines().collect();
            let shown: String = lines
                .iter()
                .take(15)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            let rest = lines.len().saturating_sub(15);
            let code_md = if rest > 0 {
                format!("{shown}\n… ({rest} líneas más)")
            } else {
                shown
            };
            let mut meta = column![
                text(format!(
                    "{} · {} · {} · ★{:.1} ({} votos) · {} ejecuciones ({} ok) · fuente {}",
                    if full.author.is_empty() {
                        "(sin autor)".to_string()
                    } else {
                        full.author.clone()
                    },
                    full.license,
                    if full.lang.is_empty() {
                        "?".to_string()
                    } else {
                        full.lang.clone()
                    },
                    full.rating,
                    full.ratings,
                    full.executions,
                    full.ok_runs,
                    if full.source.is_empty() {
                        "local".to_string()
                    } else {
                        full.source.clone()
                    },
                ))
                .size(design::fs(ts, 12))
                .color(dim),
                text(format!(
                    "etiquetas: {}",
                    if full.tags.is_empty() {
                        "(sin etiquetas)".to_string()
                    } else {
                        full.tags.clone()
                    }
                ))
                .size(design::fs(ts, 12))
                .color(dim),
            ]
            .spacing(2);
            for (k, v) in full.meta.iter().take(8) {
                let line = format!("{k}: {v}");
                meta = meta.push(text(line).size(design::fs(ts, 12)).color(dim));
            }
            let mut stars = row![].spacing(4);
            for n in 1u8..=5 {
                stars = stars
                    .push(components::icon_btn(format!("{n}★")).on_press(Message::StackRate(n)));
            }
            column![
                text(format!("#{} {}", full.id, full.title))
                    .size(design::fs(ts, type_scale::TITLE)),
                meta,
                container(
                    scrollable(
                        text(code_md)
                            .size(design::fs(ts, 12))
                            .font(iced::Font::MONOSPACE)
                    )
                    .height(200)
                )
                .padding(8)
                .style(|t: &Theme| design::card(t)),
                row![
                    components::primary_btn("Copiar a workspace".to_string(), 13)
                        .on_press(Message::StackCopyToWs),
                    components::head_btn("Pedir al agente".to_string())
                        .on_press(Message::StackAskAgent),
                ]
                .spacing(8),
                row![
                    text_input("opinión (opcional)...", &state.stack_opinion)
                        .size(design::fs(ts, 12))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::StackOpinionChanged)
                        .width(iced::Fill),
                    stars,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(6),
                row![
                    text_input("reportar bug...", &state.stack_bug)
                        .size(design::fs(ts, 12))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::StackBugChanged)
                        .on_submit(Message::StackReportBug)
                        .width(iced::Fill),
                    components::head_btn("Reportar".to_string()).on_press(Message::StackReportBug),
                ]
                .align_y(iced::Alignment::Center)
                .spacing(6),
            ]
            .spacing(6)
            .into()
        }
    };
    let preview_card = stack_card(&app_theme, compact, "Vista previa", preview);

    let save_card = stack_card(
        &app_theme,
        compact,
        "Guardar snippet",
        column![
            text(if state.config.stack_consent.share_local {
                match state.config.identity.validate() {
                    Ok(()) => format!("Se firmará como {}.", state.config.identity.author_line()),
                    Err(_) => {
                        "Sin identidad válida: el autor quedará vacío (Config → STACK).".to_string()
                    }
                }
            } else {
                "Desactivado: permite «Guardar en local» en Config → STACK.".to_string()
            })
            .size(design::fs(ts, 12))
            .color(dim),
            text_input("título...", &state.save_title)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::StackSaveTitleChanged),
            row![
                text_input("etiquetas csv...", &state.save_tags)
                    .size(design::fs(ts, 13))
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::StackSaveTagsChanged)
                    .width(iced::Fill),
                text_input("lang...", &state.save_lang)
                    .size(design::fs(ts, 13))
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::StackSaveLangChanged)
                    .width(150),
                iced::widget::pick_list(
                    crate::stack::StackLicense::ALL
                        .iter()
                        .map(|l| l.to_string())
                        .collect::<Vec<String>>(),
                    Some(state.save_license.clone()),
                    Message::StackSaveLicensePicked,
                )
                .style(|t: &Theme, s| design::field_pick(t, s)),
            ]
            .spacing(8),
            text_input("pega el código aquí...", &state.save_code)
                .size(design::fs(ts, 12))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::StackSaveCodeChanged)
                .line_height(1.2)
                .width(iced::Fill),
            row![
                components::primary_btn("Guardar".to_string(), 13).on_press(Message::StackSave),
                text(&state.stack_status)
                    .size(design::fs(ts, 12))
                    .color(dim),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
        ]
        .spacing(6),
    );

    container(
        column![
            header,
            scrollable(
                column![search_card, results, preview_card, save_card,]
                    .spacing(10)
                    .width(iced::Fill)
            )
            .height(iced::Fill),
        ]
        .spacing(8)
        .padding(16),
    )
    .width(iced::Fill)
    .height(iced::Fill)
    .into()
}

fn stack_card<'a>(
    app_theme: &Theme,
    compact: bool,
    label: &str,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    use crate::ui::{components, design};
    use iced::widget::{column, container};
    container(
        column![
            components::section_label(app_theme.clone(), label),
            body.into()
        ]
        .spacing(design::gap(compact, 8)),
    )
    .width(iced::Length::Fill)
    .padding(design::pad(compact, 12))
    .style(|t: &Theme| design::card(t))
    .into()
}
