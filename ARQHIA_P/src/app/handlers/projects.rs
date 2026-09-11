//! Handler de Proyectos, workspaces y uploads.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::llm::{ChatMsg, Role};
use crate::app::Message;
use crate::app::projects::{create_project_with_dir, enter_questionnaire, remove_project_everywhere, resolve_project_dir};
use crate::app::state::clear_turn_state;
use crate::app::View;
use crate::db;
use crate::questionnaire::Answers;
use crate::workspace;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ShowCreateModal => {
            state.view = View::Home;
            state.creating = true;
            state.status.clear();
            Task::none()
        }
        Message::HideCreateModal => {
            state.creating = false;
            state.status.clear();
            Task::none()
        }
        Message::CreateNameChanged(v) => {
            state.create_name = v;
            Task::none()
        }
        Message::CreatePathChanged(v) => {
            state.create_path = v;
            Task::none()
        }
        Message::SubmitCreateProject => {
            let name = state.create_name.trim().to_string();
            if name.is_empty() {
                state.status = "Pon un nombre al proyecto.".to_string();
                return Task::none();
            }
            match db::project_name_exists(&name) {
                Ok(true) => {
                    state.status = format!("Ya existe un proyecto llamado '{name}'.");
                    return Task::none();
                }
                Err(e) => {
                    state.status = format!("No se pudo verificar el nombre: {e}");
                    return Task::none();
                }
                Ok(false) => {}
            }
            let dir = resolve_project_dir(&name, &state.create_path.clone());
            match create_project_with_dir(state, &name, dir) {
                Ok((pid, path_str)) => {
                    state.creating = false;
                    state.create_name.clear();
                    state.create_path.clear();
                    enter_questionnaire(state, pid, &name, &path_str);
                }
                Err(e) => state.status = e,
            }
            Task::none()
        }
        Message::OpenProject => {
            // rfd bloquea: va en hilo aparte, nunca en el update directo
            state.status = "Elige una carpeta…".to_string();            Task::perform(
                async move {
                    tokio::task::spawn_blocking(|| {
                        rfd::FileDialog::new().pick_folder()
                    })
                    .await
                    .ok()
                    .flatten()
                },
                Message::FolderPicked,
            )
        }
        Message::EnterProject(pid) => {
            // Launcher: entra al proyecto (último chat o chat pendiente).
            if state.projects.iter().all(|p| p.id != pid) {
                return Task::none();
            }
            navigate_project(state, Some(pid))
        }
        Message::FolderPicked(picked) => {
            let dir = match picked {
                Some(p) => p,
                None => {
                    state.status = "Sin selección.".to_string();
                    return Task::none();
                }
            };
            let canon = match workspace::register_existing(&dir) {
                Ok(c) => c,
                Err(e) => {
                    state.status = format!("Carpeta inválida: {e}");
                    return Task::none();
                }
            };
            let base_name = canon
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Proyecto".to_string());
            let path_str = canon.to_string_lossy().to_string();
            // Reutiliza el proyecto si la ruta ya está registrada
            let (pid, is_new) = match state.projects.iter().find(|p| p.path.as_deref() == Some(&path_str)) {
                Some(p) => (p.id, false),
                None => {
                    // El nombre viene de la carpeta: si choca, auto-sufijo " (2)"
                    let name = match db::free_project_name(&base_name) {
                        Ok(n) => n,
                        Err(e) => {
                            state.status = format!("No se pudo nombrar: {e}");
                            return Task::none();
                        }
                    };
                    match db::create_project(&name) {
                        Ok(id) => {
                            let _ = db::set_project_path(id, Some(&path_str));
                            state.projects.push(db::Project {
                                id,
                                name: name.clone(),
                                path: Some(path_str.clone()),
                            });
                            (id, true)
                        }
                        Err(e) => {
                            state.status = format!("No se pudo registrar: {e}");
                            return Task::none();
                        }
                    }
                }
            };
            let shown = state
                .projects
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| base_name.clone());
            let _ = db::touch_project(pid);
            // Sin chat inicial: el chat se crea al enviar el primer mensaje.
            state.active_chat = None;
            state.messages.clear();
            state.md.clear();
            state.pending_project = Some(pid);
            state.push_log(format!("📁 abierto {shown} -> {path_str}"));
            state.status.clear();
            if is_new {
                // Proyecto nuevo -> cuestionario inmediato
                let answers = Answers {
                    nombre: shown.clone(),
                    ..Default::default()
                };
                state.q_answers = answers;
                state.q_step = 0;
                state.q_error.clear();
                state.q_project = Some(pid);
                state.view = View::Questionnaire;
            } else {
                state.view = View::Chat;
            }
            Task::none()
        }
        Message::NewProjectNameChanged(v) => {
            state.new_project_name = v;
            Task::none()
        }
        Message::NewProjectPathChanged(v) => {
            state.new_project_path = v;
            Task::none()
        }
        Message::CreateProject => {
            let name = state.new_project_name.trim().to_string();
            if name.is_empty() {
                state.status = "Pon un nombre al proyecto.".to_string();
                return Task::none();
            }
            match db::project_name_exists(&name) {
                Ok(true) => {
                    state.status = format!("Ya existe un proyecto llamado '{name}'.");
                    return Task::none();
                }
                Err(e) => {
                    state.status = format!("No se pudo verificar el nombre: {e}");
                    return Task::none();
                }
                Ok(false) => {}
            }
            // Carpeta: la escrita o ~/ARQHIA/projects/{slug} por defecto
            let dir = resolve_project_dir(&name, &state.new_project_path.clone());
            match create_project_with_dir(state, &name, dir) {
                Ok((pid, path_str)) => {
                    state.new_project_name.clear();
                    state.new_project_path.clear();
                    state.show_project_form = false;
                    state.status.clear();
                    enter_questionnaire(state, pid, &name, &path_str);
                }
                Err(e) => state.status = e,
            }
            Task::none()
        }
        Message::ToggleProjectForm => {
            state.show_project_form = !state.show_project_form;
            state.new_project_name.clear();
            state.new_project_path.clear();
            Task::none()
        }
        Message::ToggleProject(id) => {
            if !state.collapsed.remove(&id) {
                state.collapsed.insert(id);
            }
            Task::none()
        }
        Message::ToggleProjectMenu(pid) => {
            state.project_menu = if state.project_menu == Some(pid) {
                None
            } else {
                // Prefill del input con la ruta actual para "cambiar ruta".
                if let Some(cur) = state
                    .projects
                    .iter()
                    .find(|p| p.id == pid)
                    .and_then(|p| p.path.clone())
                {
                    state.workspace_inputs.insert(pid, cur);
                }
                Some(pid)
            };
            Task::none()
        }
        Message::OpenWorkspaceFolder(pid) => {
            let raw = state
                .projects
                .iter()
                .find(|p| p.id == pid)
                .and_then(|p| p.path.clone())
                .unwrap_or_default();
            let expanded = if let Some(rest) = raw.strip_prefix("~/") {
                format!(
                    "{}/{rest}",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            } else {
                raw.clone()
            };
            if std::path::Path::new(&expanded).is_dir() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(&expanded)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                state.status = "Abriendo carpeta en el explorador…".to_string();
            } else {
                state.status = "La carpeta del workspace ya no existe.".to_string();
            }
            Task::none()
        }
        Message::DeleteProject(id) => {
            state.pending_project_delete = Some(id);
            Task::none()
        }
        Message::ConfirmDeleteProject => {
            let id = match state.pending_project_delete {
                Some(id) => id,
                None => return Task::none(),
            };
            state.pending_project_delete = None;
            match remove_project_everywhere(state, id) {
                Ok(line) => {
                    state.push_log(line);
                    state.status.clear();
                }
                Err(e) => state.status = e,
            }
            Task::none()
        }
        Message::CancelDeleteProject => {
            state.pending_project_delete = None;
            Task::none()
        }
        Message::WorkspacePathChanged(pid, v) => {
            state.workspace_inputs.insert(pid, v);
            Task::none()
        }
        Message::AssignWorkspace(pid) => {
            let raw = state
                .workspace_inputs
                .get(&pid)
                .cloned()
                .unwrap_or_default();
            match workspace::validate(&raw) {
                Ok(canon) => {
                    let s = canon.to_string_lossy().to_string();
                    match db::set_project_path(pid, Some(&s)) {
                        Ok(()) => {
                            if let Some(p) = state.projects.iter_mut().find(|p| p.id == pid) {
                                p.path = Some(s.clone());
                            }
                            state.workspace_inputs.remove(&pid);
                            state.project_menu = None;
                            state.status = format!("Workspace asignado: {s}");
                            state.push_log(format!("📁 workspace -> {s}"));
                            // Aviso de privacidad una sola vez (v0.7 Track B,
                            // ver CONTEXT/POLICIES.md §2).
                            if !state.config.privacy_notice_shown {
                                state.config.privacy_notice_shown = true;
                                let _ = state.config.save();
                                state.push_log("🔒 El código del workspace viaja al proveedor activo al chatear. Ver POLICIES §2.".to_string());
                            }
                        }
                        Err(e) => state.status = format!("No se pudo guardar: {e}"),
                    }
                }
                Err(e) => state.status = format!("Workspace inválido: {e}"),
            }
            Task::none()
        }
        Message::ClearWorkspace(pid) => {
            match db::set_project_path(pid, None) {
                Ok(()) => {
                    if let Some(p) = state.projects.iter_mut().find(|p| p.id == pid) {
                        p.path = None;
                    }
                    state.status = "Workspace desasignado (vuelve a chat normal).".to_string();
                    state.project_menu = None;
                    state.push_log("📁 workspace desasignado".to_string());
                }
                Err(e) => state.status = format!("No se pudo quitar: {e}"),
            }
            Task::none()
        }
        Message::UploadFiles(pid) => {
            if state.projects.iter().all(|p| p.id != pid) {
                return Task::none();
            }
            state.status = "Elige archivos…".to_string();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(|| {
                        rfd::FileDialog::new().pick_files()
                    })
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default()
                },
                move |files| Message::FilesPicked(pid, files),
            )
        }
        Message::FilesPicked(pid, files) => {
            if files.is_empty() {
                state.status = "Sin selección.".to_string();
                return Task::none();
            }
            let ws = match state
                .projects
                .iter()
                .find(|p| p.id == pid)
                .and_then(|p| p.path.clone())
            {
                Some(raw) => {
                    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
                        format!(
                            "{}/{rest}",
                            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                        )
                    } else {
                        raw
                    };
                    std::path::PathBuf::from(expanded)
                }
                None => {
                    state.status = "El proyecto ya no existe.".to_string();
                    return Task::none();
                }
            };
            let max_bytes = state.config.limits.clamped().max_upload_mb * 1024 * 1024;
            let (oks, errs) = workspace::upload_files(&ws, &files, max_bytes);
            for name in &oks {
                state.push_log(format!("📤 {name} -> uploads/"));
            }
            state.status = if errs.is_empty() {
                format!("{} archivo(s) subidos a uploads/.", oks.len())
            } else {
                format!("{} ok, {} con error: {}", oks.len(), errs.len(), errs.join(" | "))
            };
            Task::none()
        }
        Message::DeleteUpload(pid, name) => {
            let ws = match state
                .projects
                .iter()
                .find(|p| p.id == pid)
                .and_then(|p| p.path.clone())
            {
                Some(raw) => std::path::PathBuf::from(raw),
                None => {
                    state.status = "El proyecto ya no existe.".to_string();
                    return Task::none();
                }
            };
            match workspace::delete_upload(&ws, &name) {
                Ok(()) => {
                    state.push_log(format!("🗑 upload borrado: {name}"));
                    state.status.clear();
                }
                Err(e) => state.status = format!("No se pudo borrar: {e}"),
            }
            Task::none()
        }
        Message::ConfigDeleteProject(id) => {
            state.config_pending_delete = Some(id);
            Task::none()
        }
        Message::ConfirmConfigDelete => {
            let id = match state.config_pending_delete {
                Some(id) => id,
                None => return Task::none(),
            };
            state.config_pending_delete = None;
            match remove_project_everywhere(state, id) {
                Ok(line) => {
                    state.push_log(line);
                    state.status = "Proyecto borrado (ver Log).".to_string();
                }
                Err(e) => state.status = e,
            }
            Task::none()
        }
        Message::CancelConfigDelete => {
            state.config_pending_delete = None;
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}

/// Navega a un proyecto (o a sueltos con None): último chat visible o
/// chat pendiente. Usado por el launcher y por el selector del header.
/// Nunca mueve el chat actual (eso es AssignChatProject desde el menú ⋯).
pub(crate) fn navigate_project(state: &mut App, pid: Option<i64>) -> Task<Message> {
    use iced::Task;

    if state.streaming || state.agent_running {
        return Task::none();
    }
    state.creating = false;
    state.status.clear();
    state.pending_delete = None;
    clear_turn_state(state);
    match state
        .chats
        .iter()
        .rev()
        .find(|c| c.project_id == pid && !c.archived)
        .map(|c| c.id)
    {
        Some(id) => {
            state.active_chat = Some(id);
            state.messages = db::load_chat_history(id, 500)
                .unwrap_or_default()
                .into_iter()
                .map(|(role, content)| ChatMsg {
                    role: Role::from_str(&role),
                    content,
                })
                .collect();
            state.reparse_md();
            state.pending_project = None;
        }
        None => {
            state.active_chat = None;
            state.messages.clear();
            state.md.clear();
            state.pending_project = pid;
        }
    }
    state.view = View::Chat;
    Task::none()
}
