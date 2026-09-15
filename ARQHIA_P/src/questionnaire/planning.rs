//! Contexto automático post-cuestionario (v0.9 Track D + v0.8.2 MVP).
//!
//! Puro (sin I/O salvo `write_auto_docs`/`write_mvp_docs`): la casa como
//! referencia. `plan_prompt` arma el system del turno Plan con las
//! convenciones + resumen del wizard + gaps; `mvp_prompt` pide la serie
//! completa hasta el MVP con marcadores `---FILE: ruta---` para la pantalla
//! de carga (sin pasar por el chat); `validate_version_md` exige la
//! plantilla de 8 secciones; `write_auto_docs` genera ROADMAP + VERSIONS +
//! v0.1 + ToDo; `write_mvp_docs` escribe la serie v0.1→v1.0 + ToDo
//! derivados de las respuestas (nunca genéricos, sin placeholders mudos).

use super::levels::{Categoria, Level};
use super::{Answers, QSource};

/// Las 8 secciones obligatorias de cada `VERSIONS/v0.x.md` de la casa.
pub fn house_template() -> [&'static str; 8] {
    [
        "Objetivo",
        "Alcance",
        "Paso a paso",
        "Tareas técnicas",
        "Criterio Done",
        "Test de Funcionalidad",
        "Riesgos",
        "Dependencias",
    ]
}

/// Campos del wizard que quedaron vacíos y el plan debe resolver.
/// Se marcan como `Pendiente de definir: X` (nunca placeholder mudo).
pub fn gaps(a: &Answers) -> Vec<String> {
    let mut out = Vec::new();
    if a.stacks.is_empty() && a.stack_free.trim().is_empty() {
        out.push("stack".to_string());
    }
    if a.plataformas.is_empty() {
        out.push("plataformas".to_string());
    }
    let area = |s: &str| s.trim().is_empty();
    // §1.1 — idea general: uso previsto y público objetivo también son gaps.
    if area(&a.uso_previsto) {
        out.push("uso_previsto".to_string());
    }
    if area(&a.publico_objetivo) {
        out.push("publico_objetivo".to_string());
    }
    match a.cat {
        Categoria::Aplicacion => {
            if area(&a.ui_ux) {
                out.push("ui_ux".to_string());
            }
        }
        Categoria::Servicio => {
            if area(&a.endpoints) {
                out.push("endpoints".to_string());
            }
            if area(&a.escala) {
                out.push("escala/SLA".to_string());
            }
        }
        Categoria::Libreria => {
            if area(&a.api_publica) {
                out.push("api_publica".to_string());
            }
            if a.sys == super::levels::SysType::PluginExtension && area(&a.host_api) {
                out.push("host_api".to_string());
            }
            if a.sys == super::levels::SysType::MotorEngine && area(&a.ui_ux) {
                out.push("ui_ux del editor".to_string());
            }
            if area(&a.ejemplos) {
                out.push("ejemplos/docs".to_string());
            }
        }
        Categoria::Sistema => {
            if a.sys == super::levels::SysType::LenguajeRuntime {
                if area(&a.sintaxis) {
                    out.push("sintaxis/tipos".to_string());
                }
                if area(&a.toolchain) {
                    out.push("toolchain".to_string());
                }
                if a.archs.is_empty() {
                    out.push("arch target".to_string());
                }
                if area(&a.compat) {
                    out.push("compat".to_string());
                }
            } else {
                if a.archs.is_empty() {
                    out.push("arch target".to_string());
                }
                if area(&a.arranque) {
                    out.push("arranque/hw".to_string());
                }
                if matches!(
                    a.sys,
                    super::levels::SysType::OsDistro
                        | super::levels::SysType::KernelModulo
                        | super::levels::SysType::ContenedorVm
                ) && area(&a.syscalls)
                {
                    out.push("syscalls/ABI".to_string());
                }
                if area(&a.compat) {
                    out.push("compat".to_string());
                }
            }
        }
        Categoria::Automatizacion => {
            if area(&a.inputs_secretos) {
                out.push("inputs/secretos".to_string());
            }
            if area(&a.idempotencia) {
                out.push("idempotencia/retry".to_string());
            }
        }
        Categoria::DatosIa => {
            if area(&a.dataset) {
                out.push("dataset/fuente".to_string());
            }
            if area(&a.pipeline_desc) {
                out.push("pipeline".to_string());
            }
            if area(&a.modelo_eval) {
                out.push("modelo/eval".to_string());
            }
        }
    }
    if a.funcionalidades.trim().is_empty() {
        out.push("funcionalidades".to_string());
    }
    // Bloque OSS: si se publica como abierto y faltan repo/gobierno/contrib.
    if super::levels::is_oss(a.cat, a.facturacion, a.licencia) {
        if area(&a.oss_repo) {
            out.push("oss_repo".to_string());
        }
        if area(&a.oss_gobierno) {
            out.push("oss_gobierno".to_string());
        }
        if area(&a.oss_contrib) {
            out.push("oss_contrib".to_string());
        }
    }
    out
}

