//! Handler de Configuración: providers, tema, permisos, pestañas.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;
use crate::config;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenConfig => {
            state.config_from = state.view.clone();
            state.view = View::Config;
            state.status.clear();
            Task::none()
        }
        Message::ConfigBack => {
            // Vuelve a donde estaba (Chat o Home), no siempre a Home
            state.view = state.config_from.clone();
            state.status.clear();
            Task::none()
        }
        Message::ProviderPicked(p) => {
            state.edit_provider = p;
            state.sync_edit_fields();
            state.status.clear();
            Task::none()
        }
        Message::ApiKeyChanged(v) => {
            state.edit_api_key = v;
            Task::none()
        }
        Message::BaseUrlChanged(v) => {
            state.edit_base_url = v;
            Task::none()
        }
        Message::ModelChanged(v) => {
            state.edit_model = v;
            Task::none()
        }
        Message::EditReasoningPicked(v) => {
            state.edit_reasoning = if v == "auto" { String::new() } else { v };
            Task::none()
        }
        Message::SaveConfig => {
            state.config.active = state.edit_provider;
            let target = state.config.active_config_mut();
            target.api_key = state.edit_api_key.trim().to_string();
            target.base_url = state.edit_base_url.trim().to_string();
            target.model = state.edit_model.trim().to_string();
            target.reasoning_effort = state.edit_reasoning.trim().to_string();
            match state.config.save() {
                Ok(()) => state.status = "Configuración guardada.".to_string(),
                Err(e) => state.status = format!("No se pudo guardar: {e}"),
            }
            Task::none()
        }
        Message::TestConnection => {
            if state.testing {
                return Task::none();
            }
            let provider = state.edit_provider;
            let cfg = crate::config::ProviderConfig {
                api_key: state.edit_api_key.trim().to_string(),
                base_url: state.edit_base_url.trim().to_string(),
                model: state.edit_model.trim().to_string(),
                reasoning_effort: state.edit_reasoning.trim().to_string(),
            };
            if provider.requires_key() && cfg.api_key.is_empty() {
                state.status = "Pon la API key antes de probar.".to_string();
                return Task::none();
            }
            if cfg.model.trim().is_empty() {
                state.status = "Pon el modelo antes de probar.".to_string();
                return Task::none();
            }
            state.testing = true;
            state.status = format!("Probando {provider}... (máx 20s)");
            Task::perform(
                async move { crate::llm::test_connection(provider, cfg).await },
                Message::TestResult,
            )
        }
        Message::TestResult(res) => {
            state.testing = false;
            state.status = match res {
                Ok(ok) => ok,
                Err(e) => format!("Falló: {e}"),
            };
            Task::none()
        }
        Message::UseDefaultBaseUrl => {
            state.edit_base_url = config::default_base_url(state.edit_provider);
            Task::none()
        }
        Message::UseDefaultModel => {
            state.edit_model = config::default_model(state.edit_provider);
            Task::none()
        }
        Message::AccentPicked(a) => {
            state.config.appearance.accent = a;
            match state.config.save() {
                Ok(()) => state.status = "Acento aplicado.".to_string(),
                Err(e) => state.status = format!("Acento aplicado pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::TextSizePicked(t) => {
            state.config.appearance.text_size = t;
            match state.config.save() {
                Ok(()) => state.status = "Tamaño aplicado.".to_string(),
                Err(e) => state.status = format!("Tamaño aplicado pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::DensityPicked(d) => {
            state.config.appearance.density = d;
            match state.config.save() {
                Ok(()) => state.status = "Densidad aplicada.".to_string(),
                Err(e) => state.status = format!("Densidad aplicada pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::ThemePicked(m) => {
            state.config.theme = m;
            match state.config.save() {
                Ok(()) => state.status = "Tema aplicado.".to_string(),
                Err(e) => state.status = format!("Tema aplicado pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::PermReadToggled(v) => {
            state.config.permissions.auto_read = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::PermWriteToggled(v) => {
            state.config.permissions.auto_write = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::PermBashToggled(v) => {
            state.config.permissions.auto_bash = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::PermNetToggled(v) => {
            state.config.permissions.auto_net = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::PermInstallToggled(v) => {
            state.config.permissions.auto_install = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::PermDomainsChanged(v) => {
            state.perm_domains = v;
            Task::none()
        }
        Message::PermExtraChanged(v) => {
            state.perm_extra = v;
            Task::none()
        }
        Message::SavePermLists => {
            let domains: Vec<String> = state
                .perm_domains
                .split(',')
                .map(|d| d.trim().to_lowercase())
                .filter(|d| !d.is_empty())
                .collect();
            let mut extra = Vec::new();
            let mut bad = Vec::new();
            for raw in state.perm_extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let expanded = if let Some(rest) = raw.strip_prefix("~/") {
                    std::path::PathBuf::from(
                        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
                    )
                    .join(rest)
                } else {
                    std::path::PathBuf::from(raw)
                };
                match expanded.canonicalize() {
                    Ok(c) if c.is_dir() => extra.push(c),
                    _ => bad.push(raw.to_string()),
                }
            }
            state.config.permissions.net_domains = domains;
            state.config.permissions.extra_paths = extra;
            match state.config.save() {
                Ok(()) => {
                    state.status = if bad.is_empty() {
                        "Listas guardadas.".to_string()
                    } else {
                        format!("Guardadas, pero ignoradas (no existen): {}", bad.join(", "))
                    }
                }
                Err(e) => state.status = format!("No se pudo guardar: {e}"),
            }
            Task::none()
        }
        Message::LimitItersPicked(v) => {
            state.config.limits.max_iters = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitTasksPicked(v) => {
            state.config.limits.max_tasks = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitTimeoutPicked(v) => {
            state.config.limits.bash_timeout_s = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitUploadPicked(v) => {
            state.config.limits.max_upload_mb = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitReadPicked(v) => {
            state.config.limits.max_read_kb = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitTokensPicked(v) => {
            // 0 = sin límite (default); si >0 se recorta a rango sano.
            state.config.limits.max_tokens_turn = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::LimitHistoryPicked(v) => {
            state.config.limits.history_limit = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::ConfigTab(tab) => {
            state.config_tab = tab;
            state.config_pending_delete = None;
            // Staging de listas al abrir Permisos (no se pierde lo guardado).
            state.perm_domains = state.config.permissions.net_domains.join(", ");
            state.perm_extra = state
                .config
                .permissions
                .extra_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Task::none()
        }
        Message::InitPricing => {
            if state.pricing.is_empty() {
                state.models_loading = true;
                state.model_status = "Cargando catálogo de modelos (models.dev)...".to_string();
                Task::perform(
                    async { crate::pricing::Pricing::fetch().await.map(|(p, _)| p) },
                    Message::PricingFetched,
                )
            } else {
                Task::none()
            }
        }
        Message::OpenModelBrowser => {
            state.model_browser = true;
            state.model_search.clear();
            state.model_status.clear();
            state.models_loading = false;
            if state.edit_provider == crate::config::Provider::Local {
                if state.local_models.is_empty() {
                    state.model_status = format!("Consultando modelos en {}...", state.edit_base_url);
                    if !state.edit_base_url.trim().is_empty() {
                        state.models_loading = true;
                        let cfg = edit_cfg(state);
                        return Task::perform(
                            async move { crate::llm::list_local_models(cfg).await },
                            Message::LocalModelsFetched,
                        );
                    }
                }
            } else if state.edit_api_key.trim().is_empty() {
                state.model_status =
                    "Ingresa y guarda la API key del proveedor para ver sus modelos.".to_string();
            } else if state.pricing.is_empty() {
                state.models_loading = true;
                state.model_status = "Descargando catálogo de models.dev...".to_string();
                return Task::perform(
                    async { crate::pricing::Pricing::fetch().await.map(|(p, _)| p) },
                    Message::PricingFetched,
                );
            }
            Task::none()
        }
        Message::CloseModelBrowser => {
            state.model_browser = false;
            Task::none()
        }
        Message::ModelSearchChanged(v) => {
            state.model_search = v;
            Task::none()
        }
        Message::ModelPriceFilterPicked(v) => {
            state.model_price_filter = v;
            Task::none()
        }
        Message::ModelOnlyToolsToggled(v) => {
            state.model_only_tools = v;
            Task::none()
        }
        Message::ModelSortPriceToggled(v) => {
            state.model_sort_price = v;
            Task::none()
        }
        Message::RefreshPricing => {
            state.models_loading = true;
            state.model_status = "Actualizando precios desde models.dev...".to_string();
            Task::perform(
                async { crate::pricing::Pricing::fetch().await.map(|(p, _)| p) },
                Message::PricingFetched,
            )
        }
        Message::PricingFetched(res) => {
            state.models_loading = false;
            match res {
                Ok(p) => {
                    let n = p.len();
                    let provs = p.providers().len();
                    state.pricing = p;
                    state.model_status = format!("{n} modelos de {provs} providers con precios.");
                }
                Err(e) => state.model_status = format!("Error: {e}"),
            }
            Task::none()
        }
        Message::LocalModelsFetched(res) => {
            state.models_loading = false;
            match res {
                Ok(v) => {
                    let n = v.len();
                    state.local_models = v;
                    state.model_status = format!("{n} modelos cargados en LM Studio.");
                }
                Err(e) => state.model_status = format!("Error: {e}"),
            }
            Task::none()
        }
        Message::PickModel(id) => {
            state.edit_model = id.clone();
            state.model_browser = false;
            state.model_status.clear();
            state.status = format!("Modelo elegido: {id}. Pulsa Guardar para aplicarlo.");
            Task::none()
        }
        // Inalcanzable si el dispatch exterior está al día (es total).
        _ => Task::none(),
    }
}

/// ProviderConfig a partir de los campos de edición de Config.
fn edit_cfg(state: &App) -> crate::config::ProviderConfig {
    crate::config::ProviderConfig {
        api_key: state.edit_api_key.trim().to_string(),
        base_url: state.edit_base_url.trim().to_string(),
        model: state.edit_model.trim().to_string(),
        reasoning_effort: state.edit_reasoning.trim().to_string(),
    }
}