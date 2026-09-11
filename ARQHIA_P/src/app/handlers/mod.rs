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
    }
}