/// System del turno de planificación: convenciones de la casa + resumen
/// del wizard + gaps explícitos. El turno es 1 solo Plan (sin lecturas:
/// todo viaja en el prompt) y NO toca `Project/` (planificar ≠ programar).
pub fn plan_prompt(level: Level, a: &Answers, ai: &[(String, String)], source: &QSource) -> String {
    let mut s = String::from(
        "Eres el planificador de ARQHIA. Acaba de completarse el cuestionario: \
        genera el juego de contexto del proyecto nuevo.\n\n\
        CONVENCIONES DE LA CASA (obligatorias):\n\
        - Layout: `Project/` (código + git, NO tocar en este turno), `CONTEXT/` \
        (docs: PROJECT.md, SPECS.md, CONTEXT.md, ROADMAP.md, VERSIONS.md, VERSIONS/v0.1.md), \
        `ToDo.md` (lo mantiene el agente).\n\
        - Cada versión sigue la plantilla de 8 secciones: Objetivo, Alcance (concreto), \
        Paso a paso (numerado, ejecutable), Tareas técnicas (archivos/tablas exactas), \
        Criterio Done (estricto, verificable), Test de Funcionalidad, Riesgos, Dependencias.\n\
        - Idioma: docs en español, código y commits en inglés. Versionado `v0.x` \
        (un .md por versión + índice).\n\
        - Prohibido cerrar con placeholders mudos del estilo de «se define al \
        planificar»: lo vacío se marca como gap explícito `Pendiente de definir: X` \
        o se propone un default razonado marcado como tal.\n\n",
    );
    s.push_str(&format!(
        "PROYECTO (nivel {}, origen {}):\n- Nombre: {}\n- Descripción: {}\n- Objetivo: {}\n- Tipo: {} — {}\n",
        level,
        match source {
            QSource::New => "desde cero",
            QSource::Import => "importado de código existente",
        },
        none_dash(&a.nombre),
        none_dash(&a.descripcion),
        none_dash(&a.objetivo),
        a.cat,
        a.sys,
    ));
    for (k, v) in a.level_extras(level) {
        let val = if v.trim().is_empty() || v == "(sin especificar)" {
            "Pendiente de definir (ver gaps abajo)".to_string()
        } else {
            v
        };
        s.push_str(&format!("- {k}: {val}\n"));
    }
    // Stack y plataformas siempre explícitos (hay familias que no los
    // preguntan: Servicio pre-marca Servidor, Principiante omite stack).
    // Se dice la fuente: respuesta real, default pre-marcado o gap.
    s.push_str(&format!("- Stack: {}\n", stack_prompt_line(a)));
    s.push_str(&format!("- Plataformas: {}\n", plataform_prompt_line(a)));
    let funcs: Vec<String> = a
        .funcionalidades
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_start_matches(['-', '*', '•']).trim().to_string())
        .collect();
    if funcs.is_empty() {
        s.push_str("- Funcionalidades: Pendiente de definir: funcionalidades\n");
    } else {
        s.push_str(&format!("- Funcionalidades: {}\n", funcs.join(" | ")));
    }
    if !ai.is_empty() {
        s.push_str("\nRespuestas IA del cuestionario:\n");
        for (q, r) in ai {
            s.push_str(&format!(
                "- {q}: {}\n",
                if r.trim().is_empty() {
                    "(sin responder)".to_string()
                } else {
                    r.trim().to_string()
                }
            ));
        }
    }
    let g = gaps(a);
    if !g.is_empty() {
        s.push_str(&format!("\nGAPS a resolver en v0.1: {}.\n", g.join(", ")));
    }
    s.push_str(
        "\nESCRIBE, en este orden, `CONTEXT/ROADMAP.md`, `CONTEXT/VERSIONS.md`, \
        `CONTEXT/VERSIONS/v0.1.md` y siembra `ToDo.md` con la lista inicial derivada \
        de v0.1. `VERSIONS/v0.1.md` trae datos reales del wizard (stack, plataformas, \
        familia) con pasos numerados y Done verificable. Al cerrar, resume archivos + \
        gaps pendientes en el chat.",
    );
    s
}

fn none_dash(s: &str) -> String {
    if s.trim().is_empty() {
        "(sin detallar)".to_string()
    } else {
        s.trim().to_string()
    }
}

