# ARQHIA — ROADMAP v0.1 → v1.0 (Iced 0.13 nativo, sin Tauri)

> Índice high-level. El paso a paso detallado de cada versión vive en `CONTEXT/VERSIONS/v0.x.md`. Sin estimación temporal — orden lógico.

## 1. Stack General

*   **App:** Iced 0.13 nativo (Rust 100%, sin WebView). Vistas `View::{Home, Chat, Config, Questionnaire}`.
*   **Core Rust:** Tokio (async), Serde, reqwest (streaming LLM), rusqlite/SQLite, rfd (pick_folder), minijinja (plantillas .md).
*   **UI:** Solo `iced::widget`. Tema `Theme::Dark` por defecto.
*   **Persistencia:** File-system para workspaces + SQLite (`projects, chats, messages, stack_items`) + `~/.config/arqhia/config.toml` para API keys y tema.
*   **LLM:** Clientes nativos por provider (`openai.rs`, `anthropic.rs`, `openrouter.rs`) con trait `Provider`. Cada uno con su auth y formato de body propios.
*   **STACK Nube (v1.0):** Backend Axum + Postgres + S3 o mock `~/.arqhia/cloud/`. Cliente `stack_cloud.rs` con fallback local.
*   **Target OS:** Linux.

## 2. Principios

*   Cada versión tiene **test de funcionalidad obligatorio** para considerarse Done.
*   Criterios Done estrictos y verificables.
*   `CONTEXT/` es SSOT. Cada versión actualiza `VERSIONS/v0.x.md` y `VERSIONS.md`.
*   **Definición de Estable (v1.0):** chat + agente + cuestionario + STACK local+nube funcionando, 1 proyecto real end-to-end creado solo con ARQHIA sin bugs críticos.
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

| Versión | Objetivo | Alcance concreto | Entregable | Criterio Done | Test | Dep | Link |
|---|---|---|---|---|---|---|---|
| **v0.1** | Chat normal multi-provider, sin agente | Iced shell, `App{messages,input,config}`, `text_input+button+scrollable`, Config `pick_list(OpenAI\|Anthropic\|OpenRouter)+api_key+base_url+model`, `llm/{openai,anthropic,openrouter}.rs` nativos, SQLite `messages` | Binario `ARQHIA_P` arranca, chat con stream real | 1) `cargo run` abre ventana 2) Config guarda 2 providers distintos 3) Enviar "hola" recibe stream real 4) Reinicio persiste | Configurar OpenAI, enviar hola, ver stream, reiniciar y ver historial | Ninguna | [v0.1.md](VERSIONS/v0.1.md) |
| **v0.2** | Sidebar + temas + proyectos agrupadores | `row![sidebar 260px, chat]`, `+Nuevo chat`, lista chats con borrar, `Proyectos: list+text_input+assign`, `Theme::Dark/Light` con `pick_list` en Config | Múltiples chats agrupados, tema cambia en vivo | 1) 3 chats creables/borrables 2) Proyecto agrupa chats 3) Tema persiste | Crear 3 chats, 1 proyecto, mover chats, cambiar tema, reiniciar | v0.1 | [v0.2.md](VERSIONS/v0.2.md) |
| **v0.3** | Agente simple | `Project{path}`, `text_input(path)+validar`, tools `read/write/edit/delete/list/bash` con guard `starts_with(workspace)`, allowlist bash, Log inferior | Agente modifica workspace real | 1) Crea/edita/borra en workspace 2) `bash ls` funciona 3) Fuera de workspace rechazado | "crea main.rs hola" → archivo existe, `cargo run` manual funciona | v0.2 | [v0.3.md](VERSIONS/v0.3.md) |
| **v0.4** | Home page | `View::{Home,Chat,Config}`, Home `column![Crear,Abrir,Config]`, `rfd::pick_folder` para Abrir, Config abajo API+tema | Home navegable | 1) Crear cambia a Chat 2) Abrir carga path 3) Config accesible desde Home | Crear proyecto desde Home, abrir carpeta existente | v0.3 | [v0.4.md](VERSIONS/v0.4.md) |
| **v0.5** | Cuestionario simple → .md | Wizard `step:usize`, textos `nombre/descripción/ubicación/objetivo`, múltiples `público[n..], interfaz[nativa,web,CLI,móvil,backend]`, plantilla minijinja → `CONTEXT/ESPEC.md` | ESPEC.md generado | 1) Wizard avanza/retrocede 2) Validación no vacíos 3) .md existe con 6 secciones | Completar cuestionario, abrir ESPEC.md generado | v0.4 | [v0.5.md](VERSIONS/v0.5.md) |
| **v0.6** | Orquestador + permisos + subagentes | Modal `Aprobar/Denegar`, tools +`search`, `orchestrator.rs: Planner→2 Workers generadores→Auditor`, `AGENTS.md`, `TEMP.md`. Nota: revisar repos opensource (aider, swe-agent) | Orquestador paralelo con permisos | 1) Write pide permiso 2) 2 workers en paralelo 3) Auditor escribe TEMP.md | Tarea "crea 2 archivos" pide permiso y auditor reporta | v0.5 | [v0.6.md](VERSIONS/v0.6.md) |
| **v0.7** | Polish + seguridad por defecto | Track A: tabs, uploads, botones. Track B: permisos ampliados (red/install/extra_paths, todo OFF), límites configurables, aprobación por categorías, planner read-only, acentos+fuente+densidad, aviso privacidad, `POLICIES.md` | UI + seguridad | 1) Red/install/externo piden permiso categorizado 2) Apariencia en vivo 3) `clippy/test` verde | Pedir descarga+instalación → 2 aprobaciones; cambiar apariencia, reiniciar | v0.6 | [v0.7.md](VERSIONS/v0.7.md) |
| **v0.7.1** | Agente eficiente + Modos | Modos Chat/Plan/Work (default Chat, badge+selector, PLAN.md) + atajos y apartado en Config + search v2 con matches + read paginado + outline + historial/outputs acotados + caché lecturas + presupuesto configurable + badge tokens en Log. Ref: repo opencode | <25% tokens del baseline | 1) Plan = 1 llamada 2) Default Chat persiste 3) Atajos listados | Revisar repo grande; Ctrl+2 | v0.7 | [v0.7.1.md](VERSIONS/v0.7.1.md) |
| **v0.8** | Cuestionario 3 niveles | Niveles + plantillas + ESPEC por nivel (modos movidos a v0.7.1) | 3 flujos | 1) Niveles distintos 2) ESPEC por nivel válido | 1 de cada nivel | v0.7.1 | [v0.8.md](VERSIONS/v0.8.md) |
| **v0.9** | STACK local + legal + instalador | FTS5 + 5 funciones + panel + seed + `author/license/source` + consent 3×OFF + identidad local + contador uso + `.deb` + CI mínimo | STACK legal instalable | 1) `Uso interno` no sube 2) Consent OFF bloquea 3) .deb conserva datos 4) CI verde | Guardar/valorar/buscar; instalar .deb limpio | v0.7.1 | [v0.9.md](VERSIONS/v0.9.md) |
| **v1.0** | STACK nube + release | push/pull/search + auth (token 600) + fallback + Windows + backup/export + mecanismo update + docs/tag | Release estable | 1) Push A→B 2) Offline fallback 3) Export/import conserva 4) 0 críticos | Nube A↔B; dogfooding todo-cli | v0.9 | [v1.0.md](VERSIONS/v1.0.md) |
| **v1.1** | Pro + macOS + licencias | `license.rs` ed25519 + planes Trial/$5/$12/$20 + `snapshot_ws` + `drivers: Vec` + `merge.rs` file-level + `custom.rs` agentes/flujos + editor cadena + 🔒 Free + `.dmg` aarch64 + `open` mac | Pro vendible en 3 OS | 1) Free→modal Pro 2) 2 sandboxes→merge auto 3) Conflicto→MERGE.md 4) Trial 30d 5) .dmg abre en mac limpio | Trial + E2E paralelo Groq | v1.0 | [v1.1.md](VERSIONS/v1.1.md) |

