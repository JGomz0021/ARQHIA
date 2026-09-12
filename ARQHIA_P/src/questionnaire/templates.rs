//! Plantillas minijinja de los documentos del proyecto (v0.8/v0.8.1).
//!
//! Puro: `generate_project_docs(level, answers, ai) -> (PROJECT, SPECS, CONTEXT)`.
//! El disco lo escribe el handler vía `workspace::ensure_project_layout`.

use super::levels::{ArqPreset, EstiloPreset, Facturacion, Level, TipoProyecto};
use super::Answers;

const PROJECT_T: &str = r#"# {{ nombre }}

## Descripción
{{ descripcion }}

## Objetivo
{{ objetivo }}

## Nivel
{{ nivel }}

## Funcionalidades
{{ funcionalidades }}

## Alcance
{{ alcance }}

## Usuario
Proyecto definido con el cuestionario guiado de ARQHIA (nivel {{ nivel }}).
"#;

const SPECS_T: &str = r#"# Especificación — {{ nombre }}

## Tipo de proyecto
{{ tipo }}

## Funcionalidades
{{ funcionalidades }}

{{ cuerpo }}
## Facturación
{{ facturacion }}
"#;

const CONTEXT_T: &str = r#"# {{ nombre }} — Contexto

Proyecto recién definido con el cuestionario guiado (nivel {{ nivel }}).

## Estado
- [x] Cuestionario completado (PROJECT.md + SPECS.md generados)
- [ ] Ejecutar Plan (ROADMAP.md + VERSIONS.md + VERSIONS/v0.1.md)
- [ ] Desarrollo en `Project/`

## Siguiente paso
Ejecutar Plan desde el chat en modo Plan.

## Índice de CONTEXT/
- `PROJECT.md` — visión, objetivo, alcance, usuario
- `SPECS.md` — especificación funcional
- `CONTEXT.md` — este archivo (índice + estado vivo)
- `ROADMAP.md` — versiones de alto nivel (se genera en el primer Plan)
- `VERSIONS.md` — índice de estado por versión (se genera en el primer Plan)
- `VERSIONS/` — un .md por versión (se genera en el primer Plan)
"#;

/// `(PROJECT.md, SPECS.md, CONTEXT.md)`.
pub fn generate_project_docs(
    level: Level,
    a: &Answers,
    ai: &[(String, String)],
) -> Result<(String, String, String), String> {
    let env = minijinja::Environment::new();
    let nombre = a.nombre.trim();
    let funcionalidades = bullets(&a.funcionalidades);
    let facturacion = factura_line(a.facturacion);
    let alcance = alcance_for(level, a);
    let project = env
        .render_str(
            PROJECT_T,
            minijinja::context! {
                nombre => nombre,
                descripcion => a.descripcion.trim(),
                objetivo => a.objetivo.trim(),
                nivel => level.to_string(),
                funcionalidades => funcionalidades,
                alcance => alcance,
            },
        )
        .map_err(|e| format!("No se pudo generar PROJECT.md: {e}"))?;
    let cuerpo = specs_body(level, a);
    let mut specs = env
        .render_str(
            SPECS_T,
            minijinja::context! {
                nombre => nombre,
                tipo => tipo_line(level, a.tipo),
                funcionalidades => funcionalidades,
                cuerpo => cuerpo,
                facturacion => facturacion,
            },
        )
        .map_err(|e| format!("No se pudo generar SPECS.md: {e}"))?;
    if !ai.is_empty() {
        specs.push_str("\n## Preguntas adicionales (IA)\n");
        for (q, r) in ai {
            if r.trim().is_empty() {
                specs.push_str(&format!("\n- {q} (sin responder)\n"));
            } else {
                specs.push_str(&format!("\n- {q}\n  {r}\n"));
            }
        }
    }
    let context = env
        .render_str(
            CONTEXT_T,
            minijinja::context! { nombre => nombre, nivel => level.to_string() },
        )
        .map_err(|e| format!("No se pudo generar CONTEXT.md: {e}"))?;
    Ok((project, specs, context))
}

/// Normaliza funcionalidades a bullets `- ` (una por línea).
fn bullets(raw: &str) -> String {
    let items: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let t = l.trim_start_matches(['-', '*', '•']).trim();
            format!("- {t}")
        })
        .collect();
    if items.is_empty() {
        "- (sin detallar)".to_string()
    } else {
        items.join("\n")
    }
}

fn tipo_line(level: Level, t: TipoProyecto) -> String {
    match level {
        Level::Principiante => "(se define al planificar)".to_string(),
        _ => t.to_string(),
    }
}

fn factura_line(f: Facturacion) -> String {
    match f {
        Facturacion::Suscripciones => "Modelo: suscripciones.".to_string(),
        Facturacion::PagoUnico => "Modelo: pago único.".to_string(),
        Facturacion::Api => "Modelo: API de pago.".to_string(),
        Facturacion::UsoPersonal => "Modelo: uso personal (sin facturación).".to_string(),
        Facturacion::Publicidad => "Modelo: publicidad.".to_string(),
        Facturacion::Freemium => "Modelo: freemium.".to_string(),
    }
}

fn plataform_line(a: &Answers) -> String {
    if a.plataformas.is_empty() {
        "(sin especificar)".to_string()
    } else {
        a.plataformas.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
    }
}

