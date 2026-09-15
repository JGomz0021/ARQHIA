//! STACK local de código (v0.9 Track A + legal Track B).
//!
//! SQLite + FTS5: `stack_items` con `author/license/source`, `stack_meta`
//! clave-valor y `stack_fts` sincronizado a mano. Puro dominio (sin UI):
//! `save/search/get/rate/record_execution/report_bug` + `consult` para el
//! orquestador + `can_share_to_cloud` (puerta legal: `Uso interno` no sube).

pub mod seed;

use rusqlite::params;

use crate::db;

/// Licencias admitidas al compartir (POLICIES.md §4).
/// `Uso interno` es solo local: nunca sube a la nube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackLicense {
    Mit,
    Apache2,
    #[default]
    UsoInterno,
}

impl StackLicense {
    pub const ALL: [StackLicense; 3] = [
        StackLicense::Mit,
        StackLicense::Apache2,
        StackLicense::UsoInterno,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            StackLicense::Mit => "MIT",
            StackLicense::Apache2 => "Apache-2.0",
            StackLicense::UsoInterno => "Uso interno",
        }
    }

    /// Acepta "mit", "apache", "apache-2.0", "uso interno"/"interno"/"propio".
    /// Desconocido = `Uso interno` (lo más restrictivo).
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "mit" => StackLicense::Mit,
            "apache" | "apache-2.0" | "apache2" => StackLicense::Apache2,
            _ => StackLicense::UsoInterno,
        }
    }
}

impl std::fmt::Display for StackLicense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Puerta legal: `Uso interno` nunca sube a la nube (Track B).
/// La nube real es v1.0; esto ya deja el rechazo implementado y testeado.
pub fn can_share_to_cloud(license: &str) -> Result<(), String> {
    match StackLicense::parse(license) {
        StackLicense::UsoInterno => {
            Err("Licencia «Uso interno»: este código nunca sale de tu equipo.".to_string())
        }
        _ => Ok(()),
    }
}

/// Snippet a guardar.
#[derive(Debug, Clone)]
pub struct NewItem {
    pub title: String,
    pub code: String,
    pub tags: Vec<String>,
    pub lang: String,
    pub author: String,
    pub license: String,
    pub meta: Vec<(String, String)>,
}

/// Resultado rankeado (preview de 200 chars, sin código completo).
#[derive(Debug, Clone)]
pub struct ScoredItem {
    pub id: i64,
    pub title: String,
    pub tags: String,
    pub lang: String,
    pub rating: f64,
    pub score: f64,
    pub snippet: String,
}

/// Item completo con metadatos.
#[derive(Debug, Clone)]
pub struct FullItem {
    pub id: i64,
    pub title: String,
    pub code: String,
    pub tags: String,
    pub lang: String,
    pub rating: f64,
    pub ratings: i64,
    pub executions: i64,
    pub ok_runs: i64,
    pub author: String,
    pub license: String,
    pub source: String,
    pub meta: Vec<(String, String)>,
}

fn normalize_tags(tags: &[String]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in tags {
        let c = t.trim().to_lowercase();
        if c.is_empty() || !seen.insert(c.clone()) {
            continue;
        }
        out.push(c);
    }
    out.join(",")
}

