# ARQHIA — ROADMAP v0.1 → v1.2 (Iced 0.13 nativo, sin Tauri)

> Índice high-level. El paso a paso detallado de cada versión vive en `CONTEXT/VERSIONS/v0.x.md`. Sin estimación temporal — orden lógico. La web es una **dependencia aparte** (ver `CONTEXT/WEB.md`), no una versión.

## 1. Stack General

*   **App:** Iced 0.13 nativo (Rust 100%, sin WebView). Vistas `View::{Home, Chat, Config, Questionnaire}`.
*   **Core Rust:** Tokio (async), Serde, reqwest (streaming LLM), rusqlite/SQLite, rfd (pick_folder), minijinja (plantillas .md).
*   **UI:** Solo `iced::widget`. Tema `Theme::Dark` por defecto.
*   **Persistencia:** File-system para workspaces + SQLite (`projects, chats, messages, stack_items`) + `~/.config/arqhia/config.toml` para API keys y tema.
*   **LLM:** Clientes nativos por provider (`openai.rs`, `anthropic.rs`, `openrouter.rs`, `local.rs`) con trait `Provider`.
*   **Git (v0.7.2):** soporte nativo en `git.rs`; cada workspace se inicializa como repo y el agente trabaja en la rama `ARQHIA` (base protegida). Auto-commit solo si `cargo check`+`test`+`clippy` pasan; push opcional (OFF + aprobación).
*   **Bucle de estabilidad (v0.7.3):** flujo pre-STACK `petición → analista (CONTEXT/specs/outlines) → planner → workers → auditor → TEMP → loop fix hasta verde → commit → push (opcional)`. El auto-commit solo ocurre al cerrar el bucle sin issues.
*   **STACK Nube (v1.0):** Backend Axum + Postgres + S3 o mock `~/.arqhia/cloud/`. Cliente `stack_cloud.rs` con fallback local.
*   **Web (dependencia, `CONTEXT/WEB.md`):** Astro + Cloudflare Pages. Descarga, precios, soporte, docs, legal y host de `updates/latest.json`.
*   **Distribución Windows:** Microsoft Store (registro gratis, Microsoft firma el MSIX y auto-actualiza). **Sin certificado Authenticode propio.**
*   **Distribución Linux:** `.deb`/`.tar.gz` + updater embebido (notificar + enlace).
*   **Target OS:** Linux y Windows en v1.0–v1.1. **macOS en v1.2**, condicionado a 50–100 usuarios PRO de pago.

## 2. Principios

*   Cada versión tiene **test de funcionalidad obligatorio** para considerarse Done.
*   Criterios Done estrictos y verificables.
*   `CONTEXT/` es SSOT. Cada versión actualiza `VERSIONS/v0.x.md` y `VERSIONS.md`.
*   **Definición de Estable (v1.0):** chat + agente + cuestionario + STACK local+nube funcionando, web y updater operativos, 1 proyecto real end-to-end creado solo con ARQHIA sin bugs críticos.
*   `HECHO.md` en raíz indica lo hecho y el siguiente paso.

## 3. Plantilla VERSIONS/v0.x.md

Cada archivo en `CONTEXT/VERSIONS/` sigue:

```md
# v0.x — Nombre
## Objetivo
## Alcance (concreto, widgets/structs/archivos exactos)
## Paso a paso (numerado, ejecutable)
## Tareas técnicas (crates, archivos, tablas)
## Criterio Done (estricto, verificable)
## Test de Funcionalidad
## Riesgos
## Dependencias
## Contexto / Notas
```

## 4. Roadmap Resumen

