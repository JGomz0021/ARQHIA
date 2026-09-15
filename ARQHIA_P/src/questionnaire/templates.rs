//! Plantillas minijinja de los documentos del proyecto (v0.8.1).
//!
//! Puro: `generate_project_docs(level, answers, ai, origin) -> (PROJECT, SPECS, CONTEXT)`.
//! SPECS con secciones por **familia** (no por nivel); CONTEXT anota el origen.

use super::levels::{Categoria, EstiloPreset, Level, SysType};
use super::Answers;
use super::QSource;

const PROJECT_T: &str = r#"# {{ nombre }}

## Descripción ejecutiva
{{ descripcion }}

## Objetivo principal
{{ objetivo }}

## Subobjetivos
{{ subobjetivos }}

## Público objetivo
{{ publico }}

## Uso previsto
{{ uso_previsto }}

## Funcionalidades principales
{{ funcionalidades }}

## Tipo y alcance
{{ tipo }} — {{ alcance }}

## Misión
{{ mision }}

## Visión
{{ vision }}

## Nivel y origen
Nivel {{ nivel }} — {{ tipo }} ({{ origen }}).

## Gaps
{{ gaps }}
"#;

const SPECS_T: &str = r#"# Especificación — {{ nombre }}

## Tipo de sistema
{{ tipo }}

## Funcionalidades
{{ funcionalidades }}

### Criterios de aceptación
{{ criterios }}

{{ cuerpo }}
{{ cierre }}

## Arquitectura relevante
{{ arquitectura }}

## Flujos de trabajo
{{ flujos }}

## Recurrencia / Procesos programados
{{ recurrencia }}

## Recomendaciones y convenciones
{{ convenciones }}

## Dependencias importantes
{{ dependencias }}

## Tests funcionales requeridos
{{ tests_func }}

## Pruebas QA
{{ qa }}

## Otros requisitos
{{ otros }}
"#;

const CONTEXT_T: &str = r#"# {{ nombre }} — Contexto

Proyecto {{ origen }} con el cuestionario guiado (nivel {{ nivel }}).

## Estado
- [x] Cuestionario completado (PROJECT.md + SPECS.md generados)
- [x] Plan automático (ROADMAP.md + VERSIONS.md + VERSIONS/v0.1.md + ToDo.md)
- [ ] Desarrollo en `Project/`

## Siguiente paso
Revisar el plan generado (ROADMAP.md + VERSIONS/v0.1.md) y desarrollar en modo Work.

## Índice de CONTEXT/
- `PROJECT.md` — visión, objetivo, alcance, usuario
- `SPECS.md` — especificación funcional (con criterios de aceptación)
- `CONTEXT.md` — este archivo (índice + estado vivo)
- `ROADMAP.md` — versiones de alto nivel (generado en el post-cuestionario)
- `VERSIONS.md` — índice de estado por versión (generado en el post-cuestionario)
- `VERSIONS/` — un .md por versión (v0.1 generado en el post-cuestionario)
- `ANALYSIS.md` — brief del analista (lo lee el planner)
- `PLAN.md` — plan aprobado (modo Plan)
- `TEMP.md` — último reporte del auditor (efímero)
- `UI-REVIEW.md` — reporte de `/skill ui-ux` (si se usó)
- `CODE-REVIEW.md` — reporte de `/skill code-review` (si se usó)
- `QA-REPORT.md` — reporte de `/skill test-qa` (si se usó)
"#;

