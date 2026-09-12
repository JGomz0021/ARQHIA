//! Sidebar: navegación de chats y proyectos.
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::db::ChatMeta;
use crate::titles;

pub(crate) fn chat_row<'a>(state: &'a App, chat: &'a ChatMeta) -> Element<'a, Message> {
    use iced::widget::{column, container, row, text};
    use crate::ui::{components, design};
    use crate::ui::design::type_scale;
    let selected = state.active_chat == Some(chat.id);
    let ts = state.config.appearance.text_size.scale();
    if state.pending_delete == Some(chat.id) {
        // Confirmación apilada: el texto arriba y los botones debajo para que
        // no se aplasten en el ancho del sidebar.
        return column![
            text("¿Borrar este chat?").size(design::fs(ts, type_scale::SECONDARY)),
            row![
                components::danger_btn("Sí".to_string()).on_press(Message::ConfirmDeleteChat),
                components::head_btn("No".to_string()).on_press(Message::CancelDelete),
            ]
            .spacing(6),
        ]
        .spacing(4)
        .width(iced::Fill)
        .into();
    }
    let label = titles::chat_label(&chat.title);
    let sel = selected;
    let cid = chat.id;
    let btn = iced::widget::button(text(label).size(design::fs(ts, type_scale::BODY)))
        .width(iced::Fill)
        .padding([5, 8])
        .style(move |t: &Theme, s| crate::ui::design::nav(t, s, sel))
        .on_press(Message::SelectChat(chat.id));
    let mut col = column![
        row![
            btn,
            components::icon_btn("...".to_string()).on_press(Message::ToggleChatMenu(cid)),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(2);
    // Menú del chat: borrar / archivar-restaurar / mover a proyecto.
    if state.chat_menu == Some(cid) {
        let mut menu = column![].spacing(4);
        if state.move_for == Some(cid) {
            menu = menu.push(text("Mover a:").size(design::fs(ts, 12)));
            let others: Vec<(i64, String)> = state
                .projects
                .iter()
                .filter(|p| Some(p.id) != chat.project_id)
                .map(|p| (p.id, p.name.clone()))
                .collect();
            if chat.project_id.is_some() {
                menu = menu.push(
                    components::icon_btn("Sin proyecto".to_string()).on_press(Message::AssignChatProject {
                        chat: cid,
                        project: None,
                    }),
                );
            }
            let no_targets = others.is_empty() && chat.project_id.is_none();
            for (pid, name) in &others {
                let pid = *pid;
                menu = menu.push(
                    components::icon_btn(name.clone()).on_press(Message::AssignChatProject {
                        chat: cid,
                        project: Some(pid),
                    }),
                );
            }
            if no_targets {
                menu = menu.push(text("No hay proyectos.").size(design::fs(ts, 12)));
            }
        } else {
            menu = menu.push(
                components::icon_btn("Mover a proyecto…".to_string()).on_press(Message::ToggleMovePick(cid)),
            );
        }
        if chat.archived {
            menu = menu.push(
                components::icon_btn("Restaurar".to_string()).on_press(Message::UnarchiveChat(cid)),
            );
        } else {
            menu = menu.push(
                components::icon_btn("Archivar".to_string()).on_press(Message::ArchiveChat(cid)),
            );
        }
        menu = menu.push(
            components::icon_btn("Borrar".to_string()).on_press(Message::DeleteChat(cid)),
        );
        // v0.8: regenera el session_id del chat (limpia la caché del provider).
        // Solo tiene sentido sobre el chat activo (la sesión vive por chat).
        if state.active_chat == Some(cid) {
            menu = menu.push(
                components::icon_btn("Reiniciar sesión".to_string()).on_press(Message::ResetSession),
            );
        }
        col = col.push(
            container(menu)
                .padding(8)
                .width(iced::Fill)
                .style(|t: &Theme| design::well_box(t)),
        );
    }
    col.into()
}

pub(crate) fn view_sidebar(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, row, scrollable, text, text_input};
    use crate::ui::{components, design};
    use crate::ui::design::type_scale;
    let cx = state.config.appearance.compact();
    let mut list = column![].spacing(design::gap(cx, 8));
    let ts = state.config.appearance.text_size.scale();

    // Loose chats (no project).
    let loose: Vec<&ChatMeta> = state
        .chats
        .iter()
        .filter(|c| c.project_id.is_none() && !c.archived)
        .collect();
    if !loose.is_empty() {
        list = list.push(components::section_label(super::app_theme(state), "Conversaciones"));
        for chat in loose {
            list = list.push(chat_row(state, chat));
        }
        // Separador claro entre chats sueltos y proyectos.
        list = list.push(
            iced::widget::horizontal_rule(1).style(|t: &Theme| design::hairline_rule(t)),
        );
    }
    // Projects.
    list = list.push(
        row![
            components::section_label(super::app_theme(state), "Proyectos"),
            iced::widget::horizontal_space(),
            components::head_btn("+ Nuevo".to_string()).on_press(Message::ToggleProjectForm),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    );
    if state.show_project_form {
        list = list.push(
            container(
                column![
                    text_input("Nombre del proyecto...", &state.new_project_name)
                        .size(design::fs(ts, 12))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::NewProjectNameChanged)
                        .on_submit(Message::CreateProject)
                        .width(iced::Fill),
                    text_input("Ruta (vacío = ~/ARQHIA/projects/...)", &state.new_project_path)
                        .size(design::fs(ts, 11))
                        .style(|t: &Theme, s| design::field(t, s))
                        .on_input(Message::NewProjectPathChanged)
                        .on_submit(Message::CreateProject)
                        .width(iced::Fill),
                    row![
                        components::primary_btn("Crear".to_string(), 12).on_press(Message::CreateProject),
                        components::icon_btn("Cancelar".to_string()).on_press(Message::ToggleProjectForm),
                    ]
                    .spacing(6),
                ]
                .spacing(6),
            )
            .padding(10)
            .style(|t: &Theme| design::card(t)),
        );
    }
    if state.projects.is_empty() && !state.show_project_form {
        list = list.push(components::empty_state(super::app_theme(state),
            "Sin proyectos",
            "Crea uno para agrupar chats y asignar workspace.",
        ));
    }
    for project in &state.projects {
        let project_chats: Vec<&ChatMeta> = state
            .chats
            .iter()
            .filter(|c| c.project_id == Some(project.id) && !c.archived)
            .collect();
        let collapsed = state.collapsed.contains(&project.id);
        let chevron = if collapsed { ">" } else { "v" };
        let has_ws = project
            .path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_some();
        if state.pending_project_delete == Some(project.id) {
            list = list.push(
                container(
                    column![
                        text(format!("Borrar '{}'?", project.name)).size(design::fs(ts, 12)),
                        text("Sus chats se eliminan y la carpeta va a la papelera.")
                            .size(design::fs(ts, 11)),
                        row![
                            components::danger_btn("Sí, borrar".to_string())
                                .on_press(Message::ConfirmDeleteProject),
                            components::icon_btn("No".to_string()).on_press(Message::CancelDeleteProject),
                        ]
                        .spacing(6),
                    ]
                    .spacing(4),
                )
                .padding(8)
                .style(|t: &Theme| design::card(t)),
            );
            continue;
        }
        // Project block: header + context + chats wrapped in a card so
        // projects read as units, clearly apart from loose chats.
        // Workspace actions live behind the ⋯ menu, not inline.
        let menu_open = state.project_menu == Some(project.id);
        let mut pcol = column![
            row![
                components::icon_btn(chevron.to_string()).on_press(Message::ToggleProject(project.id)),
                text(&project.name).size(design::fs(ts, type_scale::BODY)),
                components::dot(super::app_theme(state),
                    if has_ws {
                        design::Tone::Ok
                    } else {
                        design::Tone::Neutral
                    }
                ),
                text(format!("{}", project_chats.len()))
                    .size(design::fs(ts, 12)),
                iced::widget::horizontal_space(),
                components::icon_btn("+".to_string()).on_press(Message::NewChatInProject(project.id)),
                components::icon_btn("...".to_string()).on_press(Message::ToggleProjectMenu(project.id)),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(design::gap(cx, 4));
        if collapsed {
            list = list.push(
                container(pcol)
                    .padding(design::pad(cx, 6))
                    .style(|t: &Theme| design::card(t)),
            );
            continue;
        }
        match project.path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            // Con workspace: la ruta solo vive dentro del menú ⋯ (input
            // prellenado), no como línea visible permanente.
            Some(_) => {
                if menu_open {
                    let current = state
                        .workspace_inputs
                        .get(&project.id)
                        .cloned()
                        .unwrap_or_default();
                    let pid = project.id;
                    pcol = pcol.push(
                        container(
                            column![
                                text("Cambiar ruta del workspace").size(design::fs(ts, 12)),
                                row![
                                    text_input("Ruta de la carpeta...", &current)
                                        .size(design::fs(ts, 12))
                                        .style(|t: &Theme, s| design::field(t, s))
                                        .on_input(move |v| {
                                            Message::WorkspacePathChanged(pid, v)
                                        })
                                        .on_submit(Message::AssignWorkspace(pid))
                                        .width(iced::Fill),
                                    components::head_btn("Guardar".to_string())
                                        .on_press(Message::AssignWorkspace(pid)),
                                ]
                                .spacing(4)
                                .align_y(iced::Alignment::Center),
                                row![
                                    components::head_btn("Abrir carpeta".to_string())
                                        .on_press(Message::OpenWorkspaceFolder(pid)),
                                    components::head_btn("Subir archivo".to_string())
                                        .on_press(Message::UploadFiles(pid)),
                                ]
                                .spacing(4),
                                row![
                                    components::icon_btn("Quitar workspace".to_string())
                                        .on_press(Message::ClearWorkspace(pid)),
                                    iced::widget::horizontal_space(),
                                    components::danger_btn("Borrar".to_string())
                                        .on_press(Message::DeleteProject(pid)),
                                ]
                                .spacing(4)
                                .align_y(iced::Alignment::Center),
                            ]
                            .spacing(6),
                        )
                        .padding(8)
                        .style(|t: &Theme| design::well_box(t)),
                    );
                }
            }
            None => {
                let value = state
                    .workspace_inputs
                    .get(&project.id)
                    .cloned()
                    .unwrap_or_default();
                let pid = project.id;
                pcol = pcol.push(
                    row![
                        text_input("Carpeta del proyecto...", &value)
                            .size(design::fs(ts, 12))
                            .style(|t: &Theme, s| design::field(t, s))
                            .on_input(move |v| Message::WorkspacePathChanged(pid, v))
                            .on_submit(Message::AssignWorkspace(pid))
                            .width(iced::Fill),
                        components::head_btn("Asignar".to_string()).on_press(Message::AssignWorkspace(pid)),
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                );
            }
        }
        if project_chats.is_empty() {
            pcol = pcol.push(text("Sin chats todavía.").size(design::fs(ts, 12)));
        }
        for chat in project_chats {
            pcol = pcol.push(container(chat_row(state, chat)).padding(iced::Padding {
                left: 14.0,
                ..iced::Padding::ZERO
            }));
        }
        list = list.push(
            container(pcol)
                .padding(design::pad(cx, 6))
                .style(|t: &Theme| design::card(t)),
        );
    }
    if state.chats.is_empty() {
        list = list.push(text("Aún no hay chats.").size(design::fs(ts, 12)));
    }
    // Archivados: ocultos por defecto, restaurables o borrables.
    let archived: Vec<&ChatMeta> = state.chats.iter().filter(|c| c.archived).collect();
    if !archived.is_empty() {
        list = list.push(
            iced::widget::horizontal_rule(1).style(|t: &Theme| design::hairline_rule(t)),
        );
        let archived_n = state.chats.iter().filter(|c| c.archived).count();
        let archived_open = state.show_archived;
        // (Button es invariante en 'a: va envuelto en row! como el resto.)
        let toggle_row: Element<'_, Message> = row![
            components::head_btn(format!(
                "Archivados ({archived_n}) {}",
                if archived_open { "v" } else { ">" }
            ))
            .width(iced::Fill)
            .on_press(Message::ToggleArchived),
        ]
        .spacing(0)
        .into();
        list = list.push(toggle_row);
        if state.show_archived {
            for chat in archived {
                let proj = chat
                    .project_id
                    .and_then(|id| state.projects.iter().find(|p| p.id == id))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Sin proyecto".to_string());
                list = list.push(
                    column![
                        text(proj).size(design::fs(ts, 11)),
                        chat_row(state, chat),
                    ]
                    .spacing(0),
                );
            }
        }
    }

    column![
        components::primary_btn("+ Nuevo chat".to_string(), 13)
            .width(iced::Fill)
            .on_press(Message::NewChat),
        container(scrollable(list).height(iced::Fill)).height(iced::Fill),
        iced::widget::horizontal_rule(1).style(|t: &Theme| design::hairline_rule(t)),
        container(
            row![
                components::head_btn("Inicio".to_string()).on_press(Message::GoHome),
                components::head_btn("Ajustes".to_string()).on_press(Message::OpenConfig),
                iced::widget::horizontal_space(),
                components::danger_outline_btn("Salir".to_string()).on_press(Message::ExitApp),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(iced::Padding {
            top: 4.0,
            ..iced::Padding::ZERO
        }),
    ]
    .spacing(design::gap(cx, 10))
    .padding(design::pad(cx, 12))
    .into()
}
