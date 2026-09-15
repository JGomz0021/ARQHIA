//! Config Git: pestaña de repositorio + estado del workspace.
//!
//! Split de `config_view.rs` (v0.9.5): sin cambio de comportamiento.

use iced::{Element, Theme};

use crate::app::{App, Message};

pub(crate) use super::config_view::settings_card;

pub(crate) fn view_config_git(state: &App) -> Element<'_, Message> {
    use crate::config::{BranchMode, GitAutonomy};
    use crate::ui::{components, design};
    use iced::widget::{checkbox, column, pick_list, row, text, text_input};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let git = &state.config.git;

    let repo_card = settings_card(
        &app_theme,
        cx,
        "Repositorio",
        column![
            checkbox("Iniciar Git en cada workspace", git.enabled)
                .on_toggle(Message::GitEnabledToggled),
            checkbox("Hacer `git init` si el workspace no es repo", git.auto_init)
                .on_toggle(Message::GitAutoInitToggled),
            components::field(app_theme.clone(), "Rama base (protegida)", None),
            text_input("main", &state.git_base_branch)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitBaseBranchChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Rama de trabajo del agente", None),
            text_input("ARQHIA", &state.git_work_branch)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitWorkBranchChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Modo de rama", None),
            pick_list(
                BranchMode::ALL.to_vec(),
                Some(git.branch_mode),
                Message::GitBranchModePicked
            )
            .style(|t: &Theme, s| design::field_pick(t, s)),
            components::field(app_theme.clone(), "Remoto", None),
            text_input("origin", &state.git_remote)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitRemoteChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            components::field(
                app_theme.clone(),
                "Rama de push (vacío = rama de trabajo)",
                None
            ),
            text_input("ARQHIA", &state.git_push_branch)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitPushBranchChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
        ]
        .spacing(6),
    );

    let autonomy_card = settings_card(
        &app_theme,
        cx,
        "Autonomía del agente",
        column![
            components::field(
                app_theme.clone(),
                "Acciones git permitidas",
                Some("Por encima de lo elegido, lo destructivo siempre se bloquea.".to_string())
            ),
            pick_list(GitAutonomy::ALL.to_vec(), Some(git.autonomy), Message::GitAutonomyPicked)
                .style(|t: &Theme, s| design::field_pick(t, s)),
            checkbox("Permitir push a GitHub (pide aprobación al activarse)", git.push_enabled)
                .on_toggle(Message::GitPushToggled),
            components::field(app_theme.clone(), "Autor de commits (opcional)", Some("Vacío = identidad global de git.".to_string())),
            text_input("Nombre", &state.git_author_name)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitAuthorNameChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            text_input("Email", &state.git_author_email)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitAuthorEmailChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            text("Las ramas protegidas (main/master) nunca se tocan y la rama de trabajo no puede estar en la lista.")
                .size(design::fs(ts, 11))
                .color(dim),
            row![components::primary_btn("Guardar".to_string(), 13).on_press(Message::GitSave)].spacing(8),
        ]
        .spacing(6),
    );

    let status = &state.git_status;
    let status_body: Element<'_, Message> = match state.active_workspace() {
        None => text("Sin workspace activo. Abre un proyecto para ver su estado git.")
            .size(design::fs(ts, 12))
            .color(dim)
            .into(),
        Some(_) if !status.is_repo => column![
            text("El workspace activo no es un repo git.")
                .size(design::fs(ts, 12))
                .color(dim),
            row![
                components::primary_btn("Inicializar git".to_string(), 13)
                    .on_press(Message::GitInitWorkspace)
            ]
            .spacing(8),
        ]
        .spacing(6)
        .into(),
        Some(_) => {
            let remotes = if status.remotes.is_empty() {
                "(sin remotos)".to_string()
            } else {
                status.remotes.join(", ")
            };
            column![
                row![
                    text("Rama actual")
                        .size(design::fs(ts, 12))
                        .color(dim)
                        .width(140),
                    text(if status.branch.is_empty() {
                        "(desconocida)"
                    } else {
                        &status.branch
                    })
                    .size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text("Remotos")
                        .size(design::fs(ts, 12))
                        .color(dim)
                        .width(140),
                    text(remotes).size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text("Cambios")
                        .size(design::fs(ts, 12))
                        .color(dim)
                        .width(140),
                    text(format!("{} archivo(s)", status.changes)).size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    components::quiet_btn("Refrescar".to_string())
                        .on_press(Message::GitRefreshStatus),
                ]
                .spacing(8),
            ]
            .spacing(4)
            .into()
        }
    };
    let status_card = settings_card(&app_theme, cx, "Estado del workspace activo", status_body);

    column![
        row![repo_card, autonomy_card]
            .spacing(12)
            .align_y(iced::Alignment::Start),
        status_card,
        text(&state.status).size(design::fs(ts, 12)).color(dim),
    ]
    .spacing(12)
    .max_width(1200)
    .into()
}
