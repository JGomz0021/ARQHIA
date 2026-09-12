use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    OpenAI,
    Anthropic,
    OpenRouter,
    /// Servidor local OpenAI-compatible (LM Studio en http://localhost:1234).
    /// No exige API key: LM Studio acepta cualquiera.
    Local,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Anthropic => write!(f, "Anthropic"),
            Provider::OpenRouter => write!(f, "OpenRouter"),
            Provider::Local => write!(f, "Local (LM Studio)"),
        }
    }
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::OpenAI,
        Provider::Anthropic,
        Provider::OpenRouter,
        Provider::Local,
    ];

    /// LM Studio no pide key; el resto sí.
    pub fn requires_key(self) -> bool {
        !matches!(self, Provider::Local)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    /// Nivel de razonamiento elegido ("" = auto). Los valores válidos los
    /// determina cada modelo vía models.dev (low/medium/high/max, on/off...).
    #[serde(default)]
    pub reasoning_effort: String,
}

impl ProviderConfig {
    pub fn new(provider: Provider) -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(provider),
            model: default_model(provider),
            reasoning_effort: String::new(),
        }
    }

    /// Como is_configured pero sin exigir key (para el servidor local).
    pub fn is_configured_for(&self, provider: Provider) -> bool {
        if provider.requires_key() && self.api_key.trim().is_empty() {
            return false;
        }
        !self.model.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Permisos de tools del agente (v0.6 + v0.7 Track B).
/// Lo no automático pide aprobación. Lo peligroso nace desactivado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    pub auto_read: bool,
    pub auto_write: bool,
    pub auto_bash: bool,
    #[serde(default)]
    pub auto_net: bool,
    #[serde(default)]
    pub auto_install: bool,
    /// Rutas extra fuera del workspace (validadas con canonicalize).
    #[serde(default)]
    pub extra_paths: Vec<std::path::PathBuf>,
    /// Dominios permitidos para fetch_url (vacío = todos piden permiso).
    #[serde(default)]
    pub net_domains: Vec<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            auto_read: true,
            auto_write: false,
            auto_bash: false,
            auto_net: false,
            auto_install: false,
            extra_paths: Vec::new(),
            net_domains: Vec::new(),
        }
    }
}

/// Límites operativos del agente (v0.7 Track B + v0.7.1 presupuesto/historial).
/// Rangos validados en UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Pasos LLM por turno de worker (2–30).
    pub max_iters: usize,
    /// Tareas del plan por turno (1–5).
    pub max_tasks: usize,
    /// Timeout de comandos bash en segundos (5–120).
    pub bash_timeout_s: u64,
    /// Tope de subida por archivo en MB (1–200).
    pub max_upload_mb: u64,
    /// Tope de lectura por archivo en KB (64–4096).
    pub max_read_kb: u64,
    /// Presupuesto de tokens por turno de worker (v0.7.1).
    /// 0 = sin límite (default). Si >0, al 80% se avisa y al 100% se para.
    #[serde(default)]
    pub max_tokens_turn: u64,
    /// Ventana de historial enviada al modelo (v0.7.1, 5–100, def 20).
    /// Lo más viejo colapsa con marcador en vez de enviarse.
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    /// Ciclos máximos `auditor → fix` por turno Work (v0.7.3).
    /// 0 = ilimitado (default): el bucle sigue hasta quedar verde.
    #[serde(default)]
    pub max_fix_cycles: usize,
}

/// Ignorados por defecto en búsqueda/listado recursivo (v0.7.1).
/// Sin confirmaciones: la optimización sustituye al aviso.
pub fn default_ignores() -> Vec<String> {
    vec![
        "target/".to_string(),
        ".git/".to_string(),
        "node_modules/".to_string(),
        "*.lock".to_string(),
    ]
}

fn default_history_limit() -> usize {
    20
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_iters: 10,
            max_tasks: 3,
            bash_timeout_s: 30,
            max_upload_mb: 50,
            max_read_kb: 256,
            max_tokens_turn: 0,
            history_limit: 20,
            max_fix_cycles: 0,
        }
    }
}

