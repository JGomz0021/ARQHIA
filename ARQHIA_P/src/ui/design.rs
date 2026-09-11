//! ARQHIA design system.
//!
//! Identity: restrained native developer tool. Cool-gray surface hierarchy,
//! one configurable accent (violeta por defecto), text-driven hierarchy,
//! hairline borders.
//!
//! Surfaces (dark): app `#16181D`, sidebar `#1C1F26`, raised `#20242C`,
//! well (sunken) `#101216`. Light mirrors the same ramp.
//!
//! Rules:
//! - Accent (violeta) means: primary action, current selection, focus.
//! - State colors are semantic and never reused for identity:
//!   verde = éxito/activo · ámbar = advertencia · rojo = error.
//! - Everything else is surface / border / text contrast.
//! - Radius is 6 for boxes, 4 for small controls, 10 for pills. No blobs.
//! - Spacing uses the 4/8 scale in `space`.
//! - Type: 12 meta · 13 secondary · 14 body · 15 emphasis · 17 title ·
//!   19 headline · 24 brand.
//!
//! All styles are theme-aware (`&Theme`) so Dark/Light keep working.

use iced::widget::{button, container, markdown, pick_list, progress_bar, rule, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

/// Spacing scale. Use these steps, nothing in between.
#[allow(dead_code)]
pub mod space {
    pub const X2: u16 = 2;
    pub const X4: u16 = 4;
    pub const X6: u16 = 6;
    pub const X8: u16 = 8;
    pub const X12: u16 = 12;
    pub const X16: u16 = 16;
    pub const X24: u16 = 24;
}

/// Type scale.
#[allow(dead_code)]
pub mod type_scale {
    pub const META: u16 = 12;
    pub const SECONDARY: u16 = 13;
    pub const BODY: u16 = 14;
    pub const EMPHASIS: u16 = 15;
    pub const TITLE: u16 = 17;
    pub const HEADLINE: u16 = 19;
    pub const BRAND: u16 = 24;
}

/// Escala un tamaño base de texto por el ajuste global (v0.7 Track B).
/// `scale` sale de `Appearance::text_size`.
pub fn fs(scale: f32, base: u16) -> u16 {
    ((base as f32 * scale).round() as u16).max(9)
}

/// Espaciado según densidad (v0.7 Track B): compacta resta 4px (mínimo 2).
pub fn gap(compact: bool, normal: u16) -> u16 {
    if compact { normal.saturating_sub(4).max(2) } else { normal }
}

/// Padding según densidad: compacta reduce 4px dejando un mínimo de 2.
pub fn pad(compact: bool, normal: u16) -> u16 {
    if compact { normal.saturating_sub(4).max(2) } else { normal }
}

/// Radius scale.
pub mod radius {
    pub const SMALL: f32 = 4.0;
    pub const BOX: f32 = 6.0;
    pub const PILL: f32 = 10.0;
}

/// Truncado seguro con UTF-8 (por chars, nunca por bytes: cortar por bytes
/// en medio de un carácter multibyte hace panic y cierra la app).
/// Conserva el inicio y añade … si recorta.
pub fn trunc_end(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}…")
}

/// Truncado seguro que conserva el final (rutas) y añade … delante.
pub fn trunc_start(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    let tail: String = s.chars().skip(n - max_chars).collect();
    format!("…{tail}")
}

pub fn is_dark(theme: &Theme) -> bool {
    match theme {
        Theme::Dark => true,
        Theme::Light => false,
        // Temas custom (v0.7 Track B): decide por luminancia del fondo.
        _ => {
            let bg = theme.palette().background;
            0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b < 0.5
        }
    }
}

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn ca(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a }
}

// --- surfaces ---------------------------------------------------------------

/// Base application surface.
pub fn app_bg(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x16, 0x18, 0x1D) } else { c(0xF1, 0xF2, 0xF4) }
}

/// Sidebar / navigation surface, one step above the base.
pub fn side_bg(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x1C, 0x1F, 0x26) } else { c(0xE8, 0xEA, 0xEE) }
}

/// Raised surface: cards, composer, user messages.
pub fn raised(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x21, 0x25, 0x2D) } else { c(0xFF, 0xFF, 0xFF) }
}

/// Sunken surface: activity well, code-ish panels.
pub fn well(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x10, 0x12, 0x16) } else { c(0xE3, 0xE5, 0xE9) }
}

/// Hairline border color.
pub fn border(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x2C, 0x31, 0x3B) } else { c(0xD5, 0xD8, 0xDE) }
}

