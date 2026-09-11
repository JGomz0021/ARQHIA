//! ARQHIA components: small reusable elements built on design tokens.
//!
//! Conceptual catalog: SectionHeader, Badge/StatusDot, FieldLabel/Helper,
//! ActivityRow (tool activity language), EmptyState. No abstractions for
//! their own sake — each one is used on at least two screens.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Theme};

use crate::app::Message;
use crate::ui::design::{self, Tone};

// ---------------------------------------------------------------------------
// Headers & labels
// ---------------------------------------------------------------------------

/// Uppercase muted section header, e.g. "PROYECTOS", "ACTIVIDAD".
pub fn section_label(theme: Theme, label: impl Into<String>) -> Element<'static, Message> {
    text(label.into().to_uppercase())
        .size(12)
        .color(design::ink_2(&theme))
        .into()
}

/// Field label (13, primary) + optional helper (11, muted) stacked.
pub fn field(theme: Theme, label: impl Into<String>, helper: Option<String>) -> Element<'static, Message> {
    let mut col = column![text(label.into()).size(14).color(design::ink(&theme))].spacing(2);
    if let Some(h) = helper {
        let h: String = h;
        col = col.push(text(h).size(12).color(design::ink_2(&theme)));
    }
    col.into()
}

// ---------------------------------------------------------------------------
// Badges & dots
// ---------------------------------------------------------------------------

/// Pill badge: tonal wash background, 12px label.
pub fn badge(theme: Theme, kind: Tone, label: impl Into<String>) -> Element<'static, Message> {
    let label: String = label.into();
    container(text(label).size(12).color(design::tone(&theme, kind)))
        .padding([3, 10])
        .style(move |t: &Theme| design::badge(t, kind))
        .into()
}

/// Small status dot used inline before activity / checklist rows.
pub fn dot(theme: Theme, kind: Tone) -> Element<'static, Message> {
    text("●").size(10).color(design::tone(&theme, kind)).into()
}

/// Icono de estado para el feed de actividad (solo ASCII: los glifos/emoji
/// no se renderizan de forma fiable en todas las fuentes del sistema).
pub fn log_icon(kind: Tone) -> &'static str {
    match kind {
        Tone::Ok => "+",
        Tone::Err => "x",
        Tone::Warn => "!",
        Tone::Accent => ">",
        Tone::Neutral => "-",
    }
}

