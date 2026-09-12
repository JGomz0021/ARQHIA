//! Operaciones de proyecto: crear, registrar, papelera, borrado total.
//!
//! Frontera entre estado, SQLite y filesystem. La UI llama aquí vía update.

use super::events::View;
use super::state::{App, clear_turn_state};
use crate::db;
use crate::workspace;
use crate::questionnaire::Answers;

/// Crea carpeta + fila SQLite + workspace y lo registra en el estado.
/// Devuelve (pid, path). Usado por Crear (Home) y Crear (sidebar).
pub(crate) fn create_project_with_dir(
    state: &mut App,
    name: &str,
    dir: std::path::PathBuf,
) -> Result<(i64, String), String> {
    std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear la carpeta: {e}"))?;
    let canon = dir
        .canonicalize()
        .map_err(|e| format!("No se pudo resolver la ruta: {e}"))?;
    let path_str = canon.to_string_lossy().to_string();
    let pid =
        db::create_project(name).map_err(|e| format!("No se pudo guardar el proyecto: {e}"))?;
    db::set_project_path(pid, Some(&path_str))
        .map_err(|e| format!("Proyecto creado pero sin workspace: {e}"))?;
    let _ = db::touch_project(pid);
    state.projects.push(db::Project {
        id: pid,
        name: name.to_string(),
        path: Some(path_str.clone()),
    });
    init_workspace_git(state, &canon);
    Ok((pid, path_str))
}

/// Inicializa git en el workspace si la config lo pide (v0.7.2): `init` si
/// no es repo y checkout de la rama de trabajo. Nunca reinicia un repo ni
/// pisa la rama protegida. Loguea el resultado.
pub(crate) fn init_workspace_git(state: &mut App, ws: &std::path::Path) {
    let git = state.config.git.clone();
    if !git.enabled {
        return;
    }
    let result = if crate::git::is_repo(ws) {
        crate::git::ensure_work_branch(ws, &git)
    } else if git.auto_init {
        crate::git::init_repo(ws, &git.base_branch)
            .and_then(|()| crate::git::ensure_work_branch(ws, &git))
    } else {
        return;
    };
    match result {
        Ok(()) => {
            let branch = crate::git::current_branch(ws).unwrap_or_else(|| git.work_branch.clone());
            state.push_log(format!("🌿 git: rama {branch}"));
        }
        Err(e) => state.push_log(format!("⚠️ git: {e}")),
    }
}

/// Resuelve la carpeta: la escrita o ~/ARQHIA/projects/{slug} por defecto.
pub(crate) fn resolve_project_dir(name: &str, raw_path: &str) -> std::path::PathBuf {
    if raw_path.trim().is_empty() {
        workspace::default_project_dir(name)
    } else {
        let raw = raw_path.trim();
        if let Some(rest) = raw.strip_prefix("~/") {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(rest)
        } else {
            std::path::PathBuf::from(raw)
        }
    }
}

/// Deja el estado listo para el cuestionario inmediato tras crear.
pub(crate) fn enter_questionnaire(state: &mut App, pid: i64, name: &str, path_str: &str) {    state.active_chat = None;
    state.messages.clear();
    state.md.clear();
    state.pending_project = Some(pid);
    let answers = Answers {
        nombre: name.to_string(),
        ..Default::default()
    };
    state.q_answers = answers;
    state.q_step = 0;
    state.q_error.clear();
    state.q_project = Some(pid);
    // El proyecto nació en este cuestionario: si se cancela sin terminar,
    // se deshace la creación (sin proyectos fantasma).
    state.q_owns_project = true;
    // Wizard v0.8: arranca en Principiante sin preguntas IA.
    state.q_level = crate::questionnaire::Level::Principiante;
    state.q_pending_level = None;
    state.q_ai_questions.clear();
    state.q_ai_answers.clear();
    state.q_ai_loading = false;
    state.q_ai_error.clear();
    state.push_log(format!("📁 proyecto {name} -> {path_str}"));
    state.view = View::Questionnaire;
}

/// Mueve una carpeta a la papelera de ARQHIA en vez de borrarla.
/// Devuelve la ubicación destino. Con fallback copiar+borrar si rename
/// cruza filesystems.
pub(crate) fn trash_dir(dir: &str) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let trash = std::path::PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("arqhia")
        .join("papelera");
    std::fs::create_dir_all(&trash).map_err(|e| format!("No se pudo crear papelera: {e}"))?;
    let src = std::path::PathBuf::from(dir);
    let base = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "proyecto".to_string());
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = trash.join(format!(
        "{}-{}",
        workspace::slugify(&base),
        secs
    ));
    if std::fs::rename(&src, &dest).is_err() {
        copy_dir_recursive(&src, &dest)
            .map_err(|e| format!("No se pudo mover a papelera: {e}"))?;
        std::fs::remove_dir_all(&src).map_err(|e| format!("No se pudo limpiar origen: {e}"))?;
    }
    Ok(dest.to_string_lossy().to_string())
}

pub(crate) fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

/// Borra un proyecto en todos lados: papelera (si tiene carpeta), chats de la
/// DB, fila y estado. Devuelve la línea para el Log. Si la papelera falla,
/// no se toca la DB. Compartido por sidebar y Config.
pub(crate) fn remove_project_everywhere(state: &mut App, id: i64) -> Result<String, String> {
    // 1) Papelera primero: si falla, no se toca la DB
    let ws_path = state
        .projects
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| p.path.clone())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let trashed: Option<String> = match ws_path {
        Some(dir) => match trash_dir(&dir) {
            Ok(dest) => Some(dest),
            Err(e) => return Err(format!("No se borró nada: {e}")),
        },
        None => None,
    };
    // 2) Chats del proyecto + fila
    let chat_ids: Vec<i64> = state
        .chats
        .iter()
        .filter(|c| c.project_id == Some(id))
        .map(|c| c.id)
        .collect();
    for cid in &chat_ids {
        let _ = db::delete_chat(*cid);
    }
    state.chats.retain(|c| c.project_id != Some(id));
    db::delete_project(id).map_err(|e| format!("No se pudo borrar el proyecto: {e}"))?;
    state.projects.retain(|p| p.id != id);
    state.collapsed.remove(&id);
    if state.project_menu == Some(id) {
        state.project_menu = None;
    }
    if let Some(active) = state.active_chat
        && chat_ids.contains(&active) {
            state.active_chat = state
                .chats
                .iter()
                .find(|c| !c.archived)
                .map(|c| c.id);
            clear_turn_state(state);
            state.reload_active_chat();
        }
    Ok(match trashed {
        Some(dest) => format!("🗑 proyecto a papelera ({} chats): {dest}", chat_ids.len()),
        None => format!("🗑 proyecto eliminado ({} chats, sin carpeta)", chat_ids.len()),
    })
}