// --- text -------------------------------------------------------------------

pub fn ink(theme: &Theme) -> Color {
    if is_dark(theme) { c(0xE6, 0xE8, 0xEC) } else { c(0x1D, 0x21, 0x26) }
}

pub fn ink_2(theme: &Theme) -> Color {
    if is_dark(theme) { c(0xA7, 0xAD, 0xBA) } else { c(0x59, 0x60, 0x6B) }
}

/// Legacy helper kept for call sites that only know dark/light.
pub fn muted(dark: bool) -> Color {
    if dark { c(0x8B, 0x91, 0x9E) } else { c(0x6B, 0x71, 0x7D) }
}

// --- accent -----------------------------------------------------------------

/// The single ARQHIA accent: desaturated teal.
pub fn accent(theme: &Theme) -> Color {
    theme.palette().primary
}

pub fn accent_hover(theme: &Theme) -> Color {
    let a = theme.palette().primary;
    // Aclara/oscurece un paso según el fondo para el hover.
    let k = if is_dark(theme) { 1.15 } else { 0.9 };
    Color {
        r: (a.r * k).min(1.0),
        g: (a.g * k).min(1.0),
        b: (a.b * k).min(1.0),
        a: a.a,
    }
}

/// Text placed on top of the accent fill.
pub fn on_accent(theme: &Theme) -> Color {
    if is_dark(theme) { c(0x0C, 0x1A, 0x18) } else { c(0xFF, 0xFF, 0xFF) }
}

/// Translucent accent wash for current-selection backgrounds. Deriva del
/// acento activo (no de un teal fijo) para que Violeta/Ámbar sean coherentes.
pub fn accent_wash(theme: &Theme) -> Color {
    let a = accent(theme);
    Color { r: a.r, g: a.g, b: a.b, a: if is_dark(theme) { 0.16 } else { 0.10 } }
}

/// Wash reforzado para hover sobre una selección activa.
pub fn accent_wash_strong(theme: &Theme) -> Color {
    let a = accent(theme);
    Color { r: a.r, g: a.g, b: a.b, a: if is_dark(theme) { 0.26 } else { 0.16 } }
}

// --- semantic ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Ok,
    Warn,
    Err,
    Accent,
}

pub fn tone(theme: &Theme, kind: Tone) -> Color {
    match kind {
        Tone::Neutral => ink_2(theme),
        Tone::Ok => {
            if is_dark(theme) { c(0x55, 0xB8, 0x7F) } else { c(0x1E, 0x7A, 0x4C) }
        }
        Tone::Warn => {
            if is_dark(theme) { c(0xD1, 0xA7, 0x3D) } else { c(0x8A, 0x6D, 0x1A) }
        }
        Tone::Err => {
            if is_dark(theme) { c(0xDE, 0x6E, 0x64) } else { c(0xB3, 0x36, 0x2E) }
        }
        Tone::Accent => accent(theme),
    }
}

pub fn tone_wash(theme: &Theme, kind: Tone) -> Color {
    let (r, g, b, a) = match kind {
        Tone::Neutral => (0xA7, 0xAD, 0xBA, if is_dark(theme) { 0.14 } else { 0.16 }),
        Tone::Ok => (0x55, 0xB8, 0x7F, 0.14),
        Tone::Warn => (0xD1, 0xA7, 0x3D, 0.15),
        Tone::Err => (0xDE, 0x6E, 0x64, 0.14),
        Tone::Accent => return accent_wash(theme),
    };
    ca(r, g, b, a)
}

#[allow(dead_code)]
pub fn ok(dark: bool) -> Color {
    if dark { c(0x55, 0xB8, 0x7F) } else { c(0x1E, 0x7A, 0x4C) }
}

/// Clasifica un mensaje de estado de la app en un `Tone` para que el color
/// nunca se decida a mano en las vistas (contrato único de feedback).
pub fn status_tone(status: &str) -> Tone {
    let s = status.to_lowercase();
    if s.is_empty() {
        return Tone::Neutral;
    }
    const ERR: [&str; 11] = [
        "error",
        "falló",
        "no se pudo",
        "inválid",
        "rechaz",
        "agotado",
        "denegad",
        "timeout",
        "sin conexión",
        "fuera de",
        "no existe",
    ];
    const WARN: [&str; 10] = [
        "probando",
        "espera",
        "advertencia",
        "límite",
        "sin workspace",
        "sin chat",
        "no hay",
        "pon la",
        "pon el",
        "ignoradas",
    ];
    const OK: [&str; 6] = ["ok", "guardad", "aplicad", "listo", "permitid", "abriendo"];
    if ERR.iter().any(|k| s.contains(k)) {
        Tone::Err
    } else if WARN.iter().any(|k| s.contains(k)) {
        Tone::Warn
    } else if OK.iter().any(|k| s.contains(k)) {
        Tone::Ok
    } else {
        Tone::Neutral
    }
}

