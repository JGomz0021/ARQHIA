//! Taxonomía Categoría → Tipo + matriz condicional (v0.8.1).
//!
//! Puro: sin I/O, sin red. `SysType` = qué es, `Plataforma` = dónde corre.
//! `SysType` = **qué es**, `Plataforma` = **dónde corre**.
//! Wizard dinámico: `total = 1 (nivel) + preguntas(nivel,cat,sys) + 1 (IA)`,
//! donde preguntas = 4 genéricas + cat + sys + N de familia.

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

/// Familia de software (qué tipo de cosa se produce).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Categoria {
    #[default]
    Aplicacion,
    Servicio,
    Libreria,
    Sistema,
    Automatizacion,
    DatosIa,
}

impl fmt::Display for Categoria {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Categoria::Aplicacion => write!(f, "Aplicación"),
            Categoria::Servicio => write!(f, "Servicio/Backend"),
            Categoria::Libreria => write!(f, "Librería/Ecosistema"),
            Categoria::Sistema => write!(f, "Sistema"),
            Categoria::Automatizacion => write!(f, "Automatización"),
            Categoria::DatosIa => write!(f, "Datos/IA"),
        }
    }
}

impl Categoria {
    pub const ALL: [Categoria; 6] = [
        Categoria::Aplicacion,
        Categoria::Servicio,
        Categoria::Libreria,
        Categoria::Sistema,
        Categoria::Automatizacion,
        Categoria::DatosIa,
    ];
}

/// Tipo de sistema dentro de su categoría (qué es).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SysType {
    // Aplicación (la usa una persona)
    #[default]
    AppWeb,
    AppMovil,
    AppEscritorio,
    CliHerramienta,
    Juego,
    BotConversacional,
    // Servicio/Backend (corre en servidor)
    ApiBackend,
    Microservicio,
    ServidorDaemon,
    DbMiddleware,
    // Librería/Ecosistema (se consume como código o en un host)
    FrameworkLibreria,
    PluginExtension,
    MotorEngine,
    PlantillaStarter,
    // Sistema (corre la máquina)
    LenguajeRuntime,
    OsDistro,
    KernelModulo,
    DriverFirmware,
    ContenedorVm,
    UtilidadSistema,
    // Automatización (hace trabajo solo)
    Script,
    PipelineCiCd,
    IacDespliegue,
    IntegracionWebhook,
    JobRpa,
    AgenteAutonomo,
    // Datos/IA
    PipelineEtl,
    ModeloMl,
    AgenteLlmRag,
}

impl fmt::Display for SysType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SysType::AppWeb => "App web",
            SysType::AppMovil => "App móvil",
            SysType::AppEscritorio => "App escritorio",
            SysType::CliHerramienta => "CLI/Herramienta",
            SysType::Juego => "Juego",
            SysType::BotConversacional => "Bot conversacional",
            SysType::ApiBackend => "API/Backend",
            SysType::Microservicio => "Microservicio",
            SysType::ServidorDaemon => "Servidor/Daemon",
            SysType::DbMiddleware => "Base de datos/Middleware",
            SysType::FrameworkLibreria => "Framework/Librería/SDK",
            SysType::PluginExtension => "Plugin/Extensión",
            SysType::MotorEngine => "Motor/Engine",
            SysType::PlantillaStarter => "Plantilla/Starter",
            SysType::LenguajeRuntime => "Lenguaje/Runtime/Compilador",
            SysType::OsDistro => "OS/Distro",
            SysType::KernelModulo => "Kernel/Módulo",
            SysType::DriverFirmware => "Driver/Firmware",
            SysType::ContenedorVm => "Contenedor/VM/Emulador",
            SysType::UtilidadSistema => "Utilidad de sistema",
            SysType::Script => "Script",
            SysType::PipelineCiCd => "Pipeline CI/CD",
            SysType::IacDespliegue => "IaC/Despliegue",
            SysType::IntegracionWebhook => "Integración/Webhook",
            SysType::JobRpa => "Job programado/RPA",
            SysType::AgenteAutonomo => "Agente autónomo",
            SysType::PipelineEtl => "Pipeline ETL/Dataset",
            SysType::ModeloMl => "Modelo ML",
            SysType::AgenteLlmRag => "Agente LLM/RAG",
        };
        write!(f, "{s}")
    }
}

impl SysType {
    /// Tipos filtrados por categoría (cascada Cat → Tipo).
    pub fn for_cat(cat: Categoria) -> &'static [SysType] {
        match cat {
            Categoria::Aplicacion => &[
                SysType::AppWeb,
                SysType::AppMovil,
                SysType::AppEscritorio,
                SysType::CliHerramienta,
                SysType::Juego,
                SysType::BotConversacional,
            ],
            Categoria::Servicio => &[
                SysType::ApiBackend,
                SysType::Microservicio,
                SysType::ServidorDaemon,
                SysType::DbMiddleware,
            ],
            Categoria::Libreria => &[
                SysType::FrameworkLibreria,
                SysType::PluginExtension,
                SysType::MotorEngine,
                SysType::PlantillaStarter,
            ],
            Categoria::Sistema => &[
                SysType::LenguajeRuntime,
                SysType::OsDistro,
                SysType::KernelModulo,
                SysType::DriverFirmware,
                SysType::ContenedorVm,
                SysType::UtilidadSistema,
            ],
            Categoria::Automatizacion => &[
                SysType::Script,
                SysType::PipelineCiCd,
                SysType::IacDespliegue,
                SysType::IntegracionWebhook,
                SysType::JobRpa,
                SysType::AgenteAutonomo,
            ],
            Categoria::DatosIa => &[
                SysType::PipelineEtl,
                SysType::ModeloMl,
                SysType::AgenteLlmRag,
            ],
        }
    }

    pub fn categoria(self) -> Categoria {
        for cat in Categoria::ALL {
            if Self::for_cat(cat).contains(&self) {
                return cat;
            }
        }
        Categoria::Aplicacion
    }
}

/// ¿Tiene UI visible? Solo entonces se pregunta estilo/UI.
pub fn has_ui(sys: SysType) -> bool {
    matches!(
        sys,
        SysType::AppWeb
            | SysType::AppMovil
            | SysType::AppEscritorio
            | SysType::Juego
            | SysType::MotorEngine
    )
}

/// Facturación solo para App/Servicio; el resto usa Licencia.
pub fn uses_facturacion(cat: Categoria) -> bool {
    matches!(cat, Categoria::Aplicacion | Categoria::Servicio)
}

/// Dónde va a correr (9 opciones, selección múltiple; el escritorio se
/// desglosa por SO: Windows, macOS, Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plataforma {
    Web,
    Windows,
    MacOs,
    Linux,
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
            Plataforma::Windows => write!(f, "Windows"),
            Plataforma::MacOs => write!(f, "macOS"),
            Plataforma::Linux => write!(f, "Linux"),
            Plataforma::MovilIos => write!(f, "Móvil iOS"),
            Plataforma::MovilAndroid => write!(f, "Móvil Android"),
            Plataforma::ServidorNube => write!(f, "Servidor / Nube"),
            Plataforma::EmbebidoIoT => write!(f, "Embebido / IoT"),
            Plataforma::Multiplataforma => write!(f, "Multiplataforma"),
        }
    }
}

impl Plataforma {
    pub const ALL: [Plataforma; 9] = [
        Plataforma::Web,
        Plataforma::Windows,
        Plataforma::MacOs,
        Plataforma::Linux,
        Plataforma::MovilIos,
        Plataforma::MovilAndroid,
        Plataforma::ServidorNube,
        Plataforma::EmbebidoIoT,
        Plataforma::Multiplataforma,
    ];
}

/// Plataformas pre-marcadas por tipo (editables, nunca bloquean).
pub fn default_plataformas(sys: SysType) -> Vec<Plataforma> {
    match sys {
        SysType::AppWeb => vec![Plataforma::Web],
        SysType::AppMovil => vec![Plataforma::MovilIos, Plataforma::MovilAndroid],
        SysType::AppEscritorio => vec![Plataforma::Windows, Plataforma::MacOs, Plataforma::Linux],
        SysType::ApiBackend
        | SysType::Microservicio
        | SysType::ServidorDaemon
        | SysType::DbMiddleware => {
            vec![Plataforma::ServidorNube]
        }
        SysType::DriverFirmware | SysType::KernelModulo | SysType::UtilidadSistema => {
            vec![Plataforma::EmbebidoIoT]
        }
        SysType::CliHerramienta | SysType::Script => vec![Plataforma::Multiplataforma],
        _ => vec![],
    }
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
    OpenSource,
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
            Facturacion::OpenSource => write!(f, "Código abierto"),
        }
    }
}

impl Facturacion {
    pub const ALL: [Facturacion; 7] = [
        Facturacion::Suscripciones,
        Facturacion::PagoUnico,
        Facturacion::Api,
        Facturacion::UsoPersonal,
        Facturacion::Publicidad,
        Facturacion::Freemium,
        Facturacion::OpenSource,
    ];
}

/// Licencia (donde no aplica facturación). `Uso interno` nunca sube a nube.
/// Default `UsoInterno`: no se asume open-source hasta que el usuario elija
/// una licencia abierta (MIT/Apache/GPL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Licencia {
    Mit,
    Apache2,
    Gpl3,
    #[default]
    UsoInterno,
}

impl fmt::Display for Licencia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Licencia::Mit => write!(f, "MIT"),
            Licencia::Apache2 => write!(f, "Apache-2.0"),
            Licencia::Gpl3 => write!(f, "GPL-3.0"),
            Licencia::UsoInterno => write!(f, "Uso interno"),
        }
    }
}

impl Licencia {
    pub const ALL: [Licencia; 4] = [
        Licencia::Mit,
        Licencia::Apache2,
        Licencia::Gpl3,
        Licencia::UsoInterno,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ApiStyle {
    #[default]
    Rest,
    Grpc,
    Graphql,
    ADecidir,
}

impl fmt::Display for ApiStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiStyle::Rest => write!(f, "REST"),
            ApiStyle::Grpc => write!(f, "gRPC"),
            ApiStyle::Graphql => write!(f, "GraphQL"),
            ApiStyle::ADecidir => write!(f, "A decidir"),
        }
    }
}

