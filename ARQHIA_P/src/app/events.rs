//! Eventos y navegación: Message, View, ConfigTab.
//!
//! Solo datos. Sin lógica, sin I/O.

use iced::widget::markdown;

use crate::agent;
use crate::config::{AccentChoice, Density, Provider, TextSize, ThemeMode};
use crate::db::Mode;
use crate::questionnaire::{ArqPreset, EstiloPreset, Facturacion, Level, Plataforma, StackOpt, TipoProyecto};

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
    Git,
    Proyectos,
    /// v0.7.1: lista de solo lectura de atajos de teclado.
    Atajos,
}

impl ConfigTab {
    pub const ALL: [ConfigTab; 6] = [
        ConfigTab::Api,
        ConfigTab::Apariencia,
        ConfigTab::Permisos,
        ConfigTab::Git,
        ConfigTab::Proyectos,
        ConfigTab::Atajos,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConfigTab::Api => "API",
            ConfigTab::Apariencia => "Apariencia",
            ConfigTab::Permisos => "Permisos",
            ConfigTab::Git => "Git",
            ConfigTab::Proyectos => "Proyectos",
            ConfigTab::Atajos => "Atajos",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendPressed,
    /// Chunk de texto del stream (generación para ignorar resultados tardíos).
    StreamChunk(u64, String),
    /// Uso de tokens del turno (input, output, cached, coste real opcional)
    /// al terminar el stream.
    StreamUsage(u64, u32, u32, u32, Option<f64>),
    StreamDone(u64),
    StreamError(u64, String),
    OpenConfig,
    GoChat,
    GoHome,
    ConfigBack,
    ExitApp,
    /// Oculta el aviso de onboarding de API (v0.8, "Configurar después").
    DismissOnboarding,
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
    /// Nivel de razonamiento elegido en Config (se guarda con el perfil).
    EditReasoningPicked(String),
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
    /// Analista dedicado (v0.7.3): brief antes del planner.
    AgentAnalyze(u64, Result<String, String>),
    AgentLlm(u64, Result<agent::StepOutcome, String>),
    AgentExecDone(u64, Vec<agent::PendingCall>, Vec<serde_json::Value>, Vec<String>),
    AgentAudit(u64, Result<(String, Vec<String>, bool), String>),
    ApproveTools,
    DenyTools,
    DenyToolsRemember,
    StopAgent,
    // v0.7.1 — modos Chat/Plan/Work + atajos
    ModePicked(Mode),
    /// Tick de la animación del segmento de modo (v0.8.1, ~60 ms).
    /// Paso 0..=8 (8 = final, apaga); la generación evita que ticks viejos
    /// pisen una animación nueva.
    ModeAnimTick(u64, u8),
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
    /// Ciclos máximos auditor→fix por turno (v0.7.3, 0 = ilimitado).
    LimitFixCyclesPicked(usize),
    // v0.7 — config por pestañas + uploads
    ConfigTab(ConfigTab),
    // v0.7.2 — Git nativo
    GitEnabledToggled(bool),
    GitAutoInitToggled(bool),
    GitBranchModePicked(crate::config::BranchMode),
    GitAutonomyPicked(crate::config::GitAutonomy),
    GitPushToggled(bool),
    GitBaseBranchChanged(String),
    GitWorkBranchChanged(String),
    GitRemoteChanged(String),
    GitPushBranchChanged(String),
    GitAuthorNameChanged(String),
    GitAuthorEmailChanged(String),
    GitSave,
    GitRefreshStatus,
    GitInitWorkspace,
    GitInitDone(Result<String, String>),
    GitPushDone(Result<String, String>),
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
    // v0.8 — cuestionario genérico + por nivel + IA opcional
    QNext,
    QBack,
    QCancel,
    QLevelPicked(Level),
    /// Confirma el cambio de nivel pendiente (descarta respuestas de nivel).
    QLevelConfirm,
    QNombreChanged(String),
    QDescChanged(String),
    QObjChanged(String),
    QFuncChanged(String),
    QEstiloPicked(EstiloPreset),
    QEstiloFreeChanged(String),
    QUiUxChanged(String),
    QTipoPicked(TipoProyecto),
    QPlataformaToggled(Plataforma),
    QFacturacionPicked(Facturacion),
    QStackToggled(StackOpt),
    QStackFreeChanged(String),
    QArqPicked(ArqPreset),
    QArqChanged(String),
    /// Respuesta a la pregunta IA nº N.
    QAiAnswerChanged(usize, String),
    /// Pide al provider activo 3–5 preguntas adicionales.
    QAiGenerate,
    QAiGenerated(Result<Vec<String>, String>),
    FinishQuestionnaire,
    // v0.7.4 — perfiles de modelo con nombre
    ProfilePicked(String),
    ProfileNameChanged(String),
    ProfileSave,
    ProfileDelete(String),
    /// Abre/cierra el menú "···" de un perfil de la lista.
    ProfileMenuToggled(String),
    /// Abre el overlay de edición de un perfil.
    ProfileEdit(String),
    ProfileEditCancel,
    ProfileEditNameChanged(String),
    ProfileEditProviderPicked(Provider),
    ProfileEditApiChanged(String),
    ProfileEditBaseChanged(String),
    ProfileEditModelChanged(String),
    ProfileEditReasoningPicked(String),
    /// Guarda los cambios del overlay de edición.
    ProfileUpdate,
    // v0.7.4 — utilidades de chat
    UndoChat,
    BranchChatFrom(usize),
    ChatTitleFetched(u64, i64, String),
    /// Abre/cierra el menú contextual de un mensaje (clic derecho o ···).
    ChatMsgMenu(usize),
    /// Copia el contenido de un mensaje al portapapeles.
    CopyMsg(usize),
    /// Pide deshacer hasta un mensaje (muestra aviso: borra el resto).
    TruncateRequest(usize),
    ConfirmTruncate,
    CancelTruncate,
    /// Regenera el `session_id` del chat activo (v0.8, menú ⋯ del chat).
    ResetSession,
}