/// Checkbox de solo lectura con tres estados reales (pendiente / en progreso /
/// hecho). No es interactivo: el progreso lo manda el orquestador.
pub fn state_box(theme: Theme, kind: Tone, glyph: &'static str) -> Element<'static, Message> {
    container(text(glyph).size(12).color(design::tone(&theme, kind)))
        .width(18)
        .height(18)
        .center_x(18)
        .center_y(18)
        .style(move |t: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(design::tone_wash(t, kind))),
            border: iced::Border {
                color: design::tone(t, kind),
                width: 1.0,
                radius: design::radius::SMALL.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Tool activity language
// ---------------------------------------------------------------------------
// Log lines are still produced as plain strings by the agent driver
// (business logic untouched). This maps their leading marker to a tone
// and strips the emoji so the activity feed reads as designed UI.

/// (tone, display text without the leading marker glyph)
pub fn activity(line: &str) -> (Tone, &str) {
    let (head, rest) = match line.split_once(' ') {
        Some((h, r)) => (h, r),
        None => ("", line),
    };
    // Marker glyphs are non-ASCII; plain lines keep head == "".
    if head.is_empty() || head.is_ascii() {
        let low = line.to_lowercase();
        if low.contains("error") || low.starts_with("no se pudo") || low.contains("falló") {
            return (Tone::Err, line);
        }
        if low.contains("advertencia") || low.contains("warning") {
            return (Tone::Warn, line);
        }
        return (Tone::Neutral, line);
    }
    let kind = if head.contains('✅') {
        Tone::Ok
    } else if head.contains('❌') {
        Tone::Err
    } else if head.contains("⚠️") || head.contains('⛔') || head.contains('🔐') {
        Tone::Warn
    } else if head.contains('🧭') || head.contains('🔨') || head.contains('🤖') {
        Tone::Accent
    } else if head.contains('⏹') {
        Tone::Warn
    } else {
        Tone::Neutral
    };
    // Refine tool lines by outcome words.
    if head.contains('🔧') {
        if rest.contains("OK") || rest.contains('✅') || rest.contains("bytes") {
            return (Tone::Ok, rest);
        }
        if rest.contains('❌') || rest.contains("fuera de") || rest.contains("deneg") {
            return (Tone::Err, rest);
        }
        return (Tone::Neutral, rest);
    }
    (kind, rest)
}

/// One activity row: colored state icon + 12px text. Errors get an alert
/// wash and the strongest text color so they never get lost in the log.
pub fn activity_row<'a>(theme: Theme, line: &'a str) -> Element<'a, Message> {
    let (kind, body) = activity(line);
    let icon = text(log_icon(kind))
        .size(14)
        .color(design::tone(&theme, kind))
        .width(14)
        .align_x(iced::Alignment::Center);
    let body = text(body).size(14).width(iced::Length::Fill).color(if kind == Tone::Err {
        design::ink(&theme)
    } else {
        design::ink_2(&theme)
    });
    let row = row![icon, body]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(iced::Length::Fill);
    if kind == Tone::Err {
        container(row)
            .width(iced::Length::Fill)
            .padding([2, 4])
            .style(|t: &Theme| container::Style {
                background: Some(iced::Background::Color(design::tone_wash(t, Tone::Err))),
                border: iced::Border {
                    color: design::tone(t, Tone::Err),
                    width: 1.0,
                    radius: design::radius::SMALL.into(),
                },
                ..container::Style::default()
            })
            .into()
    } else {
        row.into()
    }
}

// ---------------------------------------------------------------------------
// Empty states
// ---------------------------------------------------------------------------

/// Intentional empty state: title + hint, left-aligned for panels.
pub fn empty_state(theme: Theme, title: impl Into<String>, hint: impl Into<String>) -> Element<'static, Message> {
    column![
        text(title.into()).size(14).color(design::ink(&theme)),
        text(hint.into()).size(13).color(design::ink_2(&theme)),
    ]
    .spacing(2)
    .into()
}

// ---------------------------------------------------------------------------
// Buttons (jerarquía única: primary / secondary / ghost / danger)
// ---------------------------------------------------------------------------

/// Ghost action (inline rows: ×, chevrons, +). No box on purpose.
pub fn icon_btn(label: String) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(13))
        .padding(4)
        .style(|t: &Theme, s| design::ghost(t, s))
}

/// Header-grade action (Inicio, Cuestionario, Ajustes, Volver, TEMP...):
/// always a visible boxed button, never a bare label.
pub fn head_btn(label: String) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(13))
        .padding([6, 12])
        .style(|t: &Theme, s| design::secondary(t, s))
}

/// Quiet secondary action (Abrir, Probar, Denegar...).
pub fn quiet_btn(label: String) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(13))
        .padding([7, 14])
        .style(|t: &Theme, s| design::secondary(t, s))
}

/// Primary CTA — reserved for Enviar / Crear / Guardar / Finalizar /
/// Siguiente / Permitir. Nothing else uses this style.
pub fn primary_btn(label: String, size: u16) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(size))
        .padding([6, 14])
        .style(|t: &Theme, s| design::primary(t, s))
}

/// Destructive confirm only (borrados confirmados).
pub fn danger_btn(label: String) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(13))
        .padding([7, 14])
        .style(|t: &Theme, s| design::danger_btn(t, s))
}

/// Acción destacada en rojo sin fondo (Salir): se distingue del resto.
pub fn danger_outline_btn(label: String) -> iced::widget::Button<'static, Message> {
    button(iced::widget::text(label).size(13))
        .padding([6, 12])
        .style(|t: &Theme, s| design::danger_outline(t, s))
}
