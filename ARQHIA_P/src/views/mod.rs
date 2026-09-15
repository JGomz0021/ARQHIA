use iced::Theme;

pub mod chat;
pub mod config_view;
pub mod home;
pub mod questionnaire;
pub mod sidebar;
pub mod stack;

/// Theme Iced correspondiente a la config. También se pasa a `application`.
/// Siempre custom (v0.7 Track B): la paleta base Dark/Light con el acento
/// elegido; `design::accent()` lo lee de `palette().primary`.
pub(crate) fn theme_for(config: &crate::config::AppConfig) -> Theme {
    use crate::config::{AccentChoice, ThemeMode};
    use iced::Color;

    let dark = matches!(config.theme, ThemeMode::Dark);
    let mut palette = if dark {
        Theme::Dark.palette()
    } else {
        Theme::Light.palette()
    };
    palette.primary = match (config.appearance.accent, dark) {
        (AccentChoice::Teal, true) => Color::from_rgb8(0x4F, 0xA3, 0x9B),
        (AccentChoice::Teal, false) => Color::from_rgb8(0x1F, 0x7A, 0x72),
        (AccentChoice::Amber, true) => Color::from_rgb8(0xD1, 0xA7, 0x3D),
        (AccentChoice::Amber, false) => Color::from_rgb8(0x8A, 0x6D, 0x1A),
        (AccentChoice::Violet, true) => Color::from_rgb8(0xA9, 0x8B, 0xF0),
        (AccentChoice::Violet, false) => Color::from_rgb8(0x6A, 0x52, 0xC8),
    };
    let name = match config.theme {
        ThemeMode::Dark => "arqhIA-oscuro",
        ThemeMode::Light => "arqhIA-claro",
    };
    Theme::custom(name.to_string(), palette)
}

pub(crate) fn app_theme(state: &crate::app::App) -> Theme {
    theme_for(&state.config)
}

#[cfg(test)]
mod tests {
    use super::chat;
    use super::config_view;
    use super::home;
    use super::questionnaire;
    use super::sidebar;
    use crate::app::{App, ConfigTab, View};
    use crate::llm::{ChatMsg, Role};