| Versión | Objetivo | Alcance concreto | Entregable | Criterio Done | Dep | Link |
|---|---|---|---|---|---|---|
| **v0.1** | Chat normal multi-provider | Iced shell, Config `pick_list`, `llm/{openai,anthropic,openrouter}.rs`, SQLite `messages` | Chat con stream real | Config guarda providers, enviar "hola" recibe stream, reinicio persiste | — | [v0.1.md](VERSIONS/v0.1.md) |
| **v0.2** | Sidebar + temas + proyectos | `row![sidebar, chat]`, chats con borrar, proyectos agrupadores, Dark/Light | Múltiples chats agrupados | 3 chats, 1 proyecto, tema persiste | v0.1 | [v0.2.md](VERSIONS/v0.2.md) |
| **v0.3** | Agente simple | `Project{path}`, tools CRUD+bash con guard, allowlist, Log | Agente modifica workspace real | Crea/edita/borra en ws, `bash ls`, fuera rechazado | v0.2 | [v0.3.md](VERSIONS/v0.3.md) |
| **v0.4** | Home page | Vistas Home/Chat/Config, `rfd::pick_folder` | Home navegable | Crear→Chat, Abrir carga path | v0.3 | [v0.4.md](VERSIONS/v0.4.md) |
| **v0.5** | Cuestionario simple → doc | Wizard, plantilla minijinja | `.md` de especificaciones generado | Wizard avanza/retrocede, `.md` con secciones | v0.4 | [v0.5.md](VERSIONS/v0.5.md) |
| **v0.6** | Orquestador + permisos | Modal Aprobar/Denegar, `search`, Planner→Workers→Auditor, `AGENTS.md`/`TEMP.md` | Orquestador con permisos | Write pide permiso, auditor escribe TEMP.md | v0.5 | [v0.6.md](VERSIONS/v0.6.md) |
| **v0.7** | Polish + seguridad | Tabs, uploads, permisos ampliados (OFF), límites, apariencia, `POLICIES.md` | UI + seguridad | Red/install piden permiso, apariencia en vivo | v0.6 | [v0.7.md](VERSIONS/v0.7.md) |
| **v0.7.1** | Agente eficiente + Modos | Chat/Plan/Work, atajos, search v2, read paginado, outline, presupuesto tokens | <25% tokens del baseline | Plan = 1 llamada, default Chat persiste | v0.7 | [v0.7.1.md](VERSIONS/v0.7.1.md) |
| **v0.7.2** | Git nativo + puerta de calidad | `GitConfig` + `git.rs` + política por comando + pestaña Git; auto-commit condicionado a check/test/clippy | Git seguro configurable | Rama `ARQHIA`, push pide permiso, no commitea si tests fallan | v0.7.1 | [v0.7.2.md](VERSIONS/v0.7.2.md) |
| **v0.7.3** | Bucle de estabilidad | Analista dedicado (contexto/specs/outlines) antes del planner; loop `auditor → fix` sin tope hasta verde; commit solo al cerrar verde; push opcional | Flujo pre-STACK completo | Turno con fallo fuerza ≥2 ciclos y cierra verde; rojo no commitea; `Detener`/presupuesto cortan sin commit | v0.7.2 | [v0.7.3.md](VERSIONS/v0.7.3.md) |
| **v0.7.4** | Chat UX + modelos con nombre | Perfiles de modelo con nombre visible (selector en Config y composer); título de chat por IA; undo; copiar chat; nuevo chat desde un mensaje; fecha/hora por mensaje; reintento con icono; citar fuentes tras `fetch_url` | Chat pulido + perfiles | 2 perfiles alternables persisten; título IA/fallback; undo/copiar/bifurcar; timestamps; `Fuentes` clicables | v0.7.3 | [v0.7.4.md](VERSIONS/v0.7.4.md) |
| **v0.8** | Cuestionario genérico + por nivel + IA; nueva estructura, onboarding y sesiones de chat | Genéricas + preguntas por nivel (Principiante/Intermedio/Avanzado) + preguntas opcionales IA → `PROJECT.md` + `SPECS.md`; layout `Project/`, `CONTEXT/`, `ToDo.md`; onboarding de API al primer arranque; `session_id` estable por chat (OpenRouter `session_id`, OpenAI `prompt_cache_key`) | Proyecto documentado y estructurado + sesiones | Cada nivel genera docs distintos, `SPECS.md` existe, IA opcional saltables, layout correcto, `session_id` persiste y se envía | v0.7.1 | [v0.8.md](VERSIONS/v0.8.md) |
| **v0.9** | STACK local + legal + instalador | FTS5 + 5 funciones + panel + seed + `author/license/source` + consent 3×OFF + identidad local + contador uso + `.deb` + CI mínimo | STACK legal instalable | `Uso interno` no sube, consent OFF bloquea, `.deb` conserva datos, CI verde | v0.8 | [v0.9.md](VERSIONS/v0.9.md) |
| **WEB** | **Sitio del producto (dependencia, no versionado)** | Astro + Cloudflare: descarga, precios, soporte, docs, legal, `updates/latest.json` | Web publicada | Dominio+HTTPS, descargas y legal públicos, manifiesto servido | v0.9 | [WEB.md](WEB.md) |
| **v1.0** | STACK nube + auth + updater + releases | `stack_cloud.rs` (+mock), auth token 600, **updater embebido (notificar + descarga)**, Windows vía **Microsoft Store**, backup/export, release estable | Release estable Linux+Windows | Push A→B, offline fallback, export/import conserva, banner de update, 0 críticos | WEB | [v1.0.md](VERSIONS/v1.0.md) |
| **v1.1** | Pro (Linux + Windows) | `license.rs` ed25519 + planes Trial/$5/$12/$20 + `snapshot_ws` + `drivers: Vec` + `merge.rs` + `custom.rs` + editor de flujos + 🔒 Free | Pro vendible en Linux+Windows | Free→modal Pro, 2 sandboxes→merge, conflicto→MERGE.md, Trial 30d | v1.0 | [v1.1.md](VERSIONS/v1.1.md) |
| **v1.2** | macOS (condicionado) | Firma Developer ID + notarización + `.dmg` aarch64 + `open` en vez de `xdg-open` + rutas mac | macOS firmado y notarizado | 50–100 usuarios PRO de pago; `.dmg` abre en mac limpio sin bypass | v1.1 | [v1.2.md](VERSIONS/v1.2.md) |

