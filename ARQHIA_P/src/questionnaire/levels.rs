//! Niveles y preguntas por nivel (v0.8 + v0.8.1).
//!
//! Puro: sin I/O, sin red. El wizard es dinámico:
//! `total = 1 (nivel) + questions(level).len() + 1 (IA opcional)`.
//! Desde v0.8.1 plataforma y stack son selección múltiple, hay pregunta de
//! tipo de proyecto (Intermedio/Avanzado) y presets de estilo/arquitectura.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Level {
    #[default]
    Principiante,
    Intermedio,
    Avanzado,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Principiante => write!(f, "Principiante"),
            Level::Intermedio => write!(f, "Intermedio"),
            Level::Avanzado => write!(f, "Avanzado"),
        }
    }
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Principiante, Level::Intermedio, Level::Avanzado];
}

/// Qué se va a construir (v0.8.1, Intermedio/Avanzado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TipoProyecto {
    AppWeb,
    ApiBackend,
    FrameworkLib,
    AppEscritorio,
    AppMovil,
    Cli,
    Juego,
    BotAgente,
    #[default]
    Otro,
}

impl fmt::Display for TipoProyecto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TipoProyecto::AppWeb => write!(f, "Aplicación web"),
            TipoProyecto::ApiBackend => write!(f, "API / Backend"),
            TipoProyecto::FrameworkLib => write!(f, "Framework / Librería"),
            TipoProyecto::AppEscritorio => write!(f, "App de escritorio"),
            TipoProyecto::AppMovil => write!(f, "App móvil"),
            TipoProyecto::Cli => write!(f, "CLI / Herramienta"),
            TipoProyecto::Juego => write!(f, "Juego"),
            TipoProyecto::BotAgente => write!(f, "Bot / Agente"),
            TipoProyecto::Otro => write!(f, "Otro"),
        }
    }
}

impl TipoProyecto {
    pub const ALL: [TipoProyecto; 9] = [
        TipoProyecto::AppWeb,
        TipoProyecto::ApiBackend,
        TipoProyecto::FrameworkLib,
        TipoProyecto::AppEscritorio,
        TipoProyecto::AppMovil,
        TipoProyecto::Cli,
        TipoProyecto::Juego,
        TipoProyecto::BotAgente,
        TipoProyecto::Otro,
    ];
}

/// Dónde va a vivir (v0.8.1: 7 opciones, selección múltiple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plataforma {
    Web,
    Escritorio,
    MovilIos,
    MovilAndroid,
    ServidorNube,
    EmbebidoIoT,
    Multiplataforma,
}

impl fmt::Display for Plataforma {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Plataforma::Web => write!(f, "Web"),
            Plataforma::Escritorio => write!(f, "Escritorio"),
            Plataforma::MovilIos => write!(f, "Móvil iOS"),
            Plataforma::MovilAndroid => write!(f, "Móvil Android"),
            Plataforma::ServidorNube => write!(f, "Servidor / Nube"),
            Plataforma::EmbebidoIoT => write!(f, "Embebido / IoT"),
            Plataforma::Multiplataforma => write!(f, "Multiplataforma"),
        }
    }
}

impl Plataforma {
    pub const ALL: [Plataforma; 7] = [
        Plataforma::Web,
        Plataforma::Escritorio,
        Plataforma::MovilIos,
        Plataforma::MovilAndroid,
        Plataforma::ServidorNube,
        Plataforma::EmbebidoIoT,
        Plataforma::Multiplataforma,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Facturacion {
    #[default]
    Suscripciones,
    PagoUnico,
    Api,
    UsoPersonal,
    Publicidad,
    Freemium,
}

impl fmt::Display for Facturacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Facturacion::Suscripciones => write!(f, "Suscripciones"),
            Facturacion::PagoUnico => write!(f, "Pago único"),
            Facturacion::Api => write!(f, "API de pago"),
            Facturacion::UsoPersonal => write!(f, "Uso personal"),
            Facturacion::Publicidad => write!(f, "Publicidad"),
            Facturacion::Freemium => write!(f, "Freemium"),
        }
    }
}

