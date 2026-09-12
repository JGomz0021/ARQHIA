//! Git nativo (v0.7.2): repo por workspace + rama de trabajo + commit/push.
//!
//! Dominio puro (sin UI): la política vive en `config::GitConfig` y se aplica
//! por comando en `agent/tools.rs`. Todas las llamadas son no interactivas
//! (`GIT_TERMINAL_PROMPT=0`).

use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::GitConfig;

/// Estado del workspace activo para la pestaña Git (v0.7.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceStatus {
    pub is_repo: bool,
    pub branch: String,
    pub remotes: Vec<String>,
    pub changes: usize,
}

/// Ejecuta `git -C ws <args>` sin interactividad. Devuelve stdout o el
/// stderr legible en caso de error.
fn run(ws: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(ws)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("git no disponible: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("git {} falló", args.join(" "))
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// true si el workspace ya contiene un repo (`.git` archivo o carpeta).
pub fn is_repo(ws: &Path) -> bool {
    ws.join(".git").exists()
}

/// Rama actual (`None` sin repo o HEAD desacoplado). Usa la ref simbólica
/// para funcionar también en un repo recién iniciado (aún sin commits).
pub fn current_branch(ws: &Path) -> Option<String> {
    if !is_repo(ws) {
        return None;
    }
    run(ws, &["symbolic-ref", "--short", "-q", "HEAD"])
        .ok()
        .filter(|s| !s.is_empty())
}

/// Nombres de remotos configurados.
pub fn remotes(ws: &Path) -> Vec<String> {
    run(ws, &["remote"])
        .map(|s| s.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default()
}

/// `git status --short` volcado (una línea por cambio).
pub fn status_short(ws: &Path) -> String {
    run(ws, &["status", "--short"]).unwrap_or_default()
}

/// true si no hay cambios pendientes (árbol limpio).
pub fn is_clean(ws: &Path) -> bool {
    status_short(ws).trim().is_empty()
}

/// `git init` si el workspace no es repo. Deja HEAD en `base_branch`
/// (aunque aún no existan commits). No reinicia un repo existente.
pub fn init_repo(ws: &Path, base_branch: &str) -> Result<(), String> {
    if is_repo(ws) {
        return Ok(());
    }
    run(ws, &["init", "-q"])?;
    let base = if base_branch.trim().is_empty() {
        "main"
    } else {
        base_branch.trim()
    };
    // Crea/renombra la rama base aun sin commits (HEAD simbólico).
    run(ws, &["symbolic-ref", "HEAD", &format!("refs/heads/{base}")])?;
    Ok(())
}

/// Crea/usa `work_branch` y hace checkout. Nunca sobre `protected`
/// ni sobre la base. Si el workspace no es repo, lo inicia con `auto_init`.
pub fn ensure_work_branch(ws: &Path, cfg: &GitConfig) -> Result<(), String> {
    if !is_repo(ws) {
        if cfg.auto_init {
            init_repo(ws, &cfg.base_branch)?;
        } else {
            return Err("el workspace no es un repo git".to_string());
        }
    }
    let wb = cfg.work_branch.trim();
    if wb.is_empty() || wb == cfg.base_branch || cfg.protected.iter().any(|p| p == wb) {
        return Err("rama de trabajo inválida o protegida".to_string());
    }
    if current_branch(ws).as_deref() == Some(wb) {
        return Ok(());
    }
    let exists = run(ws, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{wb}")]).is_ok();
    if exists {
        run(ws, &["checkout", "-q", wb]).map(|_| ())
    } else {
        run(ws, &["checkout", "-q", "-b", wb]).map(|_| ())
    }
}

/// `git add -A && git commit -m msg` con guarda anti-sucio: si el árbol ya
/// venía sucio ANTES del turno (`clean_before == false`) no auto-commitea
/// (evita barrer ediciones manuales ajenas).
/// Devuelve `Ok(Some(sha))` si commit hizo, `Ok(None)` si no había nada que
/// hacer (sin repo, árbol sucio de antes o sin cambios).
pub fn commit_all(
    ws: &Path,
    msg: &str,
    author: Option<(String, String)>,
    clean_before: bool,
) -> Result<Option<String>, String> {
    if !is_repo(ws) || !clean_before || is_clean(ws) {
        return Ok(None);
    }
    run(ws, &["add", "-A"])?;
    let mut args: Vec<String> = Vec::new();
    let identity = match &author {
        Some((n, e)) => Some((n.clone(), e.clone())),
        None => {
            // Sin autor configurado: si git global no tiene identidad,
            // se usa la de ARQHIA para que el commit no falle.
            if run(ws, &["config", "user.email"]).unwrap_or_default().is_empty() {
                Some(("ARQHIA".to_string(), "arqhia@localhost".to_string()))
            } else {
                None
            }
        }
    };
    if let Some((n, e)) = identity {
        args.push("-c".to_string());
        args.push(format!("user.name={n}"));
        args.push("-c".to_string());
        args.push(format!("user.email={e}"));
    }
    args.push("commit".to_string());
    args.push("-m".to_string());
    args.push(msg.to_string());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(ws, &refs)?;
    let sha = run(ws, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    Ok(Some(if sha.is_empty() { "HEAD".to_string() } else { sha }))
}

/// Push no interactivo con timeout amplio. Error legible si no hay
/// credenciales o remoto.
pub async fn push(ws: &Path, remote: &str, branch: &str) -> Result<String, String> {
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new("git")
            .arg("-C")
            .arg(ws)
            .args(["push", remote, branch])
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| "git push excedió 60s (¿faltan credenciales?)".to_string())?
    .map_err(|e| format!("git no disponible: {e}"))?;
    let tail = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    if out.status.success() {
        let s = tail(&out.stdout);
        Ok(if s.is_empty() { "push OK".to_string() } else { s })
    } else {
        let err = tail(&out.stderr);
        Err(if err.is_empty() {
            format!("git push falló ({})", out.status)
        } else {
            err
        })
    }
}

/// `git diff --stat` (+ staged) para el auditor. Vacío si no es repo.
pub fn diff_stat(ws: &Path) -> String {
    if !is_repo(ws) {
        return String::new();
    }
    let mut out = run(ws, &["diff", "--stat"]).unwrap_or_default();
    let staged = run(ws, &["diff", "--cached", "--stat"]).unwrap_or_default();
    if !staged.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&staged);
    }
    out
}

/// Estado completo del workspace para la UI (rama, remotos, nº cambios).
pub fn workspace_status(ws: &Path) -> WorkspaceStatus {
    let is = is_repo(ws);
    WorkspaceStatus {
        is_repo: is,
        branch: current_branch(ws).unwrap_or_default(),
        remotes: if is { remotes(ws) } else { Vec::new() },
        changes: if is {
            status_short(ws).lines().filter(|l| !l.trim().is_empty()).count()
        } else {
            0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_ws(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("arqhia-git-test-{name}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn init_and_work_branch_never_on_protected() {
        let ws = tmp_ws("branch");
        let cfg = GitConfig {
            author_name: "Test".to_string(),
            author_email: "t@test.dev".to_string(),
            ..GitConfig::default()
        };
        std::fs::write(ws.join("a.txt"), "hola").unwrap();
        assert!(!is_repo(&ws));
        ensure_work_branch(&ws, &cfg).unwrap();
        assert!(is_repo(&ws));
        assert_eq!(current_branch(&ws).as_deref(), Some("ARQHIA"));
        // Commit sobre la rama de trabajo.
        let sha = commit_all(&ws, "ARQHIA: primer", cfg.author(), true).unwrap();
        assert!(sha.is_some(), "debería commitear");
        assert!(is_clean(&ws));
        // Sin cambios -> no-op.
        assert!(commit_all(&ws, "ARQHIA: nada", cfg.author(), true).unwrap().is_none());
        // Árbol sucio antes del turno -> no auto-commitea.
        std::fs::write(ws.join("a.txt"), "cambio manual").unwrap();
        assert!(commit_all(&ws, "ARQHIA: sucio", cfg.author(), false).unwrap().is_none());
        // Una rama de trabajo protegida se rechaza.
        let bad = GitConfig { work_branch: "main".to_string(), ..cfg.clone() };
        assert!(ensure_work_branch(&ws, &bad).is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn init_repo_is_idempotent_and_status_counts_changes() {
        let ws = tmp_ws("status");
        init_repo(&ws, "main").unwrap();
        init_repo(&ws, "otra").unwrap(); // no reinicia ni cambia la base
        assert!(is_repo(&ws));
        assert_eq!(current_branch(&ws).as_deref(), Some("main"));
        std::fs::write(ws.join("x.txt"), "1").unwrap();
        let st = workspace_status(&ws);
        assert!(st.is_repo);
        assert_eq!(st.branch, "main");
        assert_eq!(st.changes, 1);
        assert_eq!(st.remotes, Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[tokio::test]
    async fn push_without_remote_fails_readably() {
        let ws = tmp_ws("push");
        init_repo(&ws, "main").unwrap();
        let err = push(&ws, "origin", "ARQHIA").await.unwrap_err();
        assert!(!err.is_empty(), "error legible esperado");
        let _ = std::fs::remove_dir_all(&ws);
    }
}
