## Auditoría ARQHIA — v0.9.6 (puerta de v1.0)

SIN ISSUES

## Gate backup / higiene

- Rama `ARQHIA` al día con `origin/ARQHIA` (`git rev-list --left-right --count`
  = 0/0) y árbol limpio antes de empezar. Documentado aquí.
- `unwrap()/expect(` productivo: **0** (los 2 hits de `rg` son strings de seed
  del STACK en `stack/seed.rs`, no código ejecutable). `todo!/unimplemented!/
  panic!` productivo: **0**. No se tocó ningún `unwrap` real → no aplicó backup.
- `cargo fmt --check` OK · `cargo clippy --all-targets -- -D warnings` 0 ·
  `cargo test` 203/203 OK (3 ignorados).

## Optimización (medida)

- `[profile.release]`: `opt-level=3`, `lto="thin"`, `codegen-units=1`,
  `strip=true`, `panic="unwind"` (necesario para `catch_unwind` de `main.rs`).
- Binario release: **24.3 MB** (<30 MB). Proceso hasta crear el event loop de
  Iced: instantáneo (<2 s). Apertura GUI con display: pendiente de máquina con
  X11/Wayland (este entorno es headless).

## Icono + instalador + CI

- `assets/icon.svg` + `icon-{16,32,64,128,256}.png` + `icon.rgba` (embebido).
  Fuente regenerable con `python3 packaging/make_icons.py`.
- Ventana: `main.rs::app_icon()` (`iced::window::icon::from_rgba`); Home:
  `ui/logo.rs` (canvas con la misma geometría que el SVG).
- `packaging/build_deb.sh` → `arqhia_0.9.6_amd64.deb` (8.1 MB): `/usr/bin/
  arqhia`, iconos hicolor, `.desktop`, metainfo. `packaging/install.sh`
  (genérico). `postinst` best-effort, nunca borra ni pisa datos; verificado
  con config.toml/DB sentinela que se conservan.
- `.github/workflows/ci.yml`: `fmt` + `clippy -D warnings` + `test` +
  `build --release` (falla si ≥30 MB) + job `.deb`.

## Refactor / dedup

- Ignores a fuente única: `config::IGNORE_DIRS` + `config::is_ignored_name`,
  usada por `default_ignores()` y por `workspace::scan_import` (test
  `ignored_name_single_source`).
- `estimate_tokens` (→ `llm::estimate_tokens_text`) y `resync_msg_meta`
  (→ `app/state.rs`) ya eran de fuente única desde v0.9.5.
- `trunc*`: `ui::design::trunc_end/trunc_start`, `agent::tools::truncate` y
  `skills::truncate_body` tienen semánticas distintas (UI/epílogo/bytes); las
  tres son UTF-8-safe y tienen test. Sin cambio.

## Docs

`PROJECT.md` (§4 árbol + §10/§13), `POLICIES.md` §7, `AGENTS.md` (5 roles),
`README.md` (instalación Linux), `VERSIONS.md`/`HECHO.md`.

## Sugerencia para v1.0

- Medir apertura GUI <2 s y TTFB <300 ms en una máquina con display (aquí solo
  se midió arranque headless hasta el event loop).
