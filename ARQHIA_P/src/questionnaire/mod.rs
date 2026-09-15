//! Cuestionario v0.8.1: universal (Categoría+Tipo, condicional, licencias).
//!
//! Puros (sin I/O): niveles y preguntas en `levels.rs`, prompt/parseo IA en
//! `ai.rs`, import en `import.rs`, plantillas en `templates.rs`.

pub mod ai;
pub mod feature;
pub mod import;
pub mod levels;
pub mod planning;
pub mod templates;

pub use levels::{
    ApiStyle, ArchOpt, AuthKind, Categoria, EstiloPreset, Facturacion, Level, Licencia,
    Plataforma, QKind, Question, SemverOpt, StackOpt, SysType, Trigger,
};

/// Origen del cuestionario: desde cero o importado de código existente.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QSource {
    #[default]
    New,
    Import,
}

#[derive(Debug, Clone, Default)]
pub struct Answers {
    // Genéricas / básicas (siempre, §1.1: no se repreguntará el nombre).
    pub nombre: String,
    /// El nombre llegó pre-rellenado al crear/abrir el proyecto: el wizard
    /// omite la pregunta (ya se ingresó en la pantalla de inicio).
    pub nombre_locked: bool,
    /// 1.1 — Idea general del proyecto
    pub descripcion: String,
    pub objetivo: String,
    pub uso_previsto: String,
    pub publico_objetivo: String,
    pub funcionalidades: String,
    // Taxonomía (siempre, tras el nivel).
    pub cat: Categoria,
    pub sys: SysType,
    // App.
    pub estilo: EstiloPreset,
    pub estilo_free: String,
    pub ui_ux: String,
    pub plataformas: Vec<Plataforma>,
    pub stacks: Vec<StackOpt>,
    pub stack_free: String,
    pub facturacion: Facturacion,
    pub licencia: Licencia,
    // Servicio.
    pub endpoints: String,
    pub api_style: ApiStyle,
    pub auth: AuthKind,
    pub escala: String,
    // Librería.
    pub api_publica: String,
    pub semver: SemverOpt,
    pub ejemplos: String,
    // Sistema.
    pub archs: Vec<ArchOpt>,
    pub arranque: String,
    pub compat: String,
    // Automatización.
    pub trigger: Trigger,
    pub inputs_secretos: String,
    pub idempotencia: String,
    // Datos/IA.
    pub dataset: String,
    pub pipeline_desc: String,
    pub modelo_eval: String,
    // Sistema profundo: lenguaje vs OS.
    pub sintaxis: String,
    pub toolchain: String,
    pub syscalls: String,
    // Librería profunda: host donde se instala un plugin.
    pub host_api: String,
    // Bloque transversal open-source (repo + gobierno + contribución).
    pub oss_repo: String,
    pub oss_gobierno: String,
    pub oss_contrib: String,
}

impl Answers {
    /// Al cambiar de categoría: resetea el tipo al primero de la familia.
    pub fn set_cat(&mut self, cat: Categoria) {
        self.cat = cat;
        let first = SysType::for_cat(cat)[0];
        self.sys = first;
        self.plataformas = levels::default_plataformas(first);
    }

    pub fn set_sys(&mut self, sys: SysType) {
        self.sys = sys;
        self.cat = sys.categoria();
        self.plataformas = levels::default_plataformas(sys);
    }

    pub fn toggle_plataforma(&mut self, p: Plataforma) {
        if self.plataformas.contains(&p) {
            self.plataformas.retain(|x| *x != p);
        } else {
            self.plataformas.push(p);
        }
    }

    pub fn toggle_stack(&mut self, s: StackOpt) {
        if self.stacks.contains(&s) {
            self.stacks.retain(|x| *x != s);
        } else {
            self.stacks.push(s);
        }
    }

    pub fn toggle_arch(&mut self, a: ArchOpt) {
        if self.archs.contains(&a) {
            self.archs.retain(|x| *x != a);
        } else {
            self.archs.push(a);
        }
    }