// ---------------------------------------------------------------------------
// Small builders
// ---------------------------------------------------------------------------

fn hairline(theme: &Theme) -> Border {
    Border { color: border(theme), width: 1.0, radius: radius::BOX.into() }
}

// ---------------------------------------------------------------------------
// Container styles
// ---------------------------------------------------------------------------

/// Full-bleed app background (anchors content to the window).
pub fn app(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(app_bg(theme))),
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Sidebar navigation surface.
pub fn sidebar(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(side_bg(theme))),
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Card: raised surface + hairline border.
pub fn card(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(raised(theme))),
        border: hairline(theme),
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Composer: the central interaction surface.
pub fn composer(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(raised(theme))),
        border: hairline(theme),
        shadow: Shadow {
            color: ca(0x00, 0x00, 0x00, if is_dark(theme) { 0.35 } else { 0.08 }),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 8.0,
        },
        text_color: Some(ink(theme)),
    }
}

/// Sunken well: activity log, TEMP viewer.
pub fn well_box(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(well(theme))),
        border: Border { color: border(theme), width: 1.0, radius: radius::SMALL.into() },
        text_color: Some(ink_2(theme)),
        ..container::Style::default()
    }
}

/// User message: raised + accent border, right-aligned by the caller.
pub fn user_msg(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(raised(theme))),
        border: Border { color: accent(theme), width: 1.0, radius: radius::BOX.into() },
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Agent message: sunken well + hairline border. Reads as "incoming".
pub fn ai_msg(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(well(theme))),
        border: hairline(theme),
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Segmented control track (Chat | Plan | Work), pills sit inside.
pub fn segmented(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(well(theme))),
        border: Border { color: border(theme), width: 1.0, radius: radius::PILL.into() },
        text_color: Some(ink_2(theme)),
        ..container::Style::default()
    }
}

/// Project thumbnail: monogram tile used in the Home launcher.
pub fn avatar(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(raised(theme))),
        border: Border { color: accent(theme), width: 1.0, radius: radius::BOX.into() },
        text_color: Some(accent(theme)),
        ..container::Style::default()
    }
}

/// Tooltip flotante: superficie elevada + sombra suave.
pub fn tooltip(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(raised(theme))),
        border: hairline(theme),
        text_color: Some(ink(theme)),
        shadow: Shadow {
            color: ca(0x00, 0x00, 0x00, if is_dark(theme) { 0.4 } else { 0.12 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
    }
}

/// Dock inferior (composer + log): superficie elevada con sombra superior
/// marcada para separarla con claridad del área de mensajes.
pub fn dock(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(side_bg(theme))),
        shadow: Shadow {
            color: ca(0x00, 0x00, 0x00, if is_dark(theme) { 0.5 } else { 0.14 }),
            offset: Vector::new(0.0, -3.0),
            blur_radius: 12.0,
        },
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Task / plan row card, tinted by its state (hecho, en progreso, pendiente).
pub fn task_card(theme: &Theme, kind: Tone) -> container::Style {
    let background = match kind {
        Tone::Accent => tone_wash(theme, Tone::Accent),
        Tone::Ok => tone_wash(theme, Tone::Ok),
        _ => raised(theme),
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: match kind {
                Tone::Neutral => border(theme),
                _ => tone(theme, kind),
            },
            width: 1.0,
            radius: radius::BOX.into(),
        },
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Permission approval: the one interrupting surface (warm wash + border).
pub fn approval(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(tone_wash(theme, Tone::Warn))),
        border: Border { color: tone(theme, Tone::Warn), width: 1.0, radius: radius::BOX.into() },
        text_color: Some(ink(theme)),
        ..container::Style::default()
    }
}

/// Pill badge with a tonal wash.
pub fn badge(theme: &Theme, kind: Tone) -> container::Style {
    container::Style {
        background: Some(Background::Color(tone_wash(theme, kind))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::PILL.into(),
        },
        text_color: Some(tone(theme, kind)),
        shadow: Shadow::default(),
    }
}

