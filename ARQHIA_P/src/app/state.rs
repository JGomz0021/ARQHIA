//! Estado de la aplicación: struct App + constructores + helpers puros.
//!
//! Sin vistas, sin handlers. Acceso a persistencia solo para carga inicial.

use std::collections::HashMap;

use iced::widget::markdown;

use super::events::{ConfigTab, View};
use super::orchestrator::{Driver, OrchTask};
use crate::agent;
use crate::config::{AppConfig, Provider};
use crate::db::{self, ChatMeta, Project};
use crate::llm::{ChatMsg, Role};
use crate::config::ProviderConfig;
use crate::questionnaire::Answers;

pub struct App {
    pub(crate) input: String,
    pub(crate) messages: Vec<ChatMsg>,
    pub(crate) chats: Vec<ChatMeta>,
    pub(crate) projects: Vec<Project>,
    pub(crate) active_chat: Option<i64>,
    pub(crate) view: View,
    pub(crate) config: AppConfig,
    pub(crate) edit_provider: Provider,
    pub(crate) edit_api_key: String,
    pub(crate) edit_base_url: String,
    pub(crate) edit_model: String,
    pub(crate) edit_reasoning: String,
    pub(crate) status: String,
    pub(crate) streaming: bool,
    pub(crate) testing: bool,
    pub(crate) new_project_name: String,
    pub(crate) new_project_path: String,
    pub(crate) pending_delete: Option<i64>,
    pub(crate) pending_project_delete: Option<i64>,
    /// Proyecto con el menú ⋯ abierto (solo uno a la vez).
    pub(crate) project_menu: Option<i64>,
    /// Chat con el menú ⋯ abierto (solo uno a la vez).
    pub(crate) chat_menu: Option<i64>,
    /// Chat con el selector "mover a proyecto" desplegado.
    pub(crate) move_for: Option<i64>,
    /// Muestra la sección de chats archivados en el sidebar.
    pub(crate) show_archived: bool,
    /// Staging de listas de Permisos (se sincronizan al abrir la pestaña).
    pub(crate) perm_domains: String,
    pub(crate) perm_extra: String,
    pub(crate) collapsed: std::collections::HashSet<i64>,
    pub(crate) show_project_form: bool,
    // v0.3
    pub(crate) agent_running: bool,
    pub(crate) tool_logs: Vec<String>,
    pub(crate) workspace_inputs: HashMap<i64, String>,
    // v0.6 — orquestador: driver paso a paso + tareas visibles
    pub(crate) o_provider: Option<Provider>,
    pub(crate) o_cfg: Option<ProviderConfig>,
    pub(crate) o_ws: Option<std::path::PathBuf>,
    pub(crate) o_chat: Option<i64>,
    pub(crate) o_history: Vec<ChatMsg>,
    pub(crate) driver: Option<Driver>,
    pub(crate) orch_tasks: Vec<OrchTask>,
    pub(crate) worker_answers: Vec<String>,
    pub(crate) fix_cycle: usize,
    pub(crate) pending_calls: Vec<agent::PendingCall>,
    /// Generación del turno: StopAgent la incrementa para invalidar
    /// resultados async tardíos de un turno ya cancelado.
    pub(crate) agent_gen: u64,
    /// Tools denegadas con "no volver a preguntar" en este turno.
    pub(crate) denied_tools: Vec<String>,
    // Markdown parseado en paralelo a `messages` (vacío para mensajes de usuario)
    pub(crate) md: Vec<Vec<markdown::Item>>,
    /// Uso de tokens por mensaje (paralelo a `messages`; solo sesión actual,
    /// no se persiste). Alineado por índice; ausente = sin datos.
    pub(crate) msg_usage: Vec<Option<crate::llm::Usage>>,
    /// Totales de la sesión (todos los chats desde que arrancó la app).
    pub(crate) session_in: u64,
    pub(crate) session_out: u64,
    pub(crate) session_cost: f64,
    /// Feed de actividad expandido (más líneas y más alto).
    pub(crate) log_expanded: bool,
    /// Tokens del último prompt enviado (incluye tools/lecturas del agente).
    pub(crate) context_tokens: u64,
    // Catálogo de modelos/precios (models.dev) + navegador de modelos.
    pub(crate) pricing: crate::pricing::Pricing,
    pub(crate) model_browser: bool,
    pub(crate) model_search: String,
    /// Filtro de precio del navegador: "Todos" | "Gratis" | "≤ $1" | "≤ $5" | "≤ $15".
    pub(crate) model_price_filter: String,
    pub(crate) model_only_tools: bool,
    pub(crate) model_sort_price: bool,
    pub(crate) models_loading: bool,
    pub(crate) local_models: Vec<String>,
    pub(crate) model_status: String,
    // v0.4 — modal crear proyecto
    pub(crate) creating: bool,
    pub(crate) create_name: String,
    pub(crate) create_path: String,
    // Proyecto destino del próximo chat (el chat se crea al enviar el 1er mensaje)
    pub(crate) pending_project: Option<i64>,
    // v0.5 — cuestionario
    pub(crate) q_step: usize,
    pub(crate) q_answers: Answers,
    pub(crate) q_error: String,
    pub(crate) q_project: Option<i64>,
    // Origen para el Volver de Config (Home o Chat)
    pub(crate) config_from: View,
    // v0.7
    pub(crate) config_tab: ConfigTab,
    pub(crate) config_pending_delete: Option<i64>,
    // v0.7.1 — modo Plan: PLAN.md aprobado antes de ejecutar
    pub(crate) plan_md: String,
    pub(crate) show_plan: bool,
}