/// `(PROJECT.md, SPECS.md, CONTEXT.md)` — SSOT según spec §2.
pub fn generate_project_docs(
    level: Level,
    a: &Answers,
    ai: &[(String, String)],
    source: &QSource,
) -> Result<(String, String, String), String> {
    let env = minijinja::Environment::new();
    let nombre = a.nombre.trim();
    let funcionalidades = bullets(&a.funcionalidades);
    let alcance = alcance_for(a);
    let gaps_md = gaps_md_for(a);
    let subobjetivos = subobjetivos_for(a);
    let publico = or_dash(&a.publico_objetivo);
    let uso_previsto = or_dash(&a.uso_previsto);
    let mision = format!("Cumplir el objetivo principal de {} para {}.", a.objetivo.trim().split('.').next().unwrap_or(&a.objetivo).trim(), publico);
    let vision = format!("{} usable y mantenible, con CONTEXT como fuente de verdad (usuario → cuestionario → IA → CONTEXT → plan → implementación → auditoría).", a.cat);
    let origen_str = match source {
        QSource::New => "definido desde cero",
        QSource::Import => "importado de código existente",
    };
    let project = env
        .render_str(
            PROJECT_T,
            minijinja::context! {
                nombre => nombre,
                descripcion => a.descripcion.trim(),
                objetivo => a.objetivo.trim(),
                subobjetivos => subobjetivos,
                publico => publico,
                uso_previsto => uso_previsto,
                nivel => level.to_string(),
                tipo => format!("{} — {}", a.cat, a.sys),
                funcionalidades => funcionalidades,
                alcance => alcance,
                mision => mision,
                vision => vision,
                origen => origen_str,
                gaps => gaps_md,
            },
        )
        .map_err(|e| format!("No se pudo generar PROJECT.md: {e}"))?;
    let cuerpo = specs_body(level, a);
    let cierre = specs_cierre(a);
    let criterios = specs_criterios(a);
    let oss = specs_oss(a);
    let arquitectura = arquitectura_for(a);
    let flujos = flujos_for(a);
    let recurrencia = recurrencia_for(a);
    let convenciones = convenciones_for();
    let dependencias = dependencias_for(a);
    let tests_func = tests_func_para(a);
    let qa = qa_for();
    let otros = otros_for(a, gaps_md.clone());
    let mut specs = env
        .render_str(
            SPECS_T,
            minijinja::context! {
                nombre => nombre,
                tipo => format!("{} — {}", a.cat, a.sys),
                funcionalidades => funcionalidades,
                criterios => criterios,
                cuerpo => cuerpo,
                cierre => cierre,
                arquitectura => arquitectura,
                flujos => flujos,
                recurrencia => recurrencia,
                convenciones => convenciones,
                dependencias => dependencias,
                tests_func => tests_func,
                qa => qa,
                otros => otros,
            },
        )
        .map_err(|e| format!("No se pudo generar SPECS.md: {e}"))?;
    if !oss.trim().is_empty() {
        specs.push_str(&format!("\n{oss}\n"));
    }
    if !ai.is_empty() {
        specs.push_str("\n## Preguntas adicionales (IA)\n");
        specs.push_str("> Generadas dinámicamente (5–10, sin repetir lo ya dado) para cubrir vacíos.\n");
        for (q, r) in ai {
            if r.trim().is_empty() {
                specs.push_str(&format!("\n- {q} (sin responder)\n"));
            } else {
                specs.push_str(&format!("\n- {q}\n  {r}\n"));
            }
        }
    }
    let origen = match source {
        QSource::New => "definido desde cero",
        QSource::Import => "importado de código existente",
    };
    let context = env
        .render_str(
            CONTEXT_T,
            minijinja::context! { nombre => nombre, nivel => level.to_string(), origen => origen },
        )
        .map_err(|e| format!("No se pudo generar CONTEXT.md: {e}"))?;
    Ok((project, specs, context))
}

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

fn plataform_line(a: &Answers) -> String {
    if a.plataformas.is_empty() {
        "Pendiente de definir: plataformas (ver Plan/MVP)".to_string()
    } else {
        a.plataformas.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
    }
}

