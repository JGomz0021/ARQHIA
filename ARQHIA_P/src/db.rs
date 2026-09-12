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
    // Migración v0.7.4: messages.created_at (fecha/hora por mensaje).
    // La tabla nueva ya la trae; DBs viejas la reciben aditiva.
    if !column_exists(&conn, "messages", "created_at") {
        conn.execute_batch("ALTER TABLE messages ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));")
            .map_err(|e| e.to_string())?;
    }
    // Migración v0.8: chats.session_id (id estable por chat para el provider).
    if !column_exists(&conn, "chats", "session_id") {
        conn.execute_batch("ALTER TABLE chats ADD COLUMN session_id TEXT DEFAULT NULL;")
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
    /// Id estable de sesión por chat (v0.8): se envía al provider que lo
    /// soporte (OpenRouter `session_id`, OpenAI `prompt_cache_key`).
    /// None en chats legacy hasta que envían su primer turno.
    pub session_id: Option<String>,
}

/// Genera un id de sesión corto y único sin dependencias externas
/// (nanos + pid en hex). Estable por chat: se genera una vez y persiste.
pub fn new_session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{:x}", nanos, std::process::id())
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
    let with_session = column_exists(&conn, "chats", "session_id");
    let sql = match (with_mode, with_session) {
        (true, true) => {
            "SELECT id, title, project_id, archived, mode, session_id FROM chats ORDER BY id ASC"
        }
        (true, false) => {
            "SELECT id, title, project_id, archived, mode FROM chats ORDER BY id ASC"
        }
        (false, true) => {
            "SELECT id, title, project_id, archived, session_id FROM chats ORDER BY id ASC"
        }
        (false, false) => "SELECT id, title, project_id, archived FROM chats ORDER BY id ASC",
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let archived: i64 = row.get(3)?;
            let (mode, session): (Option<String>, Option<String>) = match (with_mode, with_session) {
                (true, true) => (row.get(4)?, row.get(5)?),
                (true, false) => (row.get(4)?, None),
                (false, true) => (None, row.get(4)?),
                (false, false) => (None, None),
            };
            Ok(ChatMeta {
                id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                archived: archived != 0,
                mode: Mode::from_opt_str(mode.as_deref()),
                session_id: session.filter(|s| !s.trim().is_empty()),
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

/// Fija el `session_id` de un chat (v0.8). `None`/vacío lo limpia.
pub fn set_session_id(id: i64, session: Option<&str>) -> Result<(), String> {
    let conn = connect()?;
    let clean = session.map(str::trim).filter(|s| !s.is_empty());
    conn.execute(
        "UPDATE chats SET session_id = ?1 WHERE id = ?2",
        params![clean, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Devuelve el `session_id` persistido, o genera+fija uno nuevo si falta
/// (primer turno del chat). Nunca devuelve vacío.
pub fn ensure_session_id(id: i64) -> Result<String, String> {
    let conn = connect()?;
    let cur: Option<String> = conn
        .query_row(
            "SELECT session_id FROM chats WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if let Some(s) = cur.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        return Ok(s);
    }
    let fresh = new_session_id();
    conn.execute(
        "UPDATE chats SET session_id = ?1 WHERE id = ?2",
        params![fresh, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(fresh)
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

/// Mensaje con id y timestamp (v0.7.4): fecha/hora visible por mensaje.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Historial completo con id y created_at (v0.7.4). Legacy sin columna = "".
pub fn load_chat_history_full(chat_id: i64, limit: usize) -> Result<Vec<ChatMessage>, String> {
    let conn = connect()?;
    let with_ts = column_exists(&conn, "messages", "created_at");
    let sql = if with_ts {
        "SELECT id, role, content, created_at FROM messages WHERE chat_id = ?1 ORDER BY id ASC LIMIT ?2"
    } else {
        "SELECT id, role, content FROM messages WHERE chat_id = ?1 ORDER BY id ASC LIMIT ?2"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![chat_id, limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let role: String = row.get(1)?;
            let content: String = row.get(2)?;
            let created_at: String = if with_ts { row.get(3)? } else { String::new() };
            Ok(ChatMessage { id, role, content, created_at })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Duplica un chat completo: nuevo chat con mismo título + " (copia)",
/// proyecto y modo, y todas las filas de mensajes. Transaccional.
/// (Reservado: el "copiar" visible del chat es por mensaje al portapapeles.)
#[allow(dead_code)]
pub fn copy_chat(src_id: i64) -> Result<i64, String> {
    let mut conn = connect()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (title, project_id, mode, session): (String, Option<i64>, Option<String>, Option<String>) = tx
        .query_row(
            "SELECT title, project_id, mode, session_id FROM chats WHERE id = ?1",
            params![src_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO chats (title, project_id, mode, session_id) VALUES (?1, ?2, ?3, ?4)",
        params![format!("{title} (copia)"), project_id, mode, session],
    )
    .map_err(|e| e.to_string())?;
    let new_id = tx.last_insert_rowid();
    {
        let mut stmt = tx
            .prepare("SELECT role, content, created_at FROM messages WHERE chat_id = ?1 ORDER BY id ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![src_id], |row| {
                let role: String = row.get(0)?;
                let content: String = row.get(1)?;
                let ts: String = row.get(2)?;
                Ok((role, content, ts))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (role, content, ts) = r.map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO messages (chat_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![new_id, role, content, ts],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(new_id)
}

/// Bifurca desde un mensaje (v0.7.4): nuevo chat con el historial hasta el
/// mensaje N inclusive (por id de fila). Transaccional.
pub fn branch_chat(src_id: i64, upto_msg_id: i64) -> Result<i64, String> {
    let mut conn = connect()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (title, project_id, mode): (String, Option<i64>, Option<String>) = tx
        .query_row(
            "SELECT title, project_id, mode FROM chats WHERE id = ?1",
            params![src_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO chats (title, project_id, mode, session_id) VALUES (?1, ?2, ?3, ?4)",
        params![format!("{title} (rama)"), project_id, mode, new_session_id()],
    )
    .map_err(|e| e.to_string())?;
    let new_id = tx.last_insert_rowid();
    {
        let mut stmt = tx
            .prepare(
                "SELECT role, content, created_at FROM messages WHERE chat_id = ?1 AND id <= ?2 ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![src_id, upto_msg_id], |row| {
                let role: String = row.get(0)?;
                let content: String = row.get(1)?;
                let ts: String = row.get(2)?;
                Ok((role, content, ts))
            })
            .map_err(|e| e.to_string())?;
        let mut n = 0;
        for r in rows {
            let (role, content, ts) = r.map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO messages (chat_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![new_id, role, content, ts],
            )
            .map_err(|e| e.to_string())?;
            n += 1;
        }
        if n == 0 {
            return Err("Nada que bifurcar hasta ese mensaje.".to_string());
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(new_id)
}

/// Borra todos los mensajes posteriores a una fila (para "deshacer hasta
/// aquí"): conserva hasta `msg_id` inclusive. Devuelve cuántas borró.
pub fn delete_messages_after(chat_id: i64, msg_id: i64) -> Result<usize, String> {
    let conn = connect()?;
    let n = conn
        .execute(
            "DELETE FROM messages WHERE chat_id = ?1 AND id > ?2",
            params![chat_id, msg_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

/// Cuenta los mensajes persistidos de un chat (para Undo por diferencia).
pub fn count_messages(chat_id: i64) -> Result<usize, String> {
    let conn = connect()?;
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE chat_id = ?1",
            params![chat_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n.max(0) as usize)
}

/// Borra los últimos N mensajes de un chat (para Undo de envío).
pub fn delete_last_messages(chat_id: i64, n: usize) -> Result<(), String> {
    if n == 0 {
        return Ok(());
    }
    let conn = connect()?;
    conn.execute(
        "DELETE FROM messages WHERE id IN (SELECT id FROM messages WHERE chat_id = ?1 ORDER BY id DESC LIMIT ?2)",
        params![chat_id, n as i64],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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

    #[test]
    fn copy_and_branch_chat_duplicate_rows() {
        assert!(init().is_ok());
        let src = create_chat("v0.7.4-tmp").expect("crear chat tmp");
        save_msg(src, "user", "hola 1").unwrap();
        save_msg(src, "assistant", "respuesta 1").unwrap();
        save_msg(src, "user", "hola 2").unwrap();
        // created_at presente (migración v0.7.4).
        let full = load_chat_history_full(src, 10).unwrap();
        assert_eq!(full.len(), 3);
        assert!(full.iter().all(|m| m.id > 0));
        // Copiar duplica todo.
        let cp = copy_chat(src).expect("copiar");
        let cpf = load_chat_history_full(cp, 10).unwrap();
        assert_eq!(cpf.len(), 3);
        assert_eq!(cpf[0].content, "hola 1");
        // Bifurcar hasta el 2º mensaje deja 2.
        let upto = full[1].id;
        let br = branch_chat(src, upto).expect("bifurcar");
        let brf = load_chat_history_full(br, 10).unwrap();
        assert_eq!(brf.len(), 2);
        // Undo helper: borrar últimos 2 deja 1.
        delete_last_messages(src, 2).unwrap();
        assert_eq!(load_chat_history_full(src, 10).unwrap().len(), 1);
        assert_eq!(count_messages(src).unwrap(), 1);
        // Deshacer hasta aquí: borra lo posterior al 1er mensaje.
        save_msg(src, "assistant", "r2").unwrap();
        save_msg(src, "user", "hola 3").unwrap();
        let full2 = load_chat_history_full(src, 10).unwrap();
        assert_eq!(full2.len(), 3);
        let kept = full2[0].id;
        assert_eq!(delete_messages_after(src, kept).unwrap(), 2);
        assert_eq!(load_chat_history_full(src, 10).unwrap().len(), 1);
        // Limpieza para no contaminar la DB del dev.
        delete_chat(src).unwrap();
        delete_chat(cp).unwrap();
        delete_chat(br).unwrap();
    }

    #[test]
    fn session_id_stable_persists_and_regenerates() {
        assert!(init().is_ok());
        let id = create_chat("v0.8-sess-tmp").expect("crear chat tmp");
        // Legacy: sin sesión hasta el primer turno.
        let meta = list_chats().unwrap().into_iter().find(|c| c.id == id).unwrap();
        assert!(meta.session_id.is_none());
        let s1 = ensure_session_id(id).expect("generar sesión");
        assert!(!s1.trim().is_empty());
        // Estable: segunda llamada devuelve el mismo.
        assert_eq!(ensure_session_id(id).unwrap(), s1);
        // Regenerar: cambia.
        set_session_id(id, Some(&new_session_id())).unwrap();
        let s2 = ensure_session_id(id).unwrap();
        assert_ne!(s1, s2);
        // Copia conserva; rama estrena id propio.
        save_msg(id, "user", "hola").unwrap();
        let full = load_chat_history_full(id, 10).unwrap();
        let cp = copy_chat(id).expect("copiar");
        let cpm = list_chats().unwrap().into_iter().find(|c| c.id == cp).unwrap();
        assert_eq!(cpm.session_id.as_deref(), Some(s2.as_str()));
        let br = branch_chat(id, full[0].id).expect("bifurcar");
        let brm = list_chats().unwrap().into_iter().find(|c| c.id == br).unwrap();
        assert!(brm.session_id.is_some() && brm.session_id.as_deref() != Some(s2.as_str()));
        delete_chat(id).unwrap();
        delete_chat(cp).unwrap();
        delete_chat(br).unwrap();
    }
}
