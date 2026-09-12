//! Handler de Configuración: providers, tema, permisos, pestañas.
//!
//! Un brazo por variante de Message. Efectos vía módulos de dominio.
//! Sin widgets (el render vive en views/).

use iced::Task;

use crate::app::state::App;
use crate::app::Message;
use crate::app::View;
use crate::app::ConfigTab;
use crate::config;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenConfig => {
            // No pisar el origen si ya estamos en Config (Ctrl+, repetido):
            // si no, "Volver" quedaría atrapado en Config.
            if state.view != View::Config {
                state.config_from = state.view.clone();
            }
            state.view = View::Config;
            state.status.clear();
            state.config.ensure_profiles();
            state.profile_name = state
                .config
                .active_profile
                .as_deref()
                .and_then(|id| state.config.profile_by_id(id))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            state.profile_menu = None;
            state.editing_profile = None;
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
        Message::ProfilePicked(key) => {
            // Acepta id o label ("Nombre (Provider · modelo)") o nombre visible.
            let id = state
                .config
                .profile_by_id(&key)
                .map(|p| p.id.clone())
                .or_else(|| {
                    state
                        .config
                        .model_profiles
                        .iter()
                        .find(|p| p.label() == key || p.name == key)
                        .map(|p| p.id.clone())
                });
            match id {
                Some(pid) if state.config.apply_profile(&pid) => {
                    state.edit_provider = state.config.active;
                    state.sync_edit_fields();
                    state.profile_name = state.config.profile_by_id(&pid).map(|p| p.name.clone()).unwrap_or_default();
                    let _ = state.config.save();
                    state.status = "Perfil activado.".to_string();
                }
                _ => state.status = "Perfil no encontrado.".to_string(),
            }
            Task::none()
        }
        Message::ProfileNameChanged(v) => {
            state.profile_name = v;
            Task::none()
        }
        Message::ProfileSave => {
            // "Guardar como perfil" con el MISMO nombre actualiza el perfil
            // existente (api+base+modelo+nivel+provider); con nombre nuevo lo
            // crea. Así el perfil siempre contiene todo lo necesario para
            // cambiar de provider sin perder nada.
            let name = state.profile_name.trim();
            let name = if name.is_empty() { "Perfil" } else { name };
            let cfg = edit_cfg(state);
            // ¿Ya existe un perfil con ese nombre? -> actualiza en sitio.
            let existing_idx = state
                .config
                .model_profiles
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(name));
            if let Some(idx) = existing_idx {
                let pid;
                let final_name;
                {
                    let existing = &mut state.config.model_profiles[idx];
                    existing.provider = state.edit_provider;
                    existing.base_url = cfg.base_url.clone();
                    existing.api_key = cfg.api_key.clone();
                    existing.model = cfg.model.clone();
                    existing.reasoning_effort = cfg.reasoning_effort.clone();
                    pid = existing.id.clone();
                    final_name = existing.name.clone();
                }
                state.config.active_profile = Some(pid);
                state.config.active = state.edit_provider;
                // Sincroniza también el slot del provider para coherencia.
                let target = state.config.active_config_mut();
                *target = cfg;
                state.profile_name = final_name;
                match state.config.save() {
                    Ok(()) => state.status = "Perfil actualizado.".to_string(),
                    Err(e) => state.status = format!("Perfil actualizado pero no se guardó: {e}"),
                }
                return Task::none();
            }
            let mut p = crate::config::ModelProfile::new(name, state.edit_provider, &cfg);
            // Garantiza id único.
            while state.config.profile_by_id(&p.id).is_some() {
                p.id.push('x');
            }
            let pid = p.id.clone();
            let final_name = p.name.clone();
            state.config.model_profiles.push(p);
            state.config.active_profile = Some(pid.clone());
            state.config.active = state.edit_provider;
            state.profile_name = final_name;
            match state.config.save() {
                Ok(()) => state.status = "Perfil guardado.".to_string(),
                Err(e) => state.status = format!("Perfil creado pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::ProfileDelete(pid) => {
            if state.config.profile_by_id(&pid).is_none() {
                state.status = "Perfil no encontrado.".to_string();
                return Task::none();
            }
            if state.config.model_profiles.len() <= 1 {
                state.status = "No se puede borrar el único perfil.".to_string();
                return Task::none();
            }
            state.config.model_profiles.retain(|p| p.id != pid);
            if state.profile_menu.as_deref() == Some(pid.as_str()) {
                state.profile_menu = None;
            }
            if state.editing_profile.as_deref() == Some(pid.as_str()) {
                state.editing_profile = None;
            }
            // Si era el activo, activa el primero que quede.
            if state.config.active_profile.as_deref() == Some(pid.as_str()) {
                let first = state.config.model_profiles.first().cloned();
                if let Some(f) = first {
                    state.config.apply_profile(&f.id);
                    state.edit_provider = f.provider;
                    state.sync_edit_fields();
                    state.profile_name = f.name;
                }
            }
            match state.config.save() {
                Ok(()) => state.status = "Perfil borrado.".to_string(),
                Err(e) => state.status = format!("Borrado pero no se guardó: {e}"),
            }
            Task::none()
        }
        Message::ProfileMenuToggled(pid) => {
            state.profile_menu = if state.profile_menu.as_deref() == Some(pid.as_str()) {
                None
            } else {
                Some(pid)
            };
            Task::none()
        }
        Message::ProfileEdit(pid) => {
            let Some(p) = state.config.profile_by_id(&pid).cloned() else {
                state.status = "Perfil no encontrado.".to_string();
                return Task::none();
            };
            state.editing_profile = Some(pid);
            state.profile_menu = None;
            state.eprofile_name = p.name;
            state.eprofile_provider = p.provider;
            state.eprofile_api = p.api_key;
            state.eprofile_base = p.base_url;
            state.eprofile_model = p.model;
            state.eprofile_reasoning = p.reasoning_effort;
            Task::none()
        }
        Message::ProfileEditCancel => {
            state.editing_profile = None;
            Task::none()
        }
        Message::ProfileEditNameChanged(v) => {
            state.eprofile_name = v;
            Task::none()
        }
        Message::ProfileEditProviderPicked(p) => {
            state.eprofile_provider = p;
            Task::none()
        }
        Message::ProfileEditApiChanged(v) => {
            state.eprofile_api = v;
            Task::none()
        }
        Message::ProfileEditBaseChanged(v) => {
            state.eprofile_base = v;
            Task::none()
        }
        Message::ProfileEditModelChanged(v) => {
            state.eprofile_model = v;
            Task::none()
        }
        Message::ProfileEditReasoningPicked(v) => {
            state.eprofile_reasoning = if v == "auto" { String::new() } else { v };
            Task::none()
        }
        Message::ProfileUpdate => {
            let Some(pid) = state.editing_profile.clone() else {
                return Task::none();
            };
            let name = state.eprofile_name.trim().to_string();
            if name.is_empty() {
                state.status = "El perfil necesita un nombre.".to_string();
                return Task::none();
            }
            if state
                .config
                .model_profiles
                .iter()
                .any(|p| p.id != pid && p.name.eq_ignore_ascii_case(&name))
            {
                state.status = "Ya existe otro perfil con ese nombre.".to_string();
                return Task::none();
            }
            let (prov, api, base, model, reasoning) = (
                state.eprofile_provider,
                state.eprofile_api.trim().to_string(),
                state.eprofile_base.trim().to_string(),
                state.eprofile_model.trim().to_string(),
                state.eprofile_reasoning.trim().to_string(),
            );
            if prov.requires_key() && api.is_empty() {
                state.status = "Ese provider necesita API key.".to_string();
                return Task::none();
            }
            if model.trim().is_empty() {
                state.status = "El perfil necesita un modelo.".to_string();
                return Task::none();
            }
            if let Some(p) = state.config.model_profiles.iter_mut().find(|p| p.id == pid) {
                p.name = name;
                p.provider = prov;
                p.api_key = api;
                p.base_url = base;
                p.model = model;
                p.reasoning_effort = reasoning;
            }
            // Si es el activo, refleja el cambio en el chat de inmediato.
            if state.config.active_profile.as_deref() == Some(pid.as_str()) {
                state.config.apply_profile(&pid);
                state.edit_provider = state.config.active;
                state.sync_edit_fields();
                state.profile_name = state
                    .config
                    .profile_by_id(&pid)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
            }
            state.editing_profile = None;
            match state.config.save() {
                Ok(()) => state.status = "Perfil actualizado.".to_string(),
                Err(e) => state.status = format!("Actualizado pero no se guardó: {e}"),
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
        Message::LimitFixCyclesPicked(v) => {
            // 0 = ilimitado (default); si >0 se recorta a rango sano.
            state.config.limits.max_fix_cycles = v;
            state.config.limits = state.config.limits.clamped();
            let _ = state.config.save();
            Task::none()
        }
        Message::ConfigTab(tab) => {
            state.config_tab = tab;
            state.config_pending_delete = None;
            state.profile_menu = None;
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
            // Git: staging + estado del workspace activo (v0.7.2).
            if tab == ConfigTab::Git {
                state.sync_git_staging();
                state.refresh_git_status();
            }
            Task::none()
        }
        Message::GitEnabledToggled(v) => {
            state.config.git.enabled = v;
            state.config.git = state.config.git.clone().validated();
            let _ = state.config.save();
            Task::none()
        }
        Message::GitAutoInitToggled(v) => {
            state.config.git.auto_init = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::GitBranchModePicked(m) => {
            state.config.git.branch_mode = m;
            let _ = state.config.save();
            Task::none()
        }
        Message::GitAutonomyPicked(a) => {
            state.config.git.autonomy = a;
            let _ = state.config.save();
            Task::none()
        }
        Message::GitPushToggled(v) => {
            state.config.git.push_enabled = v;
            let _ = state.config.save();
            Task::none()
        }
        Message::GitBaseBranchChanged(v) => {
            state.git_base_branch = v;
            Task::none()
        }
        Message::GitWorkBranchChanged(v) => {
            state.git_work_branch = v;
            Task::none()
        }
        Message::GitRemoteChanged(v) => {
            state.git_remote = v;
            Task::none()
        }
        Message::GitPushBranchChanged(v) => {
            state.git_push_branch = v;
            Task::none()
        }
        Message::GitAuthorNameChanged(v) => {
            state.git_author_name = v;
            Task::none()
        }
        Message::GitAuthorEmailChanged(v) => {
            state.git_author_email = v;
            Task::none()
        }
        Message::GitSave => {
            let mut git = state.config.git.clone();
            git.base_branch = state.git_base_branch.trim().to_string();
            git.work_branch = state.git_work_branch.trim().to_string();
            git.remote = state.git_remote.trim().to_string();
            git.push_branch = state.git_push_branch.trim().to_string();
            git.author_name = state.git_author_name.trim().to_string();
            git.author_email = state.git_author_email.trim().to_string();
            state.config.git = git.validated();
            state.sync_git_staging();
            state.refresh_git_status();
            match state.config.save() {
                Ok(()) => state.status = "Configuración git guardada.".to_string(),
                Err(e) => state.status = format!("No se pudo guardar git: {e}"),
            }
            Task::none()
        }
        Message::GitRefreshStatus => {
            state.refresh_git_status();
            Task::none()
        }
        Message::GitInitWorkspace => {
            let git = state.config.git.clone();
            match state.active_workspace() {
                Some(ws) => {
                    state.status = "Inicializando git…".to_string();
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::git::init_repo(&ws, &git.base_branch)
                                    .and_then(|()| crate::git::ensure_work_branch(&ws, &git))
                                    .map(|()| {
                                        crate::git::current_branch(&ws)
                                            .unwrap_or_else(|| git.work_branch.clone())
                                    })
                            })
                            .await
                            .map_err(|e| format!("tarea git: {e}"))?
                        },
                        Message::GitInitDone,
                    )
                }
                None => {
                    state.status = "Sin workspace activo (abre un proyecto primero).".to_string();
                    Task::none()
                }
            }
        }
        Message::GitInitDone(res) => {
            state.refresh_git_status();
            match res {
                Ok(branch) => {
                    state.status = format!("Git listo en rama {branch}.");
                    state.push_log(format!("🌿 git: rama {branch}"));
                }
                Err(e) => state.status = format!("No se pudo inicializar git: {e}"),
            }
            Task::none()
        }
        Message::GitPushDone(res) => {
            match res {
                Ok(msg) => {
                    state.push_log(format!("⬆ push OK: {msg}"));
                    state.status = "Push completado.".to_string();
                }
                Err(e) => {
                    state.push_log(format!("⚠️ push falló: {e}"));
                    state.status = format!("Push falló: {e}");
                }
            }
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
            // Si el overlay de edición está abierto, el modelo elegido va a él.
            if state.editing_profile.is_some() {
                state.eprofile_model = id.clone();
            }
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