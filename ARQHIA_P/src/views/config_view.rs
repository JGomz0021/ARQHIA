//! Configuración: ajustes por pestañas.
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::{Element, Theme};

use crate::app::{App, ConfigTab, Message};
use crate::config::{Provider, ThemeMode};
use crate::workspace;

pub(crate) fn view_config(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, row, scrollable, text};
    use crate::ui::{components, design};
    use crate::ui::design::type_scale;
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let mut tabs = column![components::section_label(app_theme.clone(), "Ajustes")].spacing(2);
    for tab in ConfigTab::ALL {
        let selected = state.config_tab == tab;
        let label = tab.label().to_string();
        tabs = tabs.push(
            iced::widget::button(text(label).size(design::fs(ts, 13)))
                .width(iced::Fill)
                .padding([6, 10])
                .style(move |t: &Theme, s| design::nav(t, s, selected))
                .on_press(Message::ConfigTab(tab)),
        );
    }
    let content: Element<'_, Message> = match state.config_tab {
        ConfigTab::Api => view_config_api(state),
        ConfigTab::Apariencia => view_config_apariencia(state),
        ConfigTab::Permisos => view_config_permisos(state),
        ConfigTab::Git => view_config_git(state),
        ConfigTab::Proyectos => view_config_proyectos(state),
        ConfigTab::Atajos => view_config_atajos(state),
    };
    column![
        row![
            column![
                text("Configuración").size(design::fs(ts, type_scale::HEADLINE)),
                text("Proveedor, apariencia, permisos y proyectos.")
                    .size(design::fs(ts, 12))
                    .color(dim),
            ]
            .spacing(0),
            iced::widget::horizontal_space(),
            components::head_btn("Volver".to_string()).on_press(Message::ConfigBack),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(8),
        row![
            container(tabs.width(220)).padding([4, 8]),
            container(scrollable(content).height(iced::Fill))
                .width(iced::Fill)
                .padding([4, 16]),
        ]
        .spacing(8)
        .height(iced::Fill),
    ]
    .spacing(8)
    .padding(16)
    .into()
}

/// Settings section card: label + grouped controls.
fn settings_card<'a>(
    app_theme: &Theme,
    compact: bool,
    label: &str,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    use iced::widget::{column, container};
    use crate::ui::{components, design};
    container(
        column![components::section_label(app_theme.clone(), label), body.into()]
            .spacing(design::gap(compact, 8)),
    )
    .width(iced::Length::Fill)
    .padding(design::pad(compact, 12))
    .style(|t: &Theme| design::card(t))
    .into()
}

/// Etiqueta de límite con tooltip flotante al pasar el cursor.
fn limit_label<'a>(ts: f32, label: &str, hint_text: &str) -> Element<'a, Message> {
    use iced::widget::{container, text};
    use crate::ui::design;
    iced::widget::Tooltip::new(
        text(label.to_string())
            .size(design::fs(ts, 13))
            .width(iced::Length::Fill),
        container(text(hint_text.to_string()).size(12))
            .padding(6)
            .style(|t: &Theme| design::tooltip(t)),
        iced::widget::tooltip::Position::Bottom,
    )
    .gap(4.0)
    .into()
}

