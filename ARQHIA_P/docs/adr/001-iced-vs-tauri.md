# ADR-001 — Iced nativo en vez de Tauri

- Estado: aceptado (v0.1)
- Fecha: 2026-01 (revisión v0.9.5)

## Contexto

Se necesitaba una GUI de escritorio para Linux y Windows con un solo
lenguaje y binario distribuible sin fricción.

## Decisión

**Iced 0.13, Rust 100%, sin WebView ni frontend web.** Tema `Dark` por
defecto (`Theme::custom` con acento desde v0.7).

## Consecuencias

- (+) Un stack, un binario (~23 MB stripped), sin capa HTML/CSS/JS.
- (+) `Theme::Dark/Light` + paleta custom sin dependencias extra.
- (−) Menos widgets que el ecosistema web; el Markdown lo cubre
  `iced::widget::markdown`; los atajos/tooltips se hacen a mano.
- (−) Distribución por plataforma (`.deb`/Store/`.dmg`) en vez de URL.
