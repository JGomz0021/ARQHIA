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
    /// Generación del stream de chat plano: invalida chunks tardíos tras
    /// Detener o al iniciar otro turno.
    pub(crate) stream_gen: u64,
    /// Momento del último Esc (para el doble Esc = Detener).
    pub(crate) last_esc: Option<std::time::Instant>,
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
    pub(crate) o_mode: db::Mode,
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
    // v0.8 — nivel + IA opcional del cuestionario
    pub(crate) q_level: crate::questionnaire::Level,
    /// Nivel pendiente de confirmar (cambiar con respuestas lo descarta).
    pub(crate) q_pending_level: Option<crate::questionnaire::Level>,
    /// Preguntas IA generadas + respuestas del usuario (paralelos).
    pub(crate) q_ai_questions: Vec<String>,
    pub(crate) q_ai_answers: Vec<String>,
    pub(crate) q_ai_loading: bool,
    pub(crate) q_ai_error: String,
    /// El proyecto se creó en este cuestionario y aún no terminó: si se
    /// cancela, se deshace la creación (v0.8.1, sin proyectos fantasma).
    pub(crate) q_owns_project: bool,
    // Origen para el Volver de Config (Home o Chat)
    pub(crate) config_from: View,
    // v0.7
    pub(crate) config_tab: ConfigTab,
    pub(crate) config_pending_delete: Option<i64>,
    // v0.7.1 — modo Plan: PLAN.md aprobado antes de ejecutar
    pub(crate) plan_md: String,
    pub(crate) show_plan: bool,
    // v0.7.2 — Git: staging de textos + estado del workspace + guardas de turno
    pub(crate) git_base_branch: String,
    pub(crate) git_work_branch: String,
    pub(crate) git_remote: String,
    pub(crate) git_push_branch: String,
    pub(crate) git_author_name: String,
    pub(crate) git_author_email: String,
    pub(crate) git_status: crate::git::WorkspaceStatus,
    /// El árbol estaba limpio al iniciar el turno Work (guarda anti-sucio).
    pub(crate) git_clean_before: bool,
    /// La última auditoría pasó check+test+clippy y quedó limpia.
    pub(crate) git_verify_ok: bool,
    /// El turno se cortó por presupuesto/parada: no se commitea (v0.7.3).
    pub(crate) git_turn_interrupted: bool,
    // v0.8.1 — animación del segmento de modo al cambiar (click o atajo):
    // (instante, paso 0..=8); el paso mueve el padding 2→6→2 por ticks.
    pub(crate) mode_anim: Option<(std::time::Instant, u8)>,
    pub(crate) mode_anim_gen: u64,
    // v0.8 — onboarding de API (Home): aviso ocultado con "después".
    pub(crate) onboarding_dismissed: bool,
    // v0.7.4 — perfiles + utilidades de chat
    pub(crate) profile_name: String,
    /// Perfil con el menú "···" abierto en Config → API (solo uno).
    pub(crate) profile_menu: Option<String>,
    /// Perfil en edición (overlay). None = sin overlay.
    pub(crate) editing_profile: Option<String>,
    pub(crate) eprofile_name: String,
    pub(crate) eprofile_provider: Provider,
    pub(crate) eprofile_api: String,
    pub(crate) eprofile_base: String,
    pub(crate) eprofile_model: String,
    pub(crate) eprofile_reasoning: String,
    /// Timestamps por mensaje (paralelo a `messages`; "" = desconocido).
    pub(crate) msg_times: Vec<String>,
    /// Ids de fila en DB por mensaje (paralelo; 0 = aún no persistido).
    pub(crate) msg_ids: Vec<i64>,
    /// Snapshot de 1 paso para Undo (Ctrl+Z): envío o borrado.
    pub(crate) undo: Option<UndoSnapshot>,
    /// Mensaje con el menú contextual abierto (clic derecho o ···).
    pub(crate) msg_menu: Option<usize>,
    /// Índice con aviso de "deshacer hasta aquí" pendiente de confirmar.
    pub(crate) pending_truncate: Option<usize>,
    /// URLs consultadas vía fetch_url en el turno (bloque Fuentes).
    pub(crate) chat_sources: Vec<String>,
    /// Generación del título IA (invalida resultados tardíos).
    pub(crate) title_gen: u64,
}

/// Snapshot de 1 paso para Undo v0.7.4 (en memoria, alcance acotado).
#[derive(Debug, Clone)]
pub(crate) struct UndoSnapshot {
    pub(crate) chat_id: i64,
    /// Mensajes antes de la acción (para restaurar vista).
    pub(crate) messages: Vec<crate::llm::ChatMsg>,
    pub(crate) msg_times: Vec<String>,
    pub(crate) msg_ids: Vec<i64>,
    /// Cola truncada por "deshacer hasta aquí": (rol, contenido) para
    /// reinsertarla en DB al deshacer con Ctrl+Z.
    pub(crate) truncated_tail: Vec<(String, String)>,
    /// Chat borrado (para restaurar tras ConfirmDeleteChat).
    pub(crate) deleted_chat: Option<crate::db::ChatMeta>,
    pub(crate) deleted_messages: Vec<(String, String, String)>,
}

