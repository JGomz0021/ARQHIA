use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("arqhia")
        .join("arqhia.db")
}

fn connect() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    // Espera hasta 5s si otra instancia/test tiene la DB bloqueada,
    // en vez de fallar con "database is locked".
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn column_exists(conn: &Connection, table: &str, col: &str) -> bool {
    let sql = format!("PRAGMA table_info({table})");
    conn.prepare(&sql)
        .and_then(|mut st| {
            st.query_map([], |row| row.get::<_, String>(1))
                .map(|rows| rows.flatten().any(|name| name == col))
        })
        .unwrap_or(false)
}

pub fn init() -> Result<(), String> {
    let conn = connect()?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS chats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL DEFAULT 'General',
            project_id INTEGER NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| e.to_string())?;

    // Migración v0.1 -> v0.2: messages.chat_id
    if !column_exists(&conn, "messages", "chat_id") {
        conn.execute_batch("ALTER TABLE messages ADD COLUMN chat_id INTEGER NULL;")
            .map_err(|e| e.to_string())?;
    }
    // Migración v0.2 -> v0.3: projects.path (workspace en disco)
    if !column_exists(&conn, "projects", "path") {
        conn.execute_batch("ALTER TABLE projects ADD COLUMN path TEXT NULL;")
            .map_err(|e| e.to_string())?;
    }
    // Migración v0.3 -> v0.4: projects.last_opened (recientes futuros)
    if !column_exists(&conn, "projects", "last_opened") {
        conn.execute_batch("ALTER TABLE projects ADD COLUMN last_opened TEXT NULL;")
            .map_err(|e| e.to_string())?;
    }
    // Chats archivados (ocultos de las listas, restaurables)
    if !column_exists(&conn, "chats", "archived") {
        conn.execute_batch("ALTER TABLE chats ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;")
            .map_err(|e| e.to_string())?;
    }
    // Migración v0.7.1: chats.mode (Chat/Plan/Work, NULL legacy = Chat)
    if !column_exists(&conn, "chats", "mode") {
        conn.execute_batch("ALTER TABLE chats ADD COLUMN mode TEXT DEFAULT NULL;")
            .map_err(|e| e.to_string())?;
    }
    // Mensajes huérfanos (v0.1) van a un chat "General", pero SOLO si existen:
    // ya no se crea ningún chat automáticamente al entrar.
    let orphans: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE chat_id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if orphans > 0 {
        let general_id = ensure_general_chat(&conn)?;
        conn.execute(
            "UPDATE messages SET chat_id = ?1 WHERE chat_id IS NULL",
            params![general_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_general_chat(conn: &Connection) -> Result<i64, String> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM chats ORDER BY id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute("INSERT INTO chats (title) VALUES ('General')", [])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ---- Chats ----

/// Modo de trabajo del chat (v0.7.1). Default: Chat.
/// NULL legacy en DB = Chat, nunca error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Chat,
    Plan,
    Work,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Chat, Mode::Plan, Mode::Work];

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Chat => "chat",
            Mode::Plan => "plan",
            Mode::Work => "work",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Chat => "Chat",
            Mode::Plan => "Plan",
            Mode::Work => "Work",
        }
    }

    /// NULL / desconocido / vacío = Chat (chats legacy).
    pub fn from_opt_str(s: Option<&str>) -> Self {
        match s.map(str::trim).map(str::to_lowercase).as_deref() {
            Some("plan") => Mode::Plan,
            Some("work") => Mode::Work,
            _ => Mode::Chat,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone)]
pub struct ChatMeta {
    pub id: i64,
    pub title: String,
    pub project_id: Option<i64>,
    pub archived: bool,
    pub mode: Mode,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: Option<String>,
}