impl Limits {
    /// Recorta a rangos sanos (la UI también valida; esto es el guard).
    pub fn clamped(self) -> Self {
        Self {
            max_iters: self.max_iters.clamp(2, 30),
            max_tasks: self.max_tasks.clamp(1, 5),
            bash_timeout_s: self.bash_timeout_s.clamp(5, 120),
            max_upload_mb: self.max_upload_mb.clamp(1, 200),
            max_read_kb: self.max_read_kb.clamp(64, 4096),
            max_tokens_turn: if self.max_tokens_turn == 0 {
                0
            } else {
                self.max_tokens_turn.clamp(1_000, 500_000)
            },
            history_limit: self.history_limit.clamp(5, 100),
            max_fix_cycles: if self.max_fix_cycles == 0 {
                0
            } else {
                self.max_fix_cycles.clamp(1, 20)
            },
        }
    }

    /// true si el turno tiene presupuesto configurado (≠ 0 = ilimitado).
    pub fn has_token_budget(self) -> bool {
        self.clamped().max_tokens_turn > 0
    }

    /// true si los ciclos de fix son ilimitados (0, default).
    pub fn unlimited_fix_cycles(self) -> bool {
        self.clamped().max_fix_cycles == 0
    }
}

/// Acento de la identidad ARQHIA (v0.7 Track B). Violeta es la identidad
/// por defecto; Teal y Ámbar siguen disponibles en Apariencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccentChoice {
    Teal,
    Amber,
    #[default]
    Violet,
}

impl fmt::Display for AccentChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccentChoice::Teal => write!(f, "Verde azulado"),
            AccentChoice::Amber => write!(f, "Ámbar"),
            AccentChoice::Violet => write!(f, "Violeta"),
        }
    }
}

impl AccentChoice {
    pub const ALL: [AccentChoice; 3] = [AccentChoice::Teal, AccentChoice::Amber, AccentChoice::Violet];
}

/// Tamaño de texto global (v0.7 Track B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextSize {
    Compacto,
    #[default]
    Normal,
    Grande,
}

impl fmt::Display for TextSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextSize::Compacto => write!(f, "Compacto"),
            TextSize::Normal => write!(f, "Normal"),
            TextSize::Grande => write!(f, "Grande"),
        }
    }
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Compacto, TextSize::Normal, TextSize::Grande];

    pub fn scale(self) -> f32 {
        match self {
            TextSize::Compacto => 0.9,
            TextSize::Normal => 1.0,
            TextSize::Grande => 1.18,
        }
    }
}

/// Densidad de la interfaz (v0.7 Track B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Density {
    #[default]
    Comoda,
    Compacta,
}

impl fmt::Display for Density {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Density::Comoda => write!(f, "Cómoda"),
            Density::Compacta => write!(f, "Compacta"),
        }
    }
}

impl Density {
    pub const ALL: [Density; 2] = [Density::Comoda, Density::Compacta];
}

impl Appearance {
    /// true si la densidad es compacta (menos aire en listas y mensajes).
    pub fn compact(self) -> bool {
        matches!(self.density, Density::Compacta)
    }
}

/// Apariencia (v0.7 Track B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Appearance {
    #[serde(default)]
    pub accent: AccentChoice,
    #[serde(default)]
    pub text_size: TextSize,
    #[serde(default)]
    pub density: Density,
}

/// Estrategia de rama de trabajo (v0.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BranchMode {
    /// Una sola rama `work_branch` por workspace (def).
    #[default]
    Single,
    /// `arqhia/<tarea>` por cada tarea (reservado).
    PerTask,
}

impl fmt::Display for BranchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchMode::Single => write!(f, "ARQHIA única"),
            BranchMode::PerTask => write!(f, "Por tarea"),
        }
    }
}

impl BranchMode {
    pub const ALL: [BranchMode; 2] = [BranchMode::Single, BranchMode::PerTask];
}