    #[test]
    fn config_tabs_build_without_panic() {
        let mut app = App::default();
        for tab in ConfigTab::ALL {
            app.config_tab = tab;
            let _ = config_view::view_config(&app);
        }
        // Con uploads reales si los hay
        let _ = sidebar::view_sidebar(&app);
        let _ = chat::view_chat(&app);
        // Menú ⋯ abierto en cada proyecto con workspace.
        for p in app.projects.clone() {
            if p.path.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some() {
                app.project_menu = Some(p.id);
                let _ = sidebar::view_sidebar(&app);
            }
        }
        app.project_menu = None;
        let _ = home::view_home(home::HomeProps {
            creating: false,
            create_name: "",
            create_path: "",
            status: "",
            projects: &app.projects,
            chats: &app.chats,
            active_chat: app.active_chat,
            config: &app.config,
            onboarding_dismissed: false,
        });
        app.view = View::Questionnaire;
        let _ = questionnaire::view_questionnaire(&app);
        // v0.8.2: pantalla de carga MVP sin chat.
        app.view = View::Generating;
        app.gen_phase = "Generando…".to_string();
        app.gen_progress = 0.5;
        let _ = questionnaire::view_generating(&app);
        let _ = crate::view(&app);
        // v0.9: panel STACK vacío y con selección (preview + guardar).
        app.view = View::Stack;
        let _ = crate::view(&app);
        app.stack_results = vec![crate::stack::ScoredItem {
            id: 1,
            title: "demo".to_string(),
            tags: "rust, demo".to_string(),
            lang: "rust".to_string(),
            rating: 4.5,
            score: 9.0,
            snippet: "fn demo() {}".to_string(),
        }];
        app.stack_selected = Some(crate::stack::FullItem {
            id: 1,
            title: "demo".to_string(),
            code: (0..30).map(|i| format!("línea {i}")).collect::<Vec<_>>().join("\n"),
            tags: "rust, demo".to_string(),
            lang: "rust".to_string(),
            rating: 4.5,
            ratings: 2,
            executions: 3,
            ok_runs: 2,
            author: "test".to_string(),
            license: "MIT".to_string(),
            source: "local".to_string(),
            meta: vec![("version".to_string(), "1.0".to_string())],
        });
        app.stack_searched = true;
        let _ = crate::view(&app);
        let _ = crate::view(&app);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn model_browser_builds_without_panic() {
        let mut app = App::default();
        app.config_tab = ConfigTab::Api;
        app.model_browser = true;
        app.model_search = "gpt".to_string();
        app.model_status = "3 modelos".to_string();
        app.pricing = crate::pricing::Pricing::parse(
            r#"{"openai":{"name":"OpenAI","env":["OPENAI_API_KEY"],"api":"https://api.openai.com/v1",
                "models":{"gpt-4o":{"name":"GPT-4o","family":"gpt","tool_call":true,"reasoning":true,
                "cost":{"input":2.5,"output":10.0,"cache_read":1.25},"limit":{"context":128000}}}}}"#,
        )
        .unwrap();
        let _ = config_view::view_config(&app);
        // Rama local (LM Studio) sin catálogo.
        app.edit_provider = crate::config::Provider::Local;
        app.local_models = vec!["qwen3-8b".to_string()];
        let _ = config_view::view_config(&app);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn empty_states_build_without_panic() {
        // "Ir al chat" sin proyectos ni chats + sidebar + home vacíos.
        let mut app = App::default();
        app.projects.clear();
        app.chats.clear();
        app.active_chat = None;
        app.messages.clear();
        app.md.clear();
        app.msg_usage.clear();
        app.msg_times.clear();
        app.msg_ids.clear();
        app.pending_project = None;
        app.view = View::Chat;
        let _ = crate::view(&app);
        app.view = View::Home;
        let _ = crate::view(&app);
        // Tras borrar el último proyecto quedando un chat huérfano activo.
        app.active_chat = Some(999);
        let _ = crate::view(&app);
        app.view = View::Chat;
        let _ = crate::view(&app);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn git_tab_builds_with_workspace_and_repo() {
        // Workspace real con repo git: cubre la rama "es repo" del card de estado.
        let ws = std::env::temp_dir().join("arqhia-view-git-ws");
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        crate::git::init_repo(&ws, "main").unwrap();
        let path = ws.to_string_lossy().to_string();
        let mut app = App::default();
        app.config_tab = ConfigTab::Git;
        app.projects.push(crate::db::Project { id: -7, name: "T".to_string(), path: Some(path) });
        app.chats.push(crate::db::ChatMeta {
            id: -7,
            title: "T".to_string(),
            project_id: Some(-7),
            archived: false,
            mode: crate::db::Mode::Chat,
            session_id: None,
        });
        app.active_chat = Some(-7);
        app.refresh_git_status();
        assert!(app.git_status.is_repo);
        let _ = config_view::view_config(&app);
        // Y sin repo (misma ruta, borrando .git): rama "Inicializar git".
        let _ = std::fs::remove_dir_all(ws.join(".git"));
        app.refresh_git_status();
        assert!(!app.git_status.is_repo);
        let _ = config_view::view_config(&app);
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn slash_suggest_renders_in_composer() {
        // Escribir `/code` muestra la ventana flotante con nombre + badge.
        let mut app = App::default();
        app.view = View::Chat;
        app.input = "/code".to_string();
        app.skill_suggest = crate::skills::suggest("/code");
        assert!(app.skill_suggest.iter().any(|s| s.name == "code-review"));
        let _ = crate::view(&app);
        // Sin `/` no hay caja aunque queden restos en estado.
        app.input = "hola".to_string();
        let _ = crate::view(&app);
        // Caja con varias sugerencias tampoco panica.
        app.input = "/".to_string();
        app.skill_suggest = crate::skills::suggest("/");
        let _ = crate::view(&app);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn tricky_provider_content_never_panics() {
        // Contenido estilo OpenRouter (reasoning, fences sin cerrar, tablas,
        // multibyte) no debe tumbar ni el parseo ni la vista.
        let mut app = App::default();
        app.messages = vec![
            ChatMsg { role: Role::User, content: "hola ñandú 🚀".to_string() },
            ChatMsg {
                role: Role::Assistant,
                content: "<think>razonamiento…</think>\n```rust\nfn main() {\n| a | b |\n|---|---|\n| á | é |\n> cita\n- lista “…”.to_string()\n".to_string(),
            },
        ];
        app.reparse_md();
        let _ = chat::view_chat(&app);
        // Stream parcial: re-parseo incremental tampoco debe fallar.
        if let Some(last) = app.messages.last_mut() {
            last.content.push_str("más ``` sin cerrar y emoji 🎉");
        }
        app.reparse_last_md();
        let _ = chat::view_chat(&app);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn plan_panel_and_mode_badge_build() {
        use crate::app::orchestrator::OrchTask;
        use crate::db::Mode;
        let mut app = App::default();
        // Panel Plan con checklist: no panica y no exige workspace real.
        app.show_plan = true;
        app.plan_md = "# PLAN".to_string();
        app.orch_tasks = vec![OrchTask { desc: "Crear a.txt".to_string(), files: vec!["a.txt".to_string()], done: false, active: false }];
        let _ = chat::view_chat(&app);
        // Badge de modo Plan en un chat en memoria (sin tocar la DB).
        app.chats.push(crate::db::ChatMeta { id: -1, title: "T".to_string(), project_id: None, archived: false, mode: Mode::Plan, session_id: None });
        app.active_chat = Some(-1);
        let _ = chat::view_chat(&app);
        assert_eq!(app.active_mode(), Mode::Plan);
        // Default: sin chat activo el modo es Chat.
        app.active_chat = Some(-999);
        assert_eq!(app.active_mode(), Mode::Chat);
    }
}