impl Facturacion {
    pub const ALL: [Facturacion; 6] = [
        Facturacion::Suscripciones,
        Facturacion::PagoUnico,
        Facturacion::Api,
        Facturacion::UsoPersonal,
        Facturacion::Publicidad,
        Facturacion::Freemium,
    ];
}

/// Tecnologías (v0.8.1: 12 opciones, selección múltiple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackOpt {
    Rust,
    Python,
    JsTs,
    Go,
    JavaKotlin,
    CCpp,
    CSharp,
    Php,
    Ruby,
    Swift,
    Otro,
    ADecidir,
}

impl fmt::Display for StackOpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackOpt::Rust => write!(f, "Rust"),
            StackOpt::Python => write!(f, "Python"),
            StackOpt::JsTs => write!(f, "JS/TS"),
            StackOpt::Go => write!(f, "Go"),
            StackOpt::JavaKotlin => write!(f, "Java/Kotlin"),
            StackOpt::CCpp => write!(f, "C/C++"),
            StackOpt::CSharp => write!(f, "C#"),
            StackOpt::Php => write!(f, "PHP"),
            StackOpt::Ruby => write!(f, "Ruby"),
            StackOpt::Swift => write!(f, "Swift"),
            StackOpt::Otro => write!(f, "Otro"),
            StackOpt::ADecidir => write!(f, "A decidir"),
        }
    }
}

impl StackOpt {
    pub const ALL: [StackOpt; 12] = [
        StackOpt::Rust,
        StackOpt::Python,
        StackOpt::JsTs,
        StackOpt::Go,
        StackOpt::JavaKotlin,
        StackOpt::CCpp,
        StackOpt::CSharp,
        StackOpt::Php,
        StackOpt::Ruby,
        StackOpt::Swift,
        StackOpt::Otro,
        StackOpt::ADecidir,
    ];
}

/// Arquitectura (v0.8.1, Avanzado: preset + detalle libre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArqPreset {
    Monolito,
    PorCapas,
    Hexagonal,
    Microservicios,
    Eventos,
    Serverless,
    #[default]
    ADecidir,
}

impl fmt::Display for ArqPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArqPreset::Monolito => write!(f, "Monolito"),
            ArqPreset::PorCapas => write!(f, "Por capas"),
            ArqPreset::Hexagonal => write!(f, "Hexagonal"),
            ArqPreset::Microservicios => write!(f, "Microservicios"),
            ArqPreset::Eventos => write!(f, "Guiada por eventos"),
            ArqPreset::Serverless => write!(f, "Serverless"),
            ArqPreset::ADecidir => write!(f, "A decidir"),
        }
    }
}

impl ArqPreset {
    pub const ALL: [ArqPreset; 7] = [
        ArqPreset::Monolito,
        ArqPreset::PorCapas,
        ArqPreset::Hexagonal,
        ArqPreset::Microservicios,
        ArqPreset::Eventos,
        ArqPreset::Serverless,
        ArqPreset::ADecidir,
    ];
}

/// Estilo visual (v0.8.1, Principiante: preset + referencia libre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EstiloPreset {
    Minimalista,
    Gamer,
    Corporativo,
    Infantil,
    Retro,
    OscuroElegante,
    #[default]
    Otro,
}

impl fmt::Display for EstiloPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstiloPreset::Minimalista => write!(f, "Minimalista"),
            EstiloPreset::Gamer => write!(f, "Gamer"),
            EstiloPreset::Corporativo => write!(f, "Corporativo"),
            EstiloPreset::Infantil => write!(f, "Infantil / juguetón"),
            EstiloPreset::Retro => write!(f, "Retro"),
            EstiloPreset::OscuroElegante => write!(f, "Oscuro elegante"),
            EstiloPreset::Otro => write!(f, "Otro"),
        }
    }
}

impl EstiloPreset {
    pub const ALL: [EstiloPreset; 7] = [
        EstiloPreset::Minimalista,
        EstiloPreset::Gamer,
        EstiloPreset::Corporativo,
        EstiloPreset::Infantil,
        EstiloPreset::Retro,
        EstiloPreset::OscuroElegante,
        EstiloPreset::Otro,
    ];
}

/// Tipo de widget que pide cada pregunta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QKind {
    /// Una línea (nombre).
    Line,
    /// Texto libre (descripción, objetivo, funcionalidades...).
    Area,
    PickTipo,
    /// Preset + referencia libre (estilo de Principiante).
    PickEstilo,
    /// Selección múltiple con checkboxes.
    MultiPlataforma,
    /// Selección múltiple con checkboxes + detalle libre.
    MultiStack,
    PickFacturacion,
    /// Preset + detalle libre (arquitectura de Avanzado).
    PickArq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    /// Id estable: coincide con el campo de `Answers` que rellena.
    pub id: &'static str,
    pub title: &'static str,
    pub hint: Option<&'static str>,
    pub kind: QKind,
}

/// Bloque genérico (siempre, 4 pasos tras el nivel).
pub const GENERIC: [Question; 4] = [
    Question {
        id: "nombre",
        title: "¿Cómo se llama tu proyecto?",
        hint: Some("Un nombre corto y reconocible (máx 60)."),
        kind: QKind::Line,
    },
    Question {
        id: "descripcion",
        title: "¿Qué hace, en una o dos frases?",
        hint: Some("Mínimo 10 caracteres."),
        kind: QKind::Area,
    },
    Question {
        id: "objetivo",
        title: "¿Qué problema resuelve?",
        hint: Some("Mínimo 10 caracteres."),
        kind: QKind::Area,
    },
    Question {
        id: "funcionalidades",
        title: "¿Qué funcionalidades tendrá?",
        hint: Some("Una por línea, como una lista."),
        kind: QKind::Area,
    },
];

/// Preguntas propias de cada nivel (v0.8.1: más profundas y variadas).
pub fn level_questions(level: Level) -> Vec<Question> {
    let facturacion = Question {
        id: "facturacion",
        title: "¿Cómo se va a cobrar?",
        hint: None,
        kind: QKind::PickFacturacion,
    };
    match level {
        Level::Principiante => vec![
            Question {
                id: "estilo",
                title: "¿Qué estilo visual imaginas?",
                hint: Some("Elige uno y añade una referencia si quieres."),
                kind: QKind::PickEstilo,
            },
            Question {
                id: "plataformas",
                title: "¿Dónde va a vivir?",
                hint: Some("Marca todas las que apliquen."),
                kind: QKind::MultiPlataforma,
            },
            facturacion,
        ],
        Level::Intermedio => vec![
            Question {
                id: "ui_ux",
                title: "¿Cómo será la experiencia de uso?",
                hint: Some("Pantallas, flujo, lo que debe sentirse fácil."),
                kind: QKind::Area,
            },
            Question {
                id: "tipo",
                title: "¿Qué vas a construir?",
                hint: None,
                kind: QKind::PickTipo,
            },
            Question {
                id: "plataformas",
                title: "¿Dónde va a vivir?",
                hint: Some("Marca todas las que apliquen."),
                kind: QKind::MultiPlataforma,
            },
            Question {
                id: "stack",
                title: "¿Con qué tecnologías?",
                hint: Some("Marca las que apliquen o deja A decidir."),
                kind: QKind::MultiStack,
            },
            facturacion,
        ],
        Level::Avanzado => vec![
            Question {
                id: "ui_ux",
                title: "¿Cómo será la experiencia de uso?",
                hint: Some("Pantallas, flujo, lo que debe sentirse fácil."),
                kind: QKind::Area,
            },
            Question {
                id: "tipo",
                title: "¿Qué vas a construir?",
                hint: None,
                kind: QKind::PickTipo,
            },
            Question {
                id: "plataformas",
                title: "¿Dónde va a vivir?",
                hint: Some("Marca todas las que apliquen."),
                kind: QKind::MultiPlataforma,
            },
            Question {
                id: "stack",
                title: "¿Con qué stack?",
                hint: Some("Marca las que apliquen o deja A decidir."),
                kind: QKind::MultiStack,
            },
            Question {
                id: "arquitectura",
                title: "¿Qué arquitectura imaginas?",
                hint: Some("Elige un patrón y detalla módulos y escalado."),
                kind: QKind::PickArq,
            },
            facturacion,
        ],
    }
}

