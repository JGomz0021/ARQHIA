use std::path::{Path, PathBuf};

/// Valida una ruta de workspace: debe existir y ser directorio.
/// Devuelve la ruta canonicalizada o un mensaje legible para la UI.
pub fn validate(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Pega una ruta, ej. /tmp/demo-ws".to_string());
    }
    // Expande ~ inicial
    let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
        crate::paths::home_dir()
            .join(rest)
            .to_string_lossy()
            .to_string()
    } else {
        trimmed.to_string()
    };
    let path = Path::new(&expanded);
    if !path.exists() {
        return Err(format!("No existe: {expanded}"));
    }
    if !path.is_dir() {
        return Err(format!("No es una carpeta: {expanded}"));
    }
    path.canonicalize()
        .map_err(|e| format!("No se pudo resolver la ruta: {e}"))
}

/// Lista hasta `limit` archivos (relativos al workspace, 1 nivel + aviso).
/// Nunca falla: si no se puede leer, devuelve vec vacío.
pub fn list_top(workspace: &Path, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(workspace) else {
        return out;
    };
    for entry in entries.flatten().take(limit) {
        let name = entry.file_name().to_string_lossy().to_string();
        let suffix = if entry.path().is_dir() { "/" } else { "" };
        out.push(format!("{name}{suffix}"));
    }
    out.sort();
    out
}

/// Texto para el system prompt del agente (top 20 archivos).
pub fn context_block(workspace: &Path) -> String {
    let files = list_top(workspace, 20);
    if files.is_empty() {
        format!("Workspace: {}\nArchivos: (vacío)", workspace.display())
    } else {
        format!(
            "Workspace: {}\nArchivos:\n- {}",
            workspace.display(),
            files.join("\n- ")
        )
    }
}

/// Registra una carpeta existente como workspace (v0.4 Abrir proyecto).
/// Equivale a validar: debe existir y ser directorio.
pub fn register_existing(path: &Path) -> Result<PathBuf, String> {
    validate(&path.to_string_lossy())
}

/// Slug para nombre de carpeta: minúsculas, espacios->-, solo [a-z0-9-_].
pub fn slugify(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let mut out = String::new();
    let mut last_dash = true; // evita guiones líderes
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "proyecto".to_string()
    } else {
        out
    }
}

/// Carpeta por defecto para un proyecto nuevo: ~/ARQHIA/projects/{slug}.
pub fn default_project_dir(name: &str) -> PathBuf {
    crate::paths::projects_base().join(slugify(name))
}

/// ¿La carpeta tiene contenido (no vacía)? Para el import auto v0.8.1.
pub fn is_nonempty_dir(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    std::fs::read_dir(dir)
        .map(|mut it| it.next().is_some())
        .unwrap_or(false)
}

/// ¿Parece un proyecto con código? Manifiestos o más de 2 archivos
/// (ignora `target/.git/node_modules/*.lock`).
pub fn scan_import(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut names: Vec<String> = Vec::new();
    for e in entries.flatten().take(60) {
        let n = e.file_name().to_string_lossy().to_string();
        if crate::config::is_ignored_name(&n) {
            continue;
        }
        names.push(n);
    }
    if names.is_empty() {
        return false;
    }
    const MANIFESTS: [&str; 6] = [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
        "Dockerfile",
    ];
    if names
        .iter()
        .any(|n| MANIFESTS.contains(&n.as_str()) || n == ".git")
    {
        return true;
    }
    names.len() > 2
}

