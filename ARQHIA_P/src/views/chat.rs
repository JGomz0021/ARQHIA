//! Chat: el workspace principal (mensajes, composer, actividad).
//!
//! Render puro sobre App. Sin I/O ni tareas.

use iced::widget::markdown;
use iced::{Element, Theme};

use crate::app::{App, Message};
use crate::config::{AppConfig, Provider};
use crate::llm::Role;
use crate::ui::design::Tone;


/// Etiquetas "Provider · modelo" para el selector rápido.
/// Nube: solo con key. Local siempre visible (LM Studio puede no pedir key).
fn model_options(config: &AppConfig) -> Vec<String> {
    let mut out = Vec::new();
    for p in Provider::ALL {
        let c = match p {
            Provider::OpenAI => &config.openai,
            Provider::Anthropic => &config.anthropic,
            Provider::OpenRouter => &config.openrouter,
            Provider::Local => &config.local,
        };
        if p == Provider::Local || !c.api_key.trim().is_empty() {
            let model = if c.model.trim().is_empty() {
                "(elige modelo)".to_string()
            } else {
                c.model.clone()
            };
            out.push(format!("{p} · {model}"));
        }
    }
    if out.is_empty() {
        out.push("Sin modelos (Config)".to_string());
    }
    out
}

/// Categoría visible + resumen de una llamada pendiente:
/// `tool · objetivo · alcance` (v0.7 Track B).
fn call_summary(call: &crate::agent::PendingCall) -> (Tone, &'static str, String, String) {
    use crate::agent::tools::{ToolCat, category_of_call};
    let (cat, label, tone) = match category_of_call(&call.name, &call.args) {
        ToolCat::Read => (ToolCat::Read, "Lectura", Tone::Neutral),
        ToolCat::Write => (ToolCat::Write, "Escritura", Tone::Warn),
        ToolCat::Bash => (ToolCat::Bash, "Consola", Tone::Warn),
        ToolCat::Net => (ToolCat::Net, "Red", Tone::Accent),
        ToolCat::Install => (ToolCat::Install, "Instalación", Tone::Err),
    };
    let _ = cat;
    let objetivo = short_preview(&call.args.to_string());
    let alcance = ["path", "file", "dir", "cmd", "command", "url", "query"]
        .iter()
        .filter_map(|k| call.args.get(*k).and_then(|v| v.as_str()))
        .next()
        .map(|v| crate::ui::design::trunc_end(v.trim(), 60))
        .unwrap_or_default();
    (tone, label, objetivo, alcance)
}

/// Vista previa de una línea para args JSON en el modal de permisos.
fn short_preview(s: &str) -> String {
    let one: String = s.replace('\n', " ").chars().take(70).collect();
    if s.len() > 70 {
        format!("{one}…")
    } else {
        one
    }
}

/// Separador de miles con punto (p. ej. 264.000).
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(*b as char);
    }
    out
}

/// Estado visual de una tarea del plan (mismo contrato para Plan y Work).
fn task_state(done: bool, active: bool) -> (Tone, &'static str, &'static str) {
    if done {
        (Tone::Ok, "x", "hecho")
    } else if active {
        (Tone::Accent, ">", "en progreso")
    } else {
        (Tone::Neutral, "", "pendiente")
    }
}