    /// Pares (etiqueta, valor) del bloque básico + por familia, para prompt IA y docs.
    /// Incluye §1.1 (descripción/objetivo/tipo/uso/público/facturación/OSS) y §1.2 (familia).
    pub fn level_extras(&self, level: Level) -> Vec<(String, String)> {
        let mut out = vec![
            ("Categoría".to_string(), self.cat.to_string()),
            ("Tipo".to_string(), self.sys.to_string()),
            ("Descripción".to_string(), self.descripcion.clone()),
            ("Objetivo principal".to_string(), self.objetivo.clone()),
            ("Uso previsto".to_string(), self.uso_previsto.clone()),
            ("Público objetivo".to_string(), self.publico_objetivo.clone()),
        ];
        for q in levels::family_questions(level, self.cat, self.sys) {
            let v = match q.id {
                "ui_ux" => self.ui_ux.clone(),
                "estilo" => estilo_line(self.estilo, &self.estilo_free),
                "plataformas" | "donde" => join_display(&self.plataformas),
                "stack" => stack_line(&self.stacks, &self.stack_free),
                "facturacion" => self.facturacion.to_string(),
                "licencia" => self.licencia.to_string(),
                "endpoints" => self.endpoints.clone(),
                "api_style" => self.api_style.to_string(),
                "auth" => self.auth.to_string(),
                "escala" => self.escala.clone(),
                "api_publica" => self.api_publica.clone(),
                "semver" => self.semver.to_string(),
                "ejemplos" => self.ejemplos.clone(),
                "arch" => join_display(&self.archs),
                "arranque" => self.arranque.clone(),
                "compat" => self.compat.clone(),
                "trigger" => self.trigger.to_string(),
                "inputs" => self.inputs_secretos.clone(),
                "idempotencia" => self.idempotencia.clone(),
                "dataset" => self.dataset.clone(),
                "pipeline" => self.pipeline_desc.clone(),
                "modelo" => self.modelo_eval.clone(),
                "sintaxis" => self.sintaxis.clone(),
                "toolchain" => self.toolchain.clone(),
                "syscalls" => self.syscalls.clone(),
                "host_api" => self.host_api.clone(),
                "oss_repo" => self.oss_repo.clone(),
                "oss_gobierno" => self.oss_gobierno.clone(),
                "oss_contrib" => self.oss_contrib.clone(),
                "uso_previsto" => self.uso_previsto.clone(),
                "publico_objetivo" => self.publico_objetivo.clone(),
                _ => String::new(),
            };
            // Evita duplicar cat/tipo si la familia los repitiera.
            if q.id != "cat" && q.id != "sys" {
                out.push((q.title.to_string(), v));
            }
        }
        // Deduplica por título y filtra vacíos duplicados
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out.dedup_by(|a, b| a.0 == b.0);
        out
    }

    /// true si algún campo post-genérico tiene contenido (para avisar al
    /// cambiar de nivel: se descartarían esas respuestas).
    pub fn has_level_content(&self) -> bool {
        !self.estilo_free.trim().is_empty()
            || !self.ui_ux.trim().is_empty()
            || !self.uso_previsto.trim().is_empty()
            || !self.publico_objetivo.trim().is_empty()
            || !self.plataformas.is_empty()
            || !self.stacks.is_empty()
            || !self.stack_free.trim().is_empty()
            || !self.endpoints.trim().is_empty()
            || !self.escala.trim().is_empty()
            || !self.api_publica.trim().is_empty()
            || !self.ejemplos.trim().is_empty()
            || !self.arranque.trim().is_empty()
            || !self.compat.trim().is_empty()
            || !self.inputs_secretos.trim().is_empty()
            || !self.idempotencia.trim().is_empty()
            || !self.dataset.trim().is_empty()
            || !self.pipeline_desc.trim().is_empty()
            || !self.modelo_eval.trim().is_empty()
            || !self.archs.is_empty()
            || !self.sintaxis.trim().is_empty()
            || !self.toolchain.trim().is_empty()
            || !self.syscalls.trim().is_empty()
            || !self.host_api.trim().is_empty()
            || !self.oss_repo.trim().is_empty()
            || !self.oss_gobierno.trim().is_empty()
            || !self.oss_contrib.trim().is_empty()
    }