impl ApiStyle {
    pub const ALL: [ApiStyle; 4] = [
        ApiStyle::Rest,
        ApiStyle::Grpc,
        ApiStyle::Graphql,
        ApiStyle::ADecidir,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AuthKind {
    #[default]
    Ninguna,
    Jwt,
    Oauth,
    ApiKey,
}

impl fmt::Display for AuthKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthKind::Ninguna => write!(f, "Sin auth"),
            AuthKind::Jwt => write!(f, "JWT"),
            AuthKind::Oauth => write!(f, "OAuth"),
            AuthKind::ApiKey => write!(f, "API key"),
        }
    }
}

impl AuthKind {
    pub const ALL: [AuthKind; 4] = [
        AuthKind::Ninguna,
        AuthKind::Jwt,
        AuthKind::Oauth,
        AuthKind::ApiKey,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Trigger {
    #[default]
    Cron,
    Manual,
    Webhook,
    Ci,
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trigger::Cron => write!(f, "Programado (cron)"),
            Trigger::Manual => write!(f, "Manual"),
            Trigger::Webhook => write!(f, "Webhook"),
            Trigger::Ci => write!(f, "CI"),
        }
    }
}

impl Trigger {
    pub const ALL: [Trigger; 4] = [
        Trigger::Cron,
        Trigger::Manual,
        Trigger::Webhook,
        Trigger::Ci,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SemverOpt {
    #[default]
    Estricto,
    Compatible,
    SinCompromiso,
}

impl fmt::Display for SemverOpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemverOpt::Estricto => write!(f, "Semver estricto"),
            SemverOpt::Compatible => write!(f, "Compatible (minor)"),
            SemverOpt::SinCompromiso => write!(f, "Sin compromiso"),
        }
    }
}

impl SemverOpt {
    pub const ALL: [SemverOpt; 3] = [
        SemverOpt::Estricto,
        SemverOpt::Compatible,
        SemverOpt::SinCompromiso,
    ];
}

/// Arquitectura target (Sistema): multi x86_64/aarch64/embebido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchOpt {
    X86_64,
    Aarch64,
    Embebido,
}

impl fmt::Display for ArchOpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchOpt::X86_64 => write!(f, "x86_64"),
            ArchOpt::Aarch64 => write!(f, "aarch64"),
            ArchOpt::Embebido => write!(f, "Embebido"),
        }
    }
}

impl ArchOpt {
    pub const ALL: [ArchOpt; 3] = [ArchOpt::X86_64, ArchOpt::Aarch64, ArchOpt::Embebido];
}

/// Tecnologías (12 opciones, selección múltiple).
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

/// Estilo visual (solo si el tipo tiene UI: preset + referencia libre).
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
    Line,
    Area,
    PickCat,
    PickSys,
    PickEstilo,
    MultiPlataforma,
    MultiStack,
    PickFacturacion,
    PickLicencia,
    PickApiStyle,
    PickAuth,
    PickTrigger,
    PickSemver,
    MultiArch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    pub id: &'static str,
    pub title: &'static str,
    pub hint: Option<&'static str>,
    pub kind: QKind,
}

/// Bloque genérico contextualizado (siempre, 6 pasos tras Cat/Tipo — §1.1).
/// Cubre la idea general: descripción, objetivo, uso previsto, público objetivo y funcionalidades.
/// Mismos ids para no romper validación; solo cambian título/hint según familia.
/// El nombre se omite si ya se obtuvo en la entrada inicial (`skip_nombre`).
pub fn generic_questions(cat: Categoria, sys: SysType) -> [Question; 6] {
    let func_hint: &'static str = match cat {
        Categoria::Aplicacion => "Una por línea: pantallas o acciones del usuario.",
        Categoria::Servicio => "Una por línea: endpoints o capacidades (ej. GET /users).",
        Categoria::Libreria => "Una por línea: funciones/tipos públicos de la API.",
        Categoria::Sistema => match sys {
            SysType::LenguajeRuntime => {
                "Una por línea: rasgos del lenguaje (sintaxis, tipos, std)."
            }
            SysType::OsDistro | SysType::KernelModulo => {
                "Una por línea: capacidades del sistema (boot, procesos, drivers)."
            }
            _ => "Una por línea: capacidades o targets soportados.",
        },
        Categoria::Automatizacion => "Una por línea: trabajos que hace solo.",
        Categoria::DatosIa => "Una por línea: salidas del pipeline/modelo.",
    };
    let desc_hint: &'static str = match cat {
        Categoria::Sistema if sys == SysType::LenguajeRuntime => {
            "Qué compila/ejecuta, en una o dos frases."
        }
        Categoria::Sistema => "Qué máquina/entorno corre, en una o dos frases.",
        Categoria::Servicio => "Qué sirve y a quién, en una o dos frases.",
        Categoria::Libreria => "Qué resuelve para quien la usa, en una o dos frases.",
        _ => "Mínimo 10 caracteres.",
    };
    [
        Question {
            id: "nombre",
            title: "¿Cómo se llama tu proyecto?",
            hint: Some("Un nombre corto y reconocible (máx 60)."),
            kind: QKind::Line,
        },
        Question {
            id: "descripcion",
            title: "Describe el proyecto en una o dos frases",
            hint: Some(desc_hint),
            kind: QKind::Area,
        },
        Question {
            id: "objetivo",
            title: "¿Cuál es el objetivo principal?",
            hint: Some("Mínimo 10 caracteres. Qué pretende conseguir."),
            kind: QKind::Area,
        },
        Question {
            id: "uso_previsto",
            title: "¿Cuál es el uso previsto?",
            hint: Some("Ej. uso diario interno, producto para clientes, demo, investigación."),
            kind: QKind::Area,
        },
        Question {
            id: "publico_objetivo",
            title: "¿A quién va dirigido? (público objetivo)",
            hint: Some("Ej. desarrolladores indie, pymes, público general, equipo interno."),
            kind: QKind::Area,
        },
        Question {
            id: "funcionalidades",
            title: "¿Qué funcionalidades tendrá?",
            hint: Some(func_hint),
            kind: QKind::Area,
        },
    ]
}

