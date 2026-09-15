//! Seeds iniciales del STACK local (v0.9 Track A).
//!
//! 10 snippets curados (autor ARQHIA, MIT) que se instalan una sola vez si
//! la tabla está vacía. Sirven de base para buscar/probar desde el día 1.

use super::{NewItem, save};

pub const SEED_COUNT: usize = 14;

fn seeds() -> Vec<NewItem> {
    let meta = || vec![("version".to_string(), "1.0".to_string())];
    vec![
        NewItem {
            title: "rust-hello".to_string(),
            code: "fn main() {\n    println!(\"hola, mundo\");\n}\n".to_string(),
            tags: vec!["rust".to_string(), "hello".to_string(), "cli".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "axum-health".to_string(),
            code: "use axum::{routing::get, Router};\n\nasync fn health() -> &'static str {\n    \"ok\"\n}\n\npub fn router() -> Router {\n    Router::new().route(\"/health\", get(health))\n}\n".to_string(),
            tags: vec!["rust".to_string(), "axum".to_string(), "api".to_string(), "health".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "sqlite-open".to_string(),
            code: "use rusqlite::Connection;\n\npub fn open(path: &str) -> rusqlite::Result<Connection> {\n    let conn = Connection::open(path)?;\n    conn.busy_timeout(std::time::Duration::from_secs(5))?;\n    Ok(conn)\n}\n".to_string(),
            tags: vec!["rust".to_string(), "sqlite".to_string(), "db".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "iced-counter".to_string(),
            code: "use iced::{Element, Task};\n\n#[derive(Default)]\nstruct Counter {\n    value: i64,\n}\n\n#[derive(Debug, Clone)]\nenum Message {\n    Increment,\n}\n\nfn update(state: &mut Counter, msg: Message) -> Task<Message> {\n    match msg {\n        Message::Increment => {\n            state.value += 1;\n            Task::none()\n        }\n    }\n}\n\nfn view(state: &Counter) -> Element<'_, Message> {\n    iced::widget::button(format!(\"van {}\", state.value))\n        .on_press(Message::Increment)\n        .into()\n}\n".to_string(),
            tags: vec!["rust".to_string(), "iced".to_string(), "ui".to_string(), "counter".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "tokio-spawn".to_string(),
            code: "#[tokio::main]\nasync fn main() {\n    let handle = tokio::spawn(async {\n        tokio::time::sleep(std::time::Duration::from_millis(100)).await;\n        42\n    });\n    println!(\"resultado: {}\", handle.await.unwrap());\n}\n".to_string(),
            tags: vec!["rust".to_string(), "tokio".to_string(), "async".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "serde-json".to_string(),
            code: "use serde::{Deserialize, Serialize};\n\n#[derive(Debug, Serialize, Deserialize)]\nstruct User {\n    name: String,\n    #[serde(default)]\n    admin: bool,\n}\n\nfn parse(raw: &str) -> serde_json::Result<User> {\n    serde_json::from_str(raw)\n}\n".to_string(),
            tags: vec!["rust".to_string(), "serde".to_string(), "json".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "reqwest-get".to_string(),
            code: "async fn fetch_text(url: &str) -> Result<String, reqwest::Error> {\n    reqwest::Client::builder()\n        .timeout(std::time::Duration::from_secs(20))\n        .build()?\n        .get(url)\n        .send()\n        .await?\n        .text()\n        .await\n}\n".to_string(),
            tags: vec!["rust".to_string(), "reqwest".to_string(), "http".to_string(), "net".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "rfd-dialog".to_string(),
            code: "fn pick_folder() -> Option<std::path::PathBuf> {\n    rfd::FileDialog::new()\n        .set_title(\"Elige una carpeta\")\n        .pick_folder()\n}\n".to_string(),
            tags: vec!["rust".to_string(), "rfd".to_string(), "dialog".to_string(), "ui".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "minijinja-render".to_string(),
            code: "fn render_hello(name: &str) -> Result<String, minijinja::Error> {\n    let env = minijinja::Environment::new();\n    env.render_str(\"Hola, {{ name }}!\", minijinja::context! { name => name })\n}\n".to_string(),
            tags: vec!["rust".to_string(), "minijinja".to_string(), "templates".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "rusqlite-migrate".to_string(),
            code: "fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {\n    conn.execute_batch(\n        \"CREATE TABLE IF NOT EXISTS items(\n            id INTEGER PRIMARY KEY AUTOINCREMENT,\n            title TEXT NOT NULL\n        );\",\n    )\n}\n".to_string(),
            tags: vec!["rust".to_string(), "sqlite".to_string(), "migrate".to_string(), "db".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "iced-task-perform".to_string(),
            code: "use iced::Task;\n\n/// Tarea async que devuelve un Message (no bloquea la UI).\nfn fetch_task(url: String) -> Task<Message> {\n    Task::perform(\n        async move {\n            reqwest::get(&url).await?.text().await.map_err(|e| e.to_string())\n        },\n        Message::Fetched,\n    )\n}\n".to_string(),
            tags: vec!["rust".to_string(), "iced".to_string(), "async".to_string(), "task".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "rusqlite-column-migration".to_string(),
            code: "fn column_exists(conn: &rusqlite::Connection, table: &str, col: &str) -> bool {\n    let sql = format!(\"PRAGMA table_info({table})\");\n    conn.prepare(&sql)\n        .and_then(|mut st| {\n            st.query_map([], |row| row.get::<_, String>(1))\n                .map(|rows| rows.flatten().any(|name| name == col))\n        })\n        .unwrap_or(false)\n}\n\n/// Migración idempotente: solo añade la columna si falta.\nfn migrate_add_column(conn: &rusqlite::Connection) -> rusqlite::Result<()> {\n    if !column_exists(conn, \"items\", \"archived\") {\n        conn.execute_batch(\"ALTER TABLE items ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;\")?;\n    }\n    Ok(())\n}\n".to_string(),
            tags: vec!["rust".to_string(), "sqlite".to_string(), "migrate".to_string(), "db".to_string(), "schema".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "toml-config-roundtrip".to_string(),
            code: "use serde::{Deserialize, Serialize};\n\n/// Config con migración: claves nuevas llevan `#[serde(default)]`\n/// para no romper archivos viejos.\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\n#[serde(default)]\nstruct Settings {\n    theme: String,\n    verbose: bool,\n}\n\nfn load(path: &std::path::Path) -> Settings {\n    std::fs::read_to_string(path)\n        .ok()\n        .and_then(|s| toml::from_str(&s).ok())\n        .unwrap_or_default()\n}\n\nfn save(path: &std::path::Path, cfg: &Settings) -> std::io::Result<()> {\n    if let Some(parent) = path.parent() {\n        std::fs::create_dir_all(parent)?;\n    }\n    let s = toml::to_string_pretty(cfg).expect(\"serializable\");\n    std::fs::write(path, s)\n}\n".to_string(),
            tags: vec!["rust".to_string(), "toml".to_string(), "serde".to_string(), "config".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
        NewItem {
            title: "fts5-match-rank".to_string(),
            code: "/// Búsqueda FTS5 con ranking bm25 (requiere rusqlite `bundled`).\n/// `bm25` es negativo: mejor = más negativo, así que se suma `-bm25`.\nfn search_top(conn: &rusqlite::Connection, query: &str, limit: usize) -> rusqlite::Result<Vec<(i64, f64)>> {\n    let mut stmt = conn.prepare(\n        \"SELECT rowid, bm25(docs_fts) FROM docs_fts WHERE docs_fts MATCH ?1 ORDER BY bm25(docs_fts) LIMIT ?2\",\n    )?;\n    let rows = stmt.query_map(rusqlite::params![query, limit as i64], |r| {\n        Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))\n    })?;\n    rows.collect()\n}\n".to_string(),
            tags: vec!["rust".to_string(), "sqlite".to_string(), "fts5".to_string(), "search".to_string(), "db".to_string()],
            lang: "rust".to_string(),
            author: "ARQHIA".to_string(),
            license: "MIT".to_string(),
            meta: meta(),
        },
    ]
}

/// Inserta los seeds que falten (por título). Devuelve cuántos insertó.
/// Idempotente: con todo instalado no toca nada; con una DB vieja de 10
/// solo añade los nuevos sin duplicar.
pub fn seed_missing() -> Result<usize, String> {
    let conn = crate::db::connect()?;
    let mut stmt = conn
        .prepare("SELECT title FROM stack_items")
        .map_err(|e| e.to_string())?;
    let have: std::collections::HashSet<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    drop(conn);
    let mut done = 0;
    for item in seeds() {
        if !have.contains(&item.title) {
            save(&item)?;
            done += 1;
        }
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::super::search;
    use super::*;

    #[test]
    fn seeds_install_once_and_are_searchable() {
        let (_g, _t) = crate::db::test_guard::with_test_db("seed");
        assert_eq!(seeds().len(), SEED_COUNT, "SEED_COUNT sigue al vector");
        // Instala los que falten (por título) y la segunda vez no toca nada.
        let done = seed_missing().expect("seed");
        assert!(done <= SEED_COUNT, "nunca más que los definidos");
        let second = seed_missing().expect("re-seed");
        assert_eq!(second, 0, "el segundo seed no inserta");
        // Los 14 títulos existen (vengan de este seed o de uno anterior).
        for title in [
            "rust-hello",
            "axum-health",
            "sqlite-open",
            "iced-counter",
            "tokio-spawn",
            "serde-json",
            "reqwest-get",
            "rfd-dialog",
            "minijinja-render",
            "rusqlite-migrate",
            "iced-task-perform",
            "rusqlite-column-migration",
            "toml-config-roundtrip",
            "fts5-match-rank",
        ] {
            let hits = search(title, &[], 20).expect("buscar seed");
            assert!(hits.iter().any(|h| h.title == title), "falta seed {title}");
        }
        // "auth" no existe como seed: buscarlo no rompe aunque haya datos.
        let _ = search("auth", &[], 20).expect("buscar sin hits no falla");
    }
}