## 5. Convenciones

*   Versionado SemVer `v0.x` → `v1.0`. Cada versión actualiza `VERSIONS.md`.
*   Idioma ES docs, EN código/commits.
*   **Monetización (v1.1):** STACK gratis e ilimitado siempre (efecto red).
    Pro cobra solo potencia: workers paralelos, sandboxes, agentes y flujos
    custom. Licencias offline `ed25519` + pago externo, sin backend propio.
*   Providers nativos (no wrapper genérico) desde v0.1.

## 6. Backlog Config/UI (v0.8–v1.0)

Mejoras de personalización pendientes (cualquier versión entre v0.8 y v1.0):

*   **Más opciones en Configuración:** Apariencia (fuentes, radios, densidad fina, más acentos), Permisos y Límites ampliados.
*   **Coste:** panel de gasto por proyecto/chat/modelo y presupuestos/avisos.
*   **Propiedad del código (STACK):** opciones de licencia/propiedad y consentimiento al compartir.
*   **Control de datos:** gestión de archivos, proyectos, caché y contexto (ver, exportar, limpiar).
*   **Límites con especificación:** cada límite con descripción clara y tooltip (no solo "historial del modelo"); tooltip flotante al pasar el cursor (ya iniciado en Límites).
*   **UI:** bloques de Config lado a lado en pantallas anchas (iniciado) y sidebar de pestañas más ancha.
*   **Guardar modelos/perfiles:** además de elegir un modelo por provider, poder **guardar varios modelos** con su propia API key, `base_url` y provider (perfiles reutilizables) y cambiar entre ellos rápido.
*   **Lector de Markdown del chat:** revisar cobertura completa de sintaxis (tablas, listas anidadas, blockquotes, checkboxes, imágenes, HTML, LaTeX, código con lenguaje/resaltado) y **mejorar el aspecto visual** (tipografía, espaciado, bloques de código, enlaces, tablas).
*   **Repos guía:** estudiar los repositorios de **opencode**, **OpenHands** y **SWE-agent** como referencia para el orquestador, las tools, los prompts y el flujo de agente.
*   **Sesiones de chat:** dividir los chats en **sesiones** y enviar un id de sesión estable cuando el provider lo soporte (OpenRouter, OpenAI, Anthropic, Groq…), para mejorar afinidad de caché/coste y trazabilidad. Investigar la doc de cada provider.
