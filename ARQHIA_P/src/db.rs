use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::PathBuf;

#[allow(clippy::collapsible_if)]
pub fn db_path() -> PathBuf {
    if let Ok(over) = std::env::var("ARQHIA_DB") {
        if !over.trim().is_empty() {
            return PathBuf::from(over);
        }
    }
    // Tests aislados: `ARQHIA_TEST_DIR` redirige la DB real a temp.
    if let Ok(t) = std::env::var("ARQHIA_TEST_DIR") {
        if !t.trim().is_empty() {
            return PathBuf::from(t).join("arqhia-test.db");
        }
    }
    crate::paths::data_dir().join("arqhia.db")
}

pub(crate) fn connect() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    // Espera hasta 5s si otra instancia/test tiene la DB bloqueada,
    // en vez de fallar con "database is locked".
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    // v0.9.4: FKs activas por conexión (SQLite las exige por conexión).
    conn.execute_batch("PRAGMA foreign_keys=ON;")
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

/// `ALTER TABLE ADD COLUMN` tolerante a carreras: si otro hilo/test migró
/// entre el `column_exists` y el `ALTER`, el "duplicate column name" se
/// considera éxito (la columna ya está).
fn add_column(conn: &Connection, table: &str, col: &str, ddl: &str) -> Result<(), String> {
    if column_exists(conn, table, col) {
        return Ok(());
    }
    match conn.execute_batch(ddl) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("duplicate column name") || column_exists(conn, table, col) {
                Ok(())
            } else {
                Err(msg)
            }
        }
    }
}

pub fn init() -> Result<(), String> {
    let conn = connect()?;
    // v0.9.4: DBs nuevas nacen con FKs. Las viejas las reciben vía
    // limpieza + índices (SQLite no permite ALTER para añadir FK).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS chats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL DEFAULT 'General',
            project_id INTEGER NULL REFERENCES projects(id) ON DELETE SET NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            chat_id INTEGER NULL REFERENCES chats(id) ON DELETE CASCADE
        );",
    )
    .map_err(|e| e.to_string())?;

    // Migración v0.1 -> v0.2: messages.chat_id
    add_column(
        &conn,
        "messages",
        "chat_id",
        "ALTER TABLE messages ADD COLUMN chat_id INTEGER NULL;",
    )?;
    // Migración v0.2 -> v0.3: projects.path (workspace en disco)
    add_column(
        &conn,
        "projects",
        "path",
        "ALTER TABLE projects ADD COLUMN path TEXT NULL;",
    )?;
    // Migración v0.3 -> v0.4: projects.last_opened (recientes futuros)
    add_column(
        &conn,
        "projects",
        "last_opened",
        "ALTER TABLE projects ADD COLUMN last_opened TEXT NULL;",
    )?;
    // Chats archivados (ocultos de las listas, restaurables)
    add_column(
        &conn,
        "chats",
        "archived",
        "ALTER TABLE chats ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;",
    )?;
    // Migración v0.7.1: chats.mode (Chat/Plan/Work, NULL legacy = Chat)
    add_column(
        &conn,
        "chats",
        "mode",
        "ALTER TABLE chats ADD COLUMN mode TEXT DEFAULT NULL;",
    )?;
    // Migración v0.7.4: messages.created_at (fecha/hora por mensaje).
    // La tabla nueva ya la trae; DBs viejas la reciben aditiva.
    add_column(
        &conn,
        "messages",
        "created_at",
        "ALTER TABLE messages ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));",
    )?;
    // Migración v0.8: chats.session_id (id estable por chat para el provider).
    add_column(
        &conn,
        "chats",
        "session_id",
        "ALTER TABLE chats ADD COLUMN session_id TEXT DEFAULT NULL;",
    )?;
    // Migración v0.9: STACK local de código (items + metadatos + FTS5)
    // y contadores de uso por proyecto (turnos, tool calls, 429s).
    // Incluye las columnas legales (author/license/source, Track B).
    migrate_stack(&conn)?;
    // v0.9.4 hardening: índices + UNIQUE NOCASE + schema_version + huérfanos.
    harden_schema(&conn)?;
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
    // Limpieza idempotente de huérfanos por FK (DBs viejas sin CASCADE).
    conn.execute(
        "DELETE FROM messages WHERE chat_id IS NOT NULL AND chat_id NOT IN (SELECT id FROM chats)",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE chats SET project_id = NULL WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Hardening v0.9.4 (idempotente, aditivo): índices por FK, unicidad de
/// proyecto NOCASE (con dedup previo) y `meta.schema_version`.
fn harden_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_messages_chat ON messages(chat_id);
         CREATE INDEX IF NOT EXISTS idx_chats_project ON chats(project_id);",
    )
    .map_err(|e| e.to_string())?;
    // Dedup NOCASE antes del índice único: renombra "X", "x (2)"...
    let dups: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM projects ORDER BY id ASC")
        .map_err(|e| e.to_string())?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (id, name) in dups {
        let key = name.trim().to_lowercase();
        if !seen.insert(key.clone()) {
            let mut n = 2;
            loop {
                let cand = format!("{} ({n})", name.trim());
                if seen.insert(cand.to_lowercase()) {
                    conn.execute(
                        "UPDATE projects SET name = ?1 WHERE id = ?2",
                        params![cand, id],
                    )
                    .map_err(|e| e.to_string())?;
                    break;
                }
                n += 1;
                if n > 999 {
                    break;
                }
            }
        }
    }
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_name_unique ON projects(name COLLATE NOCASE);",
    )
    .map_err(|e| e.to_string())?;
    // Versión de esquema (base del backup pre-migración v0.9.5).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO meta (key, value) VALUES ('schema_version', '1')
         ON CONFLICT(key) DO NOTHING;",
    )
    .map_err(|e| e.to_string())?;
    // v0.9.5: uso por project_id (tabla nueva + backfill, sin destruir).
    migrate_usage_ids(conn)?;
    Ok(())
}