/// Autonomía git del agente (v0.7.2). Default: commit local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum GitAutonomy {
    /// Solo lectura: no toca el repo.
    ReadOnly,
    /// init/add/commit/stash automáticos.
    #[default]
    CommitLocal,
    /// Además push (si `push_enabled`).
    CommitAndPush,
}

impl fmt::Display for GitAutonomy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitAutonomy::ReadOnly => write!(f, "Solo lectura"),
            GitAutonomy::CommitLocal => write!(f, "Commit local"),
            GitAutonomy::CommitAndPush => write!(f, "Commit y push"),
        }
    }
}

impl GitAutonomy {
    pub const ALL: [GitAutonomy; 3] = [
        GitAutonomy::ReadOnly,
        GitAutonomy::CommitLocal,
        GitAutonomy::CommitAndPush,
    ];
}

/// Configuración de Git (v0.7.2). Repo por workspace, rama de trabajo
/// dedicada y base protegida. `#[serde(default)]` migra configs viejas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GitConfig {
    /// Inicia git en cada workspace (def true).
    pub enabled: bool,
    /// `git init` si el workspace aún no es repo (def true).
    pub auto_init: bool,
    /// Rama base protegida (def "main").
    pub base_branch: String,
    /// Rama de trabajo del agente (def "ARQHIA").
    pub work_branch: String,
    pub branch_mode: BranchMode,
    pub autonomy: GitAutonomy,
    /// Habilita el push a GitHub (def false; pide aprobación).
    pub push_enabled: bool,
    /// Remoto (def "origin").
    pub remote: String,
    /// Rama de push ("" = igual que work_branch).
    pub push_branch: String,
    /// Ramas intocables (def ["main", "master"]).
    pub protected: Vec<String>,
    /// Autor de commits ("" = identidad global de git).
    pub author_name: String,
    pub author_email: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_init: true,
            base_branch: "main".to_string(),
            work_branch: "ARQHIA".to_string(),
            branch_mode: BranchMode::Single,
            autonomy: GitAutonomy::CommitLocal,
            push_enabled: false,
            remote: "origin".to_string(),
            push_branch: String::new(),
            protected: vec!["main".to_string(), "master".to_string()],
            author_name: String::new(),
            author_email: String::new(),
        }
    }
}

/// Limpia un nombre de rama: sin espacios ni `..`; vacío -> fallback.
fn sanitize_branch(raw: &str, fallback: &str) -> String {
    let cleaned: String = raw.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned = cleaned.replace("..", "");
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

impl GitConfig {
    /// Recorta valores inseguros y garantiza que `work_branch` no esté
    /// protegida (cae a "ARQHIA"). Nunca deja `main`/`master` tocables.
    pub fn validated(mut self) -> Self {
        self.base_branch = sanitize_branch(&self.base_branch, "main");
        self.work_branch = sanitize_branch(&self.work_branch, "ARQHIA");
        self.remote = sanitize_branch(&self.remote, "origin");
        self.push_branch = sanitize_branch(&self.push_branch, "");
        self.author_name = self.author_name.trim().to_string();
        self.author_email = self.author_email.trim().to_string();
        self.protected = self
            .protected
            .iter()
            .map(|b| sanitize_branch(b, ""))
            .filter(|b| !b.is_empty())
            .collect();
        if self.protected.is_empty() {
            self.protected = vec!["main".to_string(), "master".to_string()];
        }
        // La rama de trabajo jamás puede ser la base ni una protegida.
        if self.work_branch == self.base_branch
            || self.protected.iter().any(|p| p == &self.work_branch)
        {
            self.work_branch = "ARQHIA".to_string();
        }
        self
    }

    /// true si la política permite push automático (CommitAndPush + flag).
    pub fn auto_push(&self) -> bool {
        matches!(self.autonomy, GitAutonomy::CommitAndPush) && self.push_enabled
    }

    /// Rama destino del push: `push_branch` o `work_branch` si vacía.
    pub fn push_target(&self) -> &str {
        if self.push_branch.trim().is_empty() {
            &self.work_branch
        } else {
            &self.push_branch
        }
    }

    /// Autor (nombre, email) si ambos están configurados.
    pub fn author(&self) -> Option<(String, String)> {
        if self.author_name.trim().is_empty() || self.author_email.trim().is_empty() {
            None
        } else {
            Some((self.author_name.trim().to_string(), self.author_email.trim().to_string()))
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeMode::Dark => write!(f, "Oscuro"),
            ThemeMode::Light => write!(f, "Claro"),
        }
    }
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 2] = [ThemeMode::Dark, ThemeMode::Light];
}

