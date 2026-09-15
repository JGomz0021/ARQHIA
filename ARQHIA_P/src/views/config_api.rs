//! Config API: pestaña de perfiles + navegador de modelos.
//!
//! Split de `config_view.rs` (v0.9.5): sin cambio de comportamiento.

use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::config::Provider;

pub(crate) use super::config_view::{model_browser, settings_card};

pub(crate) fn view_config_api(state: &App) -> Element<'_, Message> {
    use crate::ui::{components, design};
    use iced::widget::{column, pick_list, row, text, text_input};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let providers = Provider::ALL.to_vec();
    let status_kind = design::status_tone(&state.status);
    let browser: Element<'_, Message> = if state.model_browser {
        model_browser(state)
    } else {
        iced::widget::vertical_space().height(0).into()
    };
    // Nivel de razonamiento: opciones determinadas por el modelo (models.dev).
    let r_pid = crate::pricing::provider_id_for(state.edit_provider, &state.edit_base_url);
    let r_opts = state
        .pricing
        .effort_choices_in(r_pid.as_deref(), &state.edit_model);
    let r_current = if state.edit_reasoning.is_empty() {
        "auto".to_string()
    } else {
        state.edit_reasoning.clone()
    };
    let reason_hint: Element<'_, Message> = if state.pricing.is_empty() {
        text("Pulsa `Buscar modelo` una vez para cargar los niveles del catálogo.")
            .size(design::fs(ts, 11))
            .color(dim)
            .into()
    } else {
        iced::widget::vertical_space().height(0).into()
    };
    let reasoning_row: Element<'_, Message> = column![
        row![
            text("Nivel de razonamiento")
                .size(design::fs(ts, 12))
                .color(dim),
            pick_list(r_opts, Some(r_current), Message::EditReasoningPicked)
                .style(|t: &Theme, s| design::field_pick(t, s)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        reason_hint,
    ]
    .spacing(4)
    .into();
    let cx = state.config.appearance.compact();
    // v0.7.4 (rediseño): el centro es CREAR un perfil — nombre arriba, luego
    // provider, api + base, modelo + búsqueda, guardar/probar. Debajo, la
    // lista de perfiles con menú "···" (Borrar/Editar) y overlay de edición.
    let create_card = settings_card(
        &app_theme,
        cx,
        "Nuevo perfil",
        column![
            text("El perfil guarda todo: nombre + provider + API key + URL base + modelo + nivel. Cambiar de perfil cambia todo.")
                .size(design::fs(ts, 11))
                .color(dim),
            components::field(app_theme.clone(), "Nombre del perfil", None),
            text_input("Ej. Rápido, Potente...", &state.profile_name)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(crate::app::Message::ProfileNameChanged)
                .on_submit(crate::app::Message::ProfileSave)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Provider", None),
            pick_list(providers, Some(state.edit_provider), Message::ProviderPicked)
                .style(|t: &Theme, s| design::field_pick(t, s)),
            components::field(app_theme.clone(), "API key", None),
            text_input(
                if state.edit_provider == Provider::Local {
                    "Opcional en local (LM Studio la ignora)"
                } else {
                    "sk-... / sk-ant-... / or-..."
                },
                &state.edit_api_key,
            )
            .secure(true)
            .style(|t: &Theme, s| design::field(t, s))
            .on_input(Message::ApiKeyChanged),
            components::field(app_theme.clone(), "URL base", None),
            row![
                text_input(
                    if state.edit_provider == Provider::Local {
                        "http://localhost:1234"
                    } else {
                        "https://..."
                    },
                    &state.edit_base_url
                )
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::BaseUrlChanged)
                .width(iced::Fill),
                components::quiet_btn("Restablecer".to_string()).on_press(Message::UseDefaultBaseUrl),
            ]
            .spacing(8),
            components::field(app_theme.clone(), "Modelo", None),
            row![
                text_input(
                    if state.edit_provider == Provider::Local {
                        "id exacto en LM Studio (ej. qwen3-8b)"
                    } else {
                        "gpt-4o-mini..."
                    },
                    &state.edit_model
                )
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::ModelChanged)
                .width(iced::Fill),
                components::quiet_btn("Buscar modelo".to_string())
                    .on_press(Message::OpenModelBrowser),
                components::quiet_btn("Restablecer".to_string()).on_press(Message::UseDefaultModel),
            ]
            .spacing(8),
            reasoning_row,
            text(format!(
                "Activo para chatear: {} — {}",
                state.config.active,
                state.config.active_config().model
            ))
            .size(design::fs(ts, 11))
            .color(dim),
            row![
                components::primary_btn("Guardar perfil".to_string(), 13)
                    .on_press(crate::app::Message::ProfileSave),
                if state.testing {
                    components::quiet_btn("Probando...".to_string())
                } else {
                    components::quiet_btn("Probar conexión".to_string()).on_press(Message::TestConnection)
                },
            ]
            .spacing(8),
            row![
                text(components::log_icon(status_kind))
                    .size(12)
                    .color(design::tone(&app_theme, status_kind)),
                text(&state.status).size(design::fs(ts, 12)).color(
                    if status_kind == design::Tone::Neutral {
                        dim
                    } else {
                        design::tone(&app_theme, status_kind)
                    }
                ),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            browser,
        ]
        .spacing(6),
    );
    let list_card = profiles_list_card(state);
    let overlay: Element<'_, Message> = match state.editing_profile.clone() {
        Some(_) => profile_edit_overlay(state),
        None => iced::widget::vertical_space().height(0).into(),
    };
    column![create_card, list_card, overlay,]
        .spacing(12)
        .max_width(1200)
        .into()
}

