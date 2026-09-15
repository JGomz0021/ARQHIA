//! Skills locales (v0.9 Track C).
//!
//! Carpeta por skill con `SKILL.md` (frontmatter + instrucciones) en
//! `~/.local/share/arqhia/skills/`. Invocación con `/skill nombre` en el
//! chat (inyecta como contexto, el turno sigue normal). Sin permisos
//! propios: sus scripts pasan por el permiso Bash existente.

use std::path::PathBuf;

/// Tope de instrucciones inyectadas al LLM (más allá se trunca con aviso).
pub const SKILL_BODY_LIMIT: usize = 8 * 1024;

pub fn skills_dir() -> PathBuf {
    crate::paths::data_dir().join("skills")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillOrigin {
    Embedded,
    Local,
}

impl SkillOrigin {
    pub fn badge(self) -> &'static str {
        match self {
            SkillOrigin::Embedded => "embebida",
            SkillOrigin::Local => "local",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub license: String,
}

#[derive(Debug, Clone)]
pub struct SkillDesc {
    pub name: String,
    pub version: String,
    pub description: String,
    pub origin: SkillOrigin,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub meta: SkillMeta,
    pub body: String,
    pub truncated: bool,
    pub origin: SkillOrigin,
    /// Nombres de scripts/recursos para el índice inyectado.
    pub extras: Vec<String>,
}

struct Embedded {
    name: &'static str,
    content: &'static str,
}

const EMBEDDED: [Embedded; 4] = [
    Embedded {
        name: "commit-msg",
        content: include_str!("skills/embebidas/commit-msg.md"),
    },
    // v0.9.2: skills de dominio (reportan a CONTEXT/ para el Orquestador).
    // (`revisar-codigo` se retiró: la cubre `code-review` con reporte.)
    Embedded {
        name: "ui-ux",
        content: include_str!("skills/embebidas/ui-ux.md"),
    },
    Embedded {
        name: "code-review",
        content: include_str!("skills/embebidas/code-review.md"),
    },
    Embedded {
        name: "test-qa",
        content: include_str!("skills/embebidas/test-qa.md"),
    },
];

/// Nombre válido: slug sin rutas (`../` o raro se rechaza).
pub fn valid_skill_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty()
        && n.len() <= 64
        && n.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Instala las embebidas a disco si faltan (primer arranque / tras borrar).
/// Devuelve cuántas instaló. Borrar una embebida + Recargar la restaura.
pub fn ensure_embedded() -> Result<usize, String> {
    let dir = skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut done = 0;
    for e in EMBEDDED {
        let path = dir.join(e.name).join("SKILL.md");
        if !path.is_file() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, e.content).map_err(|e| e.to_string())?;
            done += 1;
        }
    }
    Ok(done)
}

