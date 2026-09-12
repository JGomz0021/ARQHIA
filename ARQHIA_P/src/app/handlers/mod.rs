//! Handlers de update agrupados por dominio.
//!
//! `update_inner` (main) solo enruta; cada módulo posee sus brazos.
//! El compilador verifica exhaustividad en ambos niveles.

pub mod navigation;
pub mod chat;
pub mod projects;
pub mod agent;
pub mod config;
pub mod questionnaire;

#[cfg(test)]
mod tests {
    use super::{agent, chat, config, navigation, projects, questionnaire};
    use crate::app::{App, ConfigTab, Message, View};

    /// Cada grupo responde a un mensaje representativo: demuestra que el
    /// dispatch de update_inner está cableado (un grupo descableado caería
    /// en el wildcard y el estado no cambiaría).
    #[test]
    fn dispatch_routes_every_group() {
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

        let _ = config::handle(
            &mut app,
            Message::ConfigTab(ConfigTab::Apariencia),
        );
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
        assert_eq!(crate::questionnaire::levels::total_steps(Level::Avanzado), 12);
        // IA sin API configurada no cuelga: error visible y paso saltable.
        app.q_ai_loading = false;
        let _ = questionnaire::handle(&mut app, Message::QAiGenerate);
        if !app.config.has_any_api() {
            assert!(app.q_ai_error.contains("Configura"));
        }
        // Navegar el wizard no panica en ningún paso de cada nivel.
        for level in Level::ALL {
            let total = crate::questionnaire::levels::total_steps(level);
            for step in 0..total {
                let _ = crate::questionnaire::step_title(level, step);
                let _ = crate::questionnaire::validate_step(level, step, &app.q_answers);
            }
        }
    }

    /// v0.8.1: cancelar un cuestionario sin terminar deshace el proyecto.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn questionnaire_cancel_rolls_back_unfinished_project() {
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
        app.projects.push(crate::db::Project { id: pid, name: "qcancel-tmp-xyz".to_string(), path: Some(path) });
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
        app.projects.push(crate::db::Project { id: pid2, name: "qcancel-tmp-xyz2".to_string(), path: Some(path2) });
        app.q_project = Some(pid2);
        app.q_owns_project = true;
        app.view = View::Questionnaire;
        let _ = questionnaire::handle(&mut app, Message::QCancel);
        assert!(app.projects.iter().all(|p| p.id != pid2));
        assert_eq!(std::fs::read_to_string(dir2.join("mio.txt")).unwrap(), "no borrar");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// v0.8.1: terminar el cuestionario rellena docs y abre chat en Plan.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn questionnaire_finish_writes_docs_and_enters_plan() {
        use crate::questionnaire::Answers;
        let mut app = App::default();
        let base = std::env::temp_dir().join("arqhia-qfinish-test");
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("ws");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        let pid = crate::db::create_project("qfinish-tmp-xyz").unwrap();
        crate::db::set_project_path(pid, Some(&path)).unwrap();
        app.projects.push(crate::db::Project { id: pid, name: "qfinish-tmp-xyz".to_string(), path: Some(path) });
        app.q_project = Some(pid);
        app.q_owns_project = true;
        app.q_level = crate::questionnaire::Level::Principiante;
        app.q_answers = Answers {
            nombre: "Tienda".to_string(),
            descripcion: "Una tienda online de ejemplo".to_string(),
            objetivo: "Vender productos por internet".to_string(),
            funcionalidades: "Catálogo\nCarrito".to_string(),
            ..Default::default()
        };
        app.view = View::Questionnaire;
        let _ = questionnaire::handle(&mut app, Message::FinishQuestionnaire);
        assert!(app.q_error.is_empty(), "error: {}", app.q_error);
        // Docs rellenos (no vacíos) con las secciones del nivel.
        let specs = std::fs::read_to_string(dir.join("CONTEXT").join("SPECS.md")).unwrap();
        assert!(specs.contains("Estilo visual") && specs.contains("Catálogo"));
        let project = std::fs::read_to_string(dir.join("CONTEXT").join("PROJECT.md")).unwrap();
        assert!(project.contains("Tienda"));
        assert!(dir.join("Project").is_dir() && dir.join("ToDo.md").is_file());
        // Entra en modo Plan con el prompt precargado, sin disparar el turno.
        assert_eq!(app.view, View::Chat);
        let cid = app.active_chat.expect("chat Plan creado");
        let meta = app.chats.iter().find(|c| c.id == cid).unwrap();
        assert_eq!(meta.mode, crate::db::Mode::Plan);
        assert!(app.input.contains("ROADMAP") && app.input.contains("ToDo"));
        assert!(!app.agent_running && !app.streaming);
        // Limpieza para no contaminar la DB del dev.
        crate::db::delete_chat(cid).unwrap();
        crate::db::delete_project(pid).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    /// v0.8.1: cambiar de modo anima el segmento por ticks con generación.
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn mode_change_animates_by_ticks_and_generation() {
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