fn stack_line(a: &Answers) -> String {
    let mut base = if a.stacks.is_empty() {
        "A decidir".to_string()
    } else {
        a.stacks.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
    };
    if !a.stack_free.trim().is_empty() {
        base.push_str(&format!(" ({})", a.stack_free.trim()));
    }
    base
}

fn estilo_line(a: &Answers) -> String {
    match a.estilo {
        EstiloPreset::Otro if a.estilo_free.trim().is_empty() => "Otro".to_string(),
        _ if a.estilo_free.trim().is_empty() => a.estilo.to_string(),
        _ => format!("{} ({})", a.estilo, a.estilo_free.trim()),
    }
}

fn arq_line(a: &Answers) -> String {
    match a.arq {
        ArqPreset::ADecidir if a.arquitectura.trim().is_empty() => "A decidir".to_string(),
        _ if a.arquitectura.trim().is_empty() => a.arq.to_string(),
        _ => format!("{} ({})", a.arq, a.arquitectura.trim()),
    }
}

fn alcance_for(level: Level, a: &Answers) -> String {
    match level {
        Level::Principiante => {
            format!(
                "Diseño y decisiones iniciales (estilo: {}; plataforma: {}).",
                estilo_line(a),
                plataform_line(a)
            )
        }
        Level::Intermedio => {
            format!(
                "Producto funcional (tipo: {}; plataforma: {}; stack: {}).",
                a.tipo,
                plataform_line(a),
                stack_line(a)
            )
        }
        Level::Avanzado => {
            format!(
                "Sistema con arquitectura explícita (tipo: {}; plataforma: {}; stack: {}; arquitectura: {}).",
                a.tipo,
                plataform_line(a),
                stack_line(a),
                arq_line(a)
            )
        }
    }
}

/// Cuerpo de SPECS.md según nivel (v0.8.1: secciones profundas).
fn specs_body(level: Level, a: &Answers) -> String {
    let mut out = String::new();
    match level {
        Level::Principiante => {
            out.push_str(&format!("## Estilo visual\n{}\n\n", estilo_line(a)));
            out.push_str(&format!("## Plataforma\n{}\n\n", plataform_line(a)));
        }
        Level::Intermedio => {
            out.push_str(&format!("## UI/UX\n{}\n\n", or_dash(&a.ui_ux)));
            out.push_str(&format!("## Plataforma\n{}\n\n", plataform_line(a)));
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
        }
        Level::Avanzado => {
            out.push_str(&format!("## UI/UX\n{}\n\n", or_dash(&a.ui_ux)));
            out.push_str(&format!("## Plataforma\n{}\n\n", plataform_line(a)));
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
            out.push_str(&format!("## Arquitectura\n{}\n\n", arq_line(a)));
        }
    }
    out
}

fn or_dash(s: &str) -> String {
    if s.trim().is_empty() {
        "(sin detallar)".to_string()
    } else {
        s.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::questionnaire::levels::{Level, Plataforma, StackOpt};

    fn sample(level: Level) -> Answers {
        let _ = level;
        Answers {
            nombre: "Tienda".to_string(),
            descripcion: "Una tienda online de ejemplo".to_string(),
            objetivo: "Vender productos por internet".to_string(),
            funcionalidades: "Catálogo\nCarrito\nPago".to_string(),
            estilo: EstiloPreset::Minimalista,
            estilo_free: "como esa web".to_string(),
            ui_ux: "Simple y rápida".to_string(),
            tipo: TipoProyecto::ApiBackend,
            plataformas: vec![Plataforma::Web, Plataforma::ServidorNube],
            stacks: vec![StackOpt::Rust, StackOpt::Python],
            stack_free: String::new(),
            arq: ArqPreset::PorCapas,
            arquitectura: "tres capas".to_string(),
            facturacion: Facturacion::Freemium,
        }
    }

    #[test]
    fn docs_per_level_have_expected_sections() {
        let (p, s, c) = generate_project_docs(Level::Principiante, &sample(Level::Principiante), &[]).unwrap();
        assert!(p.contains("# Tienda") && p.contains("Principiante"));
        assert!(s.contains("## Estilo visual") && s.contains("Minimalista"));
        assert!(s.contains("## Plataforma") && s.contains("Web, Servidor / Nube"));
        assert!(!s.contains("## Arquitectura"));
        assert!(c.contains("Ejecutar Plan"));

        let (_, s, _) = generate_project_docs(Level::Intermedio, &sample(Level::Intermedio), &[]).unwrap();
        assert!(s.contains("## Tipo de proyecto") && s.contains("API / Backend"));
        assert!(s.contains("## UI/UX") && s.contains("Rust, Python"));
        assert!(!s.contains("## Arquitectura"));

        let (_, s, _) = generate_project_docs(Level::Avanzado, &sample(Level::Avanzado), &[]).unwrap();
        assert!(s.contains("## Arquitectura") && s.contains("Por capas"));
        assert!(s.contains("freemium"));

        // IA anexada solo si hay respuestas.
        let ai = vec![("¿Offline?".to_string(), "Sí".to_string())];
        let (_, s2, _) = generate_project_docs(Level::Principiante, &sample(Level::Principiante), &ai).unwrap();
        assert!(s2.contains("## Preguntas adicionales (IA)") && s2.contains("¿Offline?"));
    }

    #[test]
    fn bullets_normalize() {
        assert_eq!(bullets("a\n- b\n* c"), "- a\n- b\n- c");
        assert!(bullets("").contains("sin detallar"));
    }
}