/// Exige la plantilla de la casa: las 8 cabeceras `## ...`, al menos 3
/// pasos numerados y Criterio Done con contenido. Devuelve lo que falta.
pub fn validate_version_md(text: &str) -> Result<(), Vec<String>> {
    let mut missing = Vec::new();
    for sec in house_template() {
        let want = format!("## {sec}");
        if !text.lines().any(|l| l.trim() == want) {
            missing.push(format!("falta sección {want}"));
        }
    }
    let steps = text
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.len() > 3
                && t.chars().next().is_some_and(|c| c.is_ascii_digit())
                && t.chars().nth(1).is_some_and(|c| c == '.' || c == ')')
        })
        .count();
    if steps < 3 {
        missing.push(format!("pasos numerados insuficientes ({steps}, mínimo 3)"));
    }
    if let Some(done_pos) = text.find("## Criterio Done") {
        let tail = &text[done_pos + "## Criterio Done".len()..];
        let end = tail.find("\n## ").unwrap_or(tail.len());
        if tail[..end].trim().chars().count() < 30 {
            missing.push("Criterio Done vacío o testimonial".to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// Contexto del turno auto (Finish → Plan): todo lo necesario para generar
/// los 4 archivos sin más clics. Idempotente: re-ejecutar sobrescribe.
#[derive(Debug, Clone)]
pub struct AutoPlanCtx {
    pub ws: std::path::PathBuf,
    pub level: Level,
    pub answers: Answers,
    pub ai: Vec<(String, String)>,
    pub source: QSource,
}

/// Informe de lo escrito por `write_auto_docs`.
#[derive(Debug, Clone)]
pub struct AutoDocsReport {
    pub files: Vec<String>,
    pub gaps: Vec<String>,
    pub v01_valid: bool,
    pub v01_warnings: Vec<String>,
}

fn stack_line(a: &Answers) -> String {
    let mut base = if a.stacks.is_empty() {
        "A decidir (default propuesto: el más simple que cumpla)".to_string()
    } else {
        a.stacks
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !a.stack_free.trim().is_empty() {
        base.push_str(&format!(" ({})", a.stack_free.trim()));
    }
    base
}

fn plataform_line(a: &Answers) -> String {
    if a.plataformas.is_empty() {
        "Pendiente de definir: plataformas".to_string()
    } else {
        a.plataformas
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Línea de stack para el prompt: respuesta real o default/gap marcado.
fn stack_prompt_line(a: &Answers) -> String {
    if a.stacks.is_empty() && a.stack_free.trim().is_empty() {
        "Pendiente de definir: stack".to_string()
    } else {
        stack_line(a)
    }
}

/// Línea de plataformas para el prompt: respuesta, default pre-marcado
/// (Servicio corre en Servidor/Nube) o gap explícito.
fn plataform_prompt_line(a: &Answers) -> String {
    if !a.plataformas.is_empty() {
        return a
            .plataformas
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
    }
    if a.cat == Categoria::Servicio {
        return "Servidor/Nube (default pre-marcado, editable)".to_string();
    }
    "Pendiente de definir: plataformas".to_string()
}

fn func_list(a: &Answers) -> Vec<String> {
    a.funcionalidades
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_start_matches(['-', '*', '•']).trim().to_string())
        .collect()
}

/// Genera los 4 archivos del set completo desde respuestas + tareas del
/// planner. `tasks` son las descripciones del plan (pueden venir vacías:
/// entonces se derivan de las funcionalidades).
pub fn write_auto_docs(ctx: &AutoPlanCtx, tasks: &[String]) -> Result<AutoDocsReport, String> {
    let a = &ctx.answers;
    let gaps = gaps(a);
    let funcs = func_list(a);
    let steps: Vec<String> = if tasks.is_empty() {
        funcs.iter().map(|f| format!("Implementar: {f}")).collect()
    } else {
        tasks.to_vec()
    };
    let steps = if steps.is_empty() {
        vec!["Definir el alcance mínimo con el usuario".to_string()]
    } else {
        steps
    };
    let roadmap = render_roadmap(ctx, &gaps);
    let versions = render_versions_index();
    let v01 = render_v01(ctx, &steps, &gaps);
    // 1 reintento si el v0.1 saliera sin plantilla (aquí siempre pasa por
    // construcción; el reintento regenera idéntico y avisa si sigue mal).
    let mut v01_warnings = Vec::new();
    let v01_valid = match validate_version_md(&v01) {
        Ok(()) => true,
        Err(first) => match validate_version_md(&render_v01(ctx, &steps, &gaps)) {
            Ok(()) => {
                v01_warnings.push(format!("v0.1 regenerado tras fallos: {}", first.join("; ")));
                true
            }
            Err(second) => {
                v01_warnings.push(format!("v0.1 sin plantilla: {}", second.join("; ")));
                false
            }
        },
    };
    let todo = render_todo(ctx, &steps);
    let dir = ctx.ws.join("CONTEXT");
    std::fs::create_dir_all(dir.join("VERSIONS")).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("ROADMAP.md"), &roadmap).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("VERSIONS.md"), &versions).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("VERSIONS").join("v0.1.md"), &v01).map_err(|e| e.to_string())?;
    std::fs::write(ctx.ws.join("ToDo.md"), &todo).map_err(|e| e.to_string())?;
    Ok(AutoDocsReport {
        files: vec![
            "CONTEXT/ROADMAP.md".to_string(),
            "CONTEXT/VERSIONS.md".to_string(),
            "CONTEXT/VERSIONS/v0.1.md".to_string(),
            "ToDo.md".to_string(),
        ],
        gaps,
        v01_valid,
        v01_warnings,
    })
}

fn render_roadmap(ctx: &AutoPlanCtx, gaps: &[String]) -> String {
    let a = &ctx.answers;
    let origen = match ctx.source {
        QSource::New => "desde cero",
        QSource::Import => "importado de código existente",
    };
    let mut s = format!(
        "# {} — ROADMAP\n\nGenerado automáticamente por ARQHIA (v0.9 Track D) \
        desde el cuestionario (nivel {}, {} — {}, origen {}).\n\n## v0.1 — Base funcional\n\n\
        Stack: {}. Plataformas: {}.\n",
        a.nombre.trim(),
        ctx.level,
        a.cat,
        a.sys,
        origen,
        stack_line(a),
        plataform_line(a),
    );
    if !ctx.ai.is_empty() {
        s.push_str("\nRespuestas IA del cuestionario:\n");
        for (q, r) in &ctx.ai {
            s.push_str(&format!(
                "- {q}: {}\n",
                if r.trim().is_empty() {
                    "(sin responder)".to_string()
                } else {
                    r.trim().to_string()
                }
            ));
        }
    }
    if gaps.is_empty() {
        s.push_str("\nSin gaps: el cuestionario cubrió lo necesario.\n");
    } else {
        s.push_str(&format!(
            "\nGaps a resolver en v0.1: {}.\n",
            gaps.join(", ")
        ));
    }
    s.push_str("\n## Siguientes versiones\n\nSe definen al cerrar v0.1 (un .md por versión en `VERSIONS/`).\n");
    s
}

fn render_versions_index() -> String {
    "# VERSIONS\n\nÍndice de estado por versión.\n\n| Versión | Nombre | Estado |\n|---|---|---|\n| v0.1 | Base funcional | 🟡 En curso |\n".to_string()
}

fn render_v01(ctx: &AutoPlanCtx, steps: &[String], gaps: &[String]) -> String {
    let a = &ctx.answers;
    let funcs = func_list(a);
    let funcs_md = if funcs.is_empty() {
        "- Pendiente de definir: funcionalidades".to_string()
    } else {
        funcs
            .iter()
            .map(|f| format!("- {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let numbered = steps
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# v0.1 — Base funcional de {}\n\n\
        ## Objetivo\n\n\
        Levantar la base funcional de {} ({} — {}) con {} en {}.\n\n\
        ## Alcance\n\n\
        Funcionalidades:\n{funcs_md}\n\n\
        Stack: {}. Plataformas: {}. Gaps: {}.\n\n\
        ## Paso a paso\n\n\
        {numbered}\n\n\
        ## Tareas técnicas\n\n\
        - Código en `Project/` (archivos exactos según el stack {}).\n\
        - Docs vivos en `CONTEXT/` (este juego se generó en el post-cuestionario).\n\
        - `ToDo.md` con la lista inicial (la mantiene el agente).\n\n\
        ## Criterio Done\n\n\
        - Cada funcionalidad responde end-to-end sin errores en el caso feliz.\n\
        - Los gaps listados están resueltos o marcados como `Pendiente` con dueño.\n\
        - `cargo check`/`test`/`clippy` (o equivalente del stack) en verde.\n\n\
        ## Test de Funcionalidad\n\n\
        1. Recorrer cada funcionalidad y verificar su criterio de aceptación en SPECS.md.\n\
        2. Probar el caso de error principal de cada flujo (mensaje visible, sin pánicos).\n\
        3. Revisar que `ToDo.md` refleja lo hecho.\n\n\
        ## Riesgos\n\n\
        - Stack sin decidir del todo: fijarlo antes de programar.\n\
        - Gaps abiertos ({n_gaps}): resolverlos en el primer turno Work.\n\n\
        ## Dependencias\n\n\
        - Cuestionario completado (PROJECT.md + SPECS.md + CONTEXT.md).\n\
        - `Project/` vacío al inicio; el código nace en v0.1.\n",
        a.nombre.trim(),
        a.nombre.trim(),
        a.cat,
        a.sys,
        stack_line(a),
        plataform_line(a),
        stack_line(a),
        plataform_line(a),
        gaps.join(", "),
        stack_line(a),
        n_gaps = gaps.len(),
    )
}

fn render_todo(ctx: &AutoPlanCtx, steps: &[String]) -> String {
    let mut s = format!(
        "# ToDo — {}\n\nLista inicial derivada de v0.1 (la mantiene el agente).\n\n",
        ctx.answers.nombre.trim()
    );
    for t in steps {
        s.push_str(&format!("- [ ] {t}\n"));
    }
    let gaps = gaps(&ctx.answers);
    if !gaps.is_empty() {
        s.push_str("\n## Gaps\n\n");
        for g in gaps {
            s.push_str(&format!("- [ ] Pendiente de definir: {g}\n"));
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Serie MVP v0.1 → v1.0 (v0.8.2, pantalla de carga sin chat)
// ---------------------------------------------------------------------------

/// Serie fija hasta el MVP: base → núcleo → integración → MVP.
pub fn mvp_versions() -> [(&'static str, &'static str); 4] {
    [
        ("v0.1", "Base funcional"),
        ("v0.2", "Núcleo usable"),
        ("v0.3", "Integración y calidad"),
        ("v1.0", "MVP"),
    ]
}

/// Rutas que debe devolver el modelo, en orden.
pub fn mvp_files() -> [&'static str; 7] {
    [
        "CONTEXT/ROADMAP.md",
        "CONTEXT/VERSIONS.md",
        "CONTEXT/VERSIONS/v0.1.md",
        "CONTEXT/VERSIONS/v0.2.md",
        "CONTEXT/VERSIONS/v0.3.md",
        "CONTEXT/VERSIONS/v1.0.md",
        "ToDo.md",
    ]
}

/// System prompt especial de la pantalla de carga: convenciones de la casa,
/// resumen del wizard, gaps y serie MVP. El modelo devuelve los 7 archivos
/// delimitados por `---FILE: <ruta>---` (ver `parse_mvp_files`).
pub fn mvp_prompt(level: Level, a: &Answers, ai: &[(String, String)], source: &QSource) -> String {
    let mut s = plan_prompt(level, a, ai, source);
    s.push_str(
        "\n\nSERIE MVP (obligatoria, sin pasar por el chat):\n\
        - Genera la serie completa v0.1 (Base funcional) → v0.2 (Núcleo usable) \
        → v0.3 (Integración y calidad) → v1.0 (MVP). Cada versión con la \
        plantilla de 8 secciones y datos reales del wizard (stack, plataformas, \
        familia); v1.0 cierra el MVP con Done verificable end-to-end.\n\
        - Reparte las funcionalidades: v0.1 la base mínima, v0.2 el núcleo, \
        v0.3 integración/calidad, v1.0 el cierre MVP. Lo vacío del wizard va \
        como `Pendiente de definir: X` en la primera versión que lo necesite.\n\
        - FORMATO DE SALIDA (estricto): devuelve EXACTAMENTE estos 7 bloques, \
        en este orden, cada uno precedido por una línea `---FILE: <ruta>---` \
        y sin nada fuera de los bloques:\n",
    );
    for f in mvp_files() {
        s.push_str(&format!("---FILE: {f}---\n"));
    }
    s
}

/// Trocea la respuesta del modelo en `(ruta, contenido)` por marcadores
/// `---FILE: <ruta>---`. Ignora texto fuera de bloques y rutas desconocidas.
pub fn parse_mvp_files(text: &str) -> Vec<(String, String)> {
    let known = mvp_files();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut cur: Option<String> = None;
    let mut buf = String::new();
    let flush = |cur: &mut Option<String>, buf: &mut String, out: &mut Vec<(String, String)>| {
        if let Some(path) = cur.take() {
            out.push((path, buf.trim().to_string()));
        }
        buf.clear();
    };
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("---FILE:") && t.ends_with("---") {
            let inner = t
                .trim_start_matches("---FILE:")
                .trim_end_matches("---")
                .trim();
            flush(&mut cur, &mut buf, &mut out);
            if known.contains(&inner) && !out.iter().any(|(p, _)| p == inner) {
                cur = Some(inner.to_string());
            } else {
                cur = None;
            }
            continue;
        }
        if cur.is_some() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(&mut cur, &mut buf, &mut out);
    out.retain(|(p, c)| known.contains(&p.as_str()) && !c.trim().is_empty());
    out
}

/// Informe de la escritura MVP.
#[derive(Debug, Clone)]
pub struct MvpReport {
    pub files: Vec<String>,
    pub gaps: Vec<String>,
    pub warnings: Vec<String>,
}

/// Escribe la serie MVP validando cada versión (avisa pero no bloquea).
/// Faltantes o inválidos se regeneran con el fallback determinista.
pub fn write_mvp_docs(ctx: &AutoPlanCtx, files: &[(String, String)]) -> Result<MvpReport, String> {
    let gaps = gaps(&ctx.answers);
    let mut map: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (p, c) in files {
        if mvp_files().contains(&p.as_str()) {
            map.entry(p.as_str()).or_insert(c.as_str());
        }
    }
    let fallback = deterministic_mvp(ctx);
    let mut warnings = Vec::new();
    let mut final_files: Vec<(String, String)> = Vec::new();
    for (path, fb) in &fallback {
        match map.get(path.as_str()) {
            Some(content) if !content.trim().is_empty() => {
                if path.starts_with("CONTEXT/VERSIONS/v") {
                    if let Err(missing) = validate_version_md(content) {
                        warnings.push(format!(
                            "{path} sin plantilla (fallback): {}",
                            missing.join("; ")
                        ));
                        final_files.push((path.clone(), fb.clone()));
                    } else {
                        final_files.push((path.clone(), (*content).to_string()));
                    }
                } else {
                    final_files.push((path.clone(), (*content).to_string()));
                }
            }
            _ => {
                warnings.push(format!("{path} ausente en la IA (fallback determinista)"));
                final_files.push((path.clone(), fb.clone()));
            }
        }
    }
    let dir = ctx.ws.join("CONTEXT");
    std::fs::create_dir_all(dir.join("VERSIONS")).map_err(|e| e.to_string())?;
    for (path, content) in &final_files {
        let full = ctx.ws.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&full, content).map_err(|e| e.to_string())?;
    }
    Ok(MvpReport {
        files: final_files.iter().map(|(p, _)| p.clone()).collect(),
        gaps,
        warnings,
    })
}

/// Fallback determinista offline: reparte funcionalidades en la serie.
pub fn deterministic_mvp(ctx: &AutoPlanCtx) -> Vec<(String, String)> {
    let funcs = func_list(&ctx.answers);
    let (v01, v02): (Vec<String>, Vec<String>) = if funcs.len() <= 3 {
        (funcs.clone(), Vec::new())
    } else {
        let mid = funcs.len().div_ceil(2);
        (funcs[..mid].to_vec(), funcs[mid..].to_vec())
    };
    let gaps = gaps(&ctx.answers);
    let roadmap = render_roadmap_mvp(ctx, &gaps);
    let versions = render_versions_index_mvp();
    let v01_md = render_version_mvp(
        ctx,
        "v0.1",
        "Base funcional",
        &v01,
        &gaps,
        "ninguna (primera)",
    );
    let v02_md = render_version_mvp(ctx, "v0.2", "Núcleo usable", &v02, &gaps, "v0.1");
    let v03_md = render_version_mvp(ctx, "v0.3", "Integración y calidad", &[], &gaps, "v0.2");
    let v10_md = render_version_mvp(ctx, "v1.0", "MVP", &funcs, &gaps, "v0.3");
    let todo = render_todo_mvp(ctx, &funcs);
    vec![
        ("CONTEXT/ROADMAP.md".to_string(), roadmap),
        ("CONTEXT/VERSIONS.md".to_string(), versions),
        ("CONTEXT/VERSIONS/v0.1.md".to_string(), v01_md),
        ("CONTEXT/VERSIONS/v0.2.md".to_string(), v02_md),
        ("CONTEXT/VERSIONS/v0.3.md".to_string(), v03_md),
        ("CONTEXT/VERSIONS/v1.0.md".to_string(), v10_md),
        ("ToDo.md".to_string(), todo),
    ]
}

fn render_roadmap_mvp(ctx: &AutoPlanCtx, gaps: &[String]) -> String {
    let a = &ctx.answers;
    let origen = match ctx.source {
        QSource::New => "desde cero",
        QSource::Import => "importado de código existente",
    };
    let mut s = format!(
        "# {} — ROADMAP\n\nGenerado automáticamente por ARQHIA (pantalla de carga MVP) \
        desde el cuestionario (nivel {}, {} — {}, origen {}).\n\n",
        a.nombre.trim(),
        ctx.level,
        a.cat,
        a.sys,
        origen,
    );
    s.push_str(&format!(
        "Stack: {}. Plataformas: {}.\n\n",
        stack_line(a),
        plataform_line(a)
    ));
    s.push_str("## Serie hasta el MVP\n\n");
    for (ver, title) in mvp_versions() {
        s.push_str(&format!("- `{ver}` — {title}\n"));
    }
    if !ctx.ai.is_empty() {
        s.push_str("\nRespuestas IA del cuestionario:\n");
        for (q, r) in &ctx.ai {
            s.push_str(&format!(
                "- {q}: {}\n",
                if r.trim().is_empty() {
                    "(sin responder)".to_string()
                } else {
                    r.trim().to_string()
                }
            ));
        }
    }
    if gaps.is_empty() {
        s.push_str("\nSin gaps: el cuestionario cubrió lo necesario.\n");
    } else {
        s.push_str(&format!(
            "\nGaps a resolver desde v0.1: {}.\n",
            gaps.join(", ")
        ));
    }
    s
}

fn render_versions_index_mvp() -> String {
    let mut s = "# VERSIONS\n\nÍndice de estado por versión (serie MVP).\n\n| Versión | Nombre | Estado |\n|---|---|---|\n".to_string();
    for (ver, title) in mvp_versions() {
        s.push_str(&format!("| {ver} | {title} | 🟡 En curso |\n"));
    }
    s
}

fn render_version_mvp(
    ctx: &AutoPlanCtx,
    ver: &str,
    title: &str,
    funcs: &[String],
    gaps: &[String],
    depends: &str,
) -> String {
    let a = &ctx.answers;
    let scope = if funcs.is_empty() {
        match ver {
            "v0.3" => "- Integración end-to-end de lo construido en v0.1–v0.2.\n- Calidad: errores visibles, sin pánicos, criterio de SPECS.md por flujo.".to_string(),
            "v1.0" => func_list(a).iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n"),
            _ => "- Pendiente de definir: funcionalidades (ver gaps).".to_string(),
        }
    } else {
        funcs
            .iter()
            .map(|f| format!("- {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let steps = if funcs.is_empty() && ver != "v1.0" {
        match ver {
            "v0.2" => vec![
                "Completar los gaps heredados de v0.1 con defaults razonados.".to_string(),
                "Dejar el núcleo instalable/ejecutable en el entorno objetivo.".to_string(),
                "Registrar decisiones en CONTEXT.md.".to_string(),
            ],
            _ => vec![
                "Integrar v0.1 + v0.2 end-to-end.".to_string(),
                "Cubrir el caso de error principal de cada flujo.".to_string(),
                "Pasar la puerta de calidad del stack.".to_string(),
            ],
        }
    } else {
        funcs
            .iter()
            .map(|f| format!("Implementar: {f}"))
            .collect::<Vec<_>>()
    };
    let numbered = steps
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");
    let gaps_line = if gaps.is_empty() {
        "ninguno".to_string()
    } else {
        gaps.join(", ")
    };
    format!(
        "# {ver} — {title} de {}\n\n\
        ## Objetivo\n\n\
        {title} de {} ({} — {}) con {} en {}.\n\n\
        ## Alcance\n\n\
        Funcionalidades de esta versión:\n{scope}\n\n\
        Stack: {}. Plataformas: {}. Gaps: {}.\n\n\
        ## Paso a paso\n\n\
        {numbered}\n\n\
        ## Tareas técnicas\n\n\
        - Código en `Project/` (archivos exactos según el stack {}).\n\
        - Docs vivos en `CONTEXT/` (serie MVP generada en la pantalla de carga).\n\
        - `ToDo.md` con la lista inicial (la mantiene el agente).\n\n\
        ## Criterio Done\n\n\
        - Cada funcionalidad de esta versión responde end-to-end sin errores en el caso feliz.\n\
        - Los gaps listados están resueltos o marcados como `Pendiente` con dueño.\n\
        - Puerta de calidad del stack en verde (check/test/lint o equivalente).\n\n\
        ## Test de Funcionalidad\n\n\
        1. Recorrer cada funcionalidad y verificar su criterio de aceptación en SPECS.md.\n\
        2. Probar el caso de error principal de cada flujo (mensaje visible, sin pánicos).\n\
        3. Revisar que `ToDo.md` refleja lo hecho.\n\n\
        ## Riesgos\n\n\
        - Stack sin decidir del todo: fijarlo antes de programar.\n\
        - Gaps abiertos ({n_gaps}): resolverlos en el primer turno Work.\n\n\
        ## Dependencias\n\n\
        - {depends}.\n\
        - Cuestionario completado (PROJECT.md + SPECS.md + CONTEXT.md).\n",
        a.nombre.trim(),
        a.nombre.trim(),
        a.cat,
        a.sys,
        stack_line(a),
        plataform_line(a),
        stack_line(a),
        plataform_line(a),
        gaps_line,
        stack_line(a),
        n_gaps = gaps.len(),
        depends = depends,
    )
}

fn render_todo_mvp(ctx: &AutoPlanCtx, funcs: &[String]) -> String {
    let mut s = format!(
        "# ToDo — {}\n\nSerie MVP v0.1 → v1.0 (la mantiene el agente).\n\n",
        ctx.answers.nombre.trim()
    );
    for (ver, title) in mvp_versions() {
        s.push_str(&format!("## {ver} — {title}\n\n"));
        match ver {
            "v0.1" | "v0.2" => {
                let chunk: Vec<&String> = if ver == "v0.1" {
                    funcs.iter().take(3).collect()
                } else {
                    funcs.iter().skip(3).collect()
                };
                if chunk.is_empty() {
                    s.push_str("- [ ] Resolver gaps heredados y dejar base compilable\n");
                } else {
                    for f in chunk {
                        s.push_str(&format!("- [ ] {ver}: {f}\n"));
                    }
                }
            }
            "v0.3" => s.push_str("- [ ] Integración end-to-end + caso de error por flujo\n"),
            "v1.0" => {
                s.push_str("- [ ] Cierre MVP: Done verificable de todas las funcionalidades\n")
            }
            _ => {}
        }
        s.push('\n');
    }
    let gaps = gaps(&ctx.answers);
    if !gaps.is_empty() {
        s.push_str("## Gaps\n\n");
        for g in gaps {
            s.push_str(&format!("- [ ] Pendiente de definir: {g}\n"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::questionnaire::levels::{Plataforma, StackOpt, SysType};

    fn sample_api() -> Answers {
        Answers {
            nombre: "MiAPI".to_string(),
            descripcion: "API de ejemplo para tests".to_string(),
            objetivo: "Servir datos por HTTP".to_string(),
            funcionalidades: "Listar usuarios\nCrear usuario".to_string(),
            cat: Categoria::Servicio,
            sys: SysType::ApiBackend,
            endpoints: "GET /users, POST /users".to_string(),
            stacks: vec![StackOpt::Rust],
            ..Default::default()
        }
    }

    #[test]
    fn prompt_carries_house_rules_and_gaps() {
        let a = sample_api();
        let p = plan_prompt(Level::Intermedio, &a, &[], &QSource::New);
        assert!(p.contains("CONVENCIONES DE LA CASA"));
        assert!(p.contains("Rust") && p.contains("Servidor"));
        assert!(p.contains("GET /users"));
        assert!(p.contains("escala/SLA"), "gap explícito en GAPS: {p}");
        assert!(!p.contains("(se define al planificar)"));
        let ai = vec![("¿Puerto?".to_string(), "8080".to_string())];
        let p2 = plan_prompt(Level::Intermedio, &a, &ai, &QSource::Import);
        assert!(p2.contains("importado") && p2.contains("8080"));
    }

    #[test]
    fn validate_detects_template_and_steps() {
        let ctx = AutoPlanCtx {
            ws: std::env::temp_dir(),
            level: Level::Intermedio,
            answers: sample_api(),
            ai: Vec::new(),
            source: QSource::New,
        };
        let v01 = render_v01(
            &ctx,
            &["Uno".to_string(), "Dos".to_string(), "Tres".to_string()],
            &[],
        );
        assert!(validate_version_md(&v01).is_ok(), "el v0.1 generado pasa");
        let missing = validate_version_md("# vacío\n").unwrap_err();
        assert!(missing.len() >= 8, "detecta secciones: {missing:?}");
        let thin = "# v0.1\n\n## Objetivo\n\no\n\n## Alcance\n\na\n\n## Paso a paso\n\n1. uno\n\n## Tareas técnicas\n\nt\n\n## Criterio Done\n\nx\n\n## Test de Funcionalidad\n\nt\n\n## Riesgos\n\nr\n\n## Dependencias\n\nd\n";
        let err = validate_version_md(thin).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("numerados") || e.contains("Done"))
        );
    }

    #[test]
    fn auto_docs_write_all_four_files() {
        let base = std::env::temp_dir().join("arqhia-planning-test");
        let _ = std::fs::remove_dir_all(&base);
        let ws = base.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let ctx = AutoPlanCtx {
            ws: ws.clone(),
            level: Level::Intermedio,
            answers: sample_api(),
            ai: Vec::new(),
            source: QSource::New,
        };
        let report = write_auto_docs(
            &ctx,
            &[
                "Montar router".to_string(),
                "Añadir tests".to_string(),
                "Documentar".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(report.files.len(), 4);
        assert!(report.v01_valid && report.v01_warnings.is_empty());
        assert!(report.gaps.iter().any(|g| g.contains("escala")));
        for f in ["ROADMAP.md", "VERSIONS.md"] {
            assert!(ws.join("CONTEXT").join(f).is_file(), "{f} existe");
        }
        let v01 =
            std::fs::read_to_string(ws.join("CONTEXT").join("VERSIONS").join("v0.1.md")).unwrap();
        assert!(validate_version_md(&v01).is_ok());
        assert!(v01.contains("Rust") && v01.contains("Montar router"));
        let todo = std::fs::read_to_string(ws.join("ToDo.md")).unwrap();
        assert!(
            todo.contains("Montar router") && todo.contains("Pendiente de definir: escala/SLA")
        );
        // Idempotente: re-ejecutar sobrescribe sin duplicar.
        let report2 = write_auto_docs(
            &ctx,
            &[
                "Montar router".to_string(),
                "Añadir tests".to_string(),
                "Documentar".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(report2.files, report.files);
        let todo2 = std::fs::read_to_string(ws.join("ToDo.md")).unwrap();
        assert_eq!(
            todo.matches("Montar router").count(),
            todo2.matches("Montar router").count()
        );
        // Sin tareas del planner: deriva de funcionalidades.
        let report3 = write_auto_docs(&ctx, &[]).unwrap();
        assert!(report3.v01_valid);
        let v013 =
            std::fs::read_to_string(ws.join("CONTEXT").join("VERSIONS").join("v0.1.md")).unwrap();
        assert!(v013.contains("Listar usuarios"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_wizard_marks_gaps_not_placeholders() {
        let mut a = sample_api();
        a.stacks.clear();
        a.stack_free.clear();
        a.plataformas.clear();
        let g = gaps(&a);
        assert!(g.contains(&"stack".to_string()) && g.contains(&"plataformas".to_string()));
        let p = plan_prompt(Level::Principiante, &a, &[], &QSource::New);
        assert!(
            p.contains("Pendiente de definir: stack")
                || p.contains("Pendiente de definir: plataformas")
        );
        assert!(!p.contains("(se define al planificar)"));
        // Plataformas es Vec<Plataforma>: existe el tipo (evita regresión de API).
        let _ = Plataforma::Web;
    }

    #[test]
    fn mvp_prompt_lists_all_seven_files() {
        let p = mvp_prompt(Level::Intermedio, &sample_api(), &[], &QSource::New);
        assert!(p.contains("SERIE MVP") && p.contains("---FILE: ToDo.md---"));
        for f in mvp_files() {
            assert!(p.contains(f), "falta {f}");
        }
        assert!(p.contains("v1.0") && p.contains("MVP"));
    }

    #[test]
    fn parse_mvp_splits_markers_and_ignores_unknown() {
        let raw = "texto previo\n---FILE: CONTEXT/ROADMAP.md---\n# R\n---FILE: NOPE.md---\nx\n---FILE: ToDo.md---\n- [ ] a\n";
        let files = parse_mvp_files(raw);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "CONTEXT/ROADMAP.md");
        assert_eq!(files[1].0, "ToDo.md");
        assert!(files[1].1.contains("- [ ] a"));
    }

    #[test]
    fn deterministic_mvp_writes_series_with_valid_versions() {
        let base = std::env::temp_dir().join("arqhia-mvp-test");
        let _ = std::fs::remove_dir_all(&base);
        let ws = base.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let ctx = AutoPlanCtx {
            ws: ws.clone(),
            level: Level::Intermedio,
            answers: sample_api(),
            ai: Vec::new(),
            source: QSource::New,
        };
        // Fallback puro: sin archivos de la IA, todo determinista.
        let report = write_mvp_docs(&ctx, &[]).unwrap();
        assert_eq!(report.files.len(), 7);
        assert!(
            report.warnings.len() == 7,
            "todo ausente avisa: {:?}",
            report.warnings
        );
        for ver in ["v0.1", "v0.2", "v0.3", "v1.0"] {
            let md = std::fs::read_to_string(
                ws.join("CONTEXT")
                    .join("VERSIONS")
                    .join(format!("{ver}.md")),
            )
            .unwrap();
            assert!(validate_version_md(&md).is_ok(), "{ver} válido");
        }
        let idx = std::fs::read_to_string(ws.join("CONTEXT").join("VERSIONS.md")).unwrap();
        assert!(idx.contains("v1.0") && idx.contains("MVP"));
        let todo = std::fs::read_to_string(ws.join("ToDo.md")).unwrap();
        assert!(todo.contains("v0.1") && todo.contains("v1.0"));
        // Con archivos parciales de la IA: se conserva el bueno, el malo cae a fallback.
        let good_v01 = render_version_mvp(
            &ctx,
            "v0.1",
            "Base funcional",
            &["X".to_string()],
            &[],
            "ninguna (primera)",
        );
        let files = vec![
            ("CONTEXT/VERSIONS/v0.1.md".to_string(), good_v01),
            (
                "CONTEXT/VERSIONS/v0.2.md".to_string(),
                "basura sin plantilla".to_string(),
            ),
        ];
        let report2 = write_mvp_docs(&ctx, &files).unwrap();
        let v01 =
            std::fs::read_to_string(ws.join("CONTEXT").join("VERSIONS").join("v0.1.md")).unwrap();
        assert!(v01.contains('X'), "el bueno de la IA se conserva");
        assert!(report2.warnings.iter().any(|w| w.contains("v0.2")));
        let _ = std::fs::remove_dir_all(&base);
    }
}
