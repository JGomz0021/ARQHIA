use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Publico {
    #[default]
    DevIndie,
    PequenoEquipo,
    Empresa,
    Estudiantes,
    Otro,
}

impl fmt::Display for Publico {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Publico::DevIndie => write!(f, "Desarrollador indie"),
            Publico::PequenoEquipo => write!(f, "Pequeño equipo"),
            Publico::Empresa => write!(f, "Empresa"),
            Publico::Estudiantes => write!(f, "Estudiantes"),
            Publico::Otro => write!(f, "Otro"),
        }
    }
}

impl Publico {
    pub const ALL: [Publico; 5] = [
        Publico::DevIndie,
        Publico::PequenoEquipo,
        Publico::Empresa,
        Publico::Estudiantes,
        Publico::Otro,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Interfaz {
    #[default]
    Nativa,
    Web,
    Cli,
    Movil,
    Backend,
}

impl fmt::Display for Interfaz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Interfaz::Nativa => write!(f, "Nativa"),
            Interfaz::Web => write!(f, "Web"),
            Interfaz::Cli => write!(f, "CLI"),
            Interfaz::Movil => write!(f, "Móvil"),
            Interfaz::Backend => write!(f, "Backend"),
        }
    }
}

impl Interfaz {
    pub const ALL: [Interfaz; 5] = [
        Interfaz::Nativa,
        Interfaz::Web,
        Interfaz::Cli,
        Interfaz::Movil,
        Interfaz::Backend,
    ];
}

#[derive(Debug, Clone, Default)]
pub struct Answers {
    pub nombre: String,
    pub descripcion: String,
    pub ubicacion: String,
    pub objetivo: String,
    pub publico: Publico,
    pub interfaz: Interfaz,
}

pub const TOTAL_STEPS: usize = 6;

/// Títulos cortos por paso para la barra de progreso.
pub fn step_title(step: usize) -> &'static str {
    match step {
        0 => "Nombre del proyecto",
        1 => "Descripción",
        2 => "Ubicación en tu equipo",
        3 => "Objetivo",
        4 => "Público objetivo",
        _ => "Interfaz",
    }
}

/// `Some(error)` bloquea el avance; `None` permite seguir.
pub fn validate(step: usize, a: &Answers) -> Option<String> {
    match step {
        0 => {
            if a.nombre.trim().is_empty() {
                Some("Pon un nombre al proyecto.".to_string())
            } else if a.nombre.chars().count() > 60 {
                Some("Máximo 60 caracteres.".to_string())
            } else {
                None
            }
        }
        1 => {
            if a.descripcion.trim().chars().count() < 10 {
                Some("Describe el proyecto en al menos 10 caracteres.".to_string())
            } else {
                None
            }
        }
        2 => {
            let u = a.ubicacion.trim();
            if u.is_empty() {
                None // vacío = workspace actual
            } else if !Path::new(u).is_absolute() {
                Some("Usa una ruta absoluta o deja vacío para usar el workspace.".to_string())
            } else {
                None
            }
        }
        3 => {
            if a.objetivo.trim().chars().count() < 10 {
                Some("Define el objetivo en al menos 10 caracteres.".to_string())
            } else {
                None
            }
        }
        _ => None, // picks siempre tienen valor
    }
}

/// Ubicación efectiva: la escrita o el workspace.
pub fn effective_location(a: &Answers, workspace: &Path) -> String {
    let u = a.ubicacion.trim();
    if u.is_empty() {
        workspace.to_string_lossy().to_string()
    } else {
        u.to_string()
    }
}

const TEMPLATE: &str = r#"# {{ nombre }}

## Descripción
{{ descripcion }}

## Ubicación
{{ ubicacion }}

## Objetivo
{{ objetivo }}

## Público objetivo
{{ publico }}

## Interfaz
{{ interfaz }}

_Generado por ARQHIA_
"#;

pub fn generate_spec(a: &Answers, workspace: &Path) -> Result<String, String> {
    let env = minijinja::Environment::new();
    env.render_str(
        TEMPLATE,
        minijinja::context! {
            nombre => a.nombre.trim(),
            descripcion => a.descripcion.trim(),
            ubicacion => effective_location(a, workspace),
            objetivo => a.objetivo.trim(),
            publico => a.publico.to_string(),
            interfaz => a.interfaz.to_string(),
        },
    )
    .map_err(|e| format!("No se pudo generar: {e}"))
}

/// Guarda en `{workspace}/CONTEXT/ESPEC.md` (crea CONTEXT si falta).
pub fn save_spec(workspace: &Path, content: &str) -> Result<PathBuf, String> {
    let dir = workspace.join("CONTEXT");
    std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear CONTEXT: {e}"))?;
    let file = dir.join("ESPEC.md");
    std::fs::write(&file, content).map_err(|e| format!("No se pudo escribir ESPEC.md: {e}"))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Answers {
        Answers {
            nombre: "DemoApp".to_string(),
            descripcion: "Una app demo para probar el cuestionario".to_string(),
            ubicacion: "".to_string(),
            objetivo: "Probar que el ESPEC se genera bien".to_string(),
            publico: Publico::Empresa,
            interfaz: Interfaz::Cli,
        }
    }

    #[test]
    fn spec_has_all_six_sections() {
        let spec = generate_spec(&sample(), Path::new("/tmp/ws")).unwrap();
        for section in [
            "# DemoApp",
            "## Descripción",
            "## Ubicación",
            "## Objetivo",
            "## Público objetivo",
            "## Interfaz",
            "Empresa",
            "CLI",
            "/tmp/ws",
        ] {
            assert!(spec.contains(section), "falta: {section}");
        }
    }

    #[test]
    fn validation_blocks_bad_input() {
        let mut a = sample();
        a.nombre.clear();
        assert!(validate(0, &a).is_some());
        a.nombre = "x".repeat(61);
        assert!(validate(0, &a).is_some());
        a = sample();
        a.descripcion = "corta".to_string();
        assert!(validate(1, &a).is_some());
        a = sample();
        a.ubicacion = "relativa/no".to_string();
        assert!(validate(2, &a).is_some());
        a.ubicacion = "/abs/ok".to_string();
        assert!(validate(2, &a).is_none());
        a.objetivo = "x".to_string();
        assert!(validate(3, &a).is_some());
        assert!(validate(4, &a).is_none());
        assert!(validate(5, &a).is_none());
    }
}