/// Perfil de modelo con nombre visible (v0.7.4): snapshot de provider +
/// credenciales + modelo + nivel, elegible por nombre en Config y composer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub reasoning_effort: String,
}

impl ModelProfile {
    pub fn new(name: &str, provider: Provider, cfg: &ProviderConfig) -> Self {
        let base = name.trim();
        let stem = if base.is_empty() { "Perfil".to_string() } else { base.to_string() };
        // id estable y único sin dependencias externas: slug + nanos.
        let slug: String = stem
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self {
            id: format!("{}-{nanos}", if slug.is_empty() { "perfil".to_string() } else { slug }),
            name: stem,
            provider,
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
            model: cfg.model.clone(),
            reasoning_effort: cfg.reasoning_effort.clone(),
        }
    }

    /// Etiqueta visible en selectores: "Nombre (Provider · modelo)".
    pub fn label(&self) -> String {
        let m = if self.model.trim().is_empty() {
            "(elige modelo)".to_string()
        } else {
            self.model.clone()
        };
        format!("{} ({} · {m})", self.name, self.provider)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active: Provider,
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub openrouter: ProviderConfig,
    #[serde(default = "default_local_cfg")]
    pub local: ProviderConfig,
    /// Perfiles guardados con nombre (v0.7.4). Vacío en configs viejas.
    #[serde(default)]
    pub model_profiles: Vec<ModelProfile>,
    /// Id del perfil activo (None = sin perfil, se usa `active` + configs).
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub appearance: Appearance,
    /// Configuración de Git (v0.7.2).
    #[serde(default)]
    pub git: GitConfig,
    /// Aviso de privacidad del workspace ya mostrado (v0.7 Track B).
    #[serde(default)]
    pub privacy_notice_shown: bool,
    /// Patrones ignorados en búsqueda/listado (v0.7.1). Vacío = defaults.
    /// Si el usuario los personaliza se respetan; si no, `default_ignores()`.
    #[serde(default)]
    pub search_ignores: Vec<String>,
    /// Migración de permisos peligrosos aplicada (v0.7 Track B).
    /// Ausente en configs viejas -> al cargar se fuerzan a false una vez.
    #[serde(default)]
    pub perms_migrated: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active: Provider::OpenAI,
            openai: ProviderConfig::new(Provider::OpenAI),
            anthropic: ProviderConfig::new(Provider::Anthropic),
            openrouter: ProviderConfig::new(Provider::OpenRouter),
            local: ProviderConfig::new(Provider::Local),
            model_profiles: Vec::new(),
            active_profile: None,
            theme: ThemeMode::Dark,
            permissions: Permissions::default(),
            limits: Limits::default(),
            appearance: Appearance::default(),
            git: GitConfig::default(),
            privacy_notice_shown: false,
            search_ignores: Vec::new(),
            perms_migrated: true,
        }
    }
}

impl AppConfig {
    pub fn active_config(&self) -> ProviderConfig {
        match self.active {
            Provider::OpenAI => self.openai.clone(),
            Provider::Anthropic => self.anthropic.clone(),
            Provider::OpenRouter => self.openrouter.clone(),
            Provider::Local => self.local.clone(),
        }
    }

    pub fn active_config_mut(&mut self) -> &mut ProviderConfig {
        match self.active {
            Provider::OpenAI => &mut self.openai,
            Provider::Anthropic => &mut self.anthropic,
            Provider::OpenRouter => &mut self.openrouter,
            Provider::Local => &mut self.local,
        }
    }