fn stack_line(a: &Answers) -> String {
    let mut base = if a.stacks.is_empty() {
        "Pendiente de definir: stack (se propone en el Plan/MVP)".to_string()
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

fn arch_line(a: &Answers) -> String {
    if a.archs.is_empty() {
        "Pendiente de definir: arch target (ver Plan/MVP)".to_string()
    } else {
        a.archs.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
    }
}

fn gaps_md_for(a: &Answers) -> String {
    let g = super::planning::gaps(a);
    if g.is_empty() {
        "Sin gaps: el cuestionario cubrió lo necesario.".to_string()
    } else {
        g.iter().map(|x| format!("- Pendiente de definir: {x}")).collect::<Vec<_>>().join("\n")
    }
}

/// Sección OSS: solo si el proyecto se publica como abierto.
fn specs_oss(a: &Answers) -> String {
    if !super::levels::is_oss(a.cat, a.facturacion, a.licencia) {
        return String::new();
    }
    let mut s = String::from("## Open source\n");
    s.push_str(&format!("Repo: {}\n\n", or_dash(&a.oss_repo)));
    s.push_str(&format!("Gobierno: {}\n\n", or_dash(&a.oss_gobierno)));
    s.push_str(&format!("Contribución: {}\n", or_dash(&a.oss_contrib)));
    s
}

fn alcance_for(a: &Answers) -> String {
    format!(
        "{} ({}) en {} con {}.",
        a.cat,
        a.sys,
        plataform_line(a),
        stack_line(a)
    )
}

/// Cierre: facturación (App/Servicio) o licencia (resto, con avisos).
fn specs_cierre(a: &Answers) -> String {
    if super::levels::uses_facturacion(a.cat) {
        return format!("## Facturación\n{}\n", factura_line(a));
    }
    let mut s = format!("## Licencia\n{}\n", a.licencia);
    if a.licencia == super::levels::Licencia::Gpl3 {
        s.push_str("\n> Aviso: GPL-3.0 obliga a liberar los derivados con la misma licencia.\n");
    }
    if a.licencia == super::levels::Licencia::UsoInterno {
        s.push_str("\n> Uso interno: este código nunca sube a la nube.\n");
    }
    s
}

fn factura_line(a: &Answers) -> String {
    use super::levels::Facturacion as F;
    match a.facturacion {
        F::Suscripciones => "Modelo: suscripciones.".to_string(),
        F::PagoUnico => "Modelo: pago único.".to_string(),
        F::Api => "Modelo: API de pago.".to_string(),
        F::UsoPersonal => "Modelo: uso personal (sin facturación).".to_string(),
        F::Publicidad => "Modelo: publicidad.".to_string(),
        F::Freemium => "Modelo: freemium.".to_string(),
        F::OpenSource => "Modelo: código abierto (sin cobro; ver Licencia).".to_string(),
    }
}

fn subobjetivos_for(a: &Answers) -> String {
    // Deriva 2–3 subobjetivos de funcionalidades u objetivo
    let funcs: Vec<String> = a.funcionalidades.lines().map(|l| l.trim_start_matches(['-', '*', '•']).trim().to_string()).filter(|l| !l.is_empty()).collect();
    if funcs.is_empty() {
        format!("- Completar definición de subobjetivos en Plan (derivado de: {}).", or_dash(&a.objetivo))
    } else {
        funcs.iter().take(3).map(|f| format!("- {f}: entregable verificable.")).collect::<Vec<_>>().join("\n")
    }
}

fn arquitectura_for(a: &Answers) -> String {
    if !a.archs.is_empty() || !a.sintaxis.trim().is_empty() || !a.toolchain.trim().is_empty() {
        format!("Targets: {}. Sintaxis: {}. Toolchain: {}.", arch_line(a), or_dash(&a.sintaxis), or_dash(&a.toolchain))
    } else if a.cat == Categoria::Aplicacion || a.cat == Categoria::Servicio {
        "Pendiente de definir: arquitectura relevante (monolito/capas/hexagonal/microservicios/eventos/serverless según familia Nivel Avanzado; ver Plan).".to_string()
    } else {
        "Pendiente de definir: arquitectura (ver Plan/MVP).".to_string()
    }
}

fn flujos_for(a: &Answers) -> String {
    if !a.ui_ux.trim().is_empty() || !a.pipeline_desc.trim().is_empty() || !a.endpoints.trim().is_empty() {
        format!("Usuario: {}. Datos/Pipeline: {}. Endpoints: {}.", or_dash(&a.ui_ux), or_dash(&a.pipeline_desc), or_dash(&a.endpoints))
    } else {
        "Pendiente de definir: flujos de datos y de usuario (ver Plan).".to_string()
    }
}

fn recurrencia_for(a: &Answers) -> String {
    if a.trigger != super::levels::Trigger::Cron && !a.idempotencia.trim().is_empty() {
        format!("Trigger: {}. Idempotencia: {}.", a.trigger, or_dash(&a.idempotencia))
    } else if a.cat == Categoria::Automatizacion || a.cat == Categoria::DatosIa {
        "Pendiente de definir: recurrencia y procesos programados (cron/manual/webhook/CI).".to_string()
    } else {
        "No aplica o pendiente de definir según Plan.".to_string()
    }
}

fn convenciones_for() -> String {
    "- Idioma: docs en español, código/commits en inglés.\n- Versionado SemVer v0.x → v1.0.\n- CONTEXT como SSOT; `Project/` es código + git.\n- Commits en rama `ARQHIA`, auditoría verde antes de versionar.".to_string()
}

fn dependencias_for(a: &Answers) -> String {
    let mut deps = vec![format!("Stack: {}", stack_line(a)), format!("Plataformas: {}", plataform_line(a))];
    if !a.endpoints.trim().is_empty() { deps.push(format!("APIs: {}", a.endpoints)); }
    if super::levels::is_oss(a.cat, a.facturacion, a.licencia) { deps.push("Open Source: repo/gobierno/contribución definidos".to_string()); }
    deps.join(" | ")
}

fn tests_func_para(a: &Answers) -> String {
    let items: Vec<String> = a.funcionalidades.lines().map(|l| l.trim_start_matches(['-', '*', '•']).trim()).filter(|l| !l.is_empty()).map(|s| s.to_string()).collect();
    if items.is_empty() {
        "Pendiente de definir: tests funcionales por funcionalidad (ver criterios).".to_string()
    } else {
        items.iter().map(|f| format!("- Test funcional: {} → caso feliz + caso de error principal.", f)).collect::<Vec<_>>().join("\n")
    }
}

fn qa_for() -> String {
    "- Verificar criterios de aceptación por versión (ROADMAP/VERSIONS).\n- Probar en dispositivos/plataformas objetivo.\n- Revisar regresiones y accesibilidad básica.".to_string()
}

fn otros_for(a: &Answers, gaps: String) -> String {
    format!("Gaps: {}. Público: {}. Uso: {}.", gaps.replace('\n', " | "), or_dash(&a.publico_objetivo), or_dash(&a.uso_previsto))
}

/// Cuerpo de SPECS.md por familia (secciones de la matriz que apliquen).
/// Stack/plataforma siempre se renderizan (con gap explícito si vacío).
fn specs_body(level: Level, a: &Answers) -> String {
    let _ = level;
    let mut out = String::new();
    let sec = |o: &mut String, t: &str, v: &str| {
        o.push_str(&format!("## {t}\n{}\n\n", or_dash(v)));
    };
    match a.cat {
        Categoria::Aplicacion => {
            sec(&mut out, "UI/UX", &a.ui_ux);
            if super::levels::has_ui(a.sys) {
                out.push_str(&format!("## Estilo visual\n{}\n\n", estilo_line(a)));
            }
            out.push_str(&format!("## Plataforma\n{}\n\n", plataform_line(a)));
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
        }
        Categoria::Servicio => {
            sec(&mut out, "Endpoints", &a.endpoints);
            out.push_str(&format!("## Estilo de API\n{}\n\n", a.api_style));
            out.push_str(&format!("## Autenticación\n{}\n\n", a.auth));
            sec(&mut out, "Escala/SLA", &a.escala);
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
        }
        Categoria::Libreria => {
            if a.sys == super::levels::SysType::MotorEngine {
                sec(&mut out, "UI/UX del editor", &a.ui_ux);
                out.push_str(&format!("## Estilo visual\n{}\n\n", estilo_line(a)));
            }
            sec(&mut out, "API pública", &a.api_publica);
            if a.sys == super::levels::SysType::PluginExtension {
                sec(&mut out, "Host donde se instala", &a.host_api);
            }
            out.push_str(&format!("## Compatibilidad\n{}\n\n", a.semver));
            out.push_str(&format!("## Lenguaje objetivo\n{}\n\n", stack_line(a)));
            if a.sys == super::levels::SysType::MotorEngine {
                out.push_str(&format!("## Plataforma\n{}\n\n", plataform_line(a)));
            }
            sec(&mut out, "Ejemplos/Docs", &a.ejemplos);
        }
        Categoria::Sistema => {
            if a.sys == super::levels::SysType::LenguajeRuntime {
                sec(&mut out, "Sintaxis y tipos", &a.sintaxis);
                sec(&mut out, "Toolchain", &a.toolchain);
                out.push_str(&format!("## Arquitectura objetivo\n{}\n\n", arch_line(a)));
                sec(&mut out, "Compatibilidad", &a.compat);
                out.push_str(&format!("## Lenguaje de implementación\n{}\n\n", stack_line(a)));
            } else {
                out.push_str(&format!("## Arquitectura objetivo\n{}\n\n", arch_line(a)));
                sec(&mut out, "Arranque/Hardware", &a.arranque);
                if matches!(
                    a.sys,
                    super::levels::SysType::OsDistro
                        | super::levels::SysType::KernelModulo
                        | super::levels::SysType::ContenedorVm
                ) {
                    sec(&mut out, "Syscalls/ABI", &a.syscalls);
                }
                sec(&mut out, "Compatibilidad", &a.compat);
                out.push_str(&format!("## Lenguaje de implementación\n{}\n\n", stack_line(a)));
            }
        }
        Categoria::Automatizacion => {
            out.push_str(&format!("## Disparador\n{}\n\n", a.trigger));
            sec(&mut out, "Entradas/Secretos", &a.inputs_secretos);
            sec(&mut out, "Idempotencia", &a.idempotencia);
            out.push_str(&format!("## Dónde corre\n{}\n\n", plataform_line(a)));
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
        }
        Categoria::DatosIa => {
            sec(&mut out, "Dataset/Fuente", &a.dataset);
            sec(&mut out, "Pipeline", &a.pipeline_desc);
            sec(&mut out, "Modelo/Evaluación", &a.modelo_eval);
            out.push_str(&format!("## Stack\n{}\n\n", stack_line(a)));
        }
    }
    // SysType raro (imposible por cascada, pero no rompemos docs).
    let _ = SysType::AppWeb;
    out
}

/// Criterios de aceptación por funcionalidad (v0.9 Track D): 1–3 bullets
/// verificables por cada funcionalidad listada, con verbos de la familia
/// (endpoints con verbos, UI visible, CLI con código de salida...).
/// Lo auto-propuesto se marca como tal (revisar en Plan), nunca mudo.
fn specs_criterios(a: &Answers) -> String {
    let items: Vec<String> = a
        .funcionalidades
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_start_matches(['-', '*', '•']).trim().to_string())
        .collect();
    if items.is_empty() {
        return "Pendiente de definir: funcionalidades (y sus criterios).".to_string();
    }
    let per_family: Vec<&str> = match a.cat {
        Categoria::Aplicacion => vec![
            "visible y usable desde la interfaz sin errores en el caso feliz",
            "el caso de error principal muestra un mensaje claro (sin pantallazos)",
        ],
        Categoria::Servicio => vec![
            "endpoint responde 2xx con el formato acordado en el caso feliz",
            "errores devuelven código + mensaje (4xx/5xx según el caso, sin 500 mudos)",
        ],
        Categoria::Libreria => vec![
            "la API pública compila y el ejemplo mínimo corre de punta a punta",
            "cambio incompatible solo con bump mayor (semver respetado)",
        ],
        Categoria::Sistema => vec![
            "compila para el target declarado y arranca en el entorno objetivo",
            "fallo de hardware/entorno deja registro diagnosticable",
        ],
        Categoria::Automatizacion => vec![
            "ejecución repetida da el mismo resultado (idempotente) o reintenta con backoff",
            "secretos/entradas faltantes abortan con mensaje claro antes de actuar",
        ],
        Categoria::DatosIa => vec![
            "pipeline corre sobre una muestra y deja métricas/artefactos esperados",
            "evaluación mínima definida (métrica + umbral) antes de dar por buena la salida",
        ],
    };
    items
        .iter()
        .map(|f| {
            let bullets = per_family
                .iter()
                .map(|c| format!("  - {f}: {c}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("- {f} (propuesto, revisar en Plan):\n{bullets}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn or_dash(s: &str) -> String {
    if s.trim().is_empty() {
        "Pendiente de definir: completar en Plan/MVP.".to_string()
    } else {
        s.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::questionnaire::levels::{Categoria, Level, Plataforma, StackOpt, SysType};

    fn sample_app() -> Answers {
        Answers {
            nombre: "Tienda".to_string(),
            descripcion: "Una tienda online de ejemplo".to_string(),
            objetivo: "Vender productos por internet".to_string(),
            funcionalidades: "Catálogo\nCarrito\nPago".to_string(),
            cat: Categoria::Aplicacion,
            sys: SysType::AppWeb,
            estilo: EstiloPreset::Minimalista,
            plataformas: vec![Plataforma::Web],
            facturacion: crate::questionnaire::levels::Facturacion::Freemium,
            ..Default::default()
        }
    }

    #[test]
    fn app_docs_have_family_sections() {
        let (p, s, c) =
            generate_project_docs(Level::Intermedio, &sample_app(), &[], &QSource::New).unwrap();
        assert!(p.contains("# Tienda") && p.contains("Aplicación"));
        assert!(s.contains("## Tipo de sistema") && s.contains("App web"));
        assert!(s.contains("## Estilo visual") && s.contains("Minimalista"));
        assert!(s.contains("## Facturación") && s.contains("freemium"));
        assert!(!s.contains("## Arquitectura objetivo"));
        assert!(c.contains("desde cero"));
    }

    #[test]
    fn specs_carry_acceptance_criteria_per_feature() {
        // v0.9 Track D: cada funcionalidad con criterios verificables.
        let (_, s, _) =
            generate_project_docs(Level::Intermedio, &sample_app(), &[], &QSource::New).unwrap();
        assert!(s.contains("### Criterios de aceptación"));
        assert!(s.contains("Catálogo") && s.contains("Carrito") && s.contains("Pago"));
        assert!(s.contains("sin errores en el caso feliz"), "criterio verificable: {s}");
        assert!(!s.contains("(se define al planificar)"), "cero placeholders mudos");
        // Servicio: verbos de endpoints en los criterios.
        let mut svc = sample_app();
        svc.cat = Categoria::Servicio;
        svc.sys = SysType::ApiBackend;
        let (_, ss, _) = generate_project_docs(Level::Avanzado, &svc, &[], &QSource::New).unwrap();
        assert!(ss.contains("2xx"), "criterios de servicio: {ss}");
    }

    #[test]
    fn driver_docs_ask_licence_not_billing() {
        let mut a = sample_app();
        a.cat = Categoria::Sistema;
        a.sys = SysType::DriverFirmware;
        a.licencia = crate::questionnaire::levels::Licencia::Gpl3;
        let (_, s, c) =
            generate_project_docs(Level::Avanzado, &a, &[], &QSource::Import).unwrap();
        assert!(s.contains("## Licencia") && s.contains("GPL-3.0"));
        assert!(s.contains("obliga a liberar"));
        assert!(!s.contains("## Facturación"));
        assert!(c.contains("importado"));
    }

    #[test]
    fn open_source_billing_renders_without_charge() {
        let mut a = sample_app();
        a.facturacion = crate::questionnaire::levels::Facturacion::OpenSource;
        let (_, s, _) =
            generate_project_docs(Level::Intermedio, &a, &[], &QSource::New).unwrap();
        assert!(s.contains("## Facturación") && s.contains("código abierto"), "{s}");
    }

    #[test]
    fn servicio_and_libreria_sections() {
        let mut a = sample_app();
        a.cat = Categoria::Servicio;
        a.sys = SysType::ApiBackend;
        a.endpoints = "GET /users".to_string();
        a.stacks = vec![StackOpt::Rust];
        let (_, s, _) = generate_project_docs(Level::Avanzado, &a, &[], &QSource::New).unwrap();
        assert!(s.contains("## Endpoints") && s.contains("GET /users"));
        assert!(s.contains("Servidor") || s.contains("REST") || s.contains("Facturación"));

        let mut b = sample_app();
        b.cat = Categoria::Libreria;
        b.sys = SysType::FrameworkLibreria;
        b.api_publica = "fn render()".to_string();
        let (_, s2, _) = generate_project_docs(Level::Intermedio, &b, &[], &QSource::New).unwrap();
        assert!(s2.contains("## API pública") && s2.contains("fn render()"));
        assert!(s2.contains("## Licencia"));
    }

    #[test]
    fn bullets_normalize() {
        assert_eq!(bullets("a\n- b\n* c"), "- a\n- b\n- c");
        assert!(bullets("").contains("sin detallar"));
    }
}