impl Default for App {
    fn default() -> Self {
        let _ = db::init();
        let config = AppConfig::load();
        // Sin seeds: la lista puede empezar vacía; el chat nace al conversar.
        let chats = db::list_chats().unwrap_or_default();
        let projects = db::list_projects().unwrap_or_default();
        let active_chat = chats.first().map(|c| c.id);
        let full = active_chat
            .and_then(|id| db::load_chat_history_full(id, 200).ok())
            .unwrap_or_default();
        let messages: Vec<ChatMsg> = full
            .iter()
            .map(|m| ChatMsg {
                role: Role::from_str(&m.role),
                content: m.content.clone(),
            })
            .collect();
        let msg_times: Vec<String> = full.iter().map(|m| m.created_at.clone()).collect();
        let msg_ids: Vec<i64> = full.iter().map(|m| m.id).collect();
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
            msg_times,
            msg_ids,
            chats,
            projects,
            active_chat,
            profile_name: String::new(),
            msg_menu: None,
            pending_truncate: None,
            profile_menu: None,
            editing_profile: None,
            eprofile_name: String::new(),
            eprofile_provider: Provider::OpenAI,
            eprofile_api: String::new(),
            eprofile_base: String::new(),
            eprofile_model: String::new(),
            eprofile_reasoning: String::new(),
            undo: None,
            chat_sources: Vec::new(),
            title_gen: 0,
            view: View::Home,
            config,
            edit_provider,
            edit_api_key: active.api_key,
            edit_base_url: active.base_url,
            edit_model: active.model,
            edit_reasoning: active.reasoning_effort,
            status: String::new(),
            streaming: false,
            stream_gen: 0,
            last_esc: None,
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
            o_mode: db::Mode::Chat,
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
            q_level: crate::questionnaire::Level::default(),
            q_pending_level: None,
            q_ai_questions: Vec::new(),
            q_ai_answers: Vec::new(),
            q_ai_loading: false,
            q_ai_error: String::new(),
            q_owns_project: false,
            config_from: View::Home,
            config_tab: ConfigTab::Api,
            config_pending_delete: None,
            plan_md: String::new(),
            show_plan: false,
            git_base_branch: String::new(),
            git_work_branch: String::new(),
            git_remote: String::new(),
            git_push_branch: String::new(),
            git_author_name: String::new(),
            git_author_email: String::new(),
            git_status: crate::git::WorkspaceStatus::default(),
            git_clean_before: true,
            git_verify_ok: false,
            git_turn_interrupted: false,
            onboarding_dismissed: false,
            mode_anim: None,
            mode_anim_gen: 0,
        };
        app.sync_git_staging();
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

    /// Copia la config Git a los campos de edición (v0.7.2).
    pub(crate) fn sync_git_staging(&mut self) {
        let g = self.config.git.clone();
        self.git_base_branch = g.base_branch;
        self.git_work_branch = g.work_branch;
        self.git_remote = g.remote;
        self.git_push_branch = g.push_branch;
        self.git_author_name = g.author_name;
        self.git_author_email = g.author_email;
    }

    /// Recalcula el estado git del workspace activo (v0.7.2).
    pub(crate) fn refresh_git_status(&mut self) {
        self.git_status = match self.active_workspace() {
            Some(ws) => crate::git::workspace_status(&ws),
            None => crate::git::WorkspaceStatus::default(),
        };
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
                    session_id: None,
                });
                self.active_chat = Some(id);
                self.messages.clear();
                self.md.clear();
                self.msg_times.clear();
                self.msg_ids.clear();
                self.msg_usage.clear();
                self.chat_sources.clear();
                self.undo = None;
                self.pending_project = None;
            }
    }

    /// Recarga el chat activo desde DB con ids + timestamps (v0.7.4).
    pub(crate) fn reload_active_chat(&mut self) {
        let Some(id) = self.active_chat else {
            self.messages.clear();
            self.msg_times.clear();
            self.msg_ids.clear();
            self.reparse_md();
            return;
        };
        let full = db::load_chat_history_full(id, 500).unwrap_or_default();
        self.messages = full
            .iter()
            .map(|m| ChatMsg {
                role: Role::from_str(&m.role),
                content: m.content.clone(),
            })
            .collect();
        self.msg_times = full.iter().map(|m| m.created_at.clone()).collect();
        self.msg_ids = full.iter().map(|m| m.id).collect();
        self.reparse_md();
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
        // Alinea paralelos (tiempos/ids) tras cargas externas.
        self.msg_times.resize(self.messages.len(), String::new());
        self.msg_ids.resize(self.messages.len(), 0);
    }

    /// Re-parsea solo el último mensaje (tras cada chunk de stream).
    pub(crate) fn reparse_last_md(&mut self) {
        if let Some((m, slot)) = self.messages.last().zip(self.md.last_mut())
            && m.role == Role::Assistant {
                *slot = markdown::parse(&m.content).collect();
            }
    }

    /// Quita el último mensaje y sus paralelos (md/uso/tiempos/ids) de una
    /// vez: evita desalineados entre `messages` y sus vectores paralelos.
    pub(crate) fn pop_last_message(&mut self) {
        self.messages.pop();
        self.md.pop();
        self.msg_usage.pop();
        self.msg_times.pop();
        self.msg_ids.pop();
    }

    /// Relee ids + created_at desde DB para alinear los paralelos tras un
    /// insert al final (el insert deja id real y timestamp del servidor).
    pub(crate) fn resync_msg_meta(&mut self, chat_id: i64) {
        if let Ok(full) = db::load_chat_history_full(chat_id, 500) {
            let n = self.messages.len();
            if full.len() >= n && n > 0 {
                let tail = &full[full.len() - n..];
                self.msg_ids = tail.iter().map(|m| m.id).collect();
                self.msg_times = tail.iter().map(|m| m.created_at.clone()).collect();
            }
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
    state.chat_sources.clear();
    state.undo = None;
    state.msg_menu = None;
    state.pending_truncate = None;
}
