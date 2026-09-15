//! Eventos y navegación: Message, View, ConfigTab.
//!
//! Solo datos. Sin lógica, sin I/O.

use iced::widget::markdown;

use crate::agent;
use crate::config::{AccentChoice, Density, Provider, TextSize, ThemeMode};
use crate::db::Mode;
use crate::questionnaire::{ApiStyle, ArchOpt, AuthKind, Categoria, EstiloPreset, Facturacion, Level, Licencia, Plataforma, SemverOpt, StackOpt, SysType, Trigger};

#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum View {
    #[default]
    Home,
    Chat,
    Config,
    Questionnaire,
    /// v0.8.2: pantalla de carga MVP (generación sin pasar por el chat).
    Generating,
    /// v0.9: panel del STACK local (búsqueda + preview + guardar/valorar).
    Stack,
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
    /// v0.9 Track B: consentimiento + identidad + uso.
    Stack,
    /// v0.9 Track C: administración de skills.
    Skills,
}

impl ConfigTab {
    pub const ALL: [ConfigTab; 8] = [
        ConfigTab::Api,
        ConfigTab::Apariencia,
        ConfigTab::Permisos,
        ConfigTab::Git,
        ConfigTab::Proyectos,
        ConfigTab::Atajos,
        ConfigTab::Stack,
        ConfigTab::Skills,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConfigTab::Api => "API",
            ConfigTab::Apariencia => "Apariencia",
            ConfigTab::Permisos => "Permisos",
            ConfigTab::Git => "Git",
            ConfigTab::Proyectos => "Proyectos",
            ConfigTab::Atajos => "Atajos",
            ConfigTab::Stack => "STACK",
            ConfigTab::Skills => "Skills",
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
    /// Crear proyecto sin pasar por el cuestionario (v0.9.2 tweak).
    SubmitCreateProjectSkip,
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
    /// Crear proyecto sin cuestionario desde la sidebar.
    CreateProjectSkip,
    ToggleProjectForm,
    ToggleProject(i64),
    ToggleProjectMenu(i64),
    /// Colapsa/expande la sección Proyectos de la sidebar.
    ToggleProjectsSection,
    /// Mostrar/ocultar chats sueltos más allá de 8 (sidebar compacta).
    ToggleLooseChats,
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
    /// Re-análisis del ciclo de fix (v0.9.1): el analista revisa el TEMP.md
    /// del auditor antes de lanzar el worker de fixes.
    AgentReanalyze(u64, Result<String, String>),
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
    /// El planner puede consultar docs externos al diseñar el plan
    /// (v0.9.1, OFF por defecto: si no, pide permiso Net la 1ª vez).
    PermPlannerNetToggled(bool),
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
    QUsoPrevistoChanged(String),
    QPublicoChanged(String),
    QFuncChanged(String),
    QEstiloPicked(EstiloPreset),
    QEstiloFreeChanged(String),
    QUiUxChanged(String),
    QCatPicked(Categoria),
    QSysPicked(SysType),
    QPlataformaToggled(Plataforma),
    QFacturacionPicked(Facturacion),
    QLicenciaPicked(Licencia),
    QApiStylePicked(ApiStyle),
    QAuthPicked(AuthKind),
    QTriggerPicked(Trigger),
    QSemverPicked(SemverOpt),
    QArchToggled(ArchOpt),
    QStackToggled(StackOpt),
    QStackFreeChanged(String),
    QEndpointsChanged(String),
    QEscalaChanged(String),
    QApiPublicaChanged(String),
    QEjemplosChanged(String),
    QArranqueChanged(String),
    QCompatChanged(String),
    QInputsChanged(String),
    QIdempotenciaChanged(String),
    QDatasetChanged(String),
    QPipelineChanged(String),
    QModeloEvalChanged(String),
    QSintaxisChanged(String),
    QToolchainChanged(String),
    QSyscallsChanged(String),
    QHostApiChanged(String),
    QOssRepoChanged(String),
    QOssGobiernoChanged(String),
    QOssContribChanged(String),
    /// Respuesta a la pregunta IA nº N.
    QAiAnswerChanged(usize, String),
    /// Pide al provider activo 5–10 preguntas adicionales.
    QAiGenerate,
    QAiGenerated(Result<Vec<String>, String>),
    /// Tab / Shift+Tab entre respuestas IA (adelante/atrás).
    QAiFocusCycle(bool),
    FinishQuestionnaire,
    /// v0.8.2: la pantalla de carga terminó (archivos parseados o error).
    /// La generación invalida resultados tardíos tras cancelar.
    GenDone(u64, Result<Vec<(String, String)>, String>),
    /// v0.8.2: Detener la generación MVP y volver al cuestionario.
    GenCancel,
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
    // v0.9 — STACK local (Track A: panel; Track B: consentimiento/identidad)
    OpenStack,
    StackBack,
    StackQueryChanged(String),
    StackTagsChanged(String),
    StackSearch,
    StackSelect(i64),
    StackSaveTitleChanged(String),
    StackSaveTagsChanged(String),
    StackSaveLangChanged(String),
    StackSaveLicensePicked(String),
    StackSaveCodeChanged(String),
    StackSave,
    StackOpinionChanged(String),
    /// Valora el snippet seleccionado con 1–5 estrellas (+ opinión).
    StackRate(u8),
    /// Copia el código seleccionado al workspace activo como archivo.
    StackCopyToWs,
    /// Prepara un pedido al agente para usar el snippet seleccionado.
    StackAskAgent,
    StackBugChanged(String),
    StackReportBug,
    StackUseToggled(bool),
    StackShareLocalToggled(bool),
    StackShareCloudToggled(bool),
    IdentityNameChanged(String),
    IdentityEmailChanged(String),
    IdentitySave,
    // v0.9 Track C — skills
    SkillsReload,
    SkillsDelete(String),
}