    /// Perfil activo por id, si existe.
    pub fn profile_by_id(&self, id: &str) -> Option<&ModelProfile> {
        self.model_profiles.iter().find(|p| p.id == id)
    }

    /// Garantiza al menos un perfil ("Perfil por defecto" desde el activo).
    /// Migración v0.7.4: configs viejas sin perfiles no se rompen.
    pub fn ensure_profiles(&mut self) {
        if self.model_profiles.is_empty() {
            let cfg = self.active_config();
            let mut p = ModelProfile::new("Perfil por defecto", self.active, &cfg);
            // id estable para la migración (no depende del reloj en tests).
            p.id = "default".to_string();
            self.model_profiles.push(p);
            self.active_profile = Some("default".to_string());
        }
        if self.active_profile.is_none()
            && let Some(first) = self.model_profiles.first() {
                self.active_profile = Some(first.id.clone());
            }
    }

    /// Aplica un perfil al activo (provider + credenciales en sus slots).
    /// Devuelve false si el id no existe.
    pub fn apply_profile(&mut self, id: &str) -> bool {
        let Some(p) = self.profile_by_id(id).cloned() else {
            return false;
        };
        self.active = p.provider;
        let target = self.active_config_mut();
        target.api_key = p.api_key.clone();
        target.base_url = p.base_url.clone();
        target.model = p.model.clone();
        target.reasoning_effort = p.reasoning_effort.clone();
        self.active_profile = Some(p.id);
        true
    }

    pub fn load() -> Self {
        let path = config_path();
        let mut cfg: Self = match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        cfg.ensure_profiles();
        // Migración v0.7 Track B: configs guardadas antes no conocen los
        // permisos peligrosos -> nacen desactivados una sola vez. Después se
        // respeta lo que el usuario elija (el flag queda persistido al guardar).
        if !cfg.perms_migrated {
            cfg.permissions.auto_net = false;
            cfg.permissions.auto_install = false;
            cfg.perms_migrated = true;
        }
        cfg.limits = cfg.limits.clamped();
        cfg.git = cfg.git.validated();
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// Patrones efectivos de ignorados (v0.7.1): los del usuario si los
    /// personalizó, o los defaults si la lista está vacía.
    pub fn effective_ignores(&self) -> Vec<String> {
        if self.search_ignores.is_empty() {
            default_ignores()
        } else {
            self.search_ignores.clone()
        }
    }
}

pub fn default_base_url(provider: Provider) -> String {
    match provider {
        Provider::OpenAI => "https://api.openai.com".to_string(),
        Provider::Anthropic => "https://api.anthropic.com".to_string(),
        Provider::OpenRouter => "https://openrouter.ai".to_string(),
        Provider::Local => "http://localhost:1234".to_string(),
    }
}

pub fn default_model(provider: Provider) -> String {
    match provider {
        Provider::OpenAI => "gpt-4o-mini".to_string(),
        Provider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
        Provider::OpenRouter => "openai/gpt-4o-mini".to_string(),
        // LM Studio: el id exacto del modelo cargado (se ve en la app de LM Studio)
        Provider::Local => "".to_string(),
    }
}

fn default_local_cfg() -> ProviderConfig {
    ProviderConfig::new(Provider::Local)
}

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("arqhia")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_display() {
        assert_eq!(Provider::OpenAI.to_string(), "OpenAI");
        assert_eq!(Provider::Anthropic.to_string(), "Anthropic");
        assert_eq!(Provider::OpenRouter.to_string(), "OpenRouter");
        assert_eq!(Provider::Local.to_string(), "Local (LM Studio)");
        assert!(!Provider::Local.requires_key());
        assert!(Provider::OpenAI.requires_key());
    }

    #[test]
    fn default_urls_not_empty() {
        for p in Provider::ALL {
            assert!(!default_base_url(p).is_empty());
            if p != Provider::Local {
                assert!(!default_model(p).is_empty());
            }
        }
        assert_eq!(default_base_url(Provider::Local), "http://localhost:1234");
    }

