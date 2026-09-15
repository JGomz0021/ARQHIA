//! Rutas de datos/config (v0.9.4): `directories::ProjectDirs` en vez de
//! `HOME` manual (rompe en Windows). Respeta `ARQHIA_HOME` / `ARQHIA_DB` /
//! `ARQHIA_CONFIG` como overrides (tests + portable).

use std::path::PathBuf;

/// Base override para tests/portable (`ARQHIA_HOME`), si está fijada.
fn home_override() -> Option<PathBuf> {
    std::env::var("ARQHIA_HOME")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// En builds de test, sin override, NUNCA se toca el home real: todo va a
/// un temp por proceso. Así ningún test (ni siquiera los que no usan
/// `with_test_db`) puede dejar proyectos fantasma en la app real.
#[cfg(test)]
fn test_base() -> PathBuf {
    std::env::temp_dir().join(format!("arqhia-cargo-test-{}", std::process::id()))
}

/// Directorio de datos (`arqhia.db`, `models.dev.json`, `skills/`, `papelera/`).
pub fn data_dir() -> PathBuf {
    if let Some(home) = home_override() {
        return home.join(".local").join("share").join("arqhia");
    }
    #[cfg(test)]
    return test_base().join(".local").join("share").join("arqhia");
    #[cfg(not(test))]
    {
        if let Some(proj) = directories::ProjectDirs::from("", "", "arqhia") {
            return proj.data_dir().to_path_buf();
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("arqhia")
    }
}

/// Directorio de config (`config.toml`).
pub fn config_dir() -> PathBuf {
    if let Some(home) = home_override() {
        return home.join(".config").join("arqhia");
    }
    #[cfg(test)]
    return test_base().join(".config").join("arqhia");
    #[cfg(not(test))]
    {
        if let Some(proj) = directories::ProjectDirs::from("", "", "arqhia") {
            return proj.config_dir().to_path_buf();
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("arqhia")
    }
}

/// `~/ARQHIA/projects/{slug}` se queda en HOME real (carpetas del usuario,
/// no datos de la app): solo `ARQHIA_HOME` lo redirige en tests.
pub fn projects_base() -> PathBuf {
    if let Some(home) = home_override() {
        return home.join("ARQHIA").join("projects");
    }
    #[cfg(test)]
    return test_base().join("ARQHIA").join("projects");
    #[cfg(not(test))]
    {
        if let Some(home) = dirs_home() {
            return home.join("ARQHIA").join("projects");
        }
        PathBuf::from(".").join("ARQHIA").join("projects")
    }
}

#[allow(dead_code)]
fn dirs_home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
}

/// HOME real para `~/` (v0.9.4): `ARQHIA_HOME` en tests o el home del SO.
pub fn home_dir() -> PathBuf {
    if let Some(home) = home_override() {
        return home;
    }
    #[cfg(test)]
    return test_base();
    #[cfg(not(test))]
    {
        if let Some(home) = dirs_home() {
            return home;
        }
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins_over_project_dirs() {
        let _g = crate::db::test_guard::lock();
        let tmp = std::env::temp_dir().join("arqhia-paths-test");
        unsafe { std::env::set_var("ARQHIA_HOME", &tmp) };
        assert!(data_dir().starts_with(&tmp));
        assert!(config_dir().starts_with(&tmp));
        assert!(projects_base().starts_with(&tmp));
        unsafe { std::env::remove_var("ARQHIA_HOME") };
        // En tests, sin override, todo va al temp del proceso (nunca al home real).
        let base = test_base();
        assert!(data_dir().starts_with(&base));
        assert!(config_dir().starts_with(&base));
        assert!(!data_dir().starts_with("/home"));
    }

    #[test]
    fn test_builds_never_point_at_real_home() {
        // Sin tocar env global: con override vacío el fallback de test no es home.
        let dd = test_base();
        assert!(!dd.starts_with("/home"));
    }
}