impl Default for App {
    fn default() -> Self {
        let _ = db::init();
        let config = AppConfig::load();
        // Sin seeds: la lista puede empezar vacía; el chat nace al conversar.
        let chats = db::list_chats().unwrap_or_default();
        let projects = db::list_projects().unwrap_or_default();
        let active_chat = chats.first().map(|c| c.id);
        let messages = active_chat
            .and_then(|id| db::load_chat_history(id, 200).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|(role, content)| ChatMsg {
                role: Role::from_str(&role),
                content,
            })
            .collect();
        let edit_provider = config.active;
        let active = match edit_provider {
            Provider::OpenAI => config.openai.clone(),
            Provider::Anthropic => config.anthropic.clone(),
            Provider::OpenRouter => config.openrouter.clone(),
            Provider::Local => config.local.clone(),
        };
        let mut app = Self {
            input: String::new(),
            messages,
            chats,
            projects,
            active_chat,
            view: View::Home,
            config,
            edit_provider,
            edit_api_key: active.api_key,
            edit_base_url: active.base_url,
            edit_model: active.model,
            edit_reasoning: active.reasoning_effort,
            status: String::new(),
            streaming: false,
            testing: false,
            new_project_name: String::new(),
            new_project_path: String::new(),
            pending_delete: None,
            pending_project_delete: None,
            project_menu: None,
            chat_menu: None,
            move_for: None,
            show_archived: false,
            perm_domains: String::new(),
            perm_extra: String::new(),
            collapsed: std::collections::HashSet::new(),
            show_project_form: false,
            agent_running: false,
            tool_logs: Vec::new(),
            workspace_inputs: HashMap::new(),
            o_provider: None,
            o_cfg: None,
            o_ws: None,
            o_chat: None,
            o_history: Vec::new(),
            driver: None,
            orch_tasks: Vec::new(),
            worker_answers: Vec::new(),
            fix_cycle: 0,
            pending_calls: Vec::new(),
            agent_gen: 0,
            denied_tools: Vec::new(),
            md: Vec::new(),
            msg_usage: Vec::new(),
            session_in: 0,
            session_out: 0,
            session_cost: 0.0,
            log_expanded: false,
            context_tokens: 0,
            pricing: crate::pricing::Pricing::load(),
            model_browser: false,
            model_search: String::new(),
            model_price_filter: "Todos".to_string(),
            model_only_tools: false,
            model_sort_price: false,
            models_loading: false,
            local_models: Vec::new(),
            model_status: String::new(),
            creating: false,
            create_name: String::new(),
            create_path: String::new(),
            pending_project: None,
            q_step: 0,
            q_answers: Answers::default(),
            q_error: String::new(),
            q_project: None,
            config_from: View::Home,
            config_tab: ConfigTab::Api,
            config_pending_delete: None,
            plan_md: String::new(),
            show_plan: false,
        };
        app.reparse_md();
        app
    }
}