/// Lista embebidas + disco (nombre, versión, descripción, origen).
/// El badge es `embebida` para las conocidas, `local` para el resto.
pub fn list() -> Vec<SkillDesc> {
    let _ = ensure_embedded();
    let mut out: Vec<SkillDesc> = EMBEDDED
        .iter()
        .filter_map(|e| {
            parse_skill_md(e.content).ok().map(|(meta, _)| SkillDesc {
                name: meta.name,
                version: meta.version,
                description: meta.description,
                origin: SkillOrigin::Embedded,
            })
        })
        .collect();
    let dir = skills_dir();
    let entries = std::fs::read_dir(&dir)
        .map(|r| r.collect::<Vec<_>>())
        .unwrap_or_default();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if !valid_skill_name(&name) || out.iter().any(|d| d.name == name) {
            continue;
        }
        let path = entry.path().join("SKILL.md");
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok((meta, _)) = parse_skill_md(&raw) {
            out.push(SkillDesc {
                // El nombre del directorio es la clave canónica de load/delete.
                name,
                version: meta.version,
                description: meta.description,
                origin: SkillOrigin::Local,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Carga una skill (embebida → disco). Falla claro si falta
/// `nombre`/`descripcion` o el nombre es raro.
pub fn load(name: &str) -> Result<Skill, String> {
    let n = name.trim();
    if !valid_skill_name(n) {
        return Err(format!("Nombre de skill no válido: «{n}»."));
    }
    if let Some(e) = EMBEDDED.iter().find(|e| e.name == n) {
        let (meta, body) = parse_skill_md(e.content)?;
        let (body, truncated) = truncate_body(&body);
        return Ok(Skill {
            meta,
            body,
            truncated,
            origin: SkillOrigin::Embedded,
            extras: disk_extras(n),
        });
    }
    let path = skills_dir().join(n).join("SKILL.md");
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| format!("Skill «{n}» no encontrada. Uso: /skill nombre [texto]."))?;
    let (meta, body) = parse_skill_md(&raw)?;
    let (body, truncated) = truncate_body(&body);
    Ok(Skill {
        meta,
        body,
        truncated,
        origin: SkillOrigin::Local,
        extras: disk_extras(n),
    })
}

/// Borra una skill local. Las embebidas no se borran: se restauran
/// (asegura que el archivo vuelva desde el binario).
pub fn delete(name: &str) -> Result<String, String> {
    let n = name.trim();
    if !valid_skill_name(n) {
        return Err(format!("Nombre de skill no válido: «{n}»."));
    }
    if EMBEDDED.iter().any(|e| e.name == n) {
        let path = skills_dir().join(n).join("SKILL.md");
        let _ = std::fs::remove_file(&path);
        let _ = ensure_embedded();
        return Ok(format!("«{n}» es embebida: se restauró desde ARQHIA."));
    }
    let dir = skills_dir().join(n);
    if !dir.is_dir() {
        return Err(format!("Skill «{n}» no encontrada."));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(format!("Skill «{n}» borrada."))
}

/// Divide lo escrito tras `/`: pela el verbo `skill` y separa prefijo/resto.
/// `/skill rev algo` → `("rev", "algo")`; `/rev` → `("rev", "")`.
pub fn slash_tail(input: &str) -> (String, String) {
    let t = input.trim();
    let tail = t.strip_prefix('/').map(str::trim_start).unwrap_or(t);
    // Acepta el verbo a medias (`/skill rev`): se pela para completar.
    let tail = tail
        .strip_prefix("skill")
        .filter(|s| s.is_empty() || s.starts_with([' ', '\t', '\n']))
        .map(|s| s.trim_start())
        .unwrap_or(tail);
    let mut parts = tail.splitn(2, [' ', '\t', '\n']);
    let prefix = parts.next().unwrap_or("").trim().to_string();
    let extra = parts.next().unwrap_or("").trim().to_string();
    (prefix, extra)
}

/// Sugerencia del autocompletado: nombre + descripción + origen + preview
/// del contenido (primeras líneas del cuerpo).
#[derive(Debug, Clone)]
pub struct SkillSuggestion {
    pub name: String,
    pub description: String,
    pub origin: SkillOrigin,
    #[allow(dead_code)]
    pub preview: String,
}

/// Sugerencias para lo que el usuario lleva escrito tras `/` (el input
/// completo del composer). Prefijo vacío = todas. Orden alfabético.
pub fn suggest(input: &str) -> Vec<SkillSuggestion> {
    let (prefix, _) = slash_tail(input);
    let low = prefix.to_lowercase();
    let mut out = Vec::new();
    for d in list() {
        if !low.is_empty() && !d.name.to_lowercase().starts_with(&low) {
            continue;
        }
        let preview = load(&d.name)
            .map(|s| {
                s.body
                    .lines()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(160)
                    .collect::<String>()
            })
            .unwrap_or_default();
        out.push(SkillSuggestion {
            name: d.name,
            description: d.description,
            origin: d.origin,
            preview,
        });
    }
    out
}

/// Resuelve un texto que empieza por `/`: `Ok((nombre, resto))` si hay
/// nombre exacto o prefijo ÚNICO (`/code` → `code-review`);
/// `Err((prefijo, candidatas))` si hay cero o varias.
pub fn resolve_slash(input: &str) -> Result<(String, String), (String, Vec<String>)> {
    let (prefix, extra) = slash_tail(input);
    let all = list();
    if prefix.is_empty() {
        let names = all.iter().map(|d| d.name.clone()).collect();
        return Err((prefix, names));
    }
    if all.iter().any(|d| d.name == prefix) {
        return Ok((prefix, extra));
    }
    let low = prefix.to_lowercase();
    let cands: Vec<String> = all
        .iter()
        .filter(|d| d.name.to_lowercase().starts_with(&low))
        .map(|d| d.name.clone())
        .collect();
    if cands.len() == 1 {
        let name = cands.into_iter().next().unwrap_or_default();
        return Ok((name, extra));
    }
    Err((prefix, cands))
}

/// Bloque inyectado UNA vez como mensaje de contexto del turno.
pub fn context_block(skill: &Skill, extra: &str) -> String {
    let mut s = format!(
        "Skill «{}» ({}; por {}; licencia {}): {}\n\n{}\n",
        skill.meta.name,
        skill.meta.version,
        none_dash(&skill.meta.author),
        none_dash(&skill.meta.license),
        skill.meta.description,
        skill.body
    );
    if !skill.extras.is_empty() {
        s.push_str(&format!(
            "\nRecursos/scripts disponibles: {}.\n",
            skill.extras.join(", ")
        ));
        s.push_str("Si el usuario pide ejecutar un script, pasa por el permiso Bash normal.\n");
    }
    if skill.truncated {
        s.push_str(&format!(
            "\n(Aviso: instrucciones truncadas a {} KB.)\n",
            SKILL_BODY_LIMIT / 1024
        ));
    }
    if !extra.trim().is_empty() {
        s.push_str(&format!(
            "\nPetición del usuario tras /skill: {}\n",
            extra.trim()
        ));
    }
    s
}

fn none_dash(s: &str) -> String {
    if s.trim().is_empty() {
        "—".to_string()
    } else {
        s.trim().to_string()
    }
}

fn truncate_body(body: &str) -> (String, bool) {
    if body.len() <= SKILL_BODY_LIMIT {
        return (body.to_string(), false);
    }
    // Corta por bytes sin romper UTF-8 (el tope son 8 KB reales).
    let mut end = SKILL_BODY_LIMIT;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    (body[..end].to_string(), true)
}

fn disk_extras(name: &str) -> Vec<String> {
    let base = skills_dir().join(name);
    let mut out = Vec::new();
    for sub in ["scripts", "recursos"] {
        let dir = base.join(sub);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            if e.path().is_file()
                && let Some(n) = e.file_name().to_str()
            {
                out.push(format!("{sub}/{n}"));
            }
        }
    }
    out.sort();
    out
}

/// `SKILL.md` = frontmatter `---` + cuerpo. Exige `nombre` + `descripcion`.
fn parse_skill_md(raw: &str) -> Result<(SkillMeta, String), String> {
    // Normaliza BOM + CRLF: con `\r\n` el cálculo de offsets por `len()+1`
    // quedaba desplazado y el cuerpo salía mal (v0.9.5).
    let norm = raw.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let raw = norm.as_str();
    let mut lines = raw.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("SKILL.md sin frontmatter (debe empezar con ---).".to_string());
    }
    let mut front: Vec<(String, String)> = Vec::new();
    let mut body_start: Option<usize> = None;
    let mut pos = 4; // len("---\n")
    for line in lines.by_ref() {
        pos += line.len() + 1;
        if line.trim() == "---" {
            body_start = Some(pos);
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            front.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    let Some(start) = body_start else {
        return Err("SKILL.md sin cierre de frontmatter (segundo ---).".to_string());
    };
    let get = |k: &str| {
        front
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let name = get("nombre");
    let description = get("descripcion");
    if name.trim().is_empty() || description.trim().is_empty() {
        return Err("SKILL.md debe declarar `nombre` y `descripcion`.".to_string());
    }
    let body = raw.get(start..).unwrap_or("").trim().to_string();
    if body.is_empty() {
        return Err("SKILL.md sin instrucciones (cuerpo vacío).".to_string());
    }
    Ok((
        SkillMeta {
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            version: {
                let v = get("version");
                if v.trim().is_empty() {
                    "0.1".to_string()
                } else {
                    v
                }
            },
            author: get("autor"),
            license: get("licencia"),
        },
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_forms_split_complete_and_resolve() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-slash_forms_"); // slash_tail pela `/` y el verbo `skill`.
        assert_eq!(
            slash_tail("/skill commit-msg añade login"),
            ("commit-msg".to_string(), "añade login".to_string())
        );
        assert_eq!(slash_tail("/code"), ("code".to_string(), String::new()));
        assert_eq!(
            slash_tail("  /skill code-review "),
            ("code-review".to_string(), String::new())
        );
        assert_eq!(slash_tail("/"), (String::new(), String::new()));
        assert_eq!(
            slash_tail("/skills x"),
            ("skills".to_string(), "x".to_string())
        );
        // resolve: exacto o prefijo único; si no, candidatas.
        assert_eq!(
            resolve_slash("/skill commit-msg añade login"),
            Ok(("commit-msg".to_string(), "añade login".to_string()))
        );
        assert_eq!(
            resolve_slash("/commit-m haz login"),
            Ok(("commit-msg".to_string(), "haz login".to_string()))
        );
        assert_eq!(
            resolve_slash("/code mira esto"),
            Ok(("code-review".to_string(), "mira esto".to_string()))
        );
        let (prefix, cands) = resolve_slash("/noexiste-xyz123 hola").unwrap_err();
        assert_eq!(prefix, "noexiste-xyz123");
        assert!(cands.is_empty(), "sin coincidencias");
        let (_, all) = resolve_slash("/").unwrap_err();
        assert!(all.contains(&"commit-msg".to_string()) && all.len() >= 2);
    }

    #[test]
    fn frontmatter_requires_name_and_description() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-frontmatter_");
        let ok = "---\nnombre: demo\ndescripcion: una demo\nversion: 1.2\n---\n\nHaz X.\n";
        let (m, b) = parse_skill_md(ok).unwrap();
        assert_eq!(
            (m.name, m.version, b),
            ("demo".to_string(), "1.2".to_string(), "Haz X.".to_string())
        );
        assert!(parse_skill_md("sin frontmatter").is_err());
        assert!(
            parse_skill_md("---\nnombre: x\n---\ncuerpo").is_err(),
            "falta descripcion"
        );
        assert!(
            parse_skill_md("---\nnombre: x\ndescripcion: y\n---\n").is_err(),
            "cuerpo vacío"
        );
        assert!(parse_skill_md("---\nnombre: x\ndescripcion: y\nsin cierre").is_err());
    }

    #[test]
    fn embedded_install_list_and_load() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-embedded_ins");
        assert!(ensure_embedded().is_ok());
        // Resto en disco de la retirada `revisar-codigo`: fuera para el test.
        let _ = delete("revisar-codigo");
        let names: Vec<String> = list().iter().map(|d| d.name.clone()).collect();
        assert!(names.contains(&"commit-msg".to_string()));
        assert!(
            !names.contains(&"revisar-codigo".to_string()),
            "retirada: la cubre code-review"
        );
        // v0.9.2: skills de dominio embebidas (reportan a CONTEXT/).
        for n in ["ui-ux", "code-review", "test-qa"] {
            assert!(names.contains(&n.to_string()), "falta embebida {n}");
            let skill = load(n).unwrap_or_else(|e| panic!("{n}: {e}"));
            assert_eq!(skill.origin, SkillOrigin::Embedded);
            assert!(!skill.truncated);
            assert!(skill.body.len() <= SKILL_BODY_LIMIT);
            assert!(
                skill.body.contains("CONTEXT/"),
                "{n} debe cerrar con reporte en CONTEXT/"
            );
        }
        let skill = load("commit-msg").expect("embebida");
        assert_eq!(skill.origin, SkillOrigin::Embedded);
        assert!(!skill.truncated);
        assert!(skill.body.len() <= SKILL_BODY_LIMIT);
        let ctx = context_block(&skill, "añade login");
        assert!(ctx.contains("commit-msg") && ctx.contains("añade login"));
        assert!(load("../fuera").is_err(), "slug con ruta se rechaza");
        assert!(load("no-existe-xyz123").is_err());
    }

    #[test]
    fn suggest_filters_by_prefix_with_preview() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-suggest_filt");
        let all = suggest("/");
        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"commit-msg") && names.contains(&"code-review"));
        let rev = suggest("/code");
        assert_eq!(rev.len(), 1, "prefijo único: {rev:?}");
        assert_eq!(rev[0].name, "code-review");
        assert!(!rev[0].description.is_empty() && !rev[0].preview.is_empty());
        assert!(
            suggest("/skill commit")
                .iter()
                .any(|s| s.name == "commit-msg")
        );
        assert!(suggest("/zzz-sin-nada").is_empty());
    }

    #[test]
    fn ambiguous_prefix_lists_candidates() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-ambiguou");
        assert!(ensure_embedded().is_ok());
        for name in ["v09-tmp-amb-aa", "v09-tmp-amb-ab"] {
            let dir = skills_dir().join(name);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nnombre: {name}\ndescripcion: temporal\n---\n\nHaz T.\n"),
            )
            .unwrap();
        }
        // Dos con el mismo prefijo: sugiere ambas y no resuelve.
        let sug = suggest("/v09-tmp-amb");
        assert_eq!(sug.len(), 2, "{sug:?}");
        let (prefix, cands) = resolve_slash("/v09-tmp-amb haz").unwrap_err();
        assert_eq!(prefix, "v09-tmp-amb");
        assert_eq!(cands.len(), 2);
        for name in ["v09-tmp-amb-aa", "v09-tmp-amb-ab"] {
            delete(name).unwrap();
        }
    }

    #[test]
    fn local_skill_roundtrip_and_delete() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-local_sk");
        assert!(ensure_embedded().is_ok());
        let dir = skills_dir().join("v09-tmp-skill");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            "---\nnombre: v09-tmp-skill\ndescripcion: temporal\n---\n\nHaz T.\n",
        )
        .unwrap();
        let names: Vec<String> = list().iter().map(|d| d.name.clone()).collect();
        assert!(names.contains(&"v09-tmp-skill".to_string()));
        let skill = load("v09-tmp-skill").unwrap();
        assert_eq!(skill.origin, SkillOrigin::Local);
        let msg = delete("v09-tmp-skill").unwrap();
        assert!(msg.contains("borrada"));
        assert!(!dir.exists());
        assert!(delete("v09-tmp-skill").is_err(), "doble borrado falla");
    }

    #[test]
    fn delete_embedded_restores_it() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-delete_e");
        assert!(ensure_embedded().is_ok());
        let msg = delete("commit-msg").unwrap();
        assert!(msg.contains("restaur"));
        assert!(load("commit-msg").is_ok(), "sigue disponible tras borrar");
        assert!(skills_dir().join("commit-msg").join("SKILL.md").is_file());
    }

    #[test]
    fn long_body_truncates_with_flag() {
        let big = "x".repeat(SKILL_BODY_LIMIT + 100);
        let (cut, flag) = truncate_body(&big);
        assert!(flag && cut.len() <= SKILL_BODY_LIMIT);
        let (same, flag2) = truncate_body("corto");
        assert!(!flag2 && same == "corto");
        assert!(
            !valid_skill_name("../x") && !valid_skill_name("") && valid_skill_name("mi-skill_2")
        );
    }

    #[test]
    fn big_skill_file_loads_truncated() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-big_skil");
        assert!(ensure_embedded().is_ok());
        let dir = skills_dir().join("v09-tmp-big");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let big = "á".repeat(SKILL_BODY_LIMIT + 500);
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nnombre: v09-tmp-big\ndescripcion: grande\n---\n\n{big}\n"),
        )
        .unwrap();
        let skill = load("v09-tmp-big").expect("carga aunque sea grande");
        assert!(skill.truncated, "más de 8 KB se trunca");
        assert!(skill.body.len() <= SKILL_BODY_LIMIT);
        let ctx = context_block(&skill, "");
        assert!(ctx.contains("truncadas"), "aviso en el contexto");
        delete("v09-tmp-big").unwrap();
    }

    /// v0.9.5: `/skill code-review` cita `ChatHistory`/paralelos y symlinks
    /// al inyectarse (criterio de funcionalidad: la skill cubre la calidad
    /// estructural de esta versión).
    #[test]
    fn code_review_skill_mentions_history_and_symlinks() {
        let (_g, _t) = crate::db::test_guard::with_test_db("skills-code-rev");
        assert!(ensure_embedded().is_ok());
        let skill = load("code-review").expect("embebida code-review");
        let ctx = context_block(&skill, "revisa el chat");
        assert!(ctx.contains("ChatHistory"), "cita ChatHistory/paralelos");
        assert!(ctx.contains("symlink"), "cita symlinks");
    }
}
