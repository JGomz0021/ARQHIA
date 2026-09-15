//! Handler del panel STACK (v0.9 Track A + puerta legal Track B).
//!
//! Un brazo por variante de Message. Búsqueda, preview, guardar (con
//! consentimiento `share_local`), valorar, bugs y copiar al workspace.

use iced::Task;

use crate::app::Message;
use crate::app::View;
use crate::app::state::App;
use crate::stack;

pub(crate) fn handle(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenStack => {
            if state.view != View::Stack {
                state.stack_from = state.view.clone();
            }
            state.view = View::Stack;
            state.stack_status.clear();
            Task::none()
        }
        Message::StackBack => {
            state.view = state.stack_from.clone();
            state.status.clear();
            Task::none()
        }
        Message::StackQueryChanged(v) => {
            state.stack_query = v;
            Task::none()
        }
        Message::StackTagsChanged(v) => {
            state.stack_tags = v;
            Task::none()
        }
        Message::StackSearch => {
            let tags: Vec<String> = state
                .stack_tags
                .split([',', ' '])
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            match stack::search(&state.stack_query, &tags, 20) {
                Ok(hits) => {
                    let n = hits.len();
                    state.stack_results = hits;
                    state.stack_selected = None;
                    state.stack_searched = true;
                    state.stack_status = if n == 0 {
                        "Sin resultados. Prueba con otras palabras o etiquetas.".to_string()
                    } else {
                        format!("{n} resultado(s). Toca uno para verlo.")
                    };
                }
                Err(e) => state.stack_status = format!("No se pudo buscar: {e}"),
            }
            Task::none()
        }
        Message::StackSelect(id) => {
            match stack::get(id) {
                Ok(full) => {
                    state.stack_selected = Some(full);
                    state.stack_status.clear();
                    state.stack_opinion.clear();
                    state.stack_bug.clear();
                }
                Err(e) => state.stack_status = format!("No se pudo abrir: {e}"),
            }
            Task::none()
        }
        Message::StackSaveTitleChanged(v) => {
            state.save_title = v;
            Task::none()
        }
        Message::StackSaveTagsChanged(v) => {
            state.save_tags = v;
            Task::none()
        }
        Message::StackSaveLangChanged(v) => {
            state.save_lang = v;
            Task::none()
        }
        Message::StackSaveLicensePicked(v) => {
            state.save_license = v;
            Task::none()
        }
        Message::StackSaveCodeChanged(v) => {
            state.save_code = v;
            Task::none()
        }
        Message::StackSave => {
            // Track B: sin `share_local` el guardado está bloqueado.
            if !state.config.stack_consent.share_local {
                state.stack_status =
                    "Guardar en local está desactivado: actívalo en Config → STACK.".to_string();
                return Task::none();
            }
            let tags: Vec<String> = state
                .save_tags
                .split([',', ' '])
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let author = state.config.identity.author_line();
            let item = stack::NewItem {
                title: state.save_title.clone(),
                code: state.save_code.clone(),
                tags,
                lang: state.save_lang.clone(),
                author,
                license: state.save_license.clone(),
                meta: Vec::new(),
            };
            match stack::save(&item) {
                Ok(id) => {
                    state.save_title.clear();
                    state.save_tags.clear();
                    state.save_code.clear();
                    // Puerta legal: avisa qué pasaría con la nube (v1.0).
                    let cloud_note = if state.config.stack_consent.share_cloud {
                        match stack::can_share_to_cloud(&item.license) {
                            Ok(()) => " (apto para la nube v1.0)".to_string(),
                            Err(e) => format!(" ({e})"),
                        }
                    } else {
                        String::new()
                    };
                    state.stack_status = format!("Guardado como snippet #{id}.{cloud_note}");
                }
                Err(e) => state.stack_status = format!("No se pudo guardar: {e}"),
            }
            Task::none()
        }
        Message::StackOpinionChanged(v) => {
            state.stack_opinion = v;
            Task::none()
        }
        Message::StackRate(stars) => {
            let Some(sel) = state.stack_selected.clone() else {
                state.stack_status = "Elige un snippet para valorarlo.".to_string();
                return Task::none();
            };
            match stack::rate(sel.id, stars, &state.stack_opinion, "usuario") {
                Ok(avg) => {
                    state.stack_opinion.clear();
                    state.stack_status = format!("Valorado con {stars}★ (media {avg:.1}).");
                    if let Ok(full) = stack::get(sel.id) {
                        state.stack_selected = Some(full);
                    }
                }
                Err(e) => state.stack_status = format!("No se pudo valorar: {e}"),
            }
            Task::none()
        }
        Message::StackCopyToWs => {
            let Some(sel) = state.stack_selected.clone() else {
                state.stack_status = "Elige un snippet para copiarlo.".to_string();
                return Task::none();
            };
            let Some(ws) = state.active_workspace() else {
                state.stack_status = "Sin workspace activo: abre un proyecto primero.".to_string();
                return Task::none();
            };
            let slug: String = sel
                .title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            let slug = if slug.is_empty() {
                "snippet".to_string()
            } else {
                slug
            };
            let ext = match sel.lang.trim().to_lowercase().as_str() {
                "rust" => "rs",
                "python" => "py",
                "javascript" | "js" => "js",
                "typescript" | "ts" => "ts",
                "go" => "go",
                _ => "txt",
            };
            let dest = ws.join(format!("{slug}.{ext}"));
            if dest.exists() {
                state.stack_status =
                    format!("Ya existe {}: cámbiale el nombre primero.", dest.display());
                return Task::none();
            }
            match std::fs::write(&dest, &sel.code) {
                Ok(()) => {
                    let _ = stack::record_execution(sel.id, true);
                    state.stack_status = format!("Copiado a {}.", dest.display());
                }
                Err(e) => state.stack_status = format!("No se pudo copiar: {e}"),
            }
            Task::none()
        }
        Message::StackBugChanged(v) => {
            state.stack_bug = v;
            Task::none()
        }
        Message::StackAskAgent => {
            // Llamada al agente: deja el pedido a medias en el chat para
            // que el usuario lo complete y pulse Enviar. Nombra el STACK
            // para que el planner lo consulte aunque `use_stack` esté off.
            let Some(sel) = state.stack_selected.clone() else {
                state.stack_status = "Elige un snippet para pedirlo al agente.".to_string();
                return Task::none();
            };
            state.input = format!(
                "Usa el snippet #{} «{}» del STACK para: ",
                sel.id, sel.title
            );
            state.skill_suggest.clear();
            state.view = View::Chat;
            state.status = "Completa el pedido y pulsa Enviar.".to_string();
            Task::none()
        }
        Message::StackReportBug => {
            let Some(sel) = state.stack_selected.clone() else {
                state.stack_status = "Elige un snippet para reportar.".to_string();
                return Task::none();
            };
            match stack::report_bug(sel.id, &state.stack_bug) {
                Ok(()) => {
                    state.stack_bug.clear();
                    state.stack_status = "Bug reportado (visible en el preview).".to_string();
                    if let Ok(full) = stack::get(sel.id) {
                        state.stack_selected = Some(full);
                    }
                }
                Err(e) => state.stack_status = format!("No se pudo reportar: {e}"),
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn stack_panel_opens_searches_and_returns() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-stack_panel_op");
        let mut app = App::default();
        // Abrir guarda el origen y Volver lo restaura.
        app.view = View::Chat;
        let _ = handle(&mut app, Message::OpenStack);
        assert_eq!(app.view, View::Stack);
        assert_eq!(app.stack_from, View::Chat);
        // Buscar no escribe: solo lee la DB del dev.
        app.stack_query = "axum health".to_string();
        let _ = handle(&mut app, Message::StackSearch);
        assert!(app.stack_searched);
        // Guardar bloqueado sin consentimiento share_local.
        app.config.stack_consent.share_local = false;
        app.save_title = "x".to_string();
        app.save_code = "y".to_string();
        let _ = handle(&mut app, Message::StackSave);
        assert!(app.stack_status.contains("desactivado"));
        // Sin selección, valorar/copiar/reportar avisan.
        app.stack_selected = None;
        let _ = handle(&mut app, Message::StackRate(5));
        assert!(app.stack_status.contains("Elige"));
        let _ = handle(&mut app, Message::StackCopyToWs);
        assert!(app.stack_status.contains("Elige"));
        let _ = handle(&mut app, Message::StackReportBug);
        assert!(app.stack_status.contains("Elige"));
        let _ = handle(&mut app, Message::StackBack);
        assert_eq!(app.view, View::Chat);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn stack_save_rate_bug_cycle_with_consent() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-stack_save_rat");
        let mut app = App::default();
        app.config.stack_consent.share_local = true;
        app.view = View::Stack;
        app.save_title = "v09-handler-tmp".to_string();
        app.save_tags = "v09handlertag".to_string();
        app.save_lang = "rust".to_string();
        app.save_license = "MIT".to_string();
        app.save_code = "fn v09handler() {}".to_string();
        let _ = handle(&mut app, Message::StackSave);
        assert!(app.stack_status.contains("Guardado como snippet #"));
        let id: i64 = app
            .stack_status
            .split('#')
            .nth(1)
            .and_then(|s| s.split('.').next())
            .and_then(|s| s.parse().ok())
            .expect("id en el estado");
        // Seleccionar + valorar + bug actualizan el preview.
        let _ = handle(&mut app, Message::StackSelect(id));
        assert_eq!(app.stack_selected.as_ref().map(|f| f.id), Some(id));
        app.stack_opinion = "útil".to_string();
        let _ = handle(&mut app, Message::StackRate(5));
        assert!(app.stack_status.contains("media 5.0"));
        app.stack_opinion = String::new();
        app.stack_bug = "roza en N".to_string();
        let _ = handle(&mut app, Message::StackReportBug);
        assert!(app.stack_status.contains("reportado"));
        assert!(
            app.stack_selected
                .as_ref()
                .map(|f| f.meta.iter().any(|(k, _)| k == "bug"))
                .unwrap_or(false)
        );
        crate::stack::delete_item(id).unwrap();
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn ask_agent_prefills_call_with_stack_mention() {
        let (_g, _t) = crate::db::test_guard::with_test_db("h-ask_agent_pref");
        let mut app = App::default();
        // Sin selección avisa y no toca el input.
        app.stack_selected = None;
        app.input.clear();
        let _ = handle(&mut app, Message::StackAskAgent);
        assert!(app.stack_status.contains("Elige"));
        assert!(app.input.is_empty());
        // Con selección deja el pedido a medias nombrando el STACK.
        app.stack_selected = Some(crate::stack::FullItem {
            id: 4242,
            title: "demo-call".to_string(),
            code: "fn x() {}".to_string(),
            tags: String::new(),
            lang: "rust".to_string(),
            rating: 0.0,
            ratings: 0,
            executions: 0,
            ok_runs: 0,
            author: String::new(),
            license: "MIT".to_string(),
            source: "local".to_string(),
            meta: Vec::new(),
        });
        app.view = View::Stack;
        let _ = handle(&mut app, Message::StackAskAgent);
        assert_eq!(app.view, View::Chat);
        assert!(app.input.contains("#4242") && app.input.contains("STACK"));
        // Y esa mención vale como consentimiento puntual del planner.
        assert!(crate::app::orchestrator::mentions_stack(&app.input));
    }
}