/// Checklist de tareas como cards (no texto plano): checkbox real de estado,
/// número de tarea, descripción y etiqueta de estado.
fn checklist<'a>(
    app_theme: Theme,
    ts: f32,
    tasks: &'a [crate::app::orchestrator::OrchTask],
) -> Element<'a, Message> {
    use iced::widget::{column, container, row, text};
    use crate::ui::{components, design};
    let rows = column(
        tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let (kind, glyph, state) = task_state(t.done, t.active);
                let color = design::tone(&app_theme, kind);
                container(
                    row![
                        components::state_box(app_theme.clone(), kind, glyph),
                        column![
                            text(format!("Tarea {}", i + 1))
                                .size(design::fs(ts, 11))
                                .color(color),
                            text(&t.desc).size(design::fs(ts, 13)).color(if t.done {
                                design::ink_2(&app_theme)
                            } else {
                                design::ink(&app_theme)
                            }),
                        ]
                        .spacing(1)
                        .width(iced::Fill),
                        text(state).size(design::fs(ts, 11)).color(color),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .width(iced::Fill),
                )
                .width(iced::Fill)
                .padding([6, 8])
                .style(move |th: &Theme| design::task_card(th, kind))
                .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(6);
    rows.into()
}

pub(crate) fn view_chat(state: &App) -> Element<'_, Message> {
    use iced::widget::{column, container, pick_list, row, scrollable, text, text_input};
    use crate::ui::{components, design};
    use crate::ui::design::{Tone, type_scale};
    let app_theme = super::app_theme(state);
    let dim = design::ink_2(&app_theme);
    let ts = state.config.appearance.text_size.scale();
    let cx = state.config.appearance.compact();
    let title = state
        .active_chat_meta()
        .map(|c| c.title.clone())
        .unwrap_or_else(|| "Sin chat".to_string());
    let project_of_chat = state
        .active_chat_meta()
        .and_then(|c| c.project_id)
        .and_then(|id| state.projects.iter().find(|p| p.id == id));
    // Sin chat activo, el destino pendiente manda (selector sí responde).
    let pending_name = state
        .pending_project
        .and_then(|id| state.projects.iter().find(|p| p.id == id))
        .map(|p| p.name.clone());

    let mut project_options = vec!["Sin proyecto".to_string()];
    project_options.extend(state.projects.iter().map(|p| p.name.clone()));
    let current_project = project_of_chat
        .map(|p| p.name.clone())
        .or(pending_name.clone())
        .unwrap_or_else(|| "Sin proyecto".to_string());

    // Header: proyecto · título · acciones. El modo vive en el composer y
    // volver a Inicio ya está en el sidebar (sin botón "<" duplicado).
    let mode = state.active_mode();
    let header = row![
        match project_of_chat.map(|p| p.name.clone()).or(pending_name) {
            Some(name) => components::badge(app_theme.clone(), Tone::Accent, name),
            None => components::badge(app_theme.clone(), Tone::Neutral, "Sin proyecto"),
        },
        text(title.clone()).size(design::fs(ts, type_scale::EMPHASIS)),
        iced::widget::horizontal_space(),
        pick_list(project_options, Some(current_project), Message::NavigateProject)
            .width(150)
            .style(|t: &Theme, s| design::bare_pick(t, s)),
        components::head_btn("Ajustes".to_string()).on_press(Message::OpenConfig),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // Estadísticas de la sesión (parte superior): contexto usado, tokens y
    // gasto de API acumulado. El contexto incluye tools/lecturas del agente
    // (`App.context_tokens`) además de los mensajes.
    let msg_estimate: u64 = state
        .messages
        .iter()
        .map(|m| crate::llm::estimate_tokens_text(&m.content) as u64)
        .sum();
    let context_tokens = state.context_tokens.max(msg_estimate);
    let model_now = state.config.active_config().model;
    let pid_now = crate::pricing::provider_id_for(
        state.config.active,
        &state.config.active_config().base_url,
    );
    let window = state
        .pricing
        .context_window_in(pid_now.as_deref(), &model_now)
        .unwrap_or_else(|| crate::llm::context_window(state.config.active, &model_now));
    let pct = context_tokens
        .saturating_mul(100)
        .checked_div(window)
        .unwrap_or(0)
        .min(100);
    let pct_tone = if pct >= 90 {
        Tone::Err
    } else if pct >= 70 {
        Tone::Warn
    } else {
        Tone::Ok
    };
    let stats_bar = container(
        row![
            text("Contexto").size(design::fs(ts, 11)).color(dim),
            text(format!(
                "{} tokens / {}% used",
                thousands(context_tokens),
                pct
            ))
            .size(design::fs(ts, 12))
            .color(design::tone(&app_theme, pct_tone)),
            text(format!("de {}", thousands(window)))
                .size(design::fs(ts, 11))
                .color(dim),
            iced::widget::horizontal_space(),
            text(format!(
                "Sesión  in {}  out {}",
                thousands(state.session_in),
                thousands(state.session_out)
            ))
            .size(design::fs(ts, 11))
            .color(dim),
            components::badge(
                app_theme.clone(),
                Tone::Accent,
                format!("API {}", crate::llm::format_cost(Some(state.session_cost)))
            ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 8])
    .style(|t: &Theme| design::well_box(t));

    // Aviso de contexto casi lleno: se seguirá (olvidando lo más antiguo),
    // no se detiene la conversación por ello.
    let context_warning: Element<'_, Message> = if pct >= 90 {
        container(
            row![
                components::badge(app_theme.clone(), Tone::Warn, format!("{pct}%")),
                text("Contexto casi lleno: a partir de ahora empezaré a olvidar los mensajes más antiguos para seguir.")
                    .size(design::fs(ts, 12))
                    .color(design::tone(&app_theme, Tone::Warn)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(iced::Fill)
        .padding([6, 10])
        .style(|t: &Theme| design::approval(t))
        .into()
    } else {
        iced::widget::vertical_space().height(0).into()
    };

    // Workspace del chat activo (None = chat directo contra el modelo).
    let ws = state.active_workspace();

    let msgs: Element<'_, Message> = if state.messages.is_empty() {
        // Saludo de fondo: centrado horizontal con aire arriba (sin height
        // Fill: el contenido de un scrollable nunca debe llenar su eje).
        // Se va al enviar el 1er mensaje.
        container(
            column![
                iced::widget::vertical_space().height(150),
                column![
                    text("¿Qué vamos a hacer hoy?").size(design::fs(ts, 24)),
                    text(
                        if ws.is_some() {
                            "El agente tiene tu workspace: pide cambios, archivos o comandos."
                        } else {
                            "Pregunta lo que sea, o asigna un workspace para activar el agente."
                        }
                    )
                    .size(design::fs(ts, 14))
                    .color(dim)
                    .align_x(iced::Alignment::Center),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            ]
            .align_x(iced::Alignment::Center),
        )
        .width(iced::Fill)
        .center_x(iced::Fill)
        .padding(24)
        .into()
    } else {
        column(
            state
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                if m.role == Role::Assistant {
                    if m.content.is_empty() && state.streaming {
                        return container(
                            text("Escribiendo...").size(design::fs(ts, 14)).color(dim),
                        )
                        .padding([8, 12])
                        .style(|t: &Theme| design::ai_msg(t))
                        .into();
                    }
                    let items: &[markdown::Item] = state.md.get(i).map(Vec::as_slice).unwrap_or(&[]);
                    // Burbuja del agente: fondo hundido + borde, alineada a la
                    // izquierda. Sin etiqueta "ARQHIA" (el emisor ya es evidente).
                    let usage_el: Element<'_, Message> = state
                        .msg_usage
                        .get(i)
                        .and_then(|o| *o)
                        .map(|u| {
                            let model = state.config.active_config().model;
                            let pid = crate::pricing::provider_id_for(
                                state.config.active,
                                &state.config.active_config().base_url,
                            );
                            let cost = u
                                .cost
                                .or_else(|| state.pricing.cost_in(pid.as_deref(), &model, u))
                                .or_else(|| {
                                    crate::llm::estimate_cost_usd(state.config.active, &model, u)
                                });
                            text(format!(
                                "in {} · out {} · cache {} · {}",
                                thousands(u.input as u64),
                                thousands(u.output as u64),
                                thousands(u.cached as u64),
                                crate::llm::format_cost(cost)
                            ))
                            .size(design::fs(ts, 11))
                            .color(dim)
                            .into()
                        })
                        .unwrap_or_else(|| iced::widget::horizontal_space().width(0).into());
                    container(
                        column![
                            markdown::view(
                                items.iter(),
                                markdown::Settings::with_text_size(design::fs(ts, 14)),
                                design::markdown_style(&app_theme),
                            )
                            .map(Message::LinkClicked),
                            usage_el,
                        ]
                        .spacing(6),
                    )
                    .padding(design::pad(cx, 10))
                    .width(iced::Fill)
                    .style(|t: &Theme| design::ai_msg(t))
                    .into()
                } else if m.role == Role::User {
                    // Burbuja del usuario: superficie elevada + borde de acento,
                    // alineada a la izquierda (antes se salía por la derecha).
                    container(
                        column![
                            text("Tú").size(design::fs(ts, 12)).color(dim),
                            text(&m.content).size(design::fs(ts, 14)),
                            text(format!(
                                "in ~{} tokens",
                                crate::llm::estimate_tokens_text(&m.content)
                            ))
                            .size(design::fs(ts, 11))
                            .color(dim),
                        ]
                        .spacing(2),
                    )
                    .padding(design::pad(cx, 10))
                    .width(iced::Fill)
                    .style(|t: &Theme| design::user_msg(t))
                    .into()
                } else {
                    text(&m.content).size(design::fs(ts, 12)).color(dim).into()
                }
            })
            .collect::<Vec<_>>(),
        )
        .spacing(design::gap(cx, 12))
        .into()
    };

    let status: Element<'_, Message> = if state.status.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        let kind = design::status_tone(&state.status);
        let color = design::tone(&app_theme, kind);
        let retry: Element<'_, Message> = if kind == Tone::Err
            && !state.streaming
            && !state.agent_running
        {
            components::quiet_btn("Reintentar".to_string())
                .on_press(Message::RetryLast)
                .into()
        } else {
            iced::widget::horizontal_space().width(0).into()
        };
        row![
            text(components::log_icon(kind)).size(12).color(color),
            text(&state.status)
                .size(design::fs(ts, 12))
                .color(if kind == Tone::Neutral { dim } else { color }),
            iced::widget::horizontal_space(),
            retry,
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    };

    // Composer: la caja de entrada va sola arriba; modos, modelo y datos
    // justo debajo. Envío primario a la derecha.
    let send_label = if state.streaming || state.agent_running {
        "..."
    } else {
        "Enviar"
    };
    let mut send_btn = iced::widget::button(send_label)
        .padding([10, 20])
        .style(|t: &Theme, s| design::primary(t, s));
    if !state.streaming && !state.agent_running && !state.input.trim().is_empty() {
        send_btn = send_btn.on_press(Message::SendPressed);
    }
    let stop_btn: Element<'_, Message> = if state.agent_running {
        components::quiet_btn("Detener".to_string()).on_press(Message::StopAgent).into()
    } else {
        iced::widget::horizontal_space().width(0).into()
    };
    let model_opts = model_options(&state.config);
    let active_prefix = format!("{} ·", state.config.active);
    let current_opt = model_opts
        .iter()
        .find(|o| o.starts_with(&active_prefix))
        .or(model_opts.first())
        .cloned();
    // Selector segmentado Chat | Plan | Work (v0.7.1): mismo alto para los
    // tres modos dentro de una pista, con el activo en acento.
    let mut seg = row![].spacing(2).align_y(iced::Alignment::Center);
    for m in crate::db::Mode::ALL {
        let active = m == mode;
        seg = seg.push(
            iced::widget::button(text(m.label()).size(design::fs(ts, 12)))
                .padding([5, 14])
                .style(move |t: &Theme, s| design::segment(t, s, active))
                .on_press(Message::ModePicked(m)),
        );
    }
    let mode_row = container(seg)
        .padding(2)
        .style(|t: &Theme| design::segmented(t));
    // Nivel de razonamiento del modelo activo (opciones según models.dev).
    let r_model = state.config.active_config().model;
    let r_pid = crate::pricing::provider_id_for(
        state.config.active,
        &state.config.active_config().base_url,
    );
    let r_opts = state.pricing.effort_choices_in(r_pid.as_deref(), &r_model);
    let r_current = if state.config.active_config().reasoning_effort.is_empty() {
        "auto".to_string()
    } else {
        state.config.active_config().reasoning_effort.clone()
    };
    let reasoning_pick: Element<'_, Message> = pick_list(
        r_opts,
        Some(r_current),
        Message::QuickReasoningPicked,
    )
    .width(110)
    .style(|t: &Theme, s| design::accent_pick(t, s))
    .into();
    let input_box = text_input(
        if ws.is_some() {
            "Pide al agente..."
        } else {
            "Escribe y pulsa Enter..."
        },
        &state.input,
    )
    .size(design::fs(ts, 16))
    .padding([12, 10])
    .style(|t: &Theme, s| design::bare(t, s))
    .on_input(Message::InputChanged)
    .on_submit(Message::SendPressed)
    .width(iced::Fill);
    let composer = container(
        column![
            row![input_box, stop_btn, send_btn]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            row![
                mode_row,
                text("Modelo").size(design::fs(ts, 11)).color(dim),
                pick_list(model_opts, current_opt, Message::QuickSwitchModel)
                    .width(240)
                    .style(|t: &Theme, s| design::accent_pick(t, s)),
                text("Nivel").size(design::fs(ts, 11)).color(dim),
                reasoning_pick,
                text(format!(
                    "{} · {}",
                    state.config.active,
                    state.config.active_config().model
                ))
                .size(design::fs(ts, 11))
                .color(dim),
                iced::widget::horizontal_space(),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8),
    )
    .padding([design::pad(cx, 10), 12])
    .style(|t: &Theme| design::composer(t));

    // Plan aprobable (v0.7.1, modo Plan): checklist de PLAN.md + Ejecutar.
    // El disco sigue intacto hasta que se pulsa "Ejecutar plan" (pasa a Work).
    let plan_panel: Element<'_, Message> = if !state.show_plan || state.orch_tasks.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        let done = state.orch_tasks.iter().filter(|t| t.done).count();
        container(
            column![
                row![
                    components::section_label(app_theme.clone(),
                        format!("PLAN · {done}/{}", state.orch_tasks.len())
                    ),
                    iced::widget::horizontal_space(),
                    components::badge(app_theme.clone(), Tone::Accent, "sin ejecutar"),
                ]
                .align_y(iced::Alignment::Center),
                checklist(app_theme.clone(), ts, &state.orch_tasks),
                row![
                    components::primary_btn("Ejecutar plan".to_string(), 13)
                        .on_press(Message::ExecutePlan),
                    components::quiet_btn("Descartar".to_string()).on_press(Message::DismissPlan),
                ]
                .spacing(8),
            ]
            .spacing(8),
        )
        .padding(12)
        .style(|t: &Theme| design::card(t))
        .into()
    };

    // Permission approval: warm interrupting surface, above the composer.
    let approval_panel: Element<'_, Message> = if state.pending_calls.is_empty() {
        iced::widget::vertical_space().height(0).into()
    } else {
        let lines = column(
            state
                .pending_calls
                .iter()
                .map(|c| {
                    let (tone, label, objetivo, alcance) = call_summary(c);
                    let mut cell = column![
                        row![
                            components::badge(app_theme.clone(), tone, label),
                            text(&c.name).size(design::fs(ts, 12)),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                        text(objetivo).size(design::fs(ts, 12)),
                    ]
                    .spacing(1);
                    if !alcance.is_empty() {
                        cell = cell.push(text(alcance).size(design::fs(ts, 11)));
                    }
                    cell.into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(4);
        container(
            column![
                row![
                    text(format!(
                        "El agente pide permiso · {} acciones",
                        state.pending_calls.len()
                    ))
                    .size(design::fs(ts, 13)),
                    iced::widget::horizontal_space(),
                    components::badge(app_theme.clone(), Tone::Warn, "Requiere aprobación"),
                ]
                .align_y(iced::Alignment::Center),
                scrollable(lines).height(84),
                row![
                    components::primary_btn("Permitir".to_string(), 13).on_press(Message::ApproveTools),
                    components::quiet_btn("Denegar".to_string()).on_press(Message::DenyTools),
                    components::icon_btn("No preguntar más".to_string())
                        .on_press(Message::DenyToolsRemember),
                ]
                .spacing(8),
            ]
            .spacing(6),
        )
        .padding(10)
        .style(|t: &Theme| design::approval(t))
        .into()
    };

    // Plan checklist: tonal dots, no box.
    let done_count = state.orch_tasks.iter().filter(|t| t.done).count();
    // Checklist de progreso del orquestador (Work). En modo Plan manda el
    // plan_panel de arriba, así que aquí se oculta para no duplicar.
    let tasks_panel: Element<'_, Message> =
        if state.orch_tasks.is_empty() || state.show_plan {
            iced::widget::vertical_space().height(0).into()
        } else {
            column![
                components::section_label(app_theme.clone(),
                    format!("Tareas · {done_count}/{}", state.orch_tasks.len())
                ),
                checklist(app_theme.clone(), ts, &state.orch_tasks),
            ]
            .spacing(6)
            .into()
        };

    // Activity well: solo tiene sentido con agente (Plan/Work). En modo Chat
    // no se muestra (el chat plano no ejecuta tools).
    let log_panel: Element<'_, Message> =
        if mode == crate::db::Mode::Chat
            || (state.tool_logs.is_empty() && ws.is_none())
        {
            iced::widget::vertical_space().height(0).into()
        } else {
        let visible = if state.log_expanded { 40 } else { 12 };
        let start = state.tool_logs.len().saturating_sub(visible);
        let lines = column(
            state.tool_logs[start..]
                .iter()
                .map(|l| components::activity_row(app_theme.clone(), l))
                .collect::<Vec<_>>(),
        )
        .spacing(4);
        let mut well = column![
            row![
                components::section_label(app_theme.clone(), "Actividad"),
                iced::widget::horizontal_space(),
                components::head_btn(if state.log_expanded { "Reducir" } else { "Ampliar" }.to_string())
                    .on_press(Message::ToggleLogExpand),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(6);
        if !state.tool_logs.is_empty() {
            let h = if state.log_expanded { 460.0 } else { 170.0 };
            well = well.push(scrollable(lines).height(h));
        }
        container(well)
        .padding(12)
        .style(|t: &Theme| design::well_box(t))
        .into()
    };

    // Caja de mensajes centrada y con ancho según densidad:
    // cómoda ~1020px, compacta ~860px (un 20% menos que a todo el ancho).
    let chat_w: f32 = if cx { 860.0 } else { 1020.0 };
    let messages_area = container(
        container(scrollable(container(msgs).width(iced::Fill)).height(iced::Fill))
            .max_width(chat_w)
            .height(iced::Fill),
    )
    .width(iced::Fill)
    .height(iced::Fill)
    .center_x(iced::Fill);

    let panels = container(
        column![
            context_warning,
            status,
            approval_panel,
            tasks_panel,
            plan_panel
        ]
        .spacing(10),
    )
        .width(iced::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 16.0,
        });

    // Dock inferior (composer + log): fondo elevado y sombra superior para que
    // la frontera con el chat sea clara.
    let dock = container(
        column![composer, log_panel].spacing(10),
    )
    .width(iced::Fill)
    .padding(iced::Padding {
        top: design::pad(cx, 14) as f32,
        right: 16.0,
        bottom: design::pad(cx, 14) as f32,
        left: 16.0,
    })
    .style(|t: &Theme| design::dock(t));

    column![
        container(column![header, stats_bar].spacing(8))
            .width(iced::Fill)
            .padding(iced::Padding {
                top: 12.0,
                right: 16.0,
                bottom: 6.0,
                left: 16.0,
            }),
        messages_area,
        panels,
        dock,
    ]
    .spacing(8)
    .width(iced::Fill)
    .height(iced::Fill)
    .into()
}