    /// Limpia solo el bloque post-genérico (al cambiar de nivel confirmado).
    pub fn clear_level(&mut self) {
        let locked = self.nombre_locked;
        let fresh = Self {
            nombre: self.nombre.clone(),
            nombre_locked: locked,
            descripcion: self.descripcion.clone(),
            objetivo: self.objetivo.clone(),
            uso_previsto: self.uso_previsto.clone(),
            publico_objetivo: self.publico_objetivo.clone(),
            funcionalidades: self.funcionalidades.clone(),
            ..Default::default()
        };
        *self = fresh;
    }
}

fn join_display<T: ToString>(items: &[T]) -> String {
    if items.is_empty() {
        "(sin especificar)".to_string()
    } else {
        items.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
    }
}

fn estilo_line(e: EstiloPreset, free: &str) -> String {
    if free.trim().is_empty() {
        e.to_string()
    } else {
        format!("{e} ({})", free.trim())
    }
}

fn stack_line(stacks: &[StackOpt], free: &str) -> String {
    let mut base = if stacks.is_empty() {
        "A decidir".to_string()
    } else {
        stacks.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
    };
    if !free.trim().is_empty() {
        base.push_str(&format!(" ({})", free.trim()));
    }
    base
}

/// Valida el valor de una pregunta (`Some(error)` bloquea el avance).
/// Los picks y checkboxes nunca bloquean (vacío = "sin especificar").
pub fn validate_question(q: &Question, a: &Answers) -> Option<String> {
    match q.id {
        "nombre" => {
            if a.nombre.trim().is_empty() {
                Some("Pon un nombre al proyecto.".to_string())
            } else if a.nombre.chars().count() > 60 {
                Some("Máximo 60 caracteres.".to_string())
            } else {
                None
            }
        }
        "descripcion" => {
            if a.descripcion.trim().chars().count() < 10 {
                Some("Describe el proyecto en al menos 10 caracteres.".to_string())
            } else {
                None
            }
        }
        "objetivo" => {
            if a.objetivo.trim().chars().count() < 10 {
                Some("Define el objetivo en al menos 10 caracteres.".to_string())
            } else {
                None
            }
        }
        "funcionalidades" => {
            if a.funcionalidades.trim().is_empty() {
                Some("Lista al menos una funcionalidad (una por línea).".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Valida un paso del wizard (None en Nivel e IA = siempre OK).
pub fn validate_step(level: Level, cat: Categoria, sys: SysType, step: usize, a: &Answers) -> Option<String> {
    match levels::step_question(level, cat, sys, a.nombre_locked, step) {
        Some(q) => validate_question(&q, a),
        None => None,
    }
}

/// Título corto del paso para la barra de progreso.
pub fn step_title(level: Level, cat: Categoria, sys: SysType, skip_nombre: bool, step: usize) -> String {
    if step == 0 {
        return "Nivel de experiencia".to_string();
    }
    if levels::is_ai_step(level, cat, sys, skip_nombre, step) {
        return "Preguntas adicionales (IA, opcional)".to_string();
    }
    levels::step_question(level, cat, sys, skip_nombre, step)
        .map(|q| q.title.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Answers {
        Answers {
            nombre: "DemoApp".to_string(),
            descripcion: "Una app demo para probar el cuestionario".to_string(),
            objetivo: "Probar que los docs se generan bien".to_string(),
            funcionalidades: "Alta\nBaja".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn generics_validate_and_picks_pass() {
        let (cat, sys) = (Categoria::Aplicacion, SysType::AppWeb);
        // Tras reestructura §1.1: pasos 1-2 = cat/sys, 3-8 = genéricas (6 genéricas: nombre, descripcion, objetivo, uso_previsto, publico, funcionalidades).
        for step in 1..=8 {
            assert!(validate_step(Level::Intermedio, cat, sys, step, &sample()).is_none(), "paso {step}");
        }
        let mut bad = sample();
        bad.nombre.clear();
        assert!(validate_step(Level::Intermedio, cat, sys, 3, &bad).is_some());
        bad = sample();
        bad.nombre = "x".repeat(61);
        assert!(validate_step(Level::Intermedio, cat, sys, 3, &bad).is_some());
        bad = sample();
        bad.funcionalidades = "  ".to_string();
        assert!(validate_step(Level::Intermedio, cat, sys, 8, &bad).is_some());
        let total = levels::total_steps(Level::Intermedio, cat, sys, false);
        assert!(validate_step(Level::Intermedio, cat, sys, 0, &bad).is_none());
        assert!(validate_step(Level::Intermedio, cat, sys, total - 1, &bad).is_none());
        for step in 9..total - 1 {
            assert!(validate_step(Level::Intermedio, cat, sys, step, &sample()).is_none(), "paso {step}");
        }
    }

    #[test]
    fn locked_nombre_skips_step_but_validates() {
        // Nombre pre-rellenado: el paso 3 pasa a ser la descripción.
        let mut a = sample();
        a.nombre_locked = true;
        let (cat, sys) = (Categoria::Aplicacion, SysType::AppWeb);
        let total = levels::total_steps(Level::Intermedio, cat, sys, true);
        assert_eq!(levels::step_question(Level::Intermedio, cat, sys, true, 3).unwrap().id, "descripcion");
        assert!(validate_step(Level::Intermedio, cat, sys, 3, &a).is_none());
        assert!(validate_step(Level::Intermedio, cat, sys, total - 1, &a).is_none());
        for step in 0..total {
            assert!(validate_step(Level::Intermedio, cat, sys, step, &a).is_none(), "paso {step}");
        }
    }

    #[test]
    fn cat_cascade_resets_sys_with_defaults() {
        let mut a = sample();
        a.set_cat(Categoria::Servicio);
        assert_eq!(a.sys, SysType::ApiBackend);
        assert_eq!(a.plataformas, vec![Plataforma::ServidorNube]);
        a.set_sys(SysType::DriverFirmware);
        assert_eq!(a.cat, Categoria::Sistema);
        assert!(a.plataformas.contains(&Plataforma::EmbebidoIoT));
    }

    #[test]
    fn basic_fields_are_independent_no_duplication() {
        // Regresión: uso_previsto/publico_objetivo/descripcion eran el mismo
        // campo en la vista (fallback a descripcion) y editar uno borraba el resto.
        let mut a = sample();
        a.descripcion = "desc".to_string();
        a.uso_previsto = "uso diario".to_string();
        a.publico_objetivo = "pymes".to_string();
        assert_eq!(a.descripcion, "desc");
        assert_eq!(a.uso_previsto, "uso diario");
        assert_eq!(a.publico_objetivo, "pymes");
        // Borrar uno no toca los otros.
        a.uso_previsto.clear();
        assert!(a.uso_previsto.is_empty());
        assert_eq!(a.descripcion, "desc");
        assert_eq!(a.publico_objetivo, "pymes");
        // Los ids genéricos existen y son distintos.
        let genq = levels::generic_questions(Categoria::Aplicacion, SysType::AppWeb);
        let ids: Vec<_> = genq.iter().map(|q| q.id).collect();
        assert!(ids.contains(&"uso_previsto"), "falta uso_previsto: {ids:?}");
        assert!(ids.contains(&"publico_objetivo"), "falta publico_objetivo: {ids:?}");
        assert!(ids.contains(&"descripcion"));
        // clear_level conserva los básicos (§1.1).
        a.uso_previsto = "u".to_string();
        a.publico_objetivo = "p".to_string();
        a.clear_level();
        assert_eq!(a.uso_previsto, "u");
        assert_eq!(a.publico_objetivo, "p");
    }

    #[test]
    fn multi_toggle_adds_and_removes() {
        let mut a = sample();
        assert!(!a.has_level_content());
        a.toggle_plataforma(Plataforma::Web);
        a.toggle_stack(StackOpt::Rust);
        a.toggle_arch(ArchOpt::X86_64);
        assert!(a.has_level_content());
        a.toggle_plataforma(Plataforma::Web);
        assert!(!a.plataformas.contains(&Plataforma::Web));
        a.clear_level();
        assert!(!a.has_level_content());
        assert_eq!(a.nombre, "DemoApp", "clear_level conserva genéricas");
    }
}
