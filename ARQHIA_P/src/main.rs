use iced::{Element, Task, Theme};

mod agent;
mod app;
mod config;
mod db;
mod llm;
mod pricing;
mod questionnaire;
mod titles;
mod ui;
mod views;
mod workspace;

use app::{handlers, App, Message, View};


/// Wrapper de fiabilidad: un panic en cualquier handler se convierte en
/// mensaje de error visible en vez de cerrar el proceso de golpe.
fn update(state: &mut App, message: Message) -> Task<Message> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        update_inner(state, message)
    })) {
        Ok(task) => task,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic desconocido".to_string());
            eprintln!("ARQHIA: panic capturado en update: {detail}");
            state.streaming = false;
            state.agent_running = false;
            state.driver = None;
            state.pending_calls.clear();
            state.status = format!("Error interno (la app sigue abierta): {detail}");
            Task::none()
        }
    }
}

fn update_inner(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::GoChat | Message::GoHome | Message::ExitApp => handlers::navigation::handle(state, message),
        Message::InputChanged(..) | Message::SendPressed | Message::StreamChunk(..) | Message::StreamUsage(..) | Message::StreamDone | Message::StreamError(..) | Message::NewChat | Message::NewChatInProject(..) | Message::SelectChat(..) | Message::DeleteChat(..) | Message::ConfirmDeleteChat | Message::CancelDelete | Message::ToggleChatMenu(..) | Message::ToggleMovePick(..) | Message::AssignChatProject { .. } | Message::ArchiveChat(..) | Message::UnarchiveChat(..) | Message::ToggleArchived | Message::NavigateProject(..) | Message::QuickSwitchModel(..) | Message::QuickReasoningPicked(..) | Message::LinkClicked(..) | Message::ModePicked(..) | Message::CloseOverlays | Message::RetryLast | Message::ToggleLogExpand => handlers::chat::handle(state, message),
        Message::ShowCreateModal | Message::HideCreateModal | Message::CreateNameChanged(..) | Message::CreatePathChanged(..) | Message::SubmitCreateProject | Message::OpenProject | Message::EnterProject(..) | Message::FolderPicked(..) | Message::NewProjectNameChanged(..) | Message::NewProjectPathChanged(..) | Message::CreateProject | Message::ToggleProjectForm | Message::ToggleProject(..) | Message::ToggleProjectMenu(..) | Message::OpenWorkspaceFolder(..) | Message::DeleteProject(..) | Message::ConfirmDeleteProject | Message::CancelDeleteProject | Message::WorkspacePathChanged(..) | Message::AssignWorkspace(..) | Message::ClearWorkspace(..) | Message::UploadFiles(..) | Message::FilesPicked(..) | Message::DeleteUpload(..) | Message::ConfigDeleteProject(..) | Message::ConfirmConfigDelete | Message::CancelConfigDelete => handlers::projects::handle(state, message),
        Message::AgentPlan(..) | Message::AgentLlm(..) | Message::AgentExecDone(..) | Message::ApproveTools | Message::DenyTools | Message::DenyToolsRemember | Message::StopAgent | Message::AgentAudit(..) | Message::PlanDone(..) | Message::ExecutePlan | Message::DismissPlan => handlers::agent::handle(state, message),
        Message::OpenConfig | Message::ConfigBack | Message::ProviderPicked(..) | Message::ApiKeyChanged(..) | Message::BaseUrlChanged(..) | Message::ModelChanged(..) | Message::EditReasoningPicked(..) | Message::SaveConfig | Message::TestConnection | Message::TestResult(..) | Message::UseDefaultBaseUrl | Message::UseDefaultModel | Message::ThemePicked(..) | Message::AccentPicked(..) | Message::TextSizePicked(..) | Message::DensityPicked(..) | Message::PermReadToggled(..) | Message::PermWriteToggled(..) | Message::PermBashToggled(..) | Message::PermNetToggled(..) | Message::PermInstallToggled(..) | Message::PermDomainsChanged(..) | Message::PermExtraChanged(..) | Message::SavePermLists | Message::LimitItersPicked(..) | Message::LimitTasksPicked(..) | Message::LimitTimeoutPicked(..) | Message::LimitUploadPicked(..) | Message::LimitReadPicked(..) | Message::LimitTokensPicked(..) | Message::LimitHistoryPicked(..) | Message::ConfigTab(..) | Message::OpenModelBrowser | Message::CloseModelBrowser | Message::InitPricing | Message::ModelSearchChanged(..) | Message::ModelPriceFilterPicked(..) | Message::ModelOnlyToolsToggled(..) | Message::ModelSortPriceToggled(..) | Message::RefreshPricing | Message::PricingFetched(..) | Message::LocalModelsFetched(..) | Message::PickModel(..) => handlers::config::handle(state, message),
        Message::QNext | Message::QBack | Message::QCancel | Message::QNombreChanged(..) | Message::QDescChanged(..) | Message::QUbicChanged(..) | Message::QObjChanged(..) | Message::QPublicoPicked(..) | Message::QInterfazPicked(..) | Message::FinishQuestionnaire => handlers::questionnaire::handle(state, message),
    }
}