// ---------------------------------------------------------------------------
// Button styles
// ---------------------------------------------------------------------------

fn btn_base(
    theme: &Theme,
    status: button::Status,
    bg: Color,
    bg_hover: Color,
    fg: Color,
) -> button::Style {
    let main = button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: radius::SMALL.into() },
        shadow: Shadow::default(),
    };
    match status {
        button::Status::Active | button::Status::Pressed => main,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(bg_hover)),
            ..main
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(well(theme))),
            text_color: muted(is_dark(theme)),
            ..main
        },
    }
}

/// Primary: accent fill. Reserved for Enviar / Crear / Guardar / Finalizar /
/// Siguiente / Permitir.
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    btn_base(theme, status, accent(theme), accent_hover(theme), on_accent(theme))
}

/// Secondary: raised surface + hairline border.
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let main = button::Style {
        background: Some(Background::Color(raised(theme))),
        text_color: ink(theme),
        border: hairline(theme),
        shadow: Shadow::default(),
    };
    match status {
        button::Status::Active | button::Status::Pressed => main,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(if is_dark(theme) {
                c(0x27, 0x2C, 0x36)
            } else {
                c(0xE8, 0xEA, 0xEE)
            })),
            ..main
        },
        button::Status::Disabled => button::Style {
            text_color: muted(is_dark(theme)),
            ..main
        },
    }
}

/// Ghost: transparent, hover shows a soft wash. For tertiary actions.
pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    let main = button::Style {
        background: None,
        text_color: ink_2(theme),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
    };
    match status {
        button::Status::Active | button::Status::Pressed => main,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(accent_wash(theme))),
            text_color: ink(theme),
            ..main
        },
        button::Status::Disabled => button::Style {
            text_color: muted(is_dark(theme)),
            ..main
        },
    }
}

/// Nav item: full-width row; selected gets the accent wash + accent border.
pub fn nav(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    if selected {
        let main = button::Style {
            background: Some(Background::Color(accent_wash(theme))),
            text_color: ink(theme),
            border: Border {
                color: accent(theme),
                width: 1.0,
                radius: radius::SMALL.into(),
            },
            shadow: Shadow::default(),
        };
        return match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(accent_wash_strong(theme))),
                ..main
            },
            button::Status::Disabled => button::Style {
                text_color: muted(is_dark(theme)),
                ..main
            },
            _ => main,
        };
    }
    ghost(theme, status)
}

/// Segmento del toggle de modos: radio pill, activo en acento, mismo alto
/// para todos los segmentos.
pub fn segment(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let border = Border { color: Color::TRANSPARENT, width: 0.0, radius: radius::PILL.into() };
    if active {
        let main = button::Style {
            background: Some(Background::Color(accent(theme))),
            text_color: on_accent(theme),
            border,
            shadow: Shadow::default(),
        };
        return match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(accent_hover(theme))),
                ..main
            },
            _ => main,
        };
    }
    let main = button::Style {
        background: None,
        text_color: ink_2(theme),
        border,
        shadow: Shadow::default(),
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(accent_wash(theme))),
            text_color: ink(theme),
            ..main
        },
        _ => main,
    }
}

/// Salir: rojo en outline para que destaque sin parecer un borrado.
pub fn danger_outline(theme: &Theme, status: button::Status) -> button::Style {
    let red = tone(theme, Tone::Err);
    let main = button::Style {
        background: None,
        text_color: red,
        border: Border { color: red, width: 1.0, radius: radius::SMALL.into() },
        shadow: Shadow::default(),
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(tone_wash(theme, Tone::Err))),
            ..main
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(well(theme))),
            text_color: muted(is_dark(theme)),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: radius::SMALL.into() },
            ..main
        },
        _ => main,
    }
}

/// Destructive confirm only.
pub fn danger_btn(theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(well(theme))),
            text_color: muted(is_dark(theme)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius::SMALL.into(),
            },
            shadow: Shadow::default(),
        },
        _ => iced::widget::button::danger(theme, status),
    }
}

// ---------------------------------------------------------------------------
// Input / selector styles
// ---------------------------------------------------------------------------

/// Standard field: raised surface, hairline border, accent focus ring.
pub fn field(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let passive = text_input::Style {
        background: Background::Color(raised(theme)),
        border: hairline(theme),
        icon: ink_2(theme),
        placeholder: muted(is_dark(theme)),
        value: ink(theme),
        selection: accent_wash(theme),
    };
    match status {
        text_input::Status::Focused => text_input::Style {
            border: Border { color: accent(theme), width: 1.0, radius: radius::SMALL.into() },
            ..passive
        },
        text_input::Status::Disabled => text_input::Style {
            background: Background::Color(well(theme)),
            value: muted(is_dark(theme)),
            ..passive
        },
        _ => passive,
    }
}

/// Borderless input that lives inside the composer surface.
pub fn bare(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::SMALL.into(),
        },
        icon: ink_2(theme),
        placeholder: muted(is_dark(theme)),
        value: ink(theme),
        selection: accent_wash(theme),
    }
    .apply_status(status)
}

trait ApplyStatus {
    fn apply_status(self, status: text_input::Status) -> Self;
}

impl ApplyStatus for text_input::Style {
    fn apply_status(self, status: text_input::Status) -> Self {
        match status {
            text_input::Status::Disabled => Self {
                value: muted(true),
                ..self
            },
            _ => self,
        }
    }
}

/// Selector matching the field style.
pub fn field_pick(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    pick_list::Style {
        text_color: ink(theme),
        placeholder_color: muted(is_dark(theme)),
        handle_color: ink_2(theme),
        background: Background::Color(raised(theme)),
        border: hairline(theme),
    }
    .apply_pick(status, theme)
}

trait ApplyPick {
    fn apply_pick(self, status: pick_list::Status, theme: &Theme) -> Self;
}

impl ApplyPick for pick_list::Style {
    fn apply_pick(self, status: pick_list::Status, theme: &Theme) -> Self {
        match status {
            pick_list::Status::Hovered => Self {
                border: Border { color: accent(theme), width: 1.0, radius: radius::SMALL.into() },
                ..self
            },
            _ => self,
        }
    }
}

/// Selector embebido con caja visible (borde de acento): se distingue como
/// control dentro del composer.
pub fn accent_pick(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::Style {
        text_color: ink(theme),
        placeholder_color: muted(is_dark(theme)),
        handle_color: accent(theme),
        background: Background::Color(raised(theme)),
        border: Border { color: accent(theme), width: 1.0, radius: radius::SMALL.into() },
    };
    match status {
        pick_list::Status::Hovered => pick_list::Style {
            background: Background::Color(accent_wash(theme)),
            ..base
        },
        _ => base,
    }
}

/// Borderless selector embedded in the composer.
pub fn bare_pick(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    pick_list::Style {
        text_color: ink_2(theme),
        placeholder_color: muted(is_dark(theme)),
        handle_color: muted(is_dark(theme)),
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::SMALL.into(),
        },
    }
    .apply_pick(status, theme)
}

// ---------------------------------------------------------------------------
// Rule + progress
// ---------------------------------------------------------------------------

/// Hairline rule in the border color.
pub fn hairline_rule(theme: &Theme) -> rule::Style {
    rule::Style {
        color: border(theme),
        width: 1,
        radius: radius::SMALL.into(),
        fill_mode: rule::FillMode::Full,
    }
}

pub fn progress(theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(well(theme)),
        bar: Background::Color(accent(theme)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::PILL.into(),
        },
    }
}

/// Markdown style del chat: links con el acento del tema y código con fondo
/// hundido sutil (en vez de la paleta cruda, que daba links azules).
pub fn markdown_style(theme: &Theme) -> markdown::Style {
    let mut style = markdown::Style::from_palette(theme.palette());
    style.inline_code_highlight = markdown::Highlight {
        background: Background::Color(well(theme)),
        border: Border { color: border(theme), width: 1.0, radius: radius::SMALL.into() },
    };
    style.inline_code_color = ink(theme);
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trunc_never_splits_multibyte_chars() {
        // Tildes/ñ/emoji: cortar por bytes haría panic; por chars no.
        let s = "diseño de la solución con ñandú 🚀 y más texto para recortar";
        let cut = trunc_end(s, 10);
        assert!(cut.ends_with('…'));
        assert_eq!(cut.chars().count(), 11);
        let tail = trunc_start(s, 10);
        assert!(tail.starts_with('…'));
        assert_eq!(tail.chars().count(), 11);
        // Cortos se devuelven intactos.
        assert_eq!(trunc_end("hola", 10), "hola");
        assert_eq!(trunc_start("hola", 10), "hola");
        // Exactos no se tocan.
        assert_eq!(trunc_end("acción", 6), "acción");
    }
}
