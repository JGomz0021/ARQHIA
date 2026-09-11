//! Eventos y navegación: Message, View, ConfigTab.
//!
//! Solo datos. Sin lógica, sin I/O.

use iced::widget::markdown;

use crate::agent;
use crate::config::{AccentChoice, Density, Provider, TextSize, ThemeMode};
use crate::db::Mode;
use crate::questionnaire::{Interfaz, Publico};

#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum View {
    #[default]
    Home,
    Chat,
    Config,
    Questionnaire,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    Api,
    Apariencia,
    Permisos,
    Proyectos,
    /// v0.7.1: lista de solo lectura de atajos de teclado.
    Atajos,
}

impl ConfigTab {
    pub const ALL: [ConfigTab; 5] = [
        ConfigTab::Api,
        ConfigTab::Apariencia,
        ConfigTab::Permisos,
        ConfigTab::Proyectos,
        ConfigTab::Atajos,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConfigTab::Api => "API",
            ConfigTab::Apariencia => "Apariencia",
            ConfigTab::Permisos => "Permisos",
            ConfigTab::Proyectos => "Proyectos",
            ConfigTab::Atajos => "Atajos",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendPressed,
    StreamChunk(String),
    /// Uso de tokens del turno (input, output, cached, coste real opcional)
    /// al terminar el stream.
    StreamUsage(u32, u32, u32, Option<f64>),
    StreamDone,
    StreamError(String),
    OpenConfig,
    GoChat,
    GoHome,
    ConfigBack,
    ExitApp,
    // v0.4 — home
    ShowCreateModal,
    HideCreateModal,
    CreateNameChanged(String),
    CreatePathChanged(String),
    SubmitCreateProject,
    OpenProject,
    EnterProject(i64),
    FolderPicked(Option<std::path::PathBuf>),
    ProviderPicked(Provider),
    ApiKeyChanged(String),
    BaseUrlChanged(String),
    ModelChanged(String),
    /// Nivel de razonamiento elegido en Config (se guarda con Guardar).
    EditReasoningPicked(String),
    SaveConfig,
    TestConnection,
    TestResult(Result<String, String>),
    UseDefaultBaseUrl,
    UseDefaultModel,
    // v0.2
    NewChat,
    NewChatInProject(i64),
    SelectChat(i64),
    DeleteChat(i64),
    ConfirmDeleteChat,
    CancelDelete,
    ToggleChatMenu(i64),
    ToggleMovePick(i64),
    AssignChatProject {
        chat: i64,
        project: Option<i64>,
    },
    ArchiveChat(i64),
    UnarchiveChat(i64),
    ToggleArchived,
    NewProjectNameChanged(String),
    NewProjectPathChanged(String),
    CreateProject,
    ToggleProjectForm,
    ToggleProject(i64),
    ToggleProjectMenu(i64),
    OpenWorkspaceFolder(i64),
    DeleteProject(i64),
    ConfirmDeleteProject,
    CancelDeleteProject,
    NavigateProject(String),
    ThemePicked(ThemeMode),
    AccentPicked(AccentChoice),
    TextSizePicked(TextSize),
    DensityPicked(Density),
    // v0.3 — agente simple
    WorkspacePathChanged(i64, String),
    AssignWorkspace(i64),
    ClearWorkspace(i64),
    // v0.6 — orquestador paso a paso
    AgentPlan(u64, Result<Vec<agent::WTask>, String>),
    AgentLlm(u64, Result<agent::StepOutcome, String>),
    AgentExecDone(u64, Vec<agent::PendingCall>, Vec<serde_json::Value>, Vec<String>),
    AgentAudit(u64, Result<(String, Vec<String>), String>),
    ApproveTools,
    DenyTools,
    DenyToolsRemember,
    StopAgent,
    // v0.7.1 — modos Chat/Plan/Work + atajos
    ModePicked(Mode),
    /// Planificador de modo Plan listo: solo PLAN.md, sin tools.
    PlanDone(u64, Result<Vec<agent::WTask>, String>),
    /// Botón "Ejecutar plan": pasa a Work y arranca el orquestador.
    ExecutePlan,
    DismissPlan,
    /// Esc: cierra menús y paneles, sin borrar nada.
    CloseOverlays,
    /// Reintenta el último turno de usuario fallido (sin duplicar en DB).
    RetryLast,
    /// Expande/colapsa la caja del feed de actividad.
    ToggleLogExpand,
    // Navegador de modelos (models.dev) + precios.
    OpenModelBrowser,
    CloseModelBrowser,
    /// Al arrancar: carga el catálogo desde cache y, si está vacío, lo descarga.
    InitPricing,
    ModelSearchChanged(String),
    ModelPriceFilterPicked(String),
    ModelOnlyToolsToggled(bool),
    ModelSortPriceToggled(bool),
    RefreshPricing,
    PricingFetched(Result<crate::pricing::Pricing, String>),
    LocalModelsFetched(Result<Vec<String>, String>),
    PickModel(String),
    PermReadToggled(bool),
    PermWriteToggled(bool),
    PermBashToggled(bool),
    PermNetToggled(bool),
    PermInstallToggled(bool),
    PermDomainsChanged(String),
    PermExtraChanged(String),
    SavePermLists,
    LimitItersPicked(usize),
    LimitTasksPicked(usize),
    LimitTimeoutPicked(u64),
    LimitUploadPicked(u64),
    LimitReadPicked(u64),
    LimitTokensPicked(u64),
    LimitHistoryPicked(usize),
    // v0.7 — config por pestañas + uploads
    ConfigTab(ConfigTab),
    ConfigDeleteProject(i64),
    ConfirmConfigDelete,
    CancelConfigDelete,
    UploadFiles(i64),
    FilesPicked(i64, Vec<std::path::PathBuf>),
    DeleteUpload(i64, String),
    // Entrada del chat
    QuickSwitchModel(String),
    /// Nivel de razonamiento elegido desde el composer (persistencia inmediata).
    QuickReasoningPicked(String),
    LinkClicked(markdown::Url),
    // v0.5 — cuestionario
    QNext,
    QBack,
    QCancel,
    QNombreChanged(String),
    QDescChanged(String),
    QUbicChanged(String),
    QObjChanged(String),
    QPublicoPicked(Publico),
    QInterfazPicked(Interfaz),
    FinishQuestionnaire,
}