/// Estructura v0.8 del proyecto generado: `Project/`, `ToDo.md`, `CONTEXT/`.
/// Crea lo que falte (idempotente, nunca borra). Devuelve las rutas creadas.
pub fn ensure_project_layout(workspace: &Path) -> Result<Vec<String>, String> {
    let project = workspace.join("Project");
    let context = workspace.join("CONTEXT");
    std::fs::create_dir_all(&project).map_err(|e| format!("No se pudo crear Project/: {e}"))?;
    std::fs::create_dir_all(&context).map_err(|e| format!("No se pudo crear CONTEXT: {e}"))?;
    // `ROADMAP.md`/`VERSIONS.md`/`VERSIONS/v0.1.md` los genera el primer Plan;
    // aquí solo se asegura la carpeta para que el árbol nazca completo.
    std::fs::create_dir_all(context.join("VERSIONS"))
        .map_err(|e| format!("No se pudo crear CONTEXT/VERSIONS: {e}"))?;
    let todo = workspace.join("ToDo.md");
    if !todo.exists() {
        std::fs::write(
            &todo,
            "# ToDo\n\n_Tablero de ejecución (lo mantiene el agente)._\n",
        )
        .map_err(|e| format!("No se pudo crear ToDo.md: {e}"))?;
    }
    Ok(vec![
        project.to_string_lossy().to_string(),
        todo.to_string_lossy().to_string(),
        context.to_string_lossy().to_string(),
    ])
}

/// Escribe los 3 documentos del cuestionario en `{workspace}/CONTEXT/`.
/// Devuelve las rutas escritas.
pub fn save_project_docs(
    workspace: &Path,
    project: &str,
    specs: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    let dir = workspace.join("CONTEXT");
    std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear CONTEXT: {e}"))?;
    let mut out = Vec::new();
    for (name, content) in [
        ("PROJECT.md", project),
        ("SPECS.md", specs),
        ("CONTEXT.md", context),
    ] {
        let file = dir.join(name);
        std::fs::write(&file, content).map_err(|e| format!("No se pudo escribir {name}: {e}"))?;
        out.push(file.to_string_lossy().to_string());
    }
    Ok(out)
}

/// Migra `CONTEXT/ESPEC.md` (v0.5) a `SPECS.md` (v0.8): si hay ESPEC y no hay
/// SPECS, lo copia; el ESPEC queda como legacy con nota. Idempotente.
pub fn migrate_espec(workspace: &Path) -> Option<String> {
    let dir = workspace.join("CONTEXT");
    let espec = dir.join("ESPEC.md");
    let specs = dir.join("SPECS.md");
    let Ok(old) = std::fs::read_to_string(&espec) else {
        return None;
    };
    if old.contains("(legado)") {
        return None; // ya migrado
    }
    if !specs.exists() && std::fs::write(&specs, &old).is_err() {
        return None;
    }
    let legacy = "# ESPEC.md (legado)\n\n\
        Este archivo es el formato anterior (v0.5). \
        La especificación vigente vive en `SPECS.md`.\n";
    if std::fs::write(&espec, legacy).is_err() {
        return None;
    }
    Some("ESPEC.md migrado a SPECS.md (legacy conservado)".to_string())
}

pub fn uploads_dir(workspace: &Path) -> PathBuf {
    workspace.join("uploads")
}

/// Nombre libre dentro de uploads (no sobrescribe: `nombre (2).ext`).
fn free_upload_name(dir: &Path, file_name: &str) -> String {
    if !dir.join(file_name).exists() {
        return file_name.to_string();
    }
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) if i > 0 => (&file_name[..i], &file_name[i..]),
        _ => (file_name, ""),
    };
    for n in 2..1000 {
        let candidate = format!("{stem} ({n}){ext}");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{stem} (x){ext}")
}

/// Copia archivos a `{workspace}/uploads/`. Devuelve (copiados, errores).
/// Rechaza carpetas y archivos sobre el tope. Nunca sobrescribe.
pub fn upload_files(
    workspace: &Path,
    srcs: &[PathBuf],
    max_bytes: u64,
) -> (Vec<String>, Vec<String>) {
    upload_files_with_cap(workspace, srcs, max_bytes)
}

