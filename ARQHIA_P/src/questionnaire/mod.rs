//! Cuestionario v0.8/v0.8.1: genérico + por nivel + IA opcional.
//!
//! Puros (sin I/O): niveles y preguntas en `levels.rs`, prompt/parseo IA en
//! `ai.rs`, plantillas en `templates.rs`. Desde v0.8.1 plataforma y stack
//! son selección múltiple (`Vec`) y hay presets de tipo/estilo/arquitectura.

pub mod ai;
pub mod levels;
pub mod templates;

pub use levels::{
    ArqPreset, EstiloPreset, Facturacion, Level, Plataforma, QKind, Question, StackOpt,
    TipoProyecto,
};

#[derive(Debug, Clone, Default)]
pub struct Answers {
    // Genéricas (siempre).
    pub nombre: String,
    pub descripcion: String,
    pub objetivo: String,
    pub funcionalidades: String,
    // Por nivel (solo se usan las del nivel elegido).
    pub estilo: EstiloPreset,
    pub estilo_free: String,
    pub ui_ux: String,
    pub tipo: TipoProyecto,
    pub plataformas: Vec<Plataforma>,
    pub stacks: Vec<StackOpt>,
    pub stack_free: String,
    pub arq: ArqPreset,
    pub arquitectura: String,
    pub facturacion: Facturacion,
}

impl Answers {
    /// Marca/desmarca una plataforma (checkbox múltiple).
    pub fn toggle_plataforma(&mut self, p: Plataforma) {
        if self.plataformas.contains(&p) {
            self.plataformas.retain(|x| *x != p);
        } else {
            self.plataformas.push(p);
        }
    }

    /// Marca/desmarca una tecnología (checkbox múltiple).
    pub fn toggle_stack(&mut self, s: StackOpt) {
        if self.stacks.contains(&s) {
            self.stacks.retain(|x| *x != s);
        } else {
            self.stacks.push(s);
        }
    }

    /// Pares (etiqueta, valor) del bloque por nivel, para el prompt IA y docs.
    pub fn level_extras(&self, level: Level) -> Vec<(String, String)> {
        match level {
            Level::Principiante => vec![
                ("Estilo visual".to_string(), estilo_line(self.estilo, &self.estilo_free)),
                ("Plataforma".to_string(), join_or_dash(&self.plataformas)),
                ("Facturación".to_string(), self.facturacion.to_string()),
            ],
            Level::Intermedio => vec![
                ("UI/UX".to_string(), self.ui_ux.clone()),
                ("Tipo de proyecto".to_string(), self.tipo.to_string()),
                ("Plataforma".to_string(), join_or_dash(&self.plataformas)),
                ("Stack".to_string(), stack_line(&self.stacks, &self.stack_free)),
                ("Facturación".to_string(), self.facturacion.to_string()),
            ],
            Level::Avanzado => vec![
                ("UI/UX".to_string(), self.ui_ux.clone()),
                ("Tipo de proyecto".to_string(), self.tipo.to_string()),
                ("Plataforma".to_string(), join_or_dash(&self.plataformas)),
                ("Stack".to_string(), stack_line(&self.stacks, &self.stack_free)),
                (
                    "Arquitectura".to_string(),
                    format!("{} {}", self.arq, self.arquitectura).trim().to_string(),
                ),
                ("Facturación".to_string(), self.facturacion.to_string()),
            ],
        }
    }

    /// true si algún campo del bloque por nivel tiene contenido (para avisar
    /// al cambiar de nivel: se descartarían esas respuestas).
    pub fn has_level_content(&self) -> bool {
        !self.estilo_free.trim().is_empty()
            || !self.ui_ux.trim().is_empty()
            || !self.plataformas.is_empty()
            || !self.stacks.is_empty()
            || !self.stack_free.trim().is_empty()
            || !self.arquitectura.trim().is_empty()
    }

    /// Limpia solo el bloque por nivel (al cambiar de nivel confirmado).
    pub fn clear_level(&mut self) {
        self.estilo = EstiloPreset::default();
        self.estilo_free.clear();
        self.ui_ux.clear();
        self.tipo = TipoProyecto::default();
        self.plataformas.clear();
        self.stacks.clear();
        self.stack_free.clear();
        self.arq = ArqPreset::default();
        self.arquitectura.clear();
        self.facturacion = Facturacion::default();
    }
}

fn join_or_dash(items: &[Plataforma]) -> String {
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
pub fn validate_step(level: Level, step: usize, a: &Answers) -> Option<String> {
    match levels::step_question(level, step) {
        Some(q) => validate_question(&q, a),
        None => None,
    }
}

/// Título corto del paso para la barra de progreso.
pub fn step_title(level: Level, step: usize) -> String {
    if step == 0 {
        return "Nivel de experiencia".to_string();
    }
    if levels::is_ai_step(level, step) {
        return "Preguntas adicionales (IA, opcional)".to_string();
    }
    levels::step_question(level, step).map(|q| q.title.to_string()).unwrap_or_default()
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
        let level = Level::Intermedio;
        // Pasos 1-4 = genéricas válidas.
        for step in 1..=4 {
            assert!(validate_step(level, step, &sample()).is_none(), "paso {step}");
        }
        let mut bad = sample();
        bad.nombre.clear();
        assert!(validate_step(level, 1, &bad).is_some());
        bad = sample();
        bad.nombre = "x".repeat(61);
        assert!(validate_step(level, 1, &bad).is_some());
        bad = sample();
        bad.funcionalidades = "  ".to_string();
        assert!(validate_step(level, 4, &bad).is_some());
        // Nivel (0) e IA (último) nunca bloquean.
        assert!(validate_step(level, 0, &bad).is_none());
        assert!(validate_step(level, levels::total_steps(level) - 1, &bad).is_none());
        // Picks y checkboxes por nivel nunca bloquean (vacío = sin especificar).
        let total = levels::total_steps(level);
        for step in 5..total - 1 {
            assert!(validate_step(level, step, &sample()).is_none(), "nivel paso {step}");
        }
    }

    #[test]
    fn multi_toggle_adds_and_removes() {
        let mut a = sample();
        assert!(!a.has_level_content());
        a.toggle_plataforma(Plataforma::Web);
        a.toggle_plataforma(Plataforma::ServidorNube);
        a.toggle_stack(StackOpt::Rust);
        assert!(a.has_level_content());
        assert_eq!(a.plataformas.len(), 2);
        a.toggle_plataforma(Plataforma::Web);
        assert_eq!(a.plataformas, vec![Plataforma::ServidorNube]);
        a.toggle_stack(StackOpt::Rust);
        assert!(a.stacks.is_empty());
        a.clear_level();
        assert!(!a.has_level_content());
        assert_eq!(a.facturacion, Facturacion::Suscripciones);
        let extras = sample().level_extras(Level::Avanzado);
        assert!(extras.iter().any(|(k, _)| k == "Arquitectura"));
        let princ = sample().level_extras(Level::Principiante);
        assert!(princ.iter().any(|(k, v)| k == "Plataforma" && v == "(sin especificar)"));
    }
}
