//! Handlers de update agrupados por dominio.
//!
//! `update_inner` (main) solo enruta; cada módulo posee sus brazos.
//! El compilador verifica exhaustividad en ambos niveles.

pub mod agent;
pub mod chat;
pub mod chat_history;
pub mod chat_stream;
pub mod config;
pub mod navigation;
pub mod projects;
pub mod questionnaire;
pub mod stack;

#[cfg(test)]
mod tests {
    use super::{agent, chat, config, navigation, projects, questionnaire};
    use crate::app::{App, ConfigTab, Message, View};

    /// Cada grupo responde a un mensaje representativo: demuestra que el
    /// dispatch de update_inner está cableado (un grupo descableado caería
    /// en el wildcard y el estado no cambiaría).
    #[test]
    fn dispatch_routes_every_group() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-dispatch_route");
        use crate::db::Mode;
        let mut app = App::default();

        let _ = chat::handle(&mut app, Message::InputChanged("hola".to_string()));
        assert_eq!(app.input, "hola");

        // v0.7.1: modos y overlays van al grupo de chat, sin tocar la DB.
        app.active_chat = None;
        let _ = chat::handle(&mut app, Message::ModePicked(Mode::Plan));
        assert!(app.status.contains("Sin chat activo"));
        app.chat_menu = Some(7);
        let _ = chat::handle(&mut app, Message::CloseOverlays);
        assert_eq!(app.chat_menu, None);

        // v0.7.1: Plan/Execute/Dismiss van al grupo del agente (turno
        // inexistente = ignorado, sin efectos).
        let _ = agent::handle(&mut app, Message::PlanDone(999, Ok(vec![])));
        assert!(!app.agent_running);
        let _ = agent::handle(&mut app, Message::ExecutePlan);
        assert!(!app.status.is_empty() || !app.show_plan);
        let _ = agent::handle(&mut app, Message::DismissPlan);
        assert!(!app.show_plan);

        let _ = navigation::handle(&mut app, Message::GoHome);
        assert_eq!(app.view, View::Home);

        let before = app.show_project_form;
        let _ = projects::handle(&mut app, Message::ToggleProjectForm);
        assert_eq!(app.show_project_form, !before);

        let _ = config::handle(&mut app, Message::ConfigTab(ConfigTab::Apariencia));
        assert_eq!(app.config_tab, ConfigTab::Apariencia);

        let _ = questionnaire::handle(&mut app, Message::QCancel);
        assert_eq!(app.view, View::Chat);