## 5. Convenciones

*   Versionado SemVer `v0.x` → `v1.2`. Cada versión actualiza `VERSIONS.md`.
*   Idioma ES docs, EN código/commits.
*   **Monetización (v1.1):** STACK gratis e ilimitado siempre (efecto red).
    Pro cobra solo potencia: workers paralelos, sandboxes, agentes y flujos
    custom. Licencias offline `ed25519` + pago externo, sin backend propio.
*   Providers nativos (no wrapper genérico) desde v0.1.
*   **Web y updater** no son versiones: la web es dependencia previa a v1.0
    (`CONTEXT/WEB.md`); el updater se implementa dentro de v1.0.

## 6. Backlog Config/UI (v0.8–v1.2)

*   **Más opciones en Configuración:** Apariencia (fuentes, radios, densidad fina, más acentos), Permisos y Límites ampliados.
*   **Coste:** panel de gasto por proyecto/chat/modelo y presupuestos/avisos.
*   **Propiedad del código (STACK):** opciones de licencia/propiedad y consentimiento al compartir.
*   **Control de datos:** gestión de archivos, proyectos, caché y contexto (ver, exportar, limpiar).
*   **Límites con especificación:** cada límite con descripción clara y tooltip flotante.
*   **UI:** bloques de Config lado a lado en pantallas anchas y sidebar de pestañas más ancha.
*   **Guardar modelos/perfiles:** varios modelos con su propia API key, `base_url` y provider, y un **nombre visible** que salga en el selector — **asignado a v0.7.4** (`VERSIONS/v0.7.4.md` §A).
*   **Utilidades de chat (v0.7.4):** título por IA, undo, copiar chat, nuevo chat desde un mensaje, fecha/hora por mensaje, reintento con icono y **citar fuentes** tras `fetch_url` (`VERSIONS/v0.7.4.md` §B).
*   **Lector de Markdown del chat:** cobertura completa de sintaxis y mejora visual.
*   **Repos guía:** opencode, OpenHands y SWE-agent como referencia.
*   **Sesiones de chat:** dividir chats en **sesiones** con id estable por provider (caché/coste/trazabilidad) — **asignado a v0.8** (ver `VERSIONS/v0.8.md` §E).

## 7. Backlog negocio/distribución (fuera de versiones)

*   **Microsoft Store:** validar packaging MSIX + política de comercio propio.
*   **Flathub** como canal alternativo Linux (opcional, posterior a v1.0).
*   **Firma Windows fuera de Store** (Authenticode) solo si se decide distribuir `.exe` directo.
*   **Notarización macOS** obligatoria en v1.2 (Apple Developer Program, $99/año).
*   **Observabilidad:** crash reporting opt-in y métricas de uso agregadas (sin contenido de código).