/// Bloque genérico legacy: ver `generic_questions` (contextual por familia).
const Q_CAT: Question = Question {
    id: "cat",
    title: "¿Qué tipo de software vas a producir?",
    hint: Some("La categoría filtra los tipos y adapta las preguntas."),
    kind: QKind::PickCat,
};

const Q_SYS: Question = Question {
    id: "sys",
    title: "¿Qué es, en concreto?",
    hint: Some("Filtrado por la categoría anterior."),
    kind: QKind::PickSys,
};

/// ¿El proyecto se publica como abierto? Rama transversal OSS:
/// App/Servicio con facturación `Código abierto`, o resto con licencia
/// distinta de `Uso interno`.
pub fn is_oss(cat: Categoria, fact: Facturacion, lic: Licencia) -> bool {
    if uses_facturacion(cat) {
        fact == Facturacion::OpenSource
    } else {
        lic != Licencia::UsoInterno
    }
}

/// Bloque transversal open-source (siempre al final de la familia, 3 áreas
/// cortas; picks/áreas nunca bloquean, vacío = gap explícito).
pub fn oss_questions() -> Vec<Question> {
    vec![
        Question {
            id: "oss_repo",
            title: "¿Dónde se publica el repo?",
            hint: Some("Ej. GitHub org/repo, licencia visible, README con quickstart."),
            kind: QKind::Area,
        },
        Question {
            id: "oss_gobierno",
            title: "¿Cómo se gobierna?",
            hint: Some("Mantenimiento, decisiones, roadmap público, CI visible."),
            kind: QKind::Area,
        },
        Question {
            id: "oss_contrib",
            title: "¿Cómo se contribuye?",
            hint: Some("Guía de contribución, CoC, plantillas de issue/PR."),
            kind: QKind::Area,
        },
    ]
}