fn view_config_api(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, pick_list, row, text, text_input};
    use crate::ui::{components, design};
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
    let r_opts = state.pricing.effort_choices_in(r_pid.as_deref(), &state.edit_model);
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
            text("Nivel de razonamiento").size(design::fs(ts, 12)).color(dim),
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
    use iced::widget::{column, container, row, text};
    use crate::ui::{components, design};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let mut list = column![
        text(format!("Perfiles guardados ({})", state.config.model_profiles.len()))
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
                iced::widget::button(
                    text(p.name.clone()).size(design::fs(ts, 14))
                )
                .padding(0)
                .style(|t: &Theme, s| design::nav(t, s, false))
                .on_press(Message::ProfilePicked(pid)),
                iced::widget::horizontal_space(),
                text(p.model.clone()).size(design::fs(ts, 12)).color(dim),
                if is_active {
                    components::badge(app_theme.clone(), design::Tone::Ok, "activo")
                } else {
                    components::badge(app_theme.clone(), design::Tone::Neutral, p.provider.to_string())
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
    use iced::widget::{column, pick_list, row, text, text_input};
    use crate::ui::{components, design};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let providers = Provider::ALL.to_vec();
    let r_pid =
        crate::pricing::provider_id_for(state.eprofile_provider, &state.eprofile_base);
    let r_opts = state.pricing.effort_choices_in(r_pid.as_deref(), &state.eprofile_model);
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
                components::quiet_btn("Buscar".to_string())
                    .on_press(Message::OpenModelBrowser),
            ]
            .spacing(8),
            row![
                text("Nivel de razonamiento").size(design::fs(ts, 12)).color(dim),
                pick_list(
                    r_opts,
                    Some(r_current),
                    Message::ProfileEditReasoningPicked
                )
                .style(|t: &Theme, s| design::field_pick(t, s)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                components::primary_btn("Guardar cambios".to_string(), 13)
                    .on_press(Message::ProfileUpdate),
                components::quiet_btn("Cancelar".to_string())
                    .on_press(Message::ProfileEditCancel),
            ]
            .spacing(8),
            text(&state.status).size(design::fs(ts, 12)).color(dim),
        ]
        .spacing(6),
    )
}