fn upload_files_with_cap(
    workspace: &Path,
    srcs: &[PathBuf],
    cap: u64,
) -> (Vec<String>, Vec<String>) {
    let mut oks = Vec::new();
    let mut errs = Vec::new();
    let ws_canon = match workspace.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return (Vec::new(), vec![format!("Workspace inválido: {e}")]);
        }
    };
    let dir = uploads_dir(&ws_canon);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return (Vec::new(), vec![format!("No se pudo crear uploads/: {e}")]);
    }
    for src in srcs {
        let label = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| src.to_string_lossy().to_string());
        if !src.is_file() {
            errs.push(format!("{label}: solo archivos, no carpetas"));
            continue;
        }
        match std::fs::metadata(src) {
            Ok(m) if m.len() > cap => {
                errs.push(format!(
                    "{label}: supera el tope de {} MB",
                    cap / (1024 * 1024)
                ));
                continue;
            }
            Err(e) => {
                errs.push(format!("{label}: no se pudo leer: {e}"));
                continue;
            }
            _ => {}
        }
        let dest_name = free_upload_name(&dir, &label);
        match std::fs::copy(src, dir.join(&dest_name)) {
            Ok(_) => oks.push(dest_name),
            Err(e) => errs.push(format!("{label}: no se pudo copiar: {e}")),
        }
    }
    (oks, errs)
}

pub struct UploadInfo {
    pub name: String,
    pub size: u64,
}