/// Preguntas propias de cada familia (2–6 + cierre licencia/facturación).
/// `stack` no se pregunta a Principiante; Sistema/Librería se parten por
/// subtipo (lenguaje ≠ OS ≠ driver; framework ≠ plugin ≠ motor).
pub fn family_questions(level: Level, cat: Categoria, sys: SysType) -> Vec<Question> {
    let mut out = Vec::new();
    let ask_stack = level != Level::Principiante;
    match cat {
        Categoria::Aplicacion => {
            out.push(Question {
                id: "ui_ux",
                title: "¿Cómo será la experiencia de uso?",
                hint: Some("Pantallas, flujo, lo que debe sentirse fácil."),
                kind: QKind::Area,
            });
            if has_ui(sys) {
                out.push(Question {
                    id: "estilo",
                    title: "¿Qué estilo visual imaginas?",
                    hint: Some("Elige uno y añade una referencia si quieres."),
                    kind: QKind::PickEstilo,
                });
            }
            out.push(Question {
                id: "plataformas",
                title: "¿Dónde va a correr?",
                hint: Some("Marca todas las que apliquen."),
                kind: QKind::MultiPlataforma,
            });
            if ask_stack {
                out.push(Question {
                    id: "stack",
                    title: "¿Con qué tecnologías?",
                    hint: Some("Marca las que apliquen o deja A decidir."),
                    kind: QKind::MultiStack,
                });
            }
            out.push(Question {
                id: "facturacion",
                title: "¿Cómo se va a cobrar?",
                hint: None,
                kind: QKind::PickFacturacion,
            });
        }
        Categoria::Servicio => {
            out.push(Question {
                id: "endpoints",
                title: "¿Qué endpoints o recursos expone?",
                hint: Some("Uno por línea, ej. GET /users, POST /orders."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "api_style",
                title: "¿Qué estilo de API?",
                hint: None,
                kind: QKind::PickApiStyle,
            });
            out.push(Question {
                id: "auth",
                title: "¿Cómo se autentica?",
                hint: None,
                kind: QKind::PickAuth,
            });
            out.push(Question {
                id: "escala",
                title: "¿Qué escala o SLA debe cumplir?",
                hint: Some("Ej. 1k req/s, 99.9%, latencia p95."),
                kind: QKind::Area,
            });
            if ask_stack {
                out.push(Question {
                    id: "stack",
                    title: "¿Con qué stack?",
                    hint: Some("Marca las que apliquen o deja A decidir."),
                    kind: QKind::MultiStack,
                });
            }
            out.push(Question {
                id: "facturacion",
                title: "¿Cómo se va a cobrar?",
                hint: None,
                kind: QKind::PickFacturacion,
            });
        }
        Categoria::Libreria => {
            // Framework/Librería y Starter: API pública + compat + docs.
            // Plugin: además host donde se instala. Motor: además UX/editor.
            if sys == SysType::MotorEngine && has_ui(sys) {
                out.push(Question {
                    id: "ui_ux",
                    title: "¿Cómo es el editor o la experiencia?",
                    hint: Some("Flujo del editor, viewport, assets, lo que debe sentirse fácil."),
                    kind: QKind::Area,
                });
                out.push(Question {
                    id: "estilo",
                    title: "¿Qué estilo visual imaginas?",
                    hint: Some("Elige uno y añade una referencia si quieres."),
                    kind: QKind::PickEstilo,
                });
            }
            out.push(Question {
                id: "api_publica",
                title: "¿Cómo es la API pública?",
                hint: Some("Funciones/tipos principales, un ejemplo corto."),
                kind: QKind::Area,
            });
            if sys == SysType::PluginExtension {
                out.push(Question {
                    id: "host_api",
                    title: "¿En qué host se instala?",
                    hint: Some("App/host, versiones soportadas, permisos que pide."),
                    kind: QKind::Area,
                });
            }
            out.push(Question {
                id: "semver",
                title: "¿Qué compromiso de compatibilidad?",
                hint: None,
                kind: QKind::PickSemver,
            });
            out.push(Question {
                id: "stack",
                title: "¿Lenguaje(s) objetivo?",
                hint: Some("Marca los lenguajes donde se usará."),
                kind: QKind::MultiStack,
            });
            if sys == SysType::MotorEngine {
                out.push(Question {
                    id: "plataformas",
                    title: "¿Dónde va a correr?",
                    hint: Some("Marca todas las que apliquen."),
                    kind: QKind::MultiPlataforma,
                });
            }
            out.push(Question {
                id: "ejemplos",
                title: "¿Qué ejemplos o docs necesita?",
                hint: Some("Ej. README con quickstart, docs por módulo."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "licencia",
                title: "¿Con qué licencia se publica?",
                hint: Some("GPL avisa: obliga a liberar derivados. Uso interno nunca sube a nube."),
                kind: QKind::PickLicencia,
            });
        }
        Categoria::Sistema => {
            // Lenguaje/Runtime: sintaxis + toolchain + std + arch.
            // OS/Kernel/Contenedor: arch + arranque + syscalls + compat.
            // Driver/Firmware/Utilidad: arch + arranque + compat.
            if sys == SysType::LenguajeRuntime {
                out.push(Question {
                    id: "sintaxis",
                    title: "¿Cómo es la sintaxis y el sistema de tipos?",
                    hint: Some("Paradigma, tipos, gestión de memoria, un ejemplo corto."),
                    kind: QKind::Area,
                });
                out.push(Question {
                    id: "toolchain",
                    title: "¿Qué toolchain necesita?",
                    hint: Some("Compilador/intérprete, package manager, LSP, debugger."),
                    kind: QKind::Area,
                });
                out.push(Question {
                    id: "arch",
                    title: "¿En qué arquitectura corre?",
                    hint: Some("Marca todas las objetivo."),
                    kind: QKind::MultiArch,
                });
                out.push(Question {
                    id: "compat",
                    title: "¿Con qué debe ser compatible?",
                    hint: Some("Std, FFI con C, versiones, plataformas."),
                    kind: QKind::Area,
                });
                if ask_stack {
                    out.push(Question {
                        id: "stack",
                        title: "¿Lenguaje de implementación?",
                        hint: Some("Marca el/los lenguajes + target."),
                        kind: QKind::MultiStack,
                    });
                }
            } else {
                out.push(Question {
                    id: "arch",
                    title: "¿En qué arquitectura corre?",
                    hint: Some("Marca todas las objetivo."),
                    kind: QKind::MultiArch,
                });
                out.push(Question {
                    id: "arranque",
                    title: "¿Cómo arranca o qué hardware toca?",
                    hint: Some("Bootloader, drivers, periféricos, syscalls..."),
                    kind: QKind::Area,
                });
                if matches!(
                    sys,
                    SysType::OsDistro | SysType::KernelModulo | SysType::ContenedorVm
                ) {
                    out.push(Question {
                        id: "syscalls",
                        title: "¿Qué syscalls o ABI expone?",
                        hint: Some("Llamadas, ABI estable, compat con Linux/POSIX..."),
                        kind: QKind::Area,
                    });
                }
                out.push(Question {
                    id: "compat",
                    title: "¿Con qué debe ser compatible?",
                    hint: Some("Kernel, distros, versiones, hardware."),
                    kind: QKind::Area,
                });
                if ask_stack {
                    out.push(Question {
                        id: "stack",
                        title: "¿Lenguaje de implementación?",
                        hint: Some("Marca el/los lenguajes + target."),
                        kind: QKind::MultiStack,
                    });
                }
            }
            out.push(Question {
                id: "licencia",
                title: "¿Con qué licencia se publica?",
                hint: Some("GPL avisa: obliga a liberar derivados. Uso interno nunca sube a nube."),
                kind: QKind::PickLicencia,
            });
        }
        Categoria::Automatizacion => {
            out.push(Question {
                id: "trigger",
                title: "¿Qué lo dispara?",
                hint: None,
                kind: QKind::PickTrigger,
            });
            out.push(Question {
                id: "inputs",
                title: "¿Qué entradas o secretos necesita?",
                hint: Some("Args, env, tokens, ficheros..."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "idempotencia",
                title: "¿Debe ser idempotente o reintentable?",
                hint: Some("Qué pasa si se ejecuta dos veces o falla a mitad."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "donde",
                title: "¿Dónde corre?",
                hint: Some("Marca todas las que apliquen."),
                kind: QKind::MultiPlataforma,
            });
            if ask_stack {
                out.push(Question {
                    id: "stack",
                    title: "¿Con qué lo escribes?",
                    hint: Some("Marca las que apliquen o deja A decidir."),
                    kind: QKind::MultiStack,
                });
            }
            out.push(Question {
                id: "licencia",
                title: "¿Con qué licencia se publica?",
                hint: Some("Uso interno nunca sube a nube."),
                kind: QKind::PickLicencia,
            });
        }
        Categoria::DatosIa => {
            out.push(Question {
                id: "dataset",
                title: "¿De dónde salen los datos?",
                hint: Some("Fuentes, formato, volumen, licencias."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "pipeline",
                title: "¿Cómo es el pipeline?",
                hint: Some("Ingesta → proceso → salida, corto."),
                kind: QKind::Area,
            });
            out.push(Question {
                id: "modelo",
                title: "¿Qué modelo y cómo se evalúa?",
                hint: Some("Modelo base, métricas, eval mínima."),
                kind: QKind::Area,
            });
            if ask_stack {
                out.push(Question {
                    id: "stack",
                    title: "¿Con qué stack?",
                    hint: Some("Marca las que apliquen o deja A decidir."),
                    kind: QKind::MultiStack,
                });
            }
            out.push(Question {
                id: "licencia",
                title: "¿Con qué licencia se publica?",
                hint: Some("Uso interno nunca sube a nube."),
                kind: QKind::PickLicencia,
            });
        }
    }
    out
}

/// Secuencia completa (v0.8.2): Cat → Tipo → genéricas contextualizadas
/// → familia → bloque transversal OSS. Con `skip_nombre` se omite la
/// pregunta del nombre (ya ingresado al crear/abrir el proyecto).
pub fn questions(level: Level, cat: Categoria, sys: SysType, skip_nombre: bool) -> Vec<Question> {
    let mut out = vec![Q_CAT, Q_SYS];
    out.extend(generic_questions(cat, sys));
    out.extend(family_questions(level, cat, sys));
    out.extend(oss_questions());
    if skip_nombre {
        out.retain(|q| q.id != "nombre");
    }
    out
}

/// Pasos totales: 1 (nivel) + preguntas + 1 (IA opcional).
pub fn total_steps(level: Level, cat: Categoria, sys: SysType, skip_nombre: bool) -> usize {
    1 + questions(level, cat, sys, skip_nombre).len() + 1
}

pub fn is_ai_step(
    level: Level,
    cat: Categoria,
    sys: SysType,
    skip_nombre: bool,
    step: usize,
) -> bool {
    step + 1 == total_steps(level, cat, sys, skip_nombre)
}

/// La pregunta de un paso intermedio (None = Nivel o IA).
pub fn step_question(
    level: Level,
    cat: Categoria,
    sys: SysType,
    skip_nombre: bool,
    step: usize,
) -> Option<Question> {
    if step == 0 || is_ai_step(level, cat, sys, skip_nombre, step) {
        return None;
    }
    questions(level, cat, sys, skip_nombre)
        .get(step - 1)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat_filters_sys_and_resets() {
        for cat in Categoria::ALL {
            assert!(!SysType::for_cat(cat).is_empty(), "cat {cat} sin tipos");
            for s in SysType::for_cat(cat) {
                assert_eq!(s.categoria(), cat, "sys {s} mal categorizado");
            }
        }
        // Cascada: al cambiar Cat se resetea Sys al primero de la familia.
        let first = SysType::for_cat(Categoria::Sistema)[0];
        assert_eq!(first.categoria(), Categoria::Sistema);
    }

    #[test]
    fn no_tipo_proyecto_left() {
        // La matriz nueva cubre todas las familias.
        assert_eq!(Categoria::ALL.len(), 6);
        let total_types: usize = Categoria::ALL
            .iter()
            .map(|c| SysType::for_cat(*c).len())
            .sum();
        assert!(total_types >= 24, "taxonomía amplia ({total_types})");
    }

    #[test]
    fn matrix_rules() {
        // Principiante+Script no pide estilo ni stack; Avanzado+Driver pide arch+licencia sin facturación.
        let script = family_questions(
            Level::Principiante,
            Categoria::Automatizacion,
            SysType::Script,
        );
        let ids: Vec<_> = script.iter().map(|q| q.id).collect();
        assert!(!ids.contains(&"estilo"), "script sin estilo");
        assert!(!ids.contains(&"stack"), "principiante sin stack");
        assert!(ids.contains(&"licencia") && !ids.contains(&"facturacion"));

        let driver = family_questions(Level::Avanzado, Categoria::Sistema, SysType::DriverFirmware);
        let ids: Vec<_> = driver.iter().map(|q| q.id).collect();
        assert!(ids.contains(&"arch") && ids.contains(&"licencia"));
        assert!(!ids.contains(&"facturacion"));
        assert!(
            !ids.contains(&"syscalls"),
            "driver sin syscalls (solo OS/Kernel/VM)"
        );

        // Lenguaje ≠ OS: sintaxis+toolchain solo en lenguaje; syscalls solo en OS/Kernel/VM.
        let lang = family_questions(
            Level::Avanzado,
            Categoria::Sistema,
            SysType::LenguajeRuntime,
        );
        let lang_ids: Vec<_> = lang.iter().map(|q| q.id).collect();
        assert!(lang_ids.contains(&"sintaxis") && lang_ids.contains(&"toolchain"));
        assert!(!lang_ids.contains(&"syscalls") && !lang_ids.contains(&"arranque"));
        let os = family_questions(Level::Avanzado, Categoria::Sistema, SysType::OsDistro);
        let os_ids: Vec<_> = os.iter().map(|q| q.id).collect();
        assert!(os_ids.contains(&"arranque") && os_ids.contains(&"syscalls"));

        // Plugin pide host; Motor pide UI+plataformas; Framework ni host ni UI.
        let plugin = family_questions(
            Level::Intermedio,
            Categoria::Libreria,
            SysType::PluginExtension,
        );
        assert!(plugin.iter().any(|q| q.id == "host_api"));
        let motor = family_questions(Level::Intermedio, Categoria::Libreria, SysType::MotorEngine);
        assert!(
            motor.iter().any(|q| q.id == "ui_ux") && motor.iter().any(|q| q.id == "plataformas")
        );
        let fw = family_questions(
            Level::Intermedio,
            Categoria::Libreria,
            SysType::FrameworkLibreria,
        );
        assert!(!fw.iter().any(|q| q.id == "host_api") && !fw.iter().any(|q| q.id == "ui_ux"));

        // App pide facturación; Servicio pre-marca Servidor/Nube.
        let app = family_questions(Level::Intermedio, Categoria::Aplicacion, SysType::AppWeb);
        assert!(app.iter().any(|q| q.id == "facturacion"));
        assert!(default_plataformas(SysType::ApiBackend).contains(&Plataforma::ServidorNube));
        // UI solo con UI: Script nunca, Juego sí.
        assert!(!has_ui(SysType::Script));
        assert!(has_ui(SysType::Juego));
        // Stack no se pregunta a Principiante.
        assert!(
            !family_questions(
                Level::Principiante,
                Categoria::Servicio,
                SysType::ApiBackend
            )
            .iter()
            .any(|q| q.id == "stack")
        );

        // OSS: App+OpenSource y Librería+MIT lo son; Librería+Uso interno no.
        assert!(is_oss(
            Categoria::Aplicacion,
            Facturacion::OpenSource,
            Licencia::UsoInterno
        ));
        assert!(is_oss(
            Categoria::Libreria,
            Facturacion::Suscripciones,
            Licencia::Mit
        ));
        assert!(!is_oss(
            Categoria::Libreria,
            Facturacion::Suscripciones,
            Licencia::UsoInterno
        ));
        assert_eq!(oss_questions().len(), 3);
    }

    #[test]
    fn wizard_is_dynamic_by_family() {
        let princ_app = total_steps(
            Level::Principiante,
            Categoria::Aplicacion,
            SysType::AppWeb,
            false,
        );
        let princ_script = total_steps(
            Level::Principiante,
            Categoria::Automatizacion,
            SysType::Script,
            false,
        );
        assert_ne!(princ_app, princ_script, "el total depende de la familia");
        for (level, cat, sys) in [
            (Level::Principiante, Categoria::Aplicacion, SysType::AppWeb),
            (Level::Avanzado, Categoria::Sistema, SysType::DriverFirmware),
        ] {
            let total = total_steps(level, cat, sys, false);
            assert!(step_question(level, cat, sys, false, 0).is_none());
            assert!(is_ai_step(level, cat, sys, false, total - 1));
            // v0.8.2: Cat → Tipo → genéricas (nombre es el 3er paso).
            assert_eq!(step_question(level, cat, sys, false, 1).unwrap().id, "cat");
            assert_eq!(step_question(level, cat, sys, false, 2).unwrap().id, "sys");
            assert_eq!(
                step_question(level, cat, sys, false, 3).unwrap().id,
                "nombre"
            );
            // Genéricas contextualizadas por familia (§1.1: 6 genéricas tras cat/sys).
            let genq = generic_questions(Categoria::Servicio, SysType::ApiBackend);
            assert_eq!(genq[5].id, "funcionalidades");
            assert!(
                genq[5].hint.unwrap().contains("endpoints")
                    || genq[5].hint.unwrap().contains("GET")
            );
            assert_eq!(genq[3].id, "uso_previsto");
            assert_eq!(genq[4].id, "publico_objetivo");
            // El bloque OSS cierra la secuencia antes de la IA.
            let qs = questions(level, cat, sys, false);
            assert_eq!(qs[qs.len() - 3].id, "oss_repo");
            assert_eq!(qs[qs.len() - 1].id, "oss_contrib");
        }
    }

    #[test]
    fn skip_nombre_omits_the_step() {
        // Nombre pre-rellenado (vino de la creación): un paso menos, sin "nombre".
        let (level, cat, sys) = (Level::Intermedio, Categoria::Aplicacion, SysType::AppWeb);
        let full = total_steps(level, cat, sys, false);
        let skipped = total_steps(level, cat, sys, true);
        assert_eq!(full - skipped, 1, "solo se omite el nombre");
        let qs = questions(level, cat, sys, true);
        assert!(
            !qs.iter().any(|q| q.id == "nombre"),
            "sin pregunta de nombre"
        );
        // El orden se mantiene: cat, tipo, descripción...
        assert_eq!(step_question(level, cat, sys, true, 1).unwrap().id, "cat");
        assert_eq!(step_question(level, cat, sys, true, 2).unwrap().id, "sys");
        assert_eq!(
            step_question(level, cat, sys, true, 3).unwrap().id,
            "descripcion"
        );
        assert!(is_ai_step(level, cat, sys, true, skipped - 1));
    }

    #[test]
    fn defaults_are_not_open_source() {
        // Regresión: con Licencia::Mit por defecto, cualquier proyecto
        // Librería/Sistema/Auto/Datos se tomaba como open-source sin elegirlo.
        assert_eq!(Licencia::default(), Licencia::UsoInterno);
        assert!(!is_oss(
            Categoria::Libreria,
            Facturacion::Suscripciones,
            Licencia::default()
        ));
        assert!(!is_oss(
            Categoria::Aplicacion,
            Facturacion::default(),
            Licencia::default()
        ));
        // Solo es OSS si se elige explícitamente.
        assert!(is_oss(
            Categoria::Aplicacion,
            Facturacion::OpenSource,
            Licencia::UsoInterno
        ));
        assert!(is_oss(
            Categoria::Libreria,
            Facturacion::Suscripciones,
            Licencia::Mit
        ));
    }

    #[test]
    fn facturacion_asked_for_app_and_service_only() {
        // Facturación vive en la familia App/Servicio; el resto usa Licencia.
        let app = family_questions(Level::Intermedio, Categoria::Aplicacion, SysType::AppWeb);
        assert!(
            app.iter().any(|q| q.id == "facturacion"),
            "App debe preguntar facturación"
        );
        let svc = family_questions(Level::Intermedio, Categoria::Servicio, SysType::ApiBackend);
        assert!(
            svc.iter().any(|q| q.id == "facturacion"),
            "Servicio debe preguntar facturación"
        );
        let lib = family_questions(
            Level::Intermedio,
            Categoria::Libreria,
            SysType::FrameworkLibreria,
        );
        assert!(
            lib.iter().any(|q| q.id == "licencia") && !lib.iter().any(|q| q.id == "facturacion")
        );
    }

    #[test]
    fn option_lists_are_broad() {
        assert!(Plataforma::ALL.len() >= 9);
        // El escritorio se desglosa por SO.
        for so in [Plataforma::Windows, Plataforma::MacOs, Plataforma::Linux] {
            assert!(Plataforma::ALL.contains(&so), "falta {so}");
        }
        assert!(default_plataformas(SysType::AppEscritorio).contains(&Plataforma::Linux));
        assert!(StackOpt::ALL.len() >= 12);
        assert!(EstiloPreset::ALL.len() >= 7);
        assert!(Facturacion::ALL.len() >= 6);
        assert!(Facturacion::ALL.contains(&Facturacion::OpenSource));
        assert_eq!(Facturacion::OpenSource.to_string(), "Código abierto");
        assert_eq!(Licencia::ALL.len(), 4);
    }
}