fn parse_tags(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Guarda un snippet (+ FTS + metadatos) en UNA transacción IMMEDIATE
/// (v0.9.4): insert + FTS nunca quedan a medias. Devuelve el id.
pub fn save(item: &NewItem) -> Result<i64, String> {
    if item.title.trim().is_empty() {
        return Err("El snippet necesita un título.".to_string());
    }
    if item.code.trim().is_empty() {
        return Err("El snippet necesita código.".to_string());
    }
    if item.code.len() > 200_000 {
        return Err("Snippet demasiado grande (tope 200 KB).".to_string());
    }
    let mut conn = db::connect()?;
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let tags = normalize_tags(&item.tags);
    let license = StackLicense::parse(&item.license).to_string();
    tx.execute(
        "INSERT INTO stack_items (title, code, tags, lang, author, license) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            item.title.trim(),
            item.code,
            tags,
            item.lang.trim(),
            item.author.trim(),
            license
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO stack_fts (rowid, title, code, tags) VALUES (?1, ?2, ?3, ?4)",
        params![id, item.title.trim(), item.code, tags],
    )
    .map_err(|e| e.to_string())?;
    for (k, v) in &item.meta {
        if k.trim().is_empty() {
            continue;
        }
        tx.execute(
            "INSERT INTO stack_meta (item_id, key, value) VALUES (?1, ?2, ?3)",
            params![id, k.trim(), v],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

/// Borra un item (+ FTS + meta). Solo para limpieza/tests.
#[allow(dead_code)]
pub fn delete_item(id: i64) -> Result<(), String> {
    let mut conn = db::connect()?;
    // v0.9.5: transacción: FTS, meta e item se borran juntos o no se borra nada.
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM stack_fts WHERE rowid = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM stack_meta WHERE item_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM stack_items WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get(id: i64) -> Result<FullItem, String> {
    let conn = db::connect()?;
    let row: (String, String, String, String, f64, i64, i64, i64, String, String, String) = conn
        .query_row(
            "SELECT title, code, tags, lang, rating, ratings, executions, ok_runs, author, license, source
             FROM stack_items WHERE id = ?1",
            params![id],
            |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?,
                    r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?,
                ))
            },
        )
        .map_err(|_| format!("Snippet #{id} no encontrado."))?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM stack_meta WHERE item_id = ?1 ORDER BY rowid ASC")
        .map_err(|e| e.to_string())?;
    let meta = stmt
        .query_map(params![id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(FullItem {
        id,
        title: row.0,
        code: row.1,
        tags: row.2,
        lang: row.3,
        rating: row.4,
        ratings: row.5,
        executions: row.6,
        ok_runs: row.7,
        author: row.8,
        license: row.9,
        source: row.10,
        meta,
    })
}

fn fts_escape(query: &str) -> String {
    // FTS5 MATCH con frase entre comillas; comillas internas duplicadas.
    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|t| t.len() >= 2)
        .take(8)
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    tokens.join(" OR ")
}

/// Búsqueda con ranking `tag_match*2.0 + fts + rating*0.5`, tope 20.
/// `filter_tags` suma por coincidencia exacta; el texto va contra FTS5.
/// Query vacía + sin tags = todo por rating.
pub fn search(
    query: &str,
    filter_tags: &[String],
    limit: usize,
) -> Result<Vec<ScoredItem>, String> {
    let conn = db::connect()?;
    let wanted: Vec<String> = filter_tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    // Candidatos: FTS si hay texto, o toda la tabla.
    let ids: Vec<(i64, f64)> = if query.trim().is_empty() {
        let mut stmt = conn
            .prepare("SELECT id FROM stack_items")
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<i64>, _>>()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|id| (id, 0.0))
            .collect()
    } else {
        let m = fts_escape(query);
        if m.is_empty() {
            Vec::new()
        } else {
            let mut stmt = conn
                .prepare("SELECT rowid, bm25(stack_fts) FROM stack_fts WHERE stack_fts MATCH ?1")
                .map_err(|e| e.to_string())?;
            stmt.query_map(params![m], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
        }
    };
    // Tokens del texto también puntúan contra tags (tag_match).
    let qtokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.trim().to_lowercase())
        .filter(|t| t.len() >= 2)
        .collect();
    let mut out = Vec::new();
    for (id, bm25) in ids {
        let row: Option<(String, String, String, String, f64)> = conn
            .query_row(
                "SELECT title, code, tags, lang, rating FROM stack_items WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .ok();
        let Some((title, code, tags_csv, lang, rating)) = row else {
            continue;
        };
        let tags = parse_tags(&tags_csv);
        let mut tag_match = 0.0;
        for w in &wanted {
            if tags.iter().any(|t| t == w) {
                tag_match += 1.0;
            }
        }
        for q in &qtokens {
            if tags
                .iter()
                .any(|t| t == q || t.contains(q.as_str()) || q.contains(t.as_str()))
            {
                tag_match += 0.5;
            }
        }
        // Si filtra por tags y no coincide ninguno, fuera.
        if !wanted.is_empty() && tag_match < 1.0 {
            continue;
        }
        // bm25 es negativo (mejor = más negativo): -bm25 suma a la nota.
        let score = tag_match * 2.0 + (-bm25) + rating * 0.5;
        let snippet: String = code.chars().take(200).collect();
        out.push(ScoredItem {
            id,
            title,
            tags: tags_csv,
            lang,
            rating,
            score,
            snippet,
        });
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit.clamp(1, 20));
    Ok(out)
}

/// Valora 1–5 + opinión (`by`: "usuario" | "ia"). Rating = media acumulada.
pub fn rate(id: i64, stars: u8, opinion: &str, by: &str) -> Result<f64, String> {
    if !(1..=5).contains(&stars) {
        return Err("La valoración es de 1 a 5 estrellas.".to_string());
    }
    let conn = db::connect()?;
    let row: Option<(f64, i64)> = conn
        .query_row(
            "SELECT rating, ratings FROM stack_items WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let Some((rating, ratings)) = row else {
        return Err(format!("Snippet #{id} no encontrado."));
    };
    let n = ratings + 1;
    let avg = (rating * ratings as f64 + stars as f64) / n as f64;
    conn.execute(
        "UPDATE stack_items SET rating = ?1, ratings = ?2 WHERE id = ?3",
        params![avg, n, id],
    )
    .map_err(|e| e.to_string())?;
    if !opinion.trim().is_empty() {
        let by = if by.trim().is_empty() {
            "usuario"
        } else {
            by.trim()
        };
        conn.execute(
            "INSERT INTO stack_meta (item_id, key, value) VALUES (?1, ?2, ?3)",
            params![id, format!("opinion_{by}"), opinion.trim()],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(avg)
}

/// Registra una ejecución (`ok` = salió bien).
pub fn record_execution(id: i64, ok: bool) -> Result<(), String> {
    let conn = db::connect()?;
    let n = conn
        .execute(
            "UPDATE stack_items SET executions = executions + 1, ok_runs = ok_runs + ?1 WHERE id = ?2",
            params![if ok { 1 } else { 0 }, id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("Snippet #{id} no encontrado."));
    }
    Ok(())
}

/// Reporta un bug visible en el preview (meta `bug`).
pub fn report_bug(id: i64, desc: &str) -> Result<(), String> {
    if desc.trim().is_empty() {
        return Err("Describe el bug en una línea.".to_string());
    }
    let conn = db::connect()?;
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stack_items WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("Snippet #{id} no encontrado."));
    }
    conn.execute(
        "INSERT INTO stack_meta (item_id, key, value) VALUES (?1, 'bug', ?2)",
        params![id, desc.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Palabras clave del pedido para consultar el STACK (puro, testeable).
pub fn keywords_from_request(text: &str) -> String {
    const STOP: &[&str] = &[
        "para", "como", "esta", "este", "esto", "hace", "hacer", "crea", "crear", "nuevo", "nueva",
        "quiero", "necesito", "añade", "agrega", "cambia", "arregla", "sobre", "entre", "desde",
        "donde", "cuando", "porque", "the", "with", "from", "that", "this", "and", "for", "code",
        "file", "hola", "gracias",
    ];
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for tok in text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        let t = tok.trim().to_lowercase();
        if t.len() < 3 || STOP.contains(&t.as_str()) || !seen.insert(t.clone()) {
            continue;
        }
        out.push(t);
        if out.len() >= 8 {
            break;
        }
    }
    out.join(" ")
}

/// Consulta del orquestador: top 5 formateados para inyectar al planner.
/// None si no hay nada útil (el turno sigue igual, sin Log de hits).
pub fn consult(request: &str) -> Option<String> {
    let kw = keywords_from_request(request);
    if kw.is_empty() {
        return None;
    }
    let hits = search(&kw, &[], 5).ok()?;
    if hits.is_empty() {
        return None;
    }
    let mut s = format!("STACK local: {} coincidencias para «{kw}»:\n", hits.len());
    for h in &hits {
        s.push_str(&format!(
            "- #{} {} [{}] ★{:.1}\n  {}\n",
            h.id,
            h.title,
            if h.tags.is_empty() {
                h.lang.clone()
            } else {
                h.tags.clone()
            },
            h.rating,
            h.snippet
                .replace('\n', " ")
                .chars()
                .take(160)
                .collect::<String>()
        ));
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_guard::with_test_db;

    fn tmp_item(title: &str, code: &str, tags: &[&str]) -> NewItem {
        NewItem {
            title: title.to_string(),
            code: code.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            lang: "rust".to_string(),
            author: "test".to_string(),
            license: "MIT".to_string(),
            meta: vec![("version".to_string(), "1.0".to_string())],
        }
    }

    #[test]
    fn save_search_get_rate_cycle() {
        let (_g, _t) = with_test_db("stack-cycle");
        let id = save(&tmp_item(
            "v09-auth-test",
            "fn login(user: &str) -> bool { !user.is_empty() } // auth login",
            &["auth", "login"],
        ))
        .expect("guardar");
        // Buscar "auth" lo trae arriba con rating visible.
        let hits = search("auth login", &[], 20).expect("buscar");
        assert!(hits.iter().any(|h| h.id == id), "el seed debe aparecer");
        let top = hits.into_iter().find(|h| h.id == id).unwrap();
        assert!(top.snippet.len() <= 200);
        // get trae código completo + meta.
        let full = get(id).expect("get");
        assert!(full.code.contains("fn login"));
        assert!(full.meta.iter().any(|(k, v)| k == "version" && v == "1.0"));
        assert_eq!(full.author, "test");
        assert_eq!(full.license, "MIT");
        // rate 5★ cambia la media y guarda la opinión.
        let avg = rate(id, 5, "va perfecto", "usuario").expect("valorar");
        assert!((avg - 5.0).abs() < 1e-6);
        let avg2 = rate(id, 3, "", "ia").expect("segunda");
        assert!((avg2 - 4.0).abs() < 1e-6);
        let full2 = get(id).expect("get2");
        assert!(full2.meta.iter().any(|(k, _)| k == "opinion_usuario"));
        assert!(rate(id, 0, "", "usuario").is_err(), "0 estrellas inválido");
        assert!(rate(id, 6, "", "usuario").is_err(), "6 estrellas inválido");
        delete_item(id).unwrap();
        assert!(get(id).is_err(), "borrado no debe encontrarse");
    }

    #[test]
    fn executions_and_bugs_are_visible() {
        let (_g, _t) = with_test_db("stack-exec");
        let id = save(&tmp_item(
            "v09-exec-test",
            "fn health() -> &'static str { \"ok\" }",
            &["api", "health"],
        ))
        .expect("guardar");
        record_execution(id, true).unwrap();
        record_execution(id, false).unwrap();
        let full = get(id).unwrap();
        assert_eq!((full.executions, full.ok_runs), (2, 1));
        assert!(report_bug(id, "").is_err(), "bug vacío inválido");
        report_bug(id, "falla con path raro").unwrap();
        let full2 = get(id).unwrap();
        assert!(
            full2
                .meta
                .iter()
                .any(|(k, v)| k == "bug" && v.contains("path raro"))
        );
        delete_item(id).unwrap();
    }

    #[test]
    fn tag_filter_and_ranking() {
        let (_g, _t) = with_test_db("stack-rank");
        let a = save(&tmp_item(
            "v09-rank-a",
            "código v09authq con tokens v09jwtq",
            &["v09authq", "v09jwtq"],
        ))
        .unwrap();
        let b = save(&tmp_item(
            "v09-rank-b",
            "código v09dbq de base de datos",
            &["v09dbq"],
        ))
        .unwrap();
        rate(a, 5, "", "usuario").unwrap();
        // Con filtro de tag, el de auth queda fuera aunque el texto matchee.
        let hits = search("código", &["v09dbq".to_string()], 20).unwrap();
        assert!(hits.iter().any(|h| h.id == b));
        assert!(!hits.iter().any(|h| h.id == a), "filtro de tags excluye");
        // Sin filtro, el mejor valorado con match de tag sube.
        let hits2 = search("v09authq v09jwtq", &[], 20).unwrap();
        assert_eq!(hits2.first().map(|h| h.id), Some(a));
        delete_item(a).unwrap();
        delete_item(b).unwrap();
    }

    #[test]
    fn license_gate_rejects_internal() {
        assert!(can_share_to_cloud("MIT").is_ok());
        assert!(can_share_to_cloud("Apache-2.0").is_ok());
        assert!(can_share_to_cloud("Uso interno").is_err());
        assert!(
            can_share_to_cloud("raro").is_err(),
            "desconocido = restrictivo"
        );
        assert_eq!(StackLicense::parse("apache").as_str(), "Apache-2.0");
        assert_eq!(StackLicense::parse("").as_str(), "Uso interno");
    }

    #[test]
    fn keywords_skip_stopwords_and_consult_formats() {
        let kw = keywords_from_request("quiero crear un endpoint de auth con jwt para la api");
        assert!(kw.contains("endpoint") && kw.contains("auth") && kw.contains("jwt"));
        assert!(!kw.split(' ').any(|w| w == "quiero" || w == "para"));
        assert!(keywords_from_request("y o de").is_empty());
    }

    #[test]
    fn consult_returns_block_only_with_hits() {
        let (_g, _t) = with_test_db("stack-consult");
        assert!(consult("").is_none(), "sin keywords no hay consulta");
        assert!(
            consult("zzz-sin-match-qqq").is_none(),
            "sin hits no hay bloque"
        );
        let id = save(&tmp_item(
            "v09-consult-test",
            "fn v09consultq() -> bool { true } // v09consultq",
            &["v09consultq"],
        ))
        .expect("guardar");
        let block = consult("necesito v09consultq para el test").expect("bloque con hits");
        assert!(block.contains("v09-consult-test") && block.contains("STACK local"));
        delete_item(id).unwrap();
        assert!(consult("v09consultq").is_none(), "tras borrar no hay hits");
    }
}