fn view_config_apariencia(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, pick_list, row, text};
    use crate::ui::{components, design};
    use crate::config::{AccentChoice, Density, TextSize};
    let app_theme = super::app_theme(state);
    let themes = ThemeMode::ALL.to_vec();
    let status_el: Element<'_, Message> = if state.status.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        let kind = design::status_tone(&state.status);
        row![
            text(components::log_icon(kind))
                .size(12)
                .color(design::tone(&app_theme, kind)),
            text(&state.status).size(12).color(design::tone(&app_theme, kind)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    };
    column![
        settings_card(
            &app_theme,
            state.config.appearance.compact(),
            "Apariencia",
            column![
                components::field(app_theme.clone(), "Tema", Some("Se aplica al instante y se guarda.".to_string())),
                pick_list(themes, Some(state.config.theme), Message::ThemePicked)
                    .style(|t: &Theme, s| design::field_pick(t, s)),
                components::field(app_theme.clone(), "Acento", Some("Identidad ARQHIA: acciones, selección y foco.".to_string())),
                pick_list(AccentChoice::ALL.to_vec(), Some(state.config.appearance.accent), Message::AccentPicked)
                    .style(|t: &Theme, s| design::field_pick(t, s)),
                components::field(app_theme.clone(), "Tamaño de texto", Some("Escala todo el contenido (encabezados y controles fijos no cambian).".to_string())),
                pick_list(TextSize::ALL.to_vec(), Some(state.config.appearance.text_size), Message::TextSizePicked)
                    .style(|t: &Theme, s| design::field_pick(t, s)),
                components::field(app_theme.clone(), "Densidad", Some("Compacta reduce el aire en mensajes y sidebar.".to_string())),
                pick_list(Density::ALL.to_vec(), Some(state.config.appearance.density), Message::DensityPicked)
                    .style(|t: &Theme, s| design::field_pick(t, s)),
                row![
                    components::dot(app_theme.clone(), design::Tone::Accent),
                    text("Vista previa del acento").size(13),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                status_el,
            ]
            .spacing(6)
        ),
    ]
    .spacing(10)
    .max_width(1200)
    .into()
}


fn view_config_permisos(state: &App) -> Element<'_, Message> {
    use iced::widget::{checkbox, column, pick_list, row, text, text_input};
    use crate::ui::{components, design};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let lim = state.config.limits.clamped();
    let permisos_card = settings_card(
        &app_theme,
        state.config.appearance.compact(),
        "Permisos del agente",
            column![
                text("Lo no automático pide tu aprobación en el chat, por lote y con categoría.")
                    .size(design::fs(ts, 12))
                    .color(dim),
                checkbox(
                    "Lectura automática (leer, listar, buscar)",
                    state.config.permissions.auto_read
                )
                .on_toggle(Message::PermReadToggled),
                checkbox(
                    "Escritura automática (crear, editar, borrar)",
                    state.config.permissions.auto_write
                )
                .on_toggle(Message::PermWriteToggled),
                checkbox(
                    "Consola automática (comandos permitidos)",
                    state.config.permissions.auto_bash
                )
                .on_toggle(Message::PermBashToggled),
                checkbox(
                    "Red automática (solo dominios listados abajo)",
                    state.config.permissions.auto_net
                )
                .on_toggle(Message::PermNetToggled),
                checkbox(
                    "Instalación automática (cargo/pip/npm/apt…)",
                    state.config.permissions.auto_install
                )
                .on_toggle(Message::PermInstallToggled),
                text("Recomendado: solo lectura. Red e instalación siempre piden.")
                    .size(design::fs(ts, 11))
                    .color(dim),
            ]
            .spacing(6)
    );
    let red_card = settings_card(
        &app_theme,
        state.config.appearance.compact(),
        "Red y rutas externas",
            column![
                components::field(
                    app_theme.clone(),
                    "Dominios permitidos (separados por comas)",
                    Some("Vacío = todos piden permiso. Ej: docs.rs, github.com".to_string())
                ),
                text_input("docs.rs, github.com...", &state.perm_domains)
                    .size(design::fs(ts, 13))
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::PermDomainsChanged)
                    .on_submit(Message::SavePermLists)
                    .width(iced::Fill),
                components::field(
                    app_theme.clone(),
                    "Carpetas fuera del workspace (separadas por comas)",
                    Some("Deben existir; se validan al guardar.".to_string())
                ),
                text_input("/ruta/a/datos, ...", &state.perm_extra)
                    .size(design::fs(ts, 13))
                    .style(|t: &Theme, s| design::field(t, s))
                    .on_input(Message::PermExtraChanged)
                    .on_submit(Message::SavePermLists)
                    .width(iced::Fill),
                row![
                    components::quiet_btn("Guardar listas".to_string())
                        .on_press(Message::SavePermLists),
                ]
                .spacing(8),
                text(&state.status).size(design::fs(ts, 12)).color(dim),
            ]
            .spacing(6)
    );
    let limites_card = settings_card(
        &app_theme,
        state.config.appearance.compact(),
        "Límites",
            column![
                row![
                    limit_label(ts, "Pasos por turno", "Máximo de pasos LLM→tools por tarea del worker."),
                    pick_list([3usize, 5, 10, 15, 20, 30].to_vec(), Some(lim.max_iters), Message::LimitItersPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Tareas del plan", "Número máximo de tareas que el planificador puede generar."),
                    pick_list([1usize, 2, 3, 4, 5].to_vec(), Some(lim.max_tasks), Message::LimitTasksPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Timeout consola (s)", "Tiempo máximo que puede correr un comando antes de matarlo."),
                    pick_list([10u64, 20, 30, 60, 120].to_vec(), Some(lim.bash_timeout_s), Message::LimitTimeoutPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Subida máxima (MB)", "Tamaño máximo por archivo al subirlo al workspace."),
                    pick_list([10u64, 25, 50, 100, 200].to_vec(), Some(lim.max_upload_mb), Message::LimitUploadPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Lectura máxima (KB)", "Tamaño máximo de archivo que el agente lee de una vez."),
                    pick_list([64u64, 128, 256, 512, 1024, 2048].to_vec(), Some(lim.max_read_kb), Message::LimitReadPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Historial al modelo", "Cuántos mensajes previos se envían al modelo; lo más antiguo se omite."),
                    pick_list([5usize, 10, 20, 30, 50].to_vec(), Some(lim.history_limit), Message::LimitHistoryPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Presupuesto turno", "Límite de tokens por turno del worker (0 = ilimitado). Al 80% compacta contexto."),
                    pick_list([0u64, 5_000, 10_000, 25_000, 50_000, 100_000].to_vec(), Some(lim.max_tokens_turn), Message::LimitTokensPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    limit_label(ts, "Ciclos de fix", "Ciclos auditor→fix por turno Work; 0 = ilimitado hasta quedar verde."),
                    pick_list([0usize, 1, 2, 3, 5, 10].to_vec(), Some(lim.max_fix_cycles), Message::LimitFixCyclesPicked)
                        .style(|t: &Theme, s| design::field_pick(t, s)),
                ]
                .align_y(iced::Alignment::Center),
                text(if lim.unlimited_fix_cycles() {
                    "Ciclos de fix ilimitados (0): el bucle auditor→fix sigue hasta quedar verde."
                } else {
                    "Ciclos de fix limitados: al llegar al tope con issues, no se commitea."
                })
                .size(design::fs(ts, 11))
                .color(dim),
                text(if lim.has_token_budget() {
                    "Con presupuesto: al 80% se compacta y avisa; al 100% el worker para con mensaje."
                } else {
                    "Sin presupuesto (0 = ilimitado): el Log muestra el gasto estimado (~tokens, ±30%)."
                })
                .size(design::fs(ts, 11))
                .color(dim),
                text("Se aplican al instante y se guardan.")
                    .size(design::fs(ts, 11))
                    .color(dim),
            ]
            .spacing(6)
    );
    column![
        row![permisos_card, red_card]
            .spacing(12)
            .align_y(iced::Alignment::Start),
        limites_card,
    ]
    .spacing(12)
    .max_width(1200)
    .into()
}

fn view_config_git(state: &App) -> Element<'_, Message> {
    use iced::widget::{checkbox, column, pick_list, row, text, text_input};
    use crate::config::{BranchMode, GitAutonomy};
    use crate::ui::{components, design};
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
            pick_list(BranchMode::ALL.to_vec(), Some(git.branch_mode), Message::GitBranchModePicked)
                .style(|t: &Theme, s| design::field_pick(t, s)),
            components::field(app_theme.clone(), "Remoto", None),
            text_input("origin", &state.git_remote)
                .size(design::fs(ts, 13))
                .style(|t: &Theme, s| design::field(t, s))
                .on_input(Message::GitRemoteChanged)
                .on_submit(Message::GitSave)
                .width(iced::Fill),
            components::field(app_theme.clone(), "Rama de push (vacío = rama de trabajo)", None),
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
            text("El workspace activo no es un repo git.").size(design::fs(ts, 12)).color(dim),
            row![components::primary_btn("Inicializar git".to_string(), 13).on_press(Message::GitInitWorkspace)]
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
                    text("Rama actual").size(design::fs(ts, 12)).color(dim).width(140),
                    text(if status.branch.is_empty() { "(desconocida)" } else { &status.branch })
                        .size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text("Remotos").size(design::fs(ts, 12)).color(dim).width(140),
                    text(remotes).size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    text("Cambios").size(design::fs(ts, 12)).color(dim).width(140),
                    text(format!("{} archivo(s)", status.changes)).size(design::fs(ts, 13)),
                ]
                .align_y(iced::Alignment::Center),
                row![
                    components::quiet_btn("Refrescar".to_string()).on_press(Message::GitRefreshStatus),
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

fn view_config_proyectos(state: &App) -> Element<'_, Message> {    use iced::widget::{column, container, row, text};
    use crate::ui::{components, design};
    use crate::ui::design::type_scale;
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let mut list = column![
        text("Proyectos").size(design::fs(ts, type_scale::TITLE)),
        text("Workspaces y archivos subidos. Borrar mueve a la papelera.")
            .size(design::fs(ts, 12))
            .color(dim),
        iced::widget::horizontal_rule(1).style(|t: &Theme| design::hairline_rule(t)),
    ]
    .spacing(4);
    if state.projects.is_empty() {
        list = list.push(components::empty_state(app_theme.clone(),
            "Sin proyectos todavía",
            "Crea uno desde Inicio para empezar.",
        ));
    }
    for project in &state.projects {
        let ws_path = project
            .path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let mut card = column![
            row![
                text(&project.name).size(design::fs(ts, type_scale::EMPHASIS)),
                iced::widget::horizontal_space(),
                if state.config_pending_delete == Some(project.id) {
                    components::danger_btn("Sí, borrar".to_string())
                        .on_press(Message::ConfirmConfigDelete)
                } else {
                    components::head_btn("Borrar".to_string())
                        .on_press(Message::ConfigDeleteProject(project.id))
                },
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
        ]
        .spacing(4);
        match ws_path {
            Some(ws) => {
                let uploads = workspace::list_uploads(std::path::Path::new(&ws));
                card = card.push(text(ws).size(design::fs(ts, 11)).color(dim));
                if uploads.is_empty() {
                    card = card.push(text("Sin archivos subidos.").size(design::fs(ts, 12)).color(dim));
                } else {
                    let mut files = column![text("Subidos").size(design::fs(ts, 11)).color(dim)].spacing(2);
                    for up in uploads {
                        let kb = up.size / 1024;
                        let pid = project.id;
                        let uname = up.name.clone();
                        files = files.push(
                            row![
                                text(format!("{} · {} KB", up.name, kb.max(1))).size(design::fs(ts, 12)).width(iced::Fill),
                                components::head_btn("Quitar".to_string())
                                    .on_press(Message::DeleteUpload(pid, uname)),
                            ]
                            .align_y(iced::Alignment::Center)
                            .spacing(6),
                        );
                    }
                    card = card.push(files);
                }
            }
            None => {
                card = card.push(text("Sin workspace asignado.").size(design::fs(ts, 12)).color(dim));
            }
        }
        if state.config_pending_delete == Some(project.id) {
            card = card.push(
                row![
                    text("Borrar el proyecto? Va a la papelera.").size(design::fs(ts, 12)),
                    components::head_btn("Cancelar".to_string()).on_press(Message::CancelConfigDelete),
                ]
                .spacing(8),
            );
        }
        list = list.push(
            container(card)
                .padding(12)
                .style(|t: &Theme| design::card(t)),
        );
    }
    if !state.status.is_empty() {
        list = list.push(text(&state.status).size(design::fs(ts, 12)).color(dim));
    }
    // Sin scrollable propio: view_config ya envuelve el contenido en uno.
    // (Un scrollable con height Fill dentro de otro scrollable = panic.)
    list.max_width(1200).spacing(10).into()
}

/// Apartado Atajos (v0.7.1): lista de solo lectura. Sin Tab para modos
/// (colisiona con el foco de los inputs).
fn view_config_atajos(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, row, text};
    use crate::ui::design;
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    const SHORTCUTS: [(&str, &str); 8] = [
        ("Ctrl+N", "Nuevo chat"),
        ("Ctrl+1 / Ctrl+2 / Ctrl+3", "Modos Chat / Plan / Work"),
        ("Ctrl+O", "Abrir proyecto"),
        ("Ctrl+,", "Configuración"),
        ("Ctrl+Z", "Deshacer último envío/borrado"),
        ("Esc", "Cerrar menús y paneles (o salir de Configuración)"),
        ("Doble Esc", "Detener el turno en curso"),
        ("Enter", "Enviar mensaje"),
    ];
    let mut list = column![
        text("Atajos de teclado").size(design::fs(ts, 15)),
        text("Solo lectura: funcionan en toda la app salvo escribiendo texto.")
            .size(design::fs(ts, 12))
            .color(dim),
    ]
    .spacing(4);
    for (keys, action) in SHORTCUTS {
        list = list.push(
            row![
                text(keys).size(design::fs(ts, 13)).width(200),
                text(action).size(design::fs(ts, 13)).color(dim),
            ]
            .spacing(8),
        );
    }
    settings_card(&app_theme, state.config.appearance.compact(), "Atajos", list)
}

// ---------------------------------------------------------------------------
// Navegador de modelos (models.dev) con búsqueda y precios.
// ---------------------------------------------------------------------------

fn fmt_per_m(v: f64) -> String {
    if v <= 0.0 {
        "0".to_string()
    } else if v < 0.01 {
        format!("${v:.4}")
    } else {
        format!("${v:.2}")
    }
}

fn price_line(cost: Option<&crate::pricing::Cost>) -> String {
    match cost {
        Some(c) if c.input <= 0.0 && c.output <= 0.0 => "gratis /1M".to_string(),
        Some(c) => {
            let mut s = format!("in {} · out {}", fmt_per_m(c.input), fmt_per_m(c.output));
            if let Some(cr) = c.cache_read {
                s.push_str(&format!(" · cache {}", fmt_per_m(cr)));
            }
            if let Some(cw) = c.cache_write {
                s.push_str(&format!(" / write {}", fmt_per_m(cw)));
            }
            s.push_str(" /1M");
            s
        }
        None => String::new(),
    }
}

fn context_label(ctx: Option<u64>) -> String {
    match ctx {
        Some(c) => format!("{} ctx", crate::llm::format_tokens(c)),
        None => String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn model_row<'a>(
    app_theme: &Theme,
    ts: f32,
    id: String,
    name: String,
    cost: Option<crate::pricing::Cost>,
    context: Option<u64>,
    tool: bool,
    reasoning: bool,
) -> Element<'a, Message> {
    use iced::widget::{column, container, row, text};
    use crate::ui::{components, design};
    let dim = design::ink_2(app_theme);
    let mut badges = row![].spacing(4);
    if tool {
        badges = badges.push(components::badge(app_theme.clone(), design::Tone::Ok, "tools"));
    }
    if reasoning {
        badges = badges.push(components::badge(app_theme.clone(), design::Tone::Accent, "razona"));
    }
    let pick = id.clone();
    // Sin scroll horizontal: los textos ocupan el ancho disponible (con wrap)
    // y el id largo se trunca visualmente; el click usa el id completo.
    let id_short = design::trunc_end(&id, 64);
    iced::widget::button(
        container(
            column![
                row![
                    text(name).size(design::fs(ts, 14)).width(iced::Fill),
                    badges,
                    text(price_line(cost.as_ref())).size(design::fs(ts, 12)).color(dim),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                row![
                    text(id_short).size(design::fs(ts, 11)).color(dim).width(iced::Fill),
                    text(context_label(context)).size(design::fs(ts, 11)).color(dim),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(2)
            .width(iced::Fill),
        )
        .padding(8),
    )
    .width(iced::Fill)
    .padding(0)
    .style(|t: &Theme, s| design::nav(t, s, false))
    .on_press(Message::PickModel(pick))
    .into()
}

/// id, name, coste, contexto, tools, razona.
type BrowserRow = (String, String, Option<crate::pricing::Cost>, Option<u64>, bool, bool);

fn model_browser(state: &App) -> Element<'_, Message> {
    use iced::widget::{checkbox, column, container, pick_list, row, scrollable, text, text_input};
    use crate::ui::{components, design};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let is_local = state.edit_provider == crate::config::Provider::Local;
    let provider_entry = if is_local {
        None
    } else {
        crate::pricing::provider_id_for(state.edit_provider, &state.edit_base_url)
            .and_then(|pid| state.pricing.provider(&pid))
    };
    let provider_label = provider_entry
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "LM Studio (local)".to_string());
    let header = row![
        components::section_label(app_theme.clone(), format!("Modelos · {provider_label}")),
        iced::widget::horizontal_space(),
        components::head_btn("Actualizar".to_string()).on_press(Message::RefreshPricing),
        components::head_btn("Cerrar".to_string()).on_press(Message::CloseModelBrowser),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let mut hints: Vec<String> = Vec::new();
    if let Some(p) = provider_entry {
        if !p.env.is_empty() {
            hints.push(format!("API key: {}", p.env.join(" / ")));
        }
        if let Some(api) = &p.api {
            hints.push(format!("Endpoint: {api}"));
        }
    }
    let hint_line: Element<'_, Message> = if hints.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        text(hints.join("  ·  ")).size(design::fs(ts, 11)).color(dim).into()
    };

    let search = text_input("Buscar modelo (id, nombre, familia)...", &state.model_search)
        .size(design::fs(ts, 13))
        .style(|t: &Theme, s| design::field(t, s))
        .on_input(Message::ModelSearchChanged)
        .width(iced::Fill);

    let price_filters: Vec<String> = ["Todos", "Gratis", "≤ $1", "≤ $5", "≤ $15"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let filter_row = row![
        text("Precio").size(design::fs(ts, 12)).color(dim),
        pick_list(
            price_filters,
            Some(state.model_price_filter.clone()),
            Message::ModelPriceFilterPicked
        )
        .style(|t: &Theme, s| design::field_pick(t, s)),
        checkbox("solo tools", state.model_only_tools).on_toggle(Message::ModelOnlyToolsToggled),
        checkbox("ordenar por precio", state.model_sort_price)
            .on_toggle(Message::ModelSortPriceToggled),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let status_line: Element<'_, Message> = if state.model_status.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        text(&state.model_status).size(design::fs(ts, 12)).color(dim).into()
    };

    let ql = state.model_search.to_lowercase();
    let matches = |id: &str, name: &str, family: &Option<String>| {
        ql.is_empty()
            || id.to_lowercase().contains(&ql)
            || name.to_lowercase().contains(&ql)
            || family.as_deref().map(|f| f.to_lowercase().contains(&ql)).unwrap_or(false)
    };

    let mut rows: Vec<BrowserRow> = Vec::new();
    if is_local {
        for id in &state.local_models {
            if !matches(id, id, &None) {
                continue;
            }
            let ml = state.pricing.lookup(id);
            let (cost, ctx) = (ml.and_then(|m| m.cost), ml.and_then(|m| m.context));
            rows.push((id.clone(), id.clone(), cost, ctx, false, false));
        }
    } else if let Some(p) = provider_entry {
        for m in &p.models {
            if !matches(&m.id, &m.name, &m.family) {
                continue;
            }
            rows.push((
                m.id.clone(),
                m.name.clone(),
                m.cost,
                m.context,
                m.tool_call,
                m.reasoning,
            ));
        }
    }

    // Filtro de precio (sobre input) + solo tools + orden por precio.
    let pf = state.model_price_filter.as_str();
    rows.retain(|r| {
        let c = r.2;
        match pf {
            "Gratis" => c.map(|c| c.input <= 0.0 && c.output <= 0.0).unwrap_or(false),
            "≤ $1" => c.map(|c| c.input <= 1.0).unwrap_or(false),
            "≤ $5" => c.map(|c| c.input <= 5.0).unwrap_or(false),
            "≤ $15" => c.map(|c| c.input <= 15.0).unwrap_or(false),
            _ => true,
        }
    });
    if state.model_only_tools {
        rows.retain(|r| r.4);
    }
    if state.model_sort_price {
        rows.sort_by(|a, b| {
            let pa = a.2.map(|c| c.input).unwrap_or(f64::INFINITY);
            let pb = b.2.map(|c| c.input).unwrap_or(f64::INFINITY);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let count = rows.len();
    let total_line: Element<'_, Message> = text(format!("{count} modelos"))
        .size(design::fs(ts, 11))
        .color(dim)
        .into();

    let mut list = column![].spacing(4);
    for r in rows.iter().take(300) {
        list = list.push(model_row(
            &app_theme,
            ts,
            r.0.clone(),
            r.1.clone(),
            r.2,
            r.3,
            r.4,
            r.5,
        ));
    }

    let body: Element<'_, Message> = if state.models_loading {
        text("Cargando...").size(design::fs(ts, 13)).color(dim).into()
    } else if count == 0 {
        text("Sin resultados. Ajusta la búsqueda/filtros o pulsa Actualizar.")
            .size(design::fs(ts, 12))
            .color(dim)
            .into()
    } else {
        // Aire a la derecha para que la barra vertical no tape los datos
        // de la última columna (precio/contexto). Con anchos Fill + id
        // truncado ya no hay desborde horizontal.
        scrollable(
            container(list.width(iced::Fill)).padding(iced::Padding {
                top: 0.0,
                right: 20.0,
                bottom: 12.0,
                left: 0.0,
            }),
        )
        .height(360)
        .into()
    };

    container(column![header, search, filter_row, total_line, hint_line, status_line, body].spacing(8))
        .padding(12)
        .style(|t: &Theme| design::well_box(t))
        .into()
}