pub(crate) fn view(state: &App) -> Element<'_, Message> {
    // Red de seguridad: un panic al dibujar muestra un aviso en vez de
    // cerrar el proceso (complementa el guard de update).
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| view_inner(state))) {
        Ok(el) => el,
        Err(_) => iced::widget::container(
            iced::widget::text("Error al dibujar esta vista. Vuelve al Inicio o reinicia la app.").size(14),
        )
        .width(iced::Fill)
        .height(iced::Fill)
        .center_x(iced::Fill)
        .center_y(iced::Fill)
        .into(),
    }
}

fn view_inner(state: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = match state.view {
        View::Home => views::home::view_home(views::home::HomeProps {
            creating: state.creating,
            create_name: &state.create_name,
            create_path: &state.create_path,
            status: &state.status,
            projects: &state.projects,
            chats: &state.chats,
            active_chat: state.active_chat,
            config: &state.config,
        }),
        View::Config => views::config_view::view_config(state),
        View::Questionnaire => views::questionnaire::view_questionnaire(state),
        View::Chat => view_main(state),
    };
    // App surface: anchors all content to the native window.
    iced::widget::container(content)
        .width(iced::Fill)
        .height(iced::Fill)
        .style(|t: &Theme| crate::ui::design::app(t))
        .into()
}

fn view_main(state: &App) -> Element<'_, Message> {
    use iced::widget::{container, row};
    row![
        container(views::sidebar::view_sidebar(state)).width(248).height(iced::Fill).style(
            |t: &Theme| crate::ui::design::sidebar(t)
        ),
        container(views::chat::view_chat(state)).width(iced::Fill).height(iced::Fill),
    ]
    .spacing(0)
    .into()
}

fn main() -> iced::Result {
    iced::application("ARQHIA", update, view)
        .theme(views::app_theme)
        .subscription(subscription)
        .window_size((1560.0, 880.0))
        .run()
}

/// Atajos de teclado v0.7.1 (solo con Ctrl, salvo Esc: no roban teclas
/// escribiendo en inputs porque el texto consumido no llega aquí).
/// `Ctrl+N` nuevo chat, `Ctrl+1/2/3` modos, `Ctrl+O` abrir proyecto,
/// `Ctrl+,` configuración, `Esc` cerrar menús.
fn subscription(_state: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        iced::keyboard::on_key_press(key_shortcut),
        init_pricing(),
    ])
}

/// One-shot al arrancar: si el catálogo de modelos está vacío, se descarga.
fn init_pricing() -> iced::Subscription<Message> {
    iced::Subscription::run(|| futures::stream::once(async { Message::InitPricing }))
}

fn key_shortcut(
    key: iced::keyboard::Key,
    mods: iced::keyboard::Modifiers,
) -> Option<Message> {
    use iced::keyboard::{Key, key::Named};
    if key == Key::Named(Named::Escape) {
        return Some(Message::CloseOverlays);
    }
    if !mods.control() {
        return None;
    }
    match key {
        Key::Character(ref c) => match c.as_str() {
            "n" | "N" => Some(Message::NewChat),
            "o" | "O" => Some(Message::OpenProject),
            "," => Some(Message::OpenConfig),
            "1" => Some(Message::ModePicked(db::Mode::Chat)),
            "2" => Some(Message::ModePicked(db::Mode::Plan)),
            "3" => Some(Message::ModePicked(db::Mode::Work)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{key_shortcut, Message};
    use crate::app::projects::trash_dir;
    #[test]
    fn trash_moves_dir_and_keeps_files() {
        let base = std::env::temp_dir().join("arqhia-trash-test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("origen");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub").join("f.txt"), "hola").unwrap();
        let dest = trash_dir(&src.to_string_lossy()).expect("trash falló");
        assert!(!src.exists(), "origen debería desaparecer");
        assert!(
            std::fs::read_to_string(std::path::PathBuf::from(&dest).join("sub").join("f.txt"))
                .unwrap()
                == "hola"
        );
        let _ = std::fs::remove_dir_all(&base);
        // Limpia lo que se movió a la papelera real
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn trash_rejects_missing() {
        assert!(trash_dir("/ruta/que/no/existe/xyz-arqhia").is_err());
    }

    #[test]
    fn keyboard_shortcuts_map_to_messages() {
        use iced::keyboard::{Key, Modifiers, key::Named};
        use crate::db::Mode;
        let ctrl = Modifiers::CTRL;
        let none = Modifiers::empty();
        // Los 5 atajos v0.7.1 (sin Tab: colisiona con el foco).
        assert!(matches!(key_shortcut(Key::Character("n".into()), ctrl), Some(Message::NewChat)));
        assert!(matches!(key_shortcut(Key::Character("o".into()), ctrl), Some(Message::OpenProject)));
        assert!(matches!(key_shortcut(Key::Character(",".into()), ctrl), Some(Message::OpenConfig)));
        assert!(matches!(
            key_shortcut(Key::Character("2".into()), ctrl),
            Some(Message::ModePicked(Mode::Plan))
        ));
        assert!(matches!(key_shortcut(Key::Character("1".into()), ctrl), Some(Message::ModePicked(Mode::Chat))));
        assert!(matches!(key_shortcut(Key::Character("3".into()), ctrl), Some(Message::ModePicked(Mode::Work))));
        assert!(matches!(
            key_shortcut(Key::Named(Named::Escape), none),
            Some(Message::CloseOverlays)
        ));
        // Sin Ctrl no hay atajo (no roba teclas escribiendo); otras teclas nada.
        assert!(key_shortcut(Key::Character("n".into()), none).is_none());
        assert!(key_shortcut(Key::Character("x".into()), ctrl).is_none());
    }
}