pub fn create_chat(title: &str) -> Result<i64, String> {
    let conn = connect()?;
    conn.execute("INSERT INTO chats (title) VALUES (?1)", params![title])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_chats() -> Result<Vec<ChatMeta>, String> {
    let conn = connect()?;
    let with_mode = column_exists(&conn, "chats", "mode");
    let sql = if with_mode {
        "SELECT id, title, project_id, archived, mode FROM chats ORDER BY id ASC"
    } else {
        "SELECT id, title, project_id, archived FROM chats ORDER BY id ASC"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let archived: i64 = row.get(3)?;
            let mode: Option<String> = if with_mode { row.get(4)? } else { None };
            Ok(ChatMeta {
                id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                archived: archived != 0,
                mode: Mode::from_opt_str(mode.as_deref()),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn rename_chat(id: i64, title: &str) -> Result<(), String> {
    let conn = connect()?;
    conn.execute("UPDATE chats SET title = ?1 WHERE id = ?2", params![title, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn move_chat(id: i64, project_id: Option<i64>) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE chats SET project_id = ?1 WHERE id = ?2",
        params![project_id, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_chat(id: i64) -> Result<(), String> {
    let conn = connect()?;
    conn.execute("DELETE FROM messages WHERE chat_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM chats WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Archiva (true) o restaura (false) un chat. Archivado = oculto, no borrado.
pub fn set_archived(id: i64, archived: bool) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE chats SET archived = ?1 WHERE id = ?2",
        params![if archived { 1 } else { 0 }, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Cambia el modo de un chat (v0.7.1). Persiste tras reinicio.
pub fn set_chat_mode(id: i64, mode: Mode) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE chats SET mode = ?1 WHERE id = ?2",
        params![mode.as_str(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- Projects (solo agrupadores lógicos en v0.2, sin carpetas) ----

pub fn create_project(name: &str) -> Result<i64, String> {
    let conn = connect()?;
    conn.execute("INSERT INTO projects (name) VALUES (?1)", params![name])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// true si ya existe un proyecto con ese nombre (insensible a mayúsculas).
pub fn project_name_exists(name: &str) -> Result<bool, String> {
    let conn = connect()?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE name = ?1 COLLATE NOCASE",
            params![name.trim()],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

/// Nombre libre a partir de `base`: "X", "X (2)", "X (3)"...
pub fn free_project_name(base: &str) -> Result<String, String> {
    let base = base.trim();
    if base.is_empty() {
        return Ok("Proyecto".to_string());
    }
    if !project_name_exists(base)? {
        return Ok(base.to_string());
    }
    for n in 2..1000 {
        let candidate = format!("{base} ({n})");
        if !project_name_exists(&candidate)? {
            return Ok(candidate);
        }
    }
    Err("Demasiados proyectos con ese nombre".to_string())
}

pub fn list_projects() -> Result<Vec<Project>, String> {
    let conn = connect()?;
    // path puede no existir en DBs viejas si init() aún no migró: fallback sin columna
    let with_path = column_exists(&conn, "projects", "path");
    let sql = if with_path {
        "SELECT id, name, path FROM projects ORDER BY id ASC"
    } else {
        "SELECT id, name FROM projects ORDER BY id ASC"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: if with_path { row.get(2)? } else { None },
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn set_project_path(id: i64, path: Option<&str>) -> Result<(), String> {
    let conn = connect()?;
    conn.execute("UPDATE projects SET path = ?1 WHERE id = ?2", params![path, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn touch_project(id: i64) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE projects SET last_opened = datetime('now') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_project(id: i64) -> Result<(), String> {
    let conn = connect()?;
    // Los chats del proyecto quedan sueltos (project_id NULL), no se borran
    conn.execute("UPDATE chats SET project_id = NULL WHERE project_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- Messages (por chat) ----

pub fn save_msg(chat_id: i64, role: &str, content: &str) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "INSERT INTO messages (chat_id, role, content) VALUES (?1, ?2, ?3)",
        params![chat_id, role, content],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Compat: guarda en el primer chat (usado solo por tests viejos).
#[allow(dead_code)]
pub fn save_msg_legacy(role: &str, content: &str) -> Result<(), String> {
    let conn = connect()?;
    let id = ensure_general_chat(&conn)?;
    conn.execute(
        "INSERT INTO messages (chat_id, role, content) VALUES (?1, ?2, ?3)",
        params![id, role, content],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_chat_history(chat_id: i64, limit: usize) -> Result<Vec<(String, String)>, String> {
    let conn = connect()?;
    let mut stmt = conn
        .prepare("SELECT role, content FROM messages WHERE chat_id = ?1 ORDER BY id ASC LIMIT ?2")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![chat_id, limit as i64], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((role, content))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Compat v0.1: historial global (para migración ya hecha, devuelve todo).
#[allow(dead_code)]
pub fn load_history(limit: usize) -> Result<Vec<(String, String)>, String> {
    let conn = connect()?;
    let mut stmt = conn
        .prepare("SELECT role, content FROM messages ORDER BY id ASC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit as i64], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((role, content))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_path_ends_with_arqhia_db() {
        assert!(db_path().ends_with("arqhia.db"));
    }

    #[test]
    fn init_creates_tables() {
        assert!(init().is_ok());
        // Solo lectura: no insertamos para no contaminar la DB del usuario
        assert!(list_chats().is_ok());
        assert!(list_projects().is_ok());
    }

    #[test]
    fn duplicate_helpers_are_read_only_and_case_insensitive() {
        assert!(init().is_ok());
        // "ARQHIA" existe en la DB del dev; en minúsculas también debe matchear
        let _ = project_name_exists("ARQHIA");
        let _ = project_name_exists("definitivamente-no-existe-xyz123");
        let free = free_project_name("definitivamente-no-existe-xyz123").unwrap();
        assert_eq!(free, "definitivamente-no-existe-xyz123");
    }

    #[test]
    fn mode_defaults_to_chat_and_roundtrips() {
        // NULL legacy, vacío y desconocido = Chat, nunca error.
        assert_eq!(Mode::from_opt_str(None), Mode::Chat);
        assert_eq!(Mode::from_opt_str(Some("")), Mode::Chat);
        assert_eq!(Mode::from_opt_str(Some("PLAN")), Mode::Plan);
        assert_eq!(Mode::from_opt_str(Some("work")), Mode::Work);
        assert_eq!(Mode::from_opt_str(Some("build")), Mode::Chat);
        assert_eq!(Mode::default(), Mode::Chat);
        assert_eq!(Mode::ALL.len(), 3);
        // La migración existe y listar no rompe en DBs legacy.
        assert!(init().is_ok());
        let chats = list_chats().unwrap_or_default();
        let _ = chats.iter().map(|c| c.mode).collect::<Vec<_>>();
    }
}