    #[test]
    fn local_needs_model_but_no_key() {
        let mut c = ProviderConfig::new(Provider::Local);
        assert!(!c.is_configured_for(Provider::Local)); // falta modelo
        c.model = "qwen3-8b".to_string();
        assert!(c.is_configured_for(Provider::Local)); // key vacía OK
        assert!(!c.is_configured_for(Provider::OpenAI)); // con key exige key
        c.api_key = "sk-x".to_string();
        assert!(c.is_configured_for(Provider::OpenAI));
    }

    #[test]
    fn config_roundtrip_toml() {
        let mut cfg = AppConfig {
            active: Provider::Anthropic,
            ..Default::default()
        };
        cfg.openai.api_key = "sk-test".to_string();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: AppConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.active, Provider::Anthropic);
        assert_eq!(back.openai.api_key, "sk-test");
    }

    #[test]
    fn dangerous_permissions_default_off_and_migrate_once() {
        // Config vieja sin las claves nuevas: todo peligroso OFF.
        let old = r#"
active = "OpenAI"
[openai]
api_key = ""
base_url = "https://api.openai.com"
model = "gpt-4o-mini"
[anthropic]
api_key = ""
base_url = "https://api.anthropic.com"
model = "x"
[openrouter]
api_key = ""
base_url = "https://openrouter.ai"
model = "y"
"#;
        let mut cfg: AppConfig = toml::from_str(old).unwrap();
        assert!(!cfg.perms_migrated);
        // Simula load(): migración una sola vez…
        if !cfg.perms_migrated {
            cfg.permissions.auto_net = false;
            cfg.permissions.auto_install = false;
            cfg.perms_migrated = true;
        }
        assert!(!cfg.permissions.auto_net);
        assert!(!cfg.permissions.auto_install);
        assert!(cfg.permissions.extra_paths.is_empty());
        // …y después respeta la elección explícita del usuario.
        cfg.permissions.auto_net = true;
        assert!(cfg.permissions.auto_net);
        // Límites recortados a rango.
        let lim = Limits { max_iters: 99, max_tasks: 0, bash_timeout_s: 1, max_upload_mb: 999, max_read_kb: 1, max_tokens_turn: 0, history_limit: 0, max_fix_cycles: 0 };
        let c = lim.clamped();
        assert_eq!((c.max_iters, c.max_tasks, c.bash_timeout_s, c.max_upload_mb, c.max_read_kb), (30, 1, 5, 200, 64));
        assert_eq!(c.max_tokens_turn, 0, "0 = sin límite, se respeta");
        assert_eq!(c.history_limit, 5, "historial recorta al mínimo");
        assert_eq!(c.max_fix_cycles, 0, "0 = ciclos ilimitados, se respeta");
        let budgeted = Limits { max_tokens_turn: 999_999, history_limit: 200, max_fix_cycles: 99, ..Limits::default() };
        let cb = budgeted.clamped();
        assert_eq!(cb.max_tokens_turn, 500_000);
        assert_eq!(cb.history_limit, 100);
        assert_eq!(cb.max_fix_cycles, 20);
        assert!(!Limits::default().has_token_budget());
        assert!(Limits { max_tokens_turn: 5000, ..Limits::default() }.has_token_budget());
        assert!(Limits::default().unlimited_fix_cycles());
        assert!(!Limits { max_fix_cycles: 3, ..Limits::default() }.unlimited_fix_cycles());
    }

    #[test]
    fn token_budget_and_ignores_defaults() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.limits.max_tokens_turn, 0);
        assert_eq!(cfg.limits.history_limit, 20);
        assert_eq!(cfg.effective_ignores(), default_ignores());
        assert!(default_ignores().iter().any(|p| p.contains("target")));
        // Config vieja sin las claves: serde defaults las rellenan.
        let old = r#"