/// Secuencia completa del wizard para un nivel: genéricas + propias.
/// (El paso 0 = Nivel y el último = IA opcional los pone el wizard.)
pub fn questions(level: Level) -> Vec<Question> {
    GENERIC.into_iter().chain(level_questions(level)).collect()
}

/// Pasos totales del wizard: 1 (nivel) + preguntas + 1 (IA opcional).
pub fn total_steps(level: Level) -> usize {
    1 + questions(level).len() + 1
}

/// true si el paso es el de IA opcional (el último).
pub fn is_ai_step(level: Level, step: usize) -> bool {
    step + 1 == total_steps(level)
}

/// La pregunta de un paso intermedio (None = paso Nivel o paso IA).
pub fn step_question(level: Level, step: usize) -> Option<Question> {
    if step == 0 || is_ai_step(level, step) {
        return None;
    }
    questions(level).get(step - 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_have_distinct_questions() {
        let p = questions(Level::Principiante);
        let i = questions(Level::Intermedio);
        let a = questions(Level::Avanzado);
        // Genéricas comunes + bloque propio distinto por nivel.
        for qs in [&p, &i, &a] {
            let ids: Vec<_> = qs.iter().map(|q| q.id).collect();
            for g in ["nombre", "descripcion", "objetivo", "funcionalidades"] {
                assert!(ids.contains(&g), "falta genérica {g}");
            }
        }
        let ids = |qs: &Vec<Question>| qs.iter().map(|q| q.id).collect::<Vec<_>>();
        // Tipo solo en Intermedio/Avanzado; arquitectura solo en Avanzado.
        assert!(!ids(&p).contains(&"tipo"), "Principiante sin tipo");
        assert!(ids(&i).contains(&"tipo") && ids(&a).contains(&"tipo"));
        assert!(!ids(&p).contains(&"arquitectura"));
        assert!(!ids(&i).contains(&"arquitectura"));
        assert!(ids(&a).contains(&"arquitectura"));
        // Principiante: estilo con lista, sin stack ni código.
        assert!(ids(&p).contains(&"estilo"));
        assert!(!ids(&p).contains(&"stack"));
        assert!(ids(&i).contains(&"stack") && ids(&a).contains(&"stack"));
    }

    #[test]
    fn option_lists_are_broad() {
        assert!(TipoProyecto::ALL.len() >= 9, "tipos variados");
        assert!(Plataforma::ALL.len() >= 7, "plataformas variadas");
        assert!(StackOpt::ALL.len() >= 12, "lenguajes variados");
        assert!(ArqPreset::ALL.len() >= 7, "arquitecturas variadas");
        assert!(EstiloPreset::ALL.len() >= 7, "estilos variados");
        assert!(Facturacion::ALL.len() >= 6, "facturación con publicidad y freemium");
    }

    #[test]
    fn wizard_steps_cover_level_and_ai() {
        for level in Level::ALL {
            let total = total_steps(level);
            assert!(total > 1 + 4 + 3, "mínimo nivel+4 genéricas+3 propias+IA");
            assert!(step_question(level, 0).is_none(), "paso 0 = nivel");
            assert!(is_ai_step(level, total - 1), "último = IA");
            assert!(!is_ai_step(level, 0));
            assert_eq!(step_question(level, 1).unwrap().id, "nombre");
            assert!(step_question(level, total - 1).is_none());
        }
        // Principiante: 1+4+3+1 = 9; Intermedio: 1+4+5+1 = 11; Avanzado: 1+4+6+1 = 12.
        assert_eq!(total_steps(Level::Principiante), 9);
        assert_eq!(total_steps(Level::Intermedio), 11);
        assert_eq!(total_steps(Level::Avanzado), 12);
    }
}