/// Lista archivos de `{workspace}/uploads/`. Vacío si no existe.
pub fn list_uploads(workspace: &Path) -> Vec<UploadInfo> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(uploads_dir(workspace)) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        out.push(UploadInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Borra un archivo de `{workspace}/uploads/`. El nombre no puede escapar
/// (sin `/`, sin `..`): el path final debe quedar dentro de `uploads/`.
pub fn delete_upload(workspace: &Path, name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("Nombre inválido: {name}"));
    }
    let ws_canon = workspace
        .canonicalize()
        .map_err(|e| format!("Workspace inválido: {e}"))?;
    let target = uploads_dir(&ws_canon).join(name.trim());
    // Chequeo léxico anti-escape
    let mut norm = PathBuf::new();
    for comp in target.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                norm.pop();
            }
            c => norm.push(c.as_os_str()),
        }
    }
    if !norm.starts_with(uploads_dir(&ws_canon)) {
        return Err("Fuera de uploads/".to_string());
    }
    if !norm.is_file() {
        return Err(format!("No existe: {name}"));
    }
    std::fs::remove_file(&norm).map_err(|e| format!("No se pudo borrar: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Variantes async (v0.9.5): el FS bloqueante va a `spawn_blocking` para no
// bloquear el executor ni la UI. Las sync se conservan (rutas rápidas de la
// UI como el banner Import, que solo lee 1 nivel).
// ---------------------------------------------------------------------------

/// `context_block` en background.
pub async fn context_block_async(workspace: &Path) -> String {
    let ws = workspace.to_path_buf();
    tokio::task::spawn_blocking(move || context_block(&ws))
        .await
        .unwrap_or_else(|_| "(contexto no disponible)".to_string())
}

/// `scan_import` en background.
// NOTE(v0.9.5): la ruta de la UI (banner Import) usa la sync a propósito:
// es un único `read_dir` (≤60 entradas, microsegundos). Este wrapper sirve
// a llamantes en background y está cubierto por tests.
#[allow(dead_code)]
pub async fn scan_import_async(dir: &Path) -> bool {
    let d = dir.to_path_buf();
    tokio::task::spawn_blocking(move || scan_import(&d))
        .await
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_import_detects_code_and_empty() {
        let base = std::env::temp_dir().join("arqhia-scan-import");
        let _ = std::fs::remove_dir_all(&base);
        let empty = base.join("empty");
        let code = base.join("code");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(&code).unwrap();
        assert!(!is_nonempty_dir(&empty));
        assert!(!scan_import(&empty));
        std::fs::write(code.join("Cargo.toml"), "[package]").unwrap();
        assert!(is_nonempty_dir(&code));
        assert!(scan_import(&code));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn project_layout_and_espec_migration() {
        let base = std::env::temp_dir().join("arqhia-layout-test");
        let _ = std::fs::remove_dir_all(&base);
        let ws = base.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        // Layout: crea Project/, ToDo.md y CONTEXT/ (idempotente).
        let made = ensure_project_layout(&ws).unwrap();
        assert_eq!(made.len(), 3);
        assert!(ws.join("Project").is_dir());
        assert!(ws.join("ToDo.md").is_file());
        assert!(ws.join("CONTEXT").is_dir());
        assert!(ws.join("CONTEXT").join("VERSIONS").is_dir());
        assert!(ensure_project_layout(&ws).is_ok());
        // Docs: escribe los 3.
        let files = save_project_docs(&ws, "# P", "# S", "# C").unwrap();
        assert_eq!(files.len(), 3);
        assert!(ws.join("CONTEXT").join("SPECS.md").is_file());
        // Migración: con ESPEC legacy y sin SPECS, migra; ya migrado = None.
        let ws2 = base.join("ws2");
        std::fs::create_dir_all(ws2.join("CONTEXT")).unwrap();
        std::fs::write(ws2.join("CONTEXT").join("ESPEC.md"), "# Viejo").unwrap();
        assert!(migrate_espec(&ws2).is_some());
        assert!(ws2.join("CONTEXT").join("SPECS.md").is_file());
        assert!(migrate_espec(&ws2).is_none());
        assert!(
            migrate_espec(&ws).is_none(),
            "sin ESPEC no hay nada que migrar"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_rejects_empty_and_missing() {
        assert!(validate("").is_err());
        assert!(validate("/ruta/que/no/existe/xyz123").is_err());
    }

    #[test]
    fn validate_accepts_tmp_and_lists() {
        let ws = validate("/tmp").expect("tmp debe existir");
        assert!(ws.is_absolute());
        let files = list_top(&ws, 5);
        assert!(files.len() <= 5);
        assert!(context_block(&ws).contains("Workspace:"));
    }

    #[test]
    fn slugify_basics() {
        assert_eq!(slugify("Mi Proyecto Web"), "mi-proyecto-web");
        assert_eq!(slugify("  demo--app__x  "), "demo-app-x");
        assert_eq!(slugify("---"), "proyecto");
        assert!(default_project_dir("Demo").ends_with("ARQHIA/projects/demo"));
    }

    #[test]
    fn uploads_copy_no_overwrite_and_reject_dirs() {
        let base = std::env::temp_dir().join("arqhia-upload-test");
        let _ = std::fs::remove_dir_all(&base);
        let ws = base.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let src = base.join("origen.txt");
        std::fs::write(&src, "datos").unwrap();
        let sub = base.join("subdir");
        std::fs::create_dir_all(&sub).unwrap();

        let (oks, errs) = upload_files_with_cap(&ws, &[src.clone(), sub.clone()], 1024);
        assert_eq!(oks, vec!["origen.txt"]);
        assert_eq!(errs.len(), 1); // la carpeta se rechaza

        // Segunda subida del mismo: no sobrescribe
        let (oks2, _) = upload_files_with_cap(&ws, std::slice::from_ref(&src), 1024);
        assert_eq!(oks2, vec!["origen (2).txt"]);

        // Tope
        let big = base.join("big.bin");
        std::fs::write(&big, vec![0u8; 100]).unwrap();
        let (_, errs3) = upload_files_with_cap(&ws, &[big], 10);
        assert_eq!(errs3.len(), 1);

        // Lista y borra
        let list = list_uploads(&ws);
        assert_eq!(list.len(), 2);
        assert!(delete_upload(&ws, "../fuera").is_err());
        assert!(delete_upload(&ws, "origen.txt").is_ok());
        assert_eq!(list_uploads(&ws).len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }
}