active = "OpenAI"
[openai]
api_key = ""
base_url = "https://api.openai.com"
model = "gpt-4o-mini"
[anthropic]
api_key = ""
base_url = "https://api.anthropic.com"
model = "x"
[openrouter]
api_key = ""
base_url = "https://openrouter.ai"
model = "y"
"#;
        let back: AppConfig = toml::from_str(old).unwrap();
        assert_eq!(back.limits.max_tokens_turn, 0);
        assert_eq!(back.limits.history_limit, 20);
        assert!(back.search_ignores.is_empty());
    }

    #[test]
    fn legacy_config_without_theme_defaults_to_dark() {
        // Configs guardadas en v0.1 no tienen `theme` — no deben perderse
        let s = r#"
active = "OpenAI"
[openai]
api_key = "gsk-test"
base_url = "https://api.groq.com/openai"
model = "llama-3.1-8b-instant"
[anthropic]
api_key = ""
base_url = "https://api.anthropic.com"
model = "claude-3-5-sonnet-20241022"
[openrouter]
api_key = ""
base_url = "https://openrouter.ai"
model = "openai/gpt-4o-mini"
"#;
        let back: AppConfig = toml::from_str(s).unwrap();
        assert_eq!(back.theme, ThemeMode::Dark);
        assert_eq!(back.openai.api_key, "gsk-test");
    }

    #[test]
    fn git_defaults_migrate_and_roundtrip() {
        // Defaults v0.7.2.
        let g = GitConfig::default();
        assert!(g.enabled && g.auto_init);
        assert_eq!(g.base_branch, "main");
        assert_eq!(g.work_branch, "ARQHIA");
        assert_eq!(g.autonomy, GitAutonomy::CommitLocal);
        assert!(!g.push_enabled);
        assert_eq!(g.protected, vec!["main", "master"]);
        assert!(!g.auto_push());
        assert_eq!(g.push_target(), "ARQHIA");
        // Config vieja sin [git]: serde default no rompe.
        let old = r#"
active = "OpenAI"
[openai]
api_key = ""
base_url = "https://api.openai.com"
model = "gpt-4o-mini"
[anthropic]
api_key = ""
base_url = "https://api.anthropic.com"
model = "x"
[openrouter]
api_key = ""
base_url = "https://openrouter.ai"
model = "y"
"#;
        let back: AppConfig = toml::from_str(old).unwrap();
        assert!(back.git.enabled);
        assert_eq!(back.git.work_branch, "ARQHIA");
        // Roundtrip toml.
        let mut cfg = AppConfig::default();
        cfg.git.autonomy = GitAutonomy::CommitAndPush;
        cfg.git.push_enabled = true;
        cfg.git.push_branch = "arqhia".to_string();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let again: AppConfig = toml::from_str(&s).unwrap();
        assert_eq!(again.git, cfg.git.validated());
        assert!(again.git.auto_push());
        assert_eq!(again.git.push_target(), "arqhia");
    }

    #[test]
    fn git_validation_rejects_unsafe_names_and_protected_work() {
        // Espacios y `..` fuera; work_branch protegida -> cae a ARQHIA.
        let g = GitConfig {
            base_branch: "  ma in  ".to_string(),
            work_branch: "main".to_string(),
            remote: "ori gin".to_string(),
            push_branch: "a..b".to_string(),
            protected: vec![" main ".to_string(), "".to_string()],
            ..GitConfig::default()
        }
        .validated();
        assert_eq!(g.base_branch, "main");
        assert_eq!(g.remote, "origin");
        assert_eq!(g.push_branch, "ab");
        assert_eq!(g.work_branch, "ARQHIA", "work_branch no puede ser la base/protegida");
        assert_eq!(g.protected, vec!["main"]);
        // Autonomía: solo CommitAndPush + push_enabled habilita auto push.
        let partial = GitConfig {
            autonomy: GitAutonomy::CommitAndPush,
            push_enabled: false,
            ..GitConfig::default()
        };
        assert!(!partial.auto_push());
        let readonly = GitConfig { autonomy: GitAutonomy::ReadOnly, ..GitConfig::default() };
        assert!(!readonly.auto_push());
        // Autor solo con nombre+email.
        assert!(GitConfig::default().author().is_none());
        let with_author = GitConfig {
            author_name: "Ana".to_string(),
            author_email: "ana@x.dev".to_string(),
            ..GitConfig::default()
        };
        assert_eq!(with_author.author().unwrap(), ("Ana".to_string(), "ana@x.dev".to_string()));
    }

    #[test]
    fn profiles_migrate_apply_and_roundtrip() {
        // Config vieja sin perfiles: ensure crea "Perfil por defecto".
        let old = r#"
active = "OpenAI"
[openai]
api_key = "sk-x"
base_url = "https://api.openai.com"
model = "gpt-4o-mini"
[anthropic]
api_key = ""
base_url = "https://api.anthropic.com"
model = "x"
[openrouter]
api_key = ""
base_url = "https://openrouter.ai"
model = "y"
"#;
        let mut back: AppConfig = toml::from_str(old).unwrap();
        assert!(back.model_profiles.is_empty());
        back.ensure_profiles();
        assert_eq!(back.model_profiles.len(), 1);
        assert_eq!(back.model_profiles[0].name, "Perfil por defecto");
        assert_eq!(back.active_profile.as_deref(), Some("default"));
        // Guardar 2 perfiles y alternar.
        let cfg2 = ProviderConfig { api_key: "k2".to_string(), base_url: "https://x".to_string(), model: "m2".to_string(), reasoning_effort: String::new() };
        let mut p2 = ModelProfile::new("Potente", Provider::Anthropic, &cfg2);
        p2.id = "p2".to_string();
        back.model_profiles.push(p2);
        assert!(back.apply_profile("p2"));
        assert_eq!(back.active, Provider::Anthropic);
        assert_eq!(back.active_config().model, "m2");
        assert!(!back.apply_profile("inexistente"));
        // Roundtrip toml conserva perfiles.
        let s = toml::to_string_pretty(&back).unwrap();
        let again: AppConfig = toml::from_str(&s).unwrap();
        assert_eq!(again.model_profiles.len(), 2);
        assert_eq!(again.active_profile.as_deref(), Some("p2"));
        // Label visible con nombre, no id crudo.
        assert!(again.model_profiles[1].label().starts_with("Potente (Anthropic"));
    }

    #[test]
    fn switching_profiles_restores_full_connection() {
        // El perfil es la unidad completa: provider+api+base+modelo+nivel.
        let mut cfg = AppConfig::default();
        cfg.ensure_profiles();
        let a = ProviderConfig {
            api_key: "sk-rapido".to_string(),
            base_url: "https://api.openai.com".to_string(),
            model: "gpt-4o-mini".to_string(),
            reasoning_effort: String::new(),
        };
        let b = ProviderConfig {
            api_key: "sk-potente".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            reasoning_effort: "high".to_string(),
        };
        let mut pa = ModelProfile::new("Rápido", Provider::OpenAI, &a);
        pa.id = "rapido".to_string();
        let mut pb = ModelProfile::new("Potente", Provider::Anthropic, &b);
        pb.id = "potente".to_string();
        cfg.model_profiles = vec![pa, pb];
        assert!(cfg.apply_profile("potente"));
        assert_eq!(cfg.active, Provider::Anthropic);
        assert_eq!(cfg.active_config().api_key, "sk-potente");
        assert_eq!(cfg.active_config().base_url, "https://api.anthropic.com");
        assert_eq!(cfg.active_config().model, "claude-3-5-sonnet-20241022");
        assert_eq!(cfg.active_config().reasoning_effort, "high");
        assert!(cfg.apply_profile("rapido"));
        assert_eq!(cfg.active, Provider::OpenAI);
        assert_eq!(cfg.active_config().api_key, "sk-rapido");
        assert_eq!(cfg.active_config().base_url, "https://api.openai.com");
        assert_eq!(cfg.active_config().model, "gpt-4o-mini");
    }
}