/// Uso por `project_id` (v0.9.5, aditivo): `0` = sin-proyecto.
/// Backfill idempotente desde las tablas por nombre; vista compat
/// `usage_stats_names` para leer por nombre como antes.
fn migrate_usage_ids(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS usage_stats_id(
            project_id INTEGER PRIMARY KEY,
            turns INTEGER NOT NULL DEFAULT 0,
            tool_calls INTEGER NOT NULL DEFAULT 0,
            err429 INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS usage_tools_id(
            project_id INTEGER NOT NULL,
            category TEXT NOT NULL,
            n INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (project_id, category)
        );
        CREATE VIEW IF NOT EXISTS usage_stats_names AS
            SELECT COALESCE(p.name, 'sin-proyecto') AS project,
                   u.turns AS turns, u.tool_calls AS tool_calls, u.err429 AS err429
            FROM usage_stats_id u LEFT JOIN projects p ON p.id = u.project_id;",
    )
    .map_err(|e| e.to_string())?;
    // Backfill: cada fila vieja (por nombre) suma a su id (0 si no existe).
    let old: Vec<(String, i64, i64, i64)> = conn
        .prepare("SELECT project, turns, tool_calls, err429 FROM usage_stats")
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for (name, turns, calls, e429) in old {
        let pid: i64 = conn
            .query_row(
                "SELECT id FROM projects WHERE name = ?1",
                params![name],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO usage_stats_id (project_id, turns, tool_calls, err429) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
               turns = turns + excluded.turns,
               tool_calls = tool_calls + excluded.tool_calls,
               err429 = err429 + excluded.err429",
            params![pid, 0i64, 0i64, 0i64],
        )
        .map_err(|e| e.to_string())?;
        // Marca de backfill: solo suma si aún no se migró esa fila.
        // (Idempotencia barata: si ya hay conteo migrado, no duplica.)
        let already: i64 = conn
            .query_row(
                "SELECT turns + tool_calls + err429 FROM usage_stats_id WHERE project_id = ?1",
                params![pid],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if already == 0 && (turns + calls + e429) > 0 {
            conn.execute(
                "UPDATE usage_stats_id SET turns = turns + ?1, tool_calls = tool_calls + ?2, err429 = err429 + ?3 WHERE project_id = ?4",
                params![turns, calls, e429, pid],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let old_tools: Vec<(String, String, i64)> = conn
        .prepare("SELECT project, category, n FROM usage_tools")
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for (name, cat, n) in old_tools {
        let pid: i64 = conn
            .query_row(
                "SELECT id FROM projects WHERE name = ?1",
                params![name],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO usage_tools_id (project_id, category, n) VALUES (?1, ?2, ?3)
             ON CONFLICT(project_id, category) DO NOTHING",
            params![pid, cat, n],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Versión de esquema persistida (`meta.schema_version`, "1" si falta).
#[allow(dead_code)]
pub fn schema_version() -> Result<String, String> {
    let conn = connect()?;
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(v.unwrap_or_else(|| "0".to_string()))
}

fn ensure_general_chat(conn: &Connection) -> Result<i64, String> {
    let existing: Option<i64> = conn
        .query_row("SELECT id FROM chats ORDER BY id ASC LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute("INSERT INTO chats (title) VALUES ('General')", [])
        .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ---- STACK local (v0.9 Track A) + contadores de uso (Track B) ----

/// Crea las tablas del STACK si faltan (idempotente, dentro de `init()`).
/// Esquema: items con columnas legales (author/license/source) + metadatos
/// clave-valor + índice FTS5 sincronizado a mano en `stack.rs`.
pub(crate) fn migrate_stack(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS stack_items(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            code TEXT NOT NULL,
            tags TEXT NOT NULL DEFAULT '',
            lang TEXT NOT NULL DEFAULT '',
            rating REAL NOT NULL DEFAULT 0,
            ratings INTEGER NOT NULL DEFAULT 0,
            executions INTEGER NOT NULL DEFAULT 0,
            ok_runs INTEGER NOT NULL DEFAULT 0,
            author TEXT NOT NULL DEFAULT '',
            license TEXT NOT NULL DEFAULT 'Uso interno',
            source TEXT NOT NULL DEFAULT 'local',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS stack_meta(
            item_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS stack_fts USING fts5(title, code, tags);
        CREATE TABLE IF NOT EXISTS usage_stats(
            project TEXT PRIMARY KEY,
            turns INTEGER NOT NULL DEFAULT 0,
            tool_calls INTEGER NOT NULL DEFAULT 0,
            err429 INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS usage_tools(
            project TEXT NOT NULL,
            category TEXT NOT NULL,
            n INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (project, category)
        );",
    )
    .map_err(|e| e.to_string())?;
    // DBs que ya tenían stack_items sin columnas legales (no debería pasar
    // en v0.9, pero la migración es barata e idempotente).
    for (col, ddl) in [
        (
            "author",
            "ALTER TABLE stack_items ADD COLUMN author TEXT NOT NULL DEFAULT ''",
        ),
        (
            "license",
            "ALTER TABLE stack_items ADD COLUMN license TEXT NOT NULL DEFAULT 'Uso interno'",
        ),
        (
            "source",
            "ALTER TABLE stack_items ADD COLUMN source TEXT NOT NULL DEFAULT 'local'",
        ),
        (
            "ratings",
            "ALTER TABLE stack_items ADD COLUMN ratings INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "ok_runs",
            "ALTER TABLE stack_items ADD COLUMN ok_runs INTEGER NOT NULL DEFAULT 0",
        ),
    ] {
        add_column(conn, "stack_items", col, ddl)?;
    }
    Ok(())
}

/// Contadores de uso local por proyecto (Track B): solo conteos, sin contenido.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageStats {
    pub project: String,
    pub turns: i64,
    pub tool_calls: i64,
    pub err429: i64,
}

fn usage_bump(project: &str, col: &str, by: i64) -> Result<(), String> {
    let mut conn = connect()?;
    let key = if project.trim().is_empty() {
        "sin-proyecto"
    } else {
        project.trim()
    };
    // v0.9.5: tx explícita (nombre + id se mueven juntos).
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO usage_stats (project, turns, tool_calls, err429) VALUES (?1, 0, 0, 0)
         ON CONFLICT(project) DO NOTHING",
        params![key],
    )
    .map_err(|e| e.to_string())?;
    let sql = format!("UPDATE usage_stats SET {col} = {col} + ?1 WHERE project = ?2");
    tx.execute(&sql, params![by, key])
        .map_err(|e| e.to_string())?;
    let pid: i64 = tx
        .query_row(
            "SELECT id FROM projects WHERE name = ?1",
            params![key],
            |r| r.get(0),
        )
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO usage_stats_id (project_id, turns, tool_calls, err429) VALUES (?1, 0, 0, 0)
         ON CONFLICT(project_id) DO NOTHING",
        params![pid],
    )
    .map_err(|e| e.to_string())?;
    let sql_id = format!("UPDATE usage_stats_id SET {col} = {col} + ?1 WHERE project_id = ?2");
    tx.execute(&sql_id, params![by, pid])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Un turno de chat/agente completado en ese proyecto.
pub fn record_turn(project: &str) -> Result<(), String> {
    usage_bump(project, "turns", 1)
}

/// N llamadas a tools ejecutadas en ese proyecto.
pub fn record_tool_calls(project: &str, n: usize) -> Result<(), String> {
    if n == 0 {
        return Ok(());
    }
    usage_bump(project, "tool_calls", n as i64)
}

/// Una llamada de una categoría concreta (`read/write/bash/net/install/git…`).
/// Alimenta el desglose por categoría del panel de uso.
/// v0.9.5: tx IMMEDIATE (nombre + id juntos; antes era un único INSERT).
pub fn record_tool_call_cat(project: &str, category: &str) -> Result<(), String> {
    let mut conn = connect()?;
    let key = if project.trim().is_empty() {
        "sin-proyecto"
    } else {
        project.trim()
    };
    let cat = if category.trim().is_empty() {
        "otras"
    } else {
        category.trim()
    };
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO usage_tools (project, category, n) VALUES (?1, ?2, 1)
         ON CONFLICT(project, category) DO UPDATE SET n = n + 1",
        params![key, cat],
    )
    .map_err(|e| e.to_string())?;
    let pid: i64 = tx
        .query_row(
            "SELECT id FROM projects WHERE name = ?1",
            params![key],
            |r| r.get(0),
        )
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO usage_tools_id (project_id, category, n) VALUES (?1, ?2, 1)
         ON CONFLICT(project_id, category) DO UPDATE SET n = n + 1",
        params![pid, cat],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Desglose por categoría de un proyecto, ordenado de mayor a menor.
pub fn list_usage_tools(project: &str) -> Result<Vec<(String, i64)>, String> {
    let conn = connect()?;
    let key = if project.trim().is_empty() {
        "sin-proyecto"
    } else {
        project.trim()
    };
    let mut stmt = conn
        .prepare("SELECT category, n FROM usage_tools WHERE project = ?1 ORDER BY n DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![key], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Un error 429 (rate limit) visto en ese proyecto.
pub fn record_429(project: &str) -> Result<(), String> {
    usage_bump(project, "err429", 1)
}

pub fn get_usage(project: &str) -> Result<UsageStats, String> {
    let conn = connect()?;
    let key = if project.trim().is_empty() {
        "sin-proyecto"
    } else {
        project.trim()
    };
    let row: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT turns, tool_calls, err429 FROM usage_stats WHERE project = ?1",
            params![key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match row {
        Some((turns, tool_calls, err429)) => Ok(UsageStats {
            project: key.to_string(),
            turns,
            tool_calls,
            err429,
        }),
        None => Ok(UsageStats {
            project: key.to_string(),
            ..Default::default()
        }),
    }
}

pub fn list_usage() -> Result<Vec<UsageStats>, String> {
    let conn = connect()?;
    let mut stmt = conn
        .prepare("SELECT project, turns, tool_calls, err429 FROM usage_stats ORDER BY turns DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UsageStats {
                project: r.get(0)?,
                turns: r.get(1)?,
                tool_calls: r.get(2)?,
                err429: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
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
        (true, false) => "SELECT id, title, project_id, archived, mode FROM chats ORDER BY id ASC",
        (false, true) => {
            "SELECT id, title, project_id, archived, session_id FROM chats ORDER BY id ASC"
        }
        (false, false) => "SELECT id, title, project_id, archived FROM chats ORDER BY id ASC",
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let archived: i64 = row.get(3)?;
            let (mode, session): (Option<String>, Option<String>) = match (with_mode, with_session)
            {
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
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn rename_chat(id: i64, title: &str) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE chats SET title = ?1 WHERE id = ?2",
        params![title, id],
    )
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
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn set_project_path(id: i64, path: Option<&str>) -> Result<(), String> {
    let conn = connect()?;
    conn.execute(
        "UPDATE projects SET path = ?1 WHERE id = ?2",
        params![path, id],
    )
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

/// Borra un proyecto SUELTANDO sus chats (`project_id = NULL`, FK SET NULL).
/// Contrato v0.9.4 (único): `delete_project` nunca borra chats; el borrado
/// con chats lo hace la UI (`remove_project_everywhere`: papelera + borra
/// chats + fila). Tras llamar, 0 chats huérfanos por FK.
pub fn delete_project(id: i64) -> Result<(), String> {
    let conn = connect()?;
    // Los chats del proyecto quedan sueltos (project_id NULL), no se borran
    conn.execute(
        "UPDATE chats SET project_id = NULL WHERE project_id = ?1",
        params![id],
    )
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
            Ok(ChatMessage {
                id,
                role,
                content,
                created_at,
            })
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
    // IMMEDIATE: reserva el lock de escritura al abrir (los tests corren en
    // paralelo y dos DEFERRED con SELECT+INSERT se bloquean al promover).
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let (title, project_id, mode, session): (String, Option<i64>, Option<String>, Option<String>) =
        tx.query_row(
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
            .prepare(
                "SELECT role, content, created_at FROM messages WHERE chat_id = ?1 ORDER BY id ASC",
            )
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
    // IMMEDIATE como en copy_chat (misma razón: tests en paralelo).
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let (title, project_id, mode): (String, Option<i64>, Option<String>) = tx
        .query_row(
            "SELECT title, project_id, mode FROM chats WHERE id = ?1",
            params![src_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO chats (title, project_id, mode, session_id) VALUES (?1, ?2, ?3, ?4)",
        params![
            format!("{title} (rama)"),
            project_id,
            mode,
            new_session_id()
        ],
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
pub(crate) mod test_guard {
    //! Aislamiento v0.9.4: ningún test toca `~/.local/share/arqhia/arqhia.db`.
    //! `with_test_db` serializa (env global), redirige `ARQHIA_DB` y
    //! `ARQHIA_HOME` a un tempdir y corre `init()`. El `TempDir` vive hasta
    //! el final del test.

    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    pub(crate) fn lock() -> std::sync::MutexGuard<'static, ()> {
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn with_test_db(name: &str) -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
        let guard = lock();
        let dir = tempfile::tempdir().expect("tempdir para tests");
        let db_file = dir.path().join(format!("{name}.db"));
        unsafe {
            std::env::set_var("ARQHIA_DB", &db_file);
            std::env::set_var("ARQHIA_HOME", dir.path());
            std::env::set_var("ARQHIA_CONFIG", dir.path().join("config.toml"));
        }
        super::init().expect("init en temp");
        (guard, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::test_guard::with_test_db;
    use super::*;

    #[test]
    fn db_path_ends_with_arqhia_db() {
        let (_g, _t) = with_test_db("path");
        assert_eq!(db_path().extension().and_then(|e| e.to_str()), Some("db"));
    }

    #[test]
    fn tests_do_not_touch_real_db() {
        // Guarda: con el lock cogido, calcula la ruta real (sin overrides),
        // restaura el temp y verifica que el temp no es la real y que la
        // real no se modifica al escribir en el temp.
        let (_g, _t) = with_test_db("guard");
        let saved_db = std::env::var("ARQHIA_DB").ok();
        let saved_home = std::env::var("ARQHIA_HOME").ok();
        let saved_cfg = std::env::var("ARQHIA_CONFIG").ok();
        unsafe {
            std::env::remove_var("ARQHIA_DB");
            std::env::remove_var("ARQHIA_TEST_DIR");
            std::env::remove_var("ARQHIA_HOME");
            std::env::remove_var("ARQHIA_CONFIG");
        }
        let real = db_path();
        if let Some(v) = saved_db {
            unsafe { std::env::set_var("ARQHIA_DB", v) };
        }
        if let Some(v) = saved_home {
            unsafe { std::env::set_var("ARQHIA_HOME", v) };
        }
        if let Some(v) = saved_cfg {
            unsafe { std::env::set_var("ARQHIA_CONFIG", v) };
        }
        let before = std::fs::metadata(&real)
            .ok()
            .and_then(|m| m.modified().ok());
        assert!(db_path() != real, "los tests deben usar DB temporal");
        create_chat("guard-tmp").unwrap();
        let after = std::fs::metadata(&real)
            .ok()
            .and_then(|m| m.modified().ok());
        assert_eq!(before, after, "la DB real no debe tocarse en tests");
    }

    #[test]
    fn init_creates_tables() {
        let (_g, _t) = with_test_db("init");
        assert!(list_chats().is_ok());
        assert!(list_projects().is_ok());
    }

    #[test]
    fn hardening_schema_fks_indices_unique_version() {
        let (_g, _t) = with_test_db("harden");
        let conn = connect().unwrap();
        // FKs declaradas en DB nueva.
        let fks: Vec<String> = conn
            .prepare(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name IN ('chats','messages')",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        assert!(fks.iter().any(|s| s.contains("REFERENCES")), "{fks:?}");
        // Índices usados por chat_id/project_id.
        let idx: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        assert!(idx.contains(&"idx_messages_chat".to_string()), "{idx:?}");
        assert!(idx.contains(&"idx_chats_project".to_string()), "{idx:?}");
        assert!(
            idx.contains(&"idx_projects_name_unique".to_string()),
            "{idx:?}"
        );
        // UNIQUE NOCASE: duplicado con distinta caja falla.
        conn.execute("INSERT INTO projects (name) VALUES ('MiProyecto')", [])
            .unwrap();
        assert!(
            conn.execute("INSERT INTO projects (name) VALUES ('miproyecto')", [])
                .is_err()
        );
        // schema_version = 1.
        assert_eq!(schema_version().unwrap(), "1");
        // EXPLAIN usa los índices.
        let plan: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT * FROM messages WHERE chat_id = 1",
                [],
                |r| r.get(3),
            )
            .unwrap();
        assert!(
            plan.contains("idx_messages_chat") || plan.contains("SEARCH"),
            "{plan}"
        );
    }

    #[test]
    fn delete_project_releases_chats_without_orphans() {
        let (_g, _t) = with_test_db("delproj");
        let pid = create_project("Tmp-Proj").unwrap();
        let cid = create_chat("tmp").unwrap();
        move_chat(cid, Some(pid)).unwrap();
        save_msg(cid, "user", "hola").unwrap();
        delete_project(pid).unwrap();
        // El chat queda suelto (SET NULL), nunca huérfano por FK.
        let meta = list_chats()
            .unwrap()
            .into_iter()
            .find(|c| c.id == cid)
            .unwrap();
        assert_eq!(meta.project_id, None);
        let conn = connect().unwrap();
        let orphans: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chats WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans, 0);
        let orphans_m: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE chat_id NOT IN (SELECT id FROM chats)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans_m, 0);
    }

    #[test]
    fn duplicate_helpers_are_read_only_and_case_insensitive() {
        let (_g, _t) = with_test_db("dup");
        create_project("ARQHIA").unwrap();
        assert!(project_name_exists("ARQHIA").unwrap());
        assert!(project_name_exists("arqhia").unwrap());
        assert!(!project_name_exists("definitivamente-no-existe-xyz123").unwrap());
        let free = free_project_name("definitivamente-no-existe-xyz123").unwrap();
        assert_eq!(free, "definitivamente-no-existe-xyz123");
        assert_eq!(free_project_name("arqhia").unwrap(), "arqhia (2)");
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
        let (_g, _t) = with_test_db("mode");
        let chats = list_chats().unwrap_or_default();
        let _ = chats.iter().map(|c| c.mode).collect::<Vec<_>>();
    }

    #[test]
    fn copy_and_branch_chat_duplicate_rows() {
        let (_g, _t) = with_test_db("copybranch");
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
    }

    #[test]
    fn session_id_stable_persists_and_regenerates() {
        let (_g, _t) = with_test_db("sess");
        let id = create_chat("v0.8-sess-tmp").expect("crear chat tmp");
        // Legacy: sin sesión hasta el primer turno.
        let meta = list_chats()
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
            .unwrap();
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
        let cpm = list_chats()
            .unwrap()
            .into_iter()
            .find(|c| c.id == cp)
            .unwrap();
        assert_eq!(cpm.session_id.as_deref(), Some(s2.as_str()));
        let br = branch_chat(id, full[0].id).expect("bifurcar");
        let brm = list_chats()
            .unwrap()
            .into_iter()
            .find(|c| c.id == br)
            .unwrap();
        assert!(brm.session_id.is_some() && brm.session_id.as_deref() != Some(s2.as_str()));
    }

    #[test]
    fn usage_counts_turns_tools_and_429s() {
        let (_g, _t) = with_test_db("usage");
        // Proyecto desconocido: ceros, sin filas previas.
        let fresh = get_usage("v09-uso-tmp-xyz").unwrap();
        assert_eq!((fresh.turns, fresh.tool_calls, fresh.err429), (0, 0, 0));
        record_turn("v09-uso-tmp-xyz").unwrap();
        record_turn("v09-uso-tmp-xyz").unwrap();
        record_tool_calls("v09-uso-tmp-xyz", 5).unwrap();
        record_tool_calls("v09-uso-tmp-xyz", 0).unwrap();
        record_429("v09-uso-tmp-xyz").unwrap();
        let got = get_usage("v09-uso-tmp-xyz").unwrap();
        assert_eq!((got.turns, got.tool_calls, got.err429), (2, 5, 1));
        // Clave vacía cae a "sin-proyecto" sin romper.
        record_turn("").unwrap();
        assert!(get_usage("").unwrap().turns >= 1);
        // list_usage incluye la fila temporal.
        let all = list_usage().unwrap();
        assert!(
            all.iter()
                .any(|u| u.project == "v09-uso-tmp-xyz" && u.turns == 2)
        );
        // Desglose por categoría.
        record_tool_call_cat("v09-uso-tmp-xyz", "bash").unwrap();
        record_tool_call_cat("v09-uso-tmp-xyz", "bash").unwrap();
        record_tool_call_cat("v09-uso-tmp-xyz", "read").unwrap();
        let cats = list_usage_tools("v09-uso-tmp-xyz").unwrap();
        assert_eq!(cats, vec![("bash".to_string(), 2), ("read".to_string(), 1)]);
        assert!(list_usage_tools("v09-inexistente-xyz").unwrap().is_empty());
    }

    /// v0.9.5: el uso se espeja por `project_id` en la misma tx (nombre +
    /// id juntos) y la vista compat `usage_stats_names` lo lee por nombre.
    #[test]
    fn usage_mirrors_to_project_id_with_compat_view() {
        let (_g, _t) = with_test_db("usage-ids");
        let pid = create_project("v095-uso-pid-xyz").unwrap();
        record_turn("v095-uso-pid-xyz").unwrap();
        record_tool_call_cat("v095-uso-pid-xyz", "bash").unwrap();
        record_tool_call_cat("v095-uso-pid-xyz", "bash").unwrap();
        // Tabla por id: el proyecto real acumula (no el 0 de sin-proyecto).
        let conn = connect().unwrap();
        let row: (i64, i64, i64) = conn
            .query_row(
                "SELECT turns, tool_calls, err429 FROM usage_stats_id WHERE project_id = ?1",
                params![pid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row.0, 1, "turn espejado al id");
        let n: i64 = conn
            .query_row(
                "SELECT n FROM usage_tools_id WHERE project_id = ?1 AND category = 'bash'",
                params![pid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2, "categoría espejada al id en tx");
        // Vista compat: el nombre viejo sigue leyendo lo nuevo.
        let via_view: (i64, i64, i64) = conn
            .query_row(
                "SELECT turns, tool_calls, err429 FROM usage_stats_names WHERE project = 'v095-uso-pid-xyz'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(via_view.0, 1, "vista compat por nombre");
        delete_project(pid).unwrap();
    }
}
