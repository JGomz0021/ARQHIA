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
    let exists = run(
        ws,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{wb}"),
        ],
    )
    .is_ok();
    if exists {
        run(ws, &["checkout", "-q", wb]).map(|_| ())
    } else {
        run(ws, &["checkout", "-q", "-b", wb]).map(|_| ())
    }
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
        Ok(if s.is_empty() {
            "push OK".to_string()
        } else {
            s
        })
    } else {
        let err = tail(&out.stderr);
        Err(if err.is_empty() {
            format!("git push falló ({})", out.status)
        } else {
            err
        })
    }
}

// ---------------------------------------------------------------------------
// Variantes async (v0.9.5): la UI nunca bloquea el hilo principal aunque el
// repo tenga miles de archivos. `push` ya era async; aquí `status`, `diff`,
// `branch` y `commit` ganan su versión con `tokio::process`. Las versiones
// sync se conservan para el agente (ya corre en background) y los tests.
// ---------------------------------------------------------------------------

/// `git -C ws <args>` async, no interactivo.
async fn run_async(ws: &Path, args: &[&str]) -> Result<String, String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let ws = ws.to_path_buf();
    let out = tokio::process::Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(&owned)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("git no disponible: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("git {} falló", owned.join(" "))
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Rama actual async (`None` sin repo o HEAD desacoplado).
pub async fn current_branch_async(ws: &Path) -> Option<String> {
    if !is_repo(ws) {
        return None;
    }
    run_async(ws, &["symbolic-ref", "--short", "-q", "HEAD"])
        .await
        .ok()
        .filter(|s| !s.is_empty())
}

/// `git status --short` async (una línea por cambio).
pub async fn status_short_async(ws: &Path) -> String {
    run_async(ws, &["status", "--short"])
        .await
        .unwrap_or_default()
}

/// `git diff --stat` (+ staged) async para el auditor. Vacío si no es repo.
pub async fn diff_stat_async(ws: &Path) -> String {
    if !is_repo(ws) {
        return String::new();
    }
    let mut out = run_async(ws, &["diff", "--stat"]).await.unwrap_or_default();
    let staged = run_async(ws, &["diff", "--cached", "--stat"])
        .await
        .unwrap_or_default();
    if !staged.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&staged);
    }
    out
}

/// Estado completo async para la UI: no congela ni con 10k archivos.
pub async fn workspace_status_async(ws: &Path) -> WorkspaceStatus {
    let is = is_repo(ws);
    if !is {
        return WorkspaceStatus::default();
    }
    let branch = current_branch_async(ws).await.unwrap_or_default();
    let remotes = run_async(ws, &["remote"])
        .await
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let changes = status_short_async(ws)
        .await
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    WorkspaceStatus {
        is_repo: is,
        branch,
        remotes,
        changes,
    }
}

/// `git add -A && git commit -m msg` async (v0.9.5): misma guarda anti-sucio
/// que `commit_all`, sin bloquear la UI. `push` ya era async.
pub async fn commit_all_async(
    ws: &Path,
    msg: &str,
    author: Option<(String, String)>,
    clean_before: bool,
) -> Result<Option<String>, String> {
    if !is_repo(ws) || !clean_before {
        return Ok(None);
    }
    if status_short_async(ws).await.trim().is_empty() {
        return Ok(None);
    }
    run_async(ws, &["add", "-A"]).await?;
    let mut owned: Vec<String> = Vec::new();
    let identity = match &author {
        Some((n, e)) => Some((n.clone(), e.clone())),
        None => {
            if run_async(ws, &["config", "user.email"])
                .await
                .unwrap_or_default()
                .is_empty()
            {
                Some(("ARQHIA".to_string(), "arqhia@localhost".to_string()))
            } else {
                None
            }
        }
    };
    if let Some((n, e)) = identity {
        owned.push("-c".to_string());
        owned.push(format!("user.name={n}"));
        owned.push("-c".to_string());
        owned.push(format!("user.email={e}"));
    }
    owned.push("commit".to_string());
    owned.push("-m".to_string());
    owned.push(msg.to_string());
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_async(ws, &refs).await?;
    let sha = run_async(ws, &["rev-parse", "--short", "HEAD"])
        .await
        .unwrap_or_default();
    Ok(Some(if sha.is_empty() {
        "HEAD".to_string()
    } else {
        sha
    }))
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

    #[tokio::test]
    async fn init_and_work_branch_never_on_protected() {
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
        // Commit sobre la rama de trabajo (async v0.9.5).
        let sha = commit_all_async(&ws, "ARQHIA: primer", cfg.author(), true)
            .await
            .unwrap();
        assert!(sha.is_some(), "debería commitear");
        assert!(is_clean(&ws));
        // Sin cambios -> no-op.
        assert!(
            commit_all_async(&ws, "ARQHIA: nada", cfg.author(), true)
                .await
                .unwrap()
                .is_none()
        );
        // Árbol sucio antes del turno -> no auto-commitea.
        std::fs::write(ws.join("a.txt"), "cambio manual").unwrap();
        assert!(
            commit_all_async(&ws, "ARQHIA: sucio", cfg.author(), false)
                .await
                .unwrap()
                .is_none()
        );
        // Una rama de trabajo protegida se rechaza.
        let bad = GitConfig {
            work_branch: "main".to_string(),
            ..cfg.clone()
        };
        assert!(ensure_work_branch(&ws, &bad).is_err());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[tokio::test]
    async fn init_repo_is_idempotent_and_status_counts_changes() {
        let ws = tmp_ws("status");
        init_repo(&ws, "main").unwrap();
        init_repo(&ws, "otra").unwrap(); // no reinicia ni cambia la base
        assert!(is_repo(&ws));
        assert_eq!(current_branch(&ws).as_deref(), Some("main"));
        std::fs::write(ws.join("x.txt"), "1").unwrap();
        let st = workspace_status_async(&ws).await;
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

    /// v0.9.5: el estado async coincide con el sync y no bloquea la UI
    /// (repo con 1000 archivos sin commit: el conteo cuadra en segundos).
    #[tokio::test]
    async fn async_status_matches_sync_on_big_repo() {
        let ws = tmp_ws("big-status");
        init_repo(&ws, "main").unwrap();
        for i in 0..1000 {
            std::fs::write(ws.join(format!("f{i:04}.txt")), "x").unwrap();
        }
        let sync_n = status_short(&ws)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert_eq!(sync_n, 1000);
        let t0 = std::time::Instant::now();
        let async_st = workspace_status_async(&ws).await;
        let dt = t0.elapsed();
        assert!(async_st.is_repo && async_st.changes == 1000);
        assert_eq!(async_st.changes, sync_n, "async y sync deben coincidir");
        assert!(dt.as_secs() < 20, "status async no debe colgarse: {dt:?}");
        // diff async también resuelve (untracked no sale en diff, pero no falla).
        let _ = diff_stat_async(&ws).await;
        let _ = status_short_async(&ws).await;
        let _ = std::fs::remove_dir_all(&ws);
    }
}