        // v0.7.4: menú contextual por mensaje + aviso de truncado.
        app.messages.clear();
        let _ = chat::handle(&mut app, Message::CopyMsg(0));
        assert!(app.status.contains("Nada que copiar"));
        let _ = chat::handle(&mut app, Message::ChatMsgMenu(3));
        assert_eq!(app.msg_menu, None, "índice fuera de rango no abre menú");
        let _ = chat::handle(&mut app, Message::TruncateRequest(0));
        assert_eq!(app.pending_truncate, None, "sin mensajes no hay aviso");
        let _ = chat::handle(&mut app, Message::CancelTruncate);
        assert_eq!(app.pending_truncate, None);
        let _ = chat::handle(&mut app, Message::ConfirmTruncate);
        assert!(app.pending_truncate.is_none());
        // v0.7.4: undo sin snapshot avisa; perfiles responden en config.
        let _ = chat::handle(&mut app, Message::UndoChat);
        assert!(app.status.contains("Nada que deshacer") || app.status.contains("Espera"));
        let n_profiles = app.config.model_profiles.len();
        let _ = config::handle(&mut app, Message::ProfileNameChanged("Tmp".to_string()));
        assert_eq!(app.profile_name, "Tmp");
        assert_eq!(app.config.model_profiles.len(), n_profiles);
        // v0.7.4 rediseño: menú ··· por perfil + overlay de edición.
        let _ = config::handle(&mut app, Message::ProfileMenuToggled("x".to_string()));
        assert_eq!(app.profile_menu.as_deref(), Some("x"));
        let _ = config::handle(&mut app, Message::ProfileMenuToggled("x".to_string()));
        assert_eq!(app.profile_menu, None);
        let _ = config::handle(&mut app, Message::ProfileEdit("no-existe".to_string()));
        assert!(app.status.contains("no encontrado"));
        assert_eq!(app.editing_profile, None);
        let _ = config::handle(&mut app, Message::ProfileDelete("no-existe".to_string()));
        assert_eq!(app.config.model_profiles.len(), n_profiles);
    }

    /// v0.8: cuestionario por niveles, sesión y onboarding responden.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn dispatch_routes_v08_messages() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-dispatch_route");
        use crate::questionnaire::Level;
        let mut app = App::default();
        // Sin chat activo, reiniciar sesión avisa (no panica ni toca DB).
        app.active_chat = None;
        let _ = chat::handle(&mut app, Message::ResetSession);
        assert!(app.status.contains("Sin chat activo"));
        // Onboarding: "después" lo oculta en memoria.
        assert!(!app.onboarding_dismissed);
        let _ = navigation::handle(&mut app, Message::DismissOnboarding);
        assert!(app.onboarding_dismissed);
        // Nivel: cambia y valida el wizard dinámico.
        let _ = questionnaire::handle(&mut app, Message::QLevelPicked(Level::Avanzado));
        assert_eq!(app.q_level, Level::Avanzado);
        let (cat, sys) = (app.q_answers.cat, app.q_answers.sys);
        assert!(
            crate::questionnaire::levels::total_steps(
                Level::Avanzado,
                cat,
                sys,
                app.q_answers.nombre_locked
            ) > 8
        );
        // IA sin API configurada no cuelga: error visible y paso saltable.
        app.q_ai_loading = false;
        let _ = questionnaire::handle(&mut app, Message::QAiGenerate);
        if !app.config.has_any_api() && app.q_source == crate::questionnaire::QSource::New {
            assert!(app.q_ai_error.contains("Configura"));
        }
        // Navegar el wizard no panica en ningún paso de cada familia.
        for (level, cat, sys) in [
            (
                Level::Principiante,
                crate::questionnaire::Categoria::Aplicacion,
                crate::questionnaire::SysType::AppWeb,
            ),
            (
                Level::Avanzado,
                crate::questionnaire::Categoria::Sistema,
                crate::questionnaire::SysType::DriverFirmware,
            ),
        ] {
            let skip = app.q_answers.nombre_locked;
            let total = crate::questionnaire::levels::total_steps(level, cat, sys, skip);
            for step in 0..total {
                let _ = crate::questionnaire::step_title(level, cat, sys, skip, step);
                let _ = crate::questionnaire::validate_step(level, cat, sys, step, &app.q_answers);
            }
        }
    }

    /// v0.8.1: cancelar un cuestionario sin terminar deshace el proyecto.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn questionnaire_cancel_rolls_back_unfinished_project() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-questionnaire_");
        let mut app = App::default();
        app.view = View::Questionnaire;
        // Caso 1: carpeta vacía recién creada -> se retira + fila fuera.
        let base = std::env::temp_dir().join("arqhia-qcancel-test");
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("ws-vacio");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        let pid = crate::db::create_project("qcancel-tmp-xyz").unwrap();
        crate::db::set_project_path(pid, Some(&path)).unwrap();
        app.projects.push(crate::db::Project {
            id: pid,
            name: "qcancel-tmp-xyz".to_string(),
            path: Some(path),
        });
        app.q_project = Some(pid);
        app.q_owns_project = true;
        let _ = questionnaire::handle(&mut app, Message::QCancel);
        assert!(!app.q_owns_project);
        assert!(app.projects.iter().all(|p| p.id != pid));
        assert!(!dir.exists(), "la carpeta vacía se retira");
        assert_eq!(app.view, View::Home);

        // Caso 2: carpeta con contenido del usuario -> se conserva en disco.
        let dir2 = base.join("ws-con-datos");
        std::fs::create_dir_all(&dir2).unwrap();
        std::fs::write(dir2.join("mio.txt"), "no borrar").unwrap();
        let path2 = dir2.to_string_lossy().to_string();
        let pid2 = crate::db::create_project("qcancel-tmp-xyz2").unwrap();
        crate::db::set_project_path(pid2, Some(&path2)).unwrap();
        app.projects.push(crate::db::Project {
            id: pid2,
            name: "qcancel-tmp-xyz2".to_string(),
            path: Some(path2),
        });
        app.q_project = Some(pid2);
        app.q_owns_project = true;
        app.view = View::Questionnaire;
        let _ = questionnaire::handle(&mut app, Message::QCancel);
        assert!(app.projects.iter().all(|p| p.id != pid2));
        assert_eq!(
            std::fs::read_to_string(dir2.join("mio.txt")).unwrap(),
            "no borrar"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// v0.8.2: terminar el cuestionario abre la pantalla de carga MVP
    /// (sin pasar por el chat); GenDone escribe la serie y entra al Chat.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn questionnaire_finish_writes_docs_and_enters_plan() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-questionnaire_");
        use crate::questionnaire::Answers;
        let mut app = App::default();
        // API configurada: el futuro Task::perform no se ejecuta en el test,
        // pero el Finish debe dejar la pantalla de carga activa.
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        let base = std::env::temp_dir().join("arqhia-qfinish-test");
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("ws");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        let pid = crate::db::create_project("qfinish-tmp-xyz").unwrap();
        crate::db::set_project_path(pid, Some(&path)).unwrap();
        app.projects.push(crate::db::Project {
            id: pid,
            name: "qfinish-tmp-xyz".to_string(),
            path: Some(path),
        });
        app.q_project = Some(pid);
        app.q_owns_project = true;
        app.q_level = crate::questionnaire::Level::Principiante;
        app.q_answers = Answers {
            nombre: "Tienda".to_string(),
            // Vino de la creación: el paso de nombre se omite.
            nombre_locked: true,
            descripcion: "Una tienda online de ejemplo".to_string(),
            objetivo: "Vender productos por internet".to_string(),
            funcionalidades: "Catálogo\nCarrito".to_string(),
            ..Default::default()
        };
        app.view = View::Questionnaire;
        app.active_chat = None;
        let _ = questionnaire::handle(&mut app, Message::FinishQuestionnaire);
        assert!(app.q_error.is_empty(), "error: {}", app.q_error);
        // Docs base rellenos (no vacíos) con las secciones de la familia.
        let specs = std::fs::read_to_string(dir.join("CONTEXT").join("SPECS.md")).unwrap();
        assert!(specs.contains("Tipo de sistema") && specs.contains("Catálogo"));
        // v0.9 Track D: criterios de aceptación por funcionalidad.
        assert!(specs.contains("Criterios de aceptación"));
        let project = std::fs::read_to_string(dir.join("CONTEXT").join("PROJECT.md")).unwrap();
        assert!(project.contains("Tienda"));
        assert!(dir.join("Project").is_dir() && dir.join("ToDo.md").is_file());
        // Pantalla de carga: sin chat, sin turno de agente, con contexto pendiente.
        assert_eq!(app.view, View::Generating);
        assert!(app.gen_active, "generación en curso");
        assert!(!app.agent_running && !app.streaming, "nada en el chat");
        assert!(app.plan_auto.is_some(), "contexto auto pendiente");
        assert!(app.active_chat.is_none(), "el chat nace al terminar");
        let run = app.gen_run;
        // GenDone con serie vacía (fallo IA) cae al fallback determinista y entra al Chat.
        let _ = questionnaire::handle(&mut app, Message::GenDone(run, Ok(Vec::new())));
        assert!(!app.gen_active && app.plan_auto.is_none());
        assert_eq!(app.view, View::Chat);
        let cid = app.active_chat.expect("chat Plan creado");
        let meta = app.chats.iter().find(|c| c.id == cid).unwrap();
        assert_eq!(meta.mode, crate::db::Mode::Plan);
        assert!(!app.agent_running, "sin turno automático en el chat");
        for ver in ["v0.1", "v0.2", "v0.3", "v1.0"] {
            assert!(
                dir.join("CONTEXT")
                    .join("VERSIONS")
                    .join(format!("{ver}.md"))
                    .is_file(),
                "{ver} existe"
            );
        }
        assert!(dir.join("CONTEXT").join("ROADMAP.md").is_file());
        // Limpieza para no contaminar la DB del dev.
        crate::db::delete_chat(cid).unwrap();
        crate::db::delete_project(pid).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    /// v0.8.2: GenCancel invalida la generación y vuelve al cuestionario;
    /// un GenDone tardío se ignora. Sin API, el Finish es offline directo.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn generation_cancel_and_offline_fallback() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-generation_can");
        use crate::questionnaire::Answers;
        fn setup(name: &str) -> (App, i64, std::path::PathBuf) {
            let mut app = App::default();
            let base = std::env::temp_dir().join("arqhia-qgen-test").join(name);
            let _ = std::fs::remove_dir_all(&base);
            let dir = base.join("ws");
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.to_string_lossy().to_string();
            let pid = crate::db::create_project(name).unwrap();
            crate::db::set_project_path(pid, Some(&path)).unwrap();
            app.projects.push(crate::db::Project {
                id: pid,
                name: name.to_string(),
                path: Some(path),
            });
            app.q_project = Some(pid);
            app.q_owns_project = true;
            app.q_level = crate::questionnaire::Level::Intermedio;
            app.q_answers = Answers {
                nombre: name.to_string(),
                nombre_locked: true,
                descripcion: "Descripción de ejemplo amplia".to_string(),
                objetivo: "Objetivo de ejemplo amplio".to_string(),
                funcionalidades: "Alta\nBaja".to_string(),
                ..Default::default()
            };
            app.view = View::Questionnaire;
            (app, pid, dir)
        }
        // Con API: Finish → Generating; cancelar vuelve al cuestionario.
        let (mut app, pid, dir) = setup("qgen-tmp-cancel");
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        let _ = questionnaire::handle(&mut app, Message::FinishQuestionnaire);
        assert_eq!(app.view, View::Generating);
        let run = app.gen_run;
        let _ = questionnaire::handle(&mut app, Message::GenCancel);
        assert!(!app.gen_active && app.plan_auto.is_none());
        assert_eq!(app.view, View::Questionnaire);
        // Resultado tardío: se ignora sin escribir ni cambiar de vista.
        let _ = questionnaire::handle(&mut app, Message::GenDone(run, Ok(Vec::new())));
        assert_eq!(app.view, View::Questionnaire);
        assert!(!dir.join("CONTEXT").join("ROADMAP.md").exists());
        crate::db::delete_project(pid).unwrap();
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        // Sin API: Finish escribe la serie offline y entra al Chat sin generar.
        let (mut app2, pid2, dir2) = setup("qgen-tmp-offline");
        app2.config.openai.api_key.clear();
        app2.config.anthropic.api_key.clear();
        app2.config.openrouter.api_key.clear();
        app2.config.local.model.clear();
        assert!(!app2.config.has_any_api());
        let _ = questionnaire::handle(&mut app2, Message::FinishQuestionnaire);
        assert_eq!(app2.view, View::Chat);
        assert!(!app2.gen_active);
        assert!(
            dir2.join("CONTEXT")
                .join("VERSIONS")
                .join("v1.0.md")
                .is_file()
        );
        let cid2 = app2.active_chat.unwrap();
        crate::db::delete_chat(cid2).unwrap();
        crate::db::delete_project(pid2).unwrap();
        let _ = std::fs::remove_dir_all(dir2.parent().unwrap());
    }

    /// v0.9 Track C: `/skill` inexistente avisa con la lista y no arranca
    /// turno; existente inyecta el contexto una vez y sigue normal.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn skill_message_resolves_or_lists_without_turn() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-skill_message_");
        let mut app = App::default();
        // Inexistente: error amable con la lista, sin turno ni DB.
        app.input = "/skill noexiste-xyz123 hola".to_string();
        let n_msgs = app.messages.len();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(app.status.contains("no encontrada") && app.status.contains("Instaladas"));
        assert!(!app.agent_running && !app.streaming);
        assert_eq!(
            app.messages.len(),
            n_msgs,
            "sin turno no hay mensajes nuevos"
        );
        // Existente: inyecta UNA vez como contexto y arranca el turno Chat.
        let cid = crate::db::create_chat("v0.9-skill-tmp").unwrap();
        app.chats.push(crate::db::ChatMeta {
            id: cid,
            title: "t".to_string(),
            project_id: None,
            archived: false,
            mode: crate::db::Mode::Chat,
            session_id: None,
        });
        app.active_chat = Some(cid);
        app.messages.clear();
        app.md.clear();
        app.msg_times.clear();
        app.msg_ids.clear();
        app.msg_usage.clear();
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        app.input = "/skill commit-msg añade login".to_string();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(app.streaming, "el turno Chat sigue normal tras /skill");
        // usuario + contexto de skill + placeholder del stream.
        assert_eq!(app.messages.len(), 3, "usuario + skill + placeholder");
        assert!(app.messages[0].content.contains("/skill commit-msg"));
        assert!(
            app.messages[1].content.contains("commit-msg"),
            "contexto inyectado"
        );
        assert!(app.tool_logs.iter().any(|l| l.contains("skill commit-msg")));
        crate::db::delete_chat(cid).unwrap();
    }

    /// v0.9.2: `/skill ui-ux [texto]` inyecta la skill de dominio una vez
    /// por el parseo genérico (sin rama nueva) y el turno sigue normal.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn domain_skill_injects_once_and_continues_turn() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-domain_skill_i");
        let mut app = App::default();
        let cid = crate::db::create_chat("v092-skill-tmp").unwrap();
        app.chats.push(crate::db::ChatMeta {
            id: cid,
            title: "t".to_string(),
            project_id: None,
            archived: false,
            mode: crate::db::Mode::Chat,
            session_id: None,
        });
        app.active_chat = Some(cid);
        app.messages.clear();
        app.md.clear();
        app.msg_times.clear();
        app.msg_ids.clear();
        app.msg_usage.clear();
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        app.input = "/skill ui-ux revisa el chat".to_string();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(
            app.streaming,
            "el turno sigue normal tras /skill de dominio"
        );
        assert_eq!(app.messages.len(), 3, "usuario + skill + placeholder");
        assert!(app.messages[1].content.contains("ui-ux"));
        assert!(
            app.messages[1].content.contains("UI-REVIEW.md"),
            "la skill pide su reporte"
        );
        assert!(app.tool_logs.iter().any(|l| l.contains("skill ui-ux")));
        crate::db::delete_chat(cid).unwrap();
    }

    /// Mensaje inválido (Plan sin workspace): no se persiste ni entra en
    /// contexto; el input se conserva y el estado avisa.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn invalid_plan_message_never_reaches_db_or_context() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-invalid_plan_m");
        let mut app = App::default();
        let cid = crate::db::create_chat("v09x-invalid-plan-tmp").unwrap();
        crate::db::set_chat_mode(cid, crate::db::Mode::Plan).unwrap();
        app.chats.push(crate::db::ChatMeta {
            id: cid,
            title: "t".to_string(),
            project_id: None,
            archived: false,
            mode: crate::db::Mode::Plan,
            session_id: None,
        });
        app.active_chat = Some(cid);
        app.messages.clear();
        app.md.clear();
        app.msg_times.clear();
        app.msg_ids.clear();
        app.msg_usage.clear();
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        app.input = "planifica sin workspace".to_string();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(!app.agent_running && !app.streaming, "sin turno");
        assert!(app.status.contains("workspace"), "aviso: {}", app.status);
        assert!(app.messages.is_empty(), "nada en contexto");
        assert_eq!(
            crate::db::count_messages(cid).unwrap_or(99),
            0,
            "nada en DB"
        );
        assert_eq!(app.input, "planifica sin workspace", "input conservado");
        assert!(app.undo.is_none(), "sin snapshot de undo");
        crate::db::delete_chat(cid).unwrap();
    }

    /// Tab / Shift+Tab cicla el foco entre respuestas IA (con wrap); fuera
    /// del cuestionario o sin preguntas no hace nada.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn qai_tab_cycles_focus_with_wrap() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-qai_tab_cycles");
        use crate::app::View;
        let mut app = App::default();
        app.view = View::Questionnaire;
        app.q_ai_questions = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        app.q_ai_answers = vec![String::new(), String::new(), String::new()];
        // Sin preguntas: nada.
        app.q_ai_questions.clear();
        let _ = questionnaire::handle(&mut app, Message::QAiFocusCycle(true));
        assert_eq!(app.q_ai_focus, 0);
        app.q_ai_questions = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        // Adelante con wrap 0→1→2→0.
        for expect in [1, 2, 0] {
            let _ = questionnaire::handle(&mut app, Message::QAiFocusCycle(true));
            assert_eq!(app.q_ai_focus, expect);
        }
        // Atrás con wrap 0→2.
        let _ = questionnaire::handle(&mut app, Message::QAiFocusCycle(false));
        assert_eq!(app.q_ai_focus, 2);
        // Escribir fija el foco en ese campo.
        let _ = questionnaire::handle(&mut app, Message::QAiAnswerChanged(1, "x".to_string()));
        assert_eq!(app.q_ai_focus, 1);
        // Fuera del cuestionario: nada.
        app.view = View::Chat;
        let _ = questionnaire::handle(&mut app, Message::QAiFocusCycle(true));
        assert_eq!(app.q_ai_focus, 1);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn slash_suggests_and_completes_unique_prefix() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-slash_suggests");
        let mut app = App::default();
        // Escribir `/code` puebla sugerencias con contenido.
        let _ = chat::handle(&mut app, Message::InputChanged("/code".to_string()));
        assert_eq!(app.input, "/code");
        assert!(
            app.skill_suggest.iter().any(|s| s.name == "code-review"),
            "sugiere por prefijo"
        );
        let rev = app
            .skill_suggest
            .iter()
            .find(|s| s.name == "code-review")
            .unwrap();
        assert!(!rev.description.is_empty() && !rev.preview.is_empty());
        // Sin `/` no hay sugerencias.
        let _ = chat::handle(&mut app, Message::InputChanged("hola".to_string()));
        assert!(app.skill_suggest.is_empty());
        // Enviar `/code mira esto` resuelve el prefijo único e inyecta.
        let cid = crate::db::create_chat("v09-slash-tmp").unwrap();
        app.chats.push(crate::db::ChatMeta {
            id: cid,
            title: "t".to_string(),
            project_id: None,
            archived: false,
            mode: crate::db::Mode::Chat,
            session_id: None,
        });
        app.active_chat = Some(cid);
        app.messages.clear();
        app.md.clear();
        app.msg_times.clear();
        app.msg_ids.clear();
        app.msg_usage.clear();
        app.config.active = crate::config::Provider::OpenAI;
        app.config.openai.api_key = "sk-test".to_string();
        app.config.openai.model = "gpt-test".to_string();
        app.input = "/code mira esto".to_string();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(app.streaming, "el prefijo único arranca el turno");
        assert!(app.messages[1].content.contains("code-review"));
        assert!(
            app.tool_logs
                .iter()
                .any(|l| l.contains("skill code-review"))
        );
        // Ambiguo: no hay turno y pide completar (con skills temporales).
        app.streaming = false;
        app.stream_gen += 1;
        for name in ["v09-tmp-amb2-aa", "v09-tmp-amb2-ab"] {
            let dir = crate::skills::skills_dir().join(name);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nnombre: {name}\ndescripcion: t\n---\n\nHaz.\n"),
            )
            .unwrap();
        }
        let n_msgs = app.messages.len();
        app.input = "/v09-tmp-amb2 haz".to_string();
        let _ = chat::handle(&mut app, Message::SendPressed);
        assert!(!app.streaming && !app.agent_running);
        assert!(app.status.contains("varias") && app.status.contains("v09-tmp-amb2-aa"));
        assert_eq!(app.messages.len(), n_msgs, "ambiguo no escribe mensajes");
        for name in ["v09-tmp-amb2-aa", "v09-tmp-amb2-ab"] {
            crate::skills::delete(name).unwrap();
        }
        crate::db::delete_chat(cid).unwrap();
    }

    /// v0.8.1: cambiar de modo anima el segmento por ticks con generación.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn mode_change_animates_by_ticks_and_generation() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-mode_change_an");
        let mut app = App::default();
        let cid = crate::db::create_chat("v0.8.1-anim-tmp").unwrap();
        app.chats.push(crate::db::ChatMeta {
            id: cid,
            title: "t".to_string(),
            project_id: None,
            archived: false,
            mode: crate::db::Mode::Chat,
            session_id: None,
        });
        app.active_chat = Some(cid);
        assert!(app.mode_anim.is_none());
        let _ = chat::handle(&mut app, Message::ModePicked(crate::db::Mode::Plan));
        assert_eq!(app.active_chat_meta().unwrap().mode, crate::db::Mode::Plan);
        assert!(app.mode_anim.is_some(), "el cambio arranca la animación");
        let anim_gen = app.mode_anim_gen;
        // Ticks viejos no pisan la animación nueva.
        let _ = chat::handle(&mut app, Message::ModeAnimTick(anim_gen.wrapping_sub(1), 5));
        assert_eq!(app.mode_anim.unwrap().1, 0);
        // Los ticks avanzan el paso y el 8 la cierra.
        let _ = chat::handle(&mut app, Message::ModeAnimTick(anim_gen, 4));
        assert_eq!(app.mode_anim.unwrap().1, 4);
        let _ = chat::handle(&mut app, Message::ModeAnimTick(anim_gen, 8));
        assert!(app.mode_anim.is_none());
        crate::db::delete_chat(cid).unwrap();
    }
}