/// Lista de perfiles guardados (v0.7.4 rediseño): una caja por perfil con
/// nombre + modelo y menú "···" (Borrar/Editar). Click en el nombre = usar.
fn profiles_list_card(state: &App) -> Element<'_, Message> {
    use crate::ui::{components, design};
    use iced::widget::{column, container, row, text};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let mut list = column![
        text(format!(
            "Perfiles guardados ({})",
            state.config.model_profiles.len()
        ))
        .size(design::fs(ts, 13)),
        text("Click en el nombre para usarlo en el chat. El ··· abre Borrar/Editar.")
            .size(design::fs(ts, 11))
            .color(dim),
    ]
    .spacing(4);
    if state.config.model_profiles.is_empty() {
        list = list.push(components::empty_state(
            app_theme.clone(),
            "Sin perfiles todavía",
            "Crea el primero arriba con nombre + provider + API + modelo.",
        ));
    }
    for p in &state.config.model_profiles {
        let pid = p.id.clone();
        let pid_menu = p.id.clone();
        let pid_del = p.id.clone();
        let pid_edit = p.id.clone();
        let is_active = state.config.active_profile.as_deref() == Some(p.id.as_str());
        let mut card = column![
            row![
                iced::widget::button(text(p.name.clone()).size(design::fs(ts, 14)))
                    .padding(0)
                    .style(|t: &Theme, s| design::nav(t, s, false))
                    .on_press(Message::ProfilePicked(pid)),
                iced::widget::horizontal_space(),
                text(p.model.clone()).size(design::fs(ts, 12)).color(dim),
                if is_active {
                    components::badge(app_theme.clone(), design::Tone::Ok, "activo")
                } else {
                    components::badge(
                        app_theme.clone(),
                        design::Tone::Neutral,
                        p.provider.to_string(),
                    )
                },
                components::head_btn("···".to_string())
                    .on_press(Message::ProfileMenuToggled(pid_menu)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(4);
        if state.profile_menu.as_deref() == Some(p.id.as_str()) {
            card = card.push(
                row![
                    components::quiet_btn("Editar".to_string())
                        .on_press(Message::ProfileEdit(pid_edit)),
                    components::quiet_btn("Borrar".to_string())
                        .on_press(Message::ProfileDelete(pid_del)),
                ]
                .spacing(8),
            );
        }
        list = list.push(
            container(card)
                .width(iced::Fill)
                .padding(10)
                .style(|t: &Theme| design::card(t)),
        );
    }
    settings_card(&app_theme, cx, "Mis perfiles", list)
}

/// Overlay de edición de un perfil (v0.7.4 rediseño): mini formulario
/// superpuesto con nombre, provider, api, url base, modelo y nivel.
fn profile_edit_overlay(state: &App) -> Element<'_, Message> {
    use crate::ui::{components, design};
    use iced::widget::{column, pick_list, row, text, text_input};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let providers = Provider::ALL.to_vec();
    let r_pid = crate::pricing::provider_id_for(state.eprofile_provider, &state.eprofile_base);
    let r_opts = state
        .pricing
        .effort_choices_in(r_pid.as_deref(), &state.eprofile_model);
    let r_current = if state.eprofile_reasoning.is_empty() {
        "auto".to_string()
    } else {
        state.eprofile_reasoning.clone()
    };
    settings_card(
        &app_theme,
        cx,
        "Editar perfil",
        column![
            components::field(app_theme.clone(), "Nombre", None),
            text_input("Nombre del perfil", &state.eprofile_name)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::ProfileEditNameChanged)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Provider", None),
            pick_list(
                providers,
                Some(state.eprofile_provider),
                Message::ProfileEditProviderPicked
            )
            .style(|t: &Theme, s| design::field_pick(t, s)),
            components::field(app_theme.clone(), "API key", None),
            text_input("sk-... / sk-ant-... / or-...", &state.eprofile_api)
                .secure(true)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::ProfileEditApiChanged),
            components::field(app_theme.clone(), "URL base", None),
            text_input("https://...", &state.eprofile_base)
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::ProfileEditBaseChanged)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Modelo", None),
            row![
                text_input("gpt-4o-mini...", &state.eprofile_model)
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::ProfileEditModelChanged)
                    .width(iced::Fill),
                components::quiet_btn("Buscar".to_string()).on_press(Message::OpenModelBrowser),
            ]
            .spacing(8),
            row![
                text("Nivel de razonamiento")
                    .size(design::fs(ts, 12))
                    .color(dim),
                pick_list(r_opts, Some(r_current), Message::ProfileEditReasoningPicked)
                    .style(|t: &Theme, s| design::field_pick(t, s)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                components::primary_btn("Guardar cambios".to_string(), 13)
                    .on_press(Message::ProfileUpdate),
                components::quiet_btn("Cancelar".to_string()).on_press(Message::ProfileEditCancel),
            ]
            .spacing(8),
            text(&state.status).size(design::fs(ts, 12)).color(dim),
        ]
        .spacing(6),
    )
}
