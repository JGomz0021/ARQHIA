use iced::{Element, Length, Theme};

use crate::app::Message;
use crate::config::AppConfig;
use crate::db::{ChatMeta, Project};
use crate::ui::{components, design};
use crate::ui::design::Tone;

/// Home: project launcher for a development environment, not a hero page.
///
/// Left rail: identity + status + primary/secondary actions.
/// Right: clickable project cards (or an intentional empty state).
/// Parameters for the Home launcher (bundled to keep the signature readable).
pub struct HomeProps<'a> {
    pub creating: bool,
    pub create_name: &'a str,
    pub create_path: &'a str,
    pub status: &'a str,
    pub projects: &'a [Project],
    pub chats: &'a [ChatMeta],
    pub active_chat: Option<i64>,
    pub config: &'a AppConfig,
    /// v0.8: el usuario ocultó el aviso con "Configurar después".
    pub onboarding_dismissed: bool,
}

pub fn view_home<'a>(p: HomeProps<'a>) -> Element<'a, Message> {
    let HomeProps {
        creating,
        create_name,
        create_path,
        status,
        projects,
        chats,
        active_chat,
        config,
        onboarding_dismissed,
    } = p;
    use iced::widget::{column, container, row, scrollable, text, text_input};

    // Iced Theme matching the app config, so design tokens stay consistent.
    // Reutiliza el mismo tema con acento que el resto de vistas (antes usaba
    // Theme::Dark/Light crudo y el acento elegido se perdía en Inicio).
    let app_theme = super::theme_for(config);
    let dim = design::ink_2(&app_theme);
    let ts = config.appearance.text_size.scale();

    let active_model = config.active_config().model.clone();
    // Rail izquierdo: identidad + acciones globales (siempre visible).
    let rail = column![
        text("ARQHIA").size(design::fs(ts, 26)).color(design::ink(&app_theme)),
        text("Entorno de desarrollo con agente IA.").size(design::fs(ts, 13)).color(dim),
        row![
            components::badge(app_theme.clone(), Tone::Accent, config.active.to_string()),
            components::badge(app_theme.clone(),
                Tone::Neutral,
                if active_model.trim().is_empty() {
                    "sin modelo"
                } else {
                    active_model.trim()
                }
            ),
        ]
        .spacing(6),
        components::primary_btn("Entrar al chat".to_string(), 14)
            .width(Length::Fill)
            .on_press(Message::GoChat),
        components::head_btn("Configuración".to_string())
            .width(Length::Fill)
            .on_press(Message::OpenConfig),
        iced::widget::vertical_space().height(8),
        components::section_label(app_theme.clone(), "Sesión"),
        components::head_btn("Salir".to_string())
            .width(Length::Fill)
            .on_press(Message::ExitApp),
    ]
    .spacing(10)
    .width(264);

    // Centro: acciones principales centradas + proyectos.
    let count_title = format!("Proyectos · {}", projects.len());
    let mut center = column![].spacing(8);
    // v0.8 — onboarding de primer arranque: si ningún provider tiene API
    // (ni modelo local), invita a configurar una. No bloquea: se puede
    // posponer, y al guardar la primera API desaparece y no vuelve.
    let any_api = config.has_any_api();
    if !any_api && !onboarding_dismissed {
        center = center.push(
            container(
                column![
                    components::section_label(app_theme.clone(), "Configurar API"),
                    text("Añade provider, key y modelo en Config.")
                        .size(design::fs(ts, 14)),
                    row![
                        components::primary_btn("Configurar API".to_string(), 14)
                            .on_press(Message::OpenConfig),
                        components::quiet_btn("Configurar después".to_string())
                            .on_press(Message::DismissOnboarding),
                    ]
                    .spacing(8),
                ]
                .spacing(8)
                .padding(16),
            )
            .style(|t: &Theme| design::card(t)),
        );
    }
    if creating {
        center = center.push(
            container(
                column![
                    components::section_label(app_theme.clone(), "Nuevo proyecto"),
                    text_input("Nombre...", create_name)
                        .size(design::fs(ts, 14))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::CreateNameChanged)
                        .on_submit(Message::SubmitCreateProject)
                        .width(Length::Fill),
                    text_input("Ruta (vacío = ~/ARQHIA/projects/...)", create_path)
                        .size(design::fs(ts, 13))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::CreatePathChanged)
                        .on_submit(Message::SubmitCreateProject)
                        .width(Length::Fill),
                    row![
                        components::primary_btn("Crear".to_string(), 14)
                            .on_press(Message::SubmitCreateProject),
                        components::quiet_btn("Sin cuestionario".to_string())
                            .on_press(Message::SubmitCreateProjectSkip),
                        components::head_btn("Cancelar".to_string())
                            .on_press(Message::HideCreateModal),
                    ]
                    .spacing(8),
                    text(status).size(design::fs(ts, 13)).color(dim),
                ]
                .spacing(8),
            )
            .padding(14)
            .style(|t: &Theme| design::card(t)),
        );
    }
    center = center.push(
        container(
            column![
                row![
                    components::primary_btn("Crear proyecto".to_string(), 14)
                        .on_press(Message::ShowCreateModal),
                    components::quiet_btn("Abrir proyecto".to_string())
                        .on_press(Message::OpenProject),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
    );
    center = center.push(components::section_label(app_theme.clone(), count_title));
    if projects.is_empty() {
        center = center.push(
            container(
                column![
                    text("Sin proyectos todavía").size(design::fs(ts, 19)),
                    text(
                        "Define PROJECT.md + SPECS.md; el agente genera estructura."
                    )
                    .size(design::fs(ts, 14))
                    .color(dim),
                    row![
                        components::primary_btn("Crear proyecto".to_string(), 14)
                            .on_press(Message::ShowCreateModal),
                        components::quiet_btn("Abrir existente".to_string())
                            .on_press(Message::OpenProject),
                    ]
                    .spacing(8),
                ]
                .spacing(10)
                .padding(20),
            )
            .style(|t: &Theme| design::card(t)),
        );
    } else {
        for p in projects.iter().take(12) {
            let n = chats.iter().filter(|c| c.project_id == Some(p.id)).count();
            let has_ws = p
                .path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .is_some();
            let path = p
                .path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("sin workspace asignado");
            let short = design::trunc_start(path, 52);
            let pid = p.id;
            let is_active = active_chat
                .and_then(|id| chats.iter().find(|c| c.id == id))
                .and_then(|c| c.project_id)
                == Some(pid);
            let active_badge: Element<'_, Message> = if is_active {
                components::badge(app_theme.clone(), Tone::Ok, "abierto")
            } else {
                iced::widget::horizontal_space().width(0).into()
            };
            // Thumbnail: monograma de identidad del proyecto.
            let initial = p
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "•".to_string());
            let avatar = container(text(initial).size(design::fs(ts, 18)).color(design::accent(&app_theme)))
                .width(40)
                .height(40)
                .center_x(40)
                .center_y(40)
                .style(|t: &Theme| design::avatar(t));
            center = center.push(
                iced::widget::button(
                    container(
                        row![
                            avatar,
                            column![
                                row![
                                    text(&p.name).size(design::fs(ts, 16)),
                                    components::badge(app_theme.clone(),
                                        if has_ws { Tone::Ok } else { Tone::Neutral },
                                        if has_ws { "agente" } else { "chat" }
                                    ),
                                    active_badge,
                                    iced::widget::horizontal_space(),
                                    text(format!(
                                        "{} {}",
                                        n,
                                        if n == 1 { "chat" } else { "chats" }
                                    ))
                                    .size(design::fs(ts, 12))
                                    .color(dim),
                                    text("Abrir").size(design::fs(ts, 13)).color(design::accent(&app_theme)),
                                ]
                                .spacing(8)
                                .align_y(iced::Alignment::Center),
                                text(short).size(design::fs(ts, 12)).color(dim),
                            ]
                            .spacing(4)
                            .width(Length::Fill),
                        ]
                        .spacing(12)
                        .align_y(iced::Alignment::Center)
                        .width(Length::Fill),
                    )
                    .padding(14),
                )
                .width(Length::Fill)
                .padding(0)
                .style(move |t: &Theme, s| design::nav(t, s, is_active))
                .on_press(Message::EnterProject(pid)),
            );
        }
    }

    if !creating && !status.is_empty() {
        center = center.push(
            container(text(status).size(design::fs(ts, 13)).color(dim))
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    }

    row![
        container(scrollable(rail).height(Length::Fill))
            .width(288)
            .height(Length::Fill)
            .padding(16)
            .style(|t: &Theme| design::sidebar(t)),
        container(
            container(scrollable(center).height(Length::Fill))
                .height(Length::Fill)
                .max_width(760)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .center_x(Length::Fill),
    ]
    .spacing(0)
    .into()
}