impl App {
    pub(crate) fn sync_edit_fields(&mut self) {
        let active = match self.edit_provider {
            Provider::OpenAI => &self.config.openai,
            Provider::Anthropic => &self.config.anthropic,
            Provider::OpenRouter => &self.config.openrouter,
            Provider::Local => &self.config.local,
        };
        self.edit_api_key = active.api_key.clone();
        self.edit_base_url = active.base_url.clone();
        self.edit_model = active.model.clone();
        self.edit_reasoning = active.reasoning_effort.clone();
    }

    pub(crate) fn active_chat_meta(&self) -> Option<&ChatMeta> {
        self.active_chat
            .and_then(|id| self.chats.iter().find(|c| c.id == id))
    }

    /// Modo del chat activo (v0.7.1). Default: Chat (con o sin workspace
    /// se empieza en Chat y se sube a Plan/Work explícitamente).
    pub(crate) fn active_mode(&self) -> db::Mode {
        self.active_chat_meta().map(|c| c.mode).unwrap_or(db::Mode::Chat)
    }

    pub(crate) fn ensure_active_chat(&mut self) {
        if self.active_chat.is_none()
            && let Ok(id) = db::create_chat("Nuevo chat") {
                // Si venimos de un proyecto (Home/creado), el chat nace dentro
                let pid = self.pending_project;
                if let Some(p) = pid {
                    let _ = db::move_chat(id, Some(p));
                }
                self.chats.push(ChatMeta {
                    id,
                    title: "Nuevo chat".to_string(),
                    project_id: pid,
                    archived: false,
                    mode: db::Mode::Chat,
                });
                self.active_chat = Some(id);
                self.messages.clear();
                self.md.clear();
                self.pending_project = None;
            }
    }

    /// Workspace del proyecto del chat activo, si está asignado y existe.
    pub(crate) fn active_workspace(&self) -> Option<std::path::PathBuf> {
        let chat = self.active_chat_meta()?;
        let pid = chat.project_id?;
        let proj = self.projects.iter().find(|p| p.id == pid)?;
        let raw = proj.path.as_ref()?.trim();
        if raw.is_empty() {
            return None;
        }
        let expanded = if let Some(rest) = raw.strip_prefix("~/") {
            format!("{}/{rest}", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        } else {
            raw.to_string()
        };
        let p = std::path::PathBuf::from(expanded);
        if p.is_dir() { Some(p) } else { None }
    }

    pub(crate) fn push_log(&mut self, line: String) {
        self.tool_logs.push(line);
        if self.tool_logs.len() > 50 {
            let excess = self.tool_logs.len() - 50;
            self.tool_logs.drain(..excess);
        }
    }
    /// Re-parsea todo el historial a markdown (tras cargar/cambiar de chat).
    pub(crate) fn reparse_md(&mut self) {
        self.md = self
            .messages
            .iter()
            .map(|m| {
                if m.role == Role::Assistant {
                    markdown::parse(&m.content).collect()
                } else {
                    Vec::new()
                }
            })
            .collect();
        // El uso de tokens no se persiste: al recargar, se limpia.
        self.msg_usage = vec![None; self.messages.len()];
    }

    /// Re-parsea solo el último mensaje (tras cada chunk de stream).
    pub(crate) fn reparse_last_md(&mut self) {
        if let Some((m, slot)) = self.messages.last().zip(self.md.last_mut())
            && m.role == Role::Assistant {
                *slot = markdown::parse(&m.content).collect();
            }
    }
}

/// Limpia el estado de turno/log al cambiar de conversación: la actividad
/// pertenece al chat donde ocurrió, no viaja con el usuario.
pub(crate) fn clear_turn_state(state: &mut App) {
    state.tool_logs.clear();
    state.orch_tasks.clear();
    state.worker_answers.clear();
    state.pending_calls.clear();
    state.denied_tools.clear();
    state.show_plan = false;
    state.plan_md.clear();
    state.chat_menu = None;
    state.move_for = None;
}
