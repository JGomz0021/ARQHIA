# ARQHIA — PROJECT.md

## 1. Descripción

ARQHIA es un agente de IA nativo que acompaña al usuario desde la definición de la idea hasta la resolución estable del proyecto. A diferencia de otros asistentes que solo generan código, ARQHIA ayuda a diseñar el proyecto, sus especificaciones, funcionalidades y stack, adaptándose al nivel de experiencia del usuario y trabajando de forma delegable y estable paso a paso.

Producto dirigido a desarrolladores independientes, pequeños grupos e incluso grandes empresas que requieren un orquestador confiable capaz de planificar, construir, auditar y corregir hasta alcanzar una versión estable.

## 2. Visión / Misión

**Visión:** Ser el IDE-agente estándar para pasar de idea a producto sin fricción, con trazabilidad total del contexto y del plan por versión.

**Misión:** Proveer un flujo guiado (cuestionario + conversación IA) que genera una especificación sólida y un agente orquestador que ejecuta, audita y corrige de forma autónoma sobre un workspace estructurado y un STACK de conocimiento reutilizable.

## 3. Stack Tecnológico

### 3.1 Decisión Principal: Iced nativo (sin Tauri)

Se adopta **Iced 0.13 desde v0.1** como stack único.

*   **App:** Rust 100% + Iced 0.13 (GUI nativa, sin WebView, sin frontend web).
*   **Motivo:** Ya no se usará Tauri. Se quiere binario nativo puro Rust, sin capa HTML/CSS/JS.
*   **Core:** Rust + Tokio (async), Serde, reqwest (LLM APIs con streaming), rusqlite/SQLite, rfd (diálogos nativos de archivos), minijinja (plantillas .md).
*   **UI:** `iced::widget` — `column/row/container/text/text_input/button/scrollable/pick_list/checkbox/slider/vertical_rule/rule/modal`.
*   **Tema:** `iced::Theme::Dark` por defecto, `Light` configurable desde v0.2, colores custom en v0.7 vía `Theme::custom`.

> Lógica y UI 100% en Rust en el crate `ARQHIA_P/`.

### 3.2 Componentes

*   **Core:** Rust + Tokio, Serde, Iced 0.13
*   **Persistencia:** File-system + SQLite (índice de proyectos/chats, historial, STACK local)
*   **Git (v0.7.2):** soporte nativo en `git.rs` (init, rama de trabajo `ARQHIA`, estado, commit, push). La política vive en `config.rs::GitConfig` y se aplica por comando en `agent/tools.rs`; el auto-commit solo ocurre si la puerta de calidad (`cargo check` + `test` + `clippy`) pasa.
*   **LLM:** Clientes **nativos por provider** (`llm/openai.rs`, `llm/anthropic.rs`, `llm/openrouter.rs`, `llm/local.rs`) con trait común `Provider { chat_stream }`. Keys configurables en pantalla Config. Cada provider con su `base_url`, `api_key`, `model` y su formato de auth/cuerpo propio.
*   **Local (LM Studio):** provider `Local (LM Studio)`, OpenAI-compatible contra `http://localhost:1234` (normaliza `/v1` pegado). No exige API key (LM Studio la ignora); sí exige el id exacto del modelo cargado (se ve en la app de LM Studio). `Probar conexión` lista modelos vía `GET /v1/models` sin auth.
*   **Catálogo de modelos/precios:** `pricing.rs` usa `https://models.dev/api.json` (público) para precios por 1M tokens (input/output/cache), contexto, capacidades (tools/reasoning) y **niveles de razonamiento por modelo** (`reasoning_options`). Cache en disco; `RefreshPricing` lo actualiza. Alimenta el coste por mensaje, la ventana de contexto, el navegador de modelos y el selector de nivel.
*   **Updater (v1.0):** `updates.rs` consulta `{web}/updates/latest.json`, compara semver y muestra banner en Home (notificar + descargar). En Windows-Store la Store actualiza sola.
*   **Web (dependencia, `CONTEXT/WEB.md`):** sitio Astro + Cloudflare con descarga, precios, soporte, docs, legal y manifiesto de updates. No es una versión: es prerrequisito de v1.0.
*   **Distribución:** Linux `.deb`/`.tar.gz`; Windows vía **Microsoft Store** (registro gratis, Microsoft firma el MSIX y auto-actualiza; sin certificado Authenticode propio). macOS (firma Developer ID + notarización) en v1.2, condicionado a base de usuarios PRO.
*   **Build Target:** Linux y Windows en v1.0–v1.1; macOS en v1.2 (Iced lo permite sin cambios mayores).

## 4. Arquitectura Workspace

Desde v0.8 cada proyecto creado por ARQHIA se estructura así:

```
{workspace}/
├── Project/                 # Código + git del proyecto del usuario
│   └── (vacío al inicio)
├── ToDo.md                  # Tablero de ejecución (lo mantiene el agente)
└── CONTEXT/                 # SSOT del proyecto generado
    ├── CONTEXT.md           # Índice + estado vivo + siguiente paso (entrada del agente)
    ├── PROJECT.md           # Visión, objetivo, alcance, usuario
    ├── SPECS.md             # Especificación funcional (reemplaza ESPEC.md)
    ├── ROADMAP.md           # Versiones de alto nivel
    ├── VERSIONS.md          # Índice de estado por versión
    ├── VERSIONS/            # Un .md por versión (v0.1.md, ... v1.0.md)
    └── TEMP.md              # Errores de auditoría (efímero, desde v0.3/v0.6)
```

*   **`CONTEXT.md` es índice + estado vivo**, no un volcado de todo: resume situación actual y siguiente paso (rol que en el producto cumple `HECHO.md`) y evita duplicar `PROJECT/SPECS/VERSIONS`.
*   **`Project/` es la raíz de código/git.** El guard del agente permite escribir en `Project/`, `CONTEXT/` y `ToDo.md`, y sigue bloqueando fuera del workspace.
*   `ROADMAP.md`, `VERSIONS.md` y `VERSIONS/v0.1.md` se generan al ejecutar el primer **Plan**; el cuestionario crea `PROJECT.md`, `SPECS.md` y `CONTEXT.md`.

En v0.2 `Proyecto` es solo agrupador lógico de chats (sin path en disco). Desde v0.3 cada proyecto tiene `path: PathBuf` asignado como workspace real con guards `canonicalize + starts_with`. Desde v0.7.2, si Git está habilitado, el workspace se inicializa como repo y el agente trabaja en la rama de trabajo `ARQHIA`, dejando la rama base (`main`) protegida. **El naming estándar de especificaciones es `SPECS.md`** (se retira `ESPEC.md`; migración con nota legacy).

*   `AGENTS.md:1` (desde v0.6) — Define herramientas permitidas, permisos y roles orquestador/worker/auditor.
*   `CONTEXT/` es leído en cada petición del agente antes de revisar código.

Código fuente de ARQHIA vive en `ARQHIA_P/` en la raíz del repo:

```
ARQHIA_P/
├── Cargo.toml           # tokio slim + tracing (v0.9.5)
├── README.md → ver raíz  # quickstart en /README.md (v0.9.5)
├── docs/                # ARCHITECTURE.md, CONFIG.md, SECURITY.md, adr/ (v0.9.5)
└── src/
    ├── main.rs            # shell: update/view guards + iced::application
    ├── app/
    │   ├── events.rs      # Message / View / ConfigTab (solo datos)
    │   ├── state.rs       # App + carga inicial + history_* atómicos (v0.9.5)
    │   ├── history.rs     # ChatHistory alineado + tests (v0.9.5)
    │   ├── projects.rs    # crear / papelera / borrado total (SQLite + FS)
    │   ├── orchestrator.rs# driver planner → workers → auditor + run_fix_cycle (v0.9.5)
    │   └── handlers/      # brazos de update por dominio
    │       └── {navigation,chat,chat_stream,chat_history,projects,agent,config,questionnaire,stack}.rs  # splits v0.9.5
    ├── views/             # render puro (home, sidebar, chat, questionnaire, config_view, config_api, config_git, stack)
    ├── ui/                # design tokens + componentes (design.rs, components.rs)
    ├── config.rs          # providers + tema + permisos + git + cuenta/licencia (toml)
    ├── db.rs              # SQLite (chats, projects, messages, stack, uso por project_id v0.9.5)
    ├── titles.rs          # helpers de títulos (puro)
    ├── workspace.rs       # guards + uploads + context_block (+ async v0.9.5)
    ├── paths.rs           # datos/config/home vía `directories` (v0.9.4)
    ├── git.rs             # repo/rama/commit/push async (v0.7.2, async v0.9.5)
    ├── stack/{mod.rs, seed.rs}   # STACK local: FTS5 + seed + legal (v0.9)
    ├── skills.rs + skills/embebidas/  # skills locales: SKILL.md + /skill + pestaña Config (v0.9 Track C)
    ├── mcp.rs             # cliente MCP stdio/HTTP + tools `mcp__*` (mínimo v0.9.3, resources/prompts en v1.0.1)
    ├── questionnaire/{mod.rs, levels.rs, ai.rs, feature.rs, import.rs, templates.rs, planning.rs}  # PROJECT.md + SPECS.md + contexto auto (planning.rs v0.9 Track D)
    ├── llm/{mod.rs, openai.rs, anthropic.rs, openrouter.rs, local.rs}
    ├── pricing.rs         # catálogo de modelos/precios (models.dev) + cache
    └── agent/{mod.rs, tools.rs, roles.rs}   # 5 roles explícitos (v0.9.1)
```

> Solo archivos reales (v0.9.5): lo futuro (`updates.rs`, `license.rs`,
> `custom.rs`, `agent/merge.rs`, `stack/cloud.rs`, snapshot v1.1, MCP
> resources/prompts v1.0.1) vive en `ROADMAP.md`, no en este árbol.

Regla de capas: `views/` no toca DB ni red; `handlers/` coordina vía
`app/` + dominio (`db`, `llm`, `agent`, `workspace`); los providers y las
tools no conocen el estado UI.

## 5. UI/UX

### 5.1 Principios

*   Estilo **ChatGPT** — minimalista, conversación central.
*   Tema **oscuro** por defecto, claro configurable.
*   Nativo Iced, ventana única, vistas `View::{Home, Chat, Config, Questionnaire}`.

### 5.2 Pantalla de Inicio / Home (v0.4)

```
┌─────────────────────────────────┐
│ ARQHIA                          │
│                                 │
│  [ Crear proyecto ]             │
│  [ Abrir proyecto ]             │
│                                 │
│  ─────────────────────────────  │
│  [ Configuración ]              │
└─────────────────────────────────┘
```

*   `Crear proyecto` → crea `Project{name, path}`.
*   `Abrir proyecto` → `rfd::FileDialog::pick_folder()`.
*   **Onboarding (v0.8):** si no hay ningún provider configurado, Home muestra un aviso de bienvenida que pide configurar una API (provider + key + model) con opción `Configurar después`. Al guardar la primera API, el aviso desaparece y no vuelve.
*   **Updater (v1.0):** si `updates/latest.json` reporta una versión superior, Home muestra un banner (versión + novedades + `Descargar`). En Windows-Store se omite porque la Store auto-actualiza.
*   `Configuración` abajo → API + tema.
*   Desde v0.7 la Configuración va por pestañas: `API | Apariencia | Permisos | Git | Proyectos` (columna izquierda + contenido). La pestaña **Git** (v0.7.2) configura el repositorio (enabled, auto-init, rama base, rama de trabajo, remoto, rama de push), la autonomía del agente (`Solo lectura / Commit local / Commit y push`), el permiso de push a GitHub, el autor de commits y el estado del workspace activo (rama, cambios, botón `Inicializar git`). La pestaña Proyectos lista workspaces, borra proyectos (a papelera) y gestiona archivos subidos. Desde v0.7.1 hay apartado **Atajos** (lista de solo lectura: `Ctrl+N` nuevo chat, `Ctrl+1/2/3` modos, `Ctrl+O` abrir proyecto, `Ctrl+,` configuración, `Esc` cerrar, `Doble Esc` detener el turno en curso).

### 5.3 Layout Principal (Chat + Sidebar)

```
┌──────────┬──────────────────────────────┐
│ Sidebar  │ Main Chat                    │
│ Left 260 │ Mensajes IA/Usuario (md)     │
│ ──────── │                              │
│ +Chat    │  [Modelo ▾][ Input  ][Enviar]│
│ chats... │  ─────────────────────────── │
│ ──────── │  Log tools (v0.3+, abajo)    │
│ Proyectos│                              │
└──────────┴──────────────────────────────┘
```

*   **Sidebar Left (v0.2):** `column![button(+ Nuevo chat), scrollable(chats), rule, proyectos + text_input(nuevo) + assign]`. Cada proyecto tiene `+` para crear chats dentro y campo de workspace (v0.3). Desde el pulido: secciones separadas `Chats` y `Proyectos` (con regla divisoria), proyectos plegables (`▸/▾`), crear proyecto inline con `+` en la cabecera de la sección (sin formulario fijo abajo), iconos compactos. Borrar un proyecto pide confirmación y elimina sus chats de la DB **y su carpeta del disco** (`remove_dir_all`, con aviso en el Log si falla).
*   **Main Chat (v0.1):** `column![scrollable(mensajes), row![model_picker, text_input, button(Enviar)]]` + `Task::stream` para streaming.
*   **Layout (UI polish):** mensajes centrados con ancho según **densidad** (`cómoda ~1020px`, `compacta ~860px`; ~20% menos que a todo el ancho) en una ventana inicial `1560×880`; sidebar `248px`. Cabecera, barra de stats y **dock inferior** a todo el ancho. Texto de mensajes unificado a 14px (usuario y modelo). Sin botón `"<"` de volver en el header (ya está `Inicio` en el sidebar). El dock (composer + log) usa fondo elevado con sombra superior (`design::dock`) para marcar con claridad dónde empieza la zona de entrada. **Composer en dos filas:** arriba solo la caja de entrada + Enviar/Detener; debajo, modos, selector de modelo y datos del modelo activo. La **densidad** (Cómoda/Compacta) reduce de verdad espaciados y paddings (`design::gap/pad`): separación entre mensajes, burbujas, dock, sidebar y cards de Config.
*   **Burbujas (UI polish):** mensajes del usuario alineados a la izquierda sobre superficie elevada con borde de acento; respuestas del agente en pozo (well) con borde neutro. Sin etiqueta "ARQHIA" repetida por mensaje ni modelo duplicado en el header (el modelo solo vive en el selector del composer).
*   **Tokens y coste (UI polish):** bajo cada mensaje se muestra `in/out/cache` y coste; barra superior con `N tokens / X% used` (miles con punto), tokens de sesión y gasto de API. El **coste prefiere el reportado por el proveedor** (OpenRouter `usage.cost`) > catálogo models.dev > tabla local. El **contexto incluye tools/lecturas del agente** (`App.context_tokens`, actualizado por `account_tokens` con `system + raw`), no solo los mensajes. Datos de uso en un vector paralelo `App.msg_usage` (no se persisten).
*   **Precios reales (models.dev):** módulo `pricing.rs` descarga y cachea `https://models.dev/api.json` (~200 providers, miles de modelos) con precio por 1M tokens de input/output y cache read/write + ventana de contexto + capacidades (tools/reasoning). El coste por mensaje usa el catálogo con fallback a `llm::estimate_cost_usd`. Cache en `~/.local/share/arqhia/models.dev.json`.
*   **Navegador de modelos (Config → API):** botón `Buscar modelo` junto al campo Modelo. Exige **API key del proveedor** antes de listar (para nube; Local consulta `GET /v1/models` de LM Studio). Incluye búsqueda por id/nombre/familia, **filtro de precio** (`Todos / Gratis / ≤ $1 / ≤ $5 / ≤ $15` sobre input), casillas `solo tools` y `ordenar por precio`, un contador de resultados, precios, contexto y badges `tools`/`razona`; `Actualizar` refresca el catálogo. Panel ancho (900px) con lista scrollable. Mapea el provider de ARQHIA a su id de models.dev (reconoce Groq/Google/etc. por `base_url`).
*   **Nivel de razonamiento por modelo:** `models.dev` expone `reasoning_options` por modelo (`effort` con valores propios, p.ej. `low/medium/high/max`, o `toggle`). El selector está **siempre visible** en **Config → API** (sobre el modelo en edición) y en el **composer** (modelo activo); si el catálogo no está cargado, se descarga solo al arrancar (una vez, cacheado), así los niveles aparecen sin acción manual. `auto` = sin parámetro. Los pickers de modelo y razonamiento del composer llevan **caja con borde de acento** (`design::accent_pick`) para distinguirse como controles; cambiar el nivel no genera mensaje de estado. Se persiste en `ProviderConfig.reasoning_effort` y se envía: `reasoning_effort` (OpenAI/Local), `reasoning.effort` (OpenRouter) y `thinking.budget_tokens` mapeado (Anthropic).
*   **Iconografía:** se retiraron emoji/glifos no fiables de botones y del feed; el Log usa marcadores ASCII (`+ ok`, `x err`, `! warn`, `> accent`) y el estado de tareas es un checkbox ASCII (`x` hecho, `>` en progreso, vacío pendiente).
*   **Selector de modelo (v0.3+):** `pick_list` en la entrada del chat con los providers configurados (`"OpenAI · gpt-4o-mini"`). Cambia `config.active` al instante y persiste en `config.toml`. Solo lista providers con API key cargada; si ninguno, muestra `"Sin modelos (Config)"` y lleva a Configuración.
*   **Render Markdown (v0.3+):** las respuestas de la IA se renderizan con `iced::widget::markdown` (negritas, cursivas, encabezados, listas, código inline y bloques de código estilo ChatGPT/Claude, monoespaciado por Iced). Los mensajes del usuario van en texto plano. Click en enlaces abre con `xdg-open`.
*   **Log inferior (v0.3+, ampliable):** panel del dock con las últimas líneas del stream de tools. **Solo se muestra en modos Plan/Work** (oculto en Chat, que no ejecuta tools). Texto 14px, con iconos de estado y errores resaltados con color de alerta; botón `Ampliar/Reducir` que alterna 12↔40 líneas y alto 170↔460px.

### 5.4 Mejoras futuras de Config/UI (v0.8–v1.0)

Pendiente de implementar en cualquiera de las versiones v0.8–v1.0:

*   Más opciones de personalización en Configuración (Apariencia, Permisos, Límites).
*   Coste: desglose por proyecto/chat/modelo y presupuestos.
*   **Límites de API por tiempo/gasto:** topes configurables por periodo (día/semana/mes) y por gasto acumulado (USD), por provider y globales; aviso al 80%, bloqueo opcional al 100%. Conteo local, sin telemetría.
*   Opciones de propiedad del código para el STACK (licencia/consentimiento).
*   Control de datos: archivos, proyectos, caché y contexto (ver/exportar/limpiar).
*   Especificaciones claras en cada límite (no solo historial del modelo), con **tooltip flotante** al pasar el cursor (ya iniciado en Límites).
*   Config con sidebar de pestañas más ancha y bloques pequeños lado a lado (iniciado).
*   **Guardar modelos/perfiles:** además de un modelo por provider, guardar varios modelos con su propia API key, `base_url`, provider y un **nombre visible** que aparezca en el selector. **Asignado a v0.7.4** (ver `CONTEXT/VERSIONS/v0.7.4.md` §A).
*   **Utilidades de chat:** título generado por IA, undo, copiar chat, nuevo chat desde un mensaje, fecha/hora por mensaje, reintento con icono y **citar fuentes** tras investigar en la red (`fetch_url`). **Asignado a v0.7.4** (`§B`).
*   **Lector de Markdown del chat:** verificar cobertura de sintaxis completa (tablas, listas anidadas, blockquotes, checkboxes, imágenes, HTML, LaTeX, código con lenguaje) y mejorar el aspecto visual (tipografía, espaciado, bloques de código, enlaces, tablas).
*   **Repos guía:** usar los repositorios de **opencode**, **OpenHands** y **SWE-agent** como referencia para el orquestador, tools, prompts y flujo de agente.
*   **Sesiones de chat:** separar los chats en **sesiones** con un id estable cuando el provider lo permita (OpenRouter, OpenAI, Anthropic, Groq…), para que lo detecten y mejoren caché/coste y trazabilidad (revisar doc de cada provider). **Asignado a v0.8** (ver `CONTEXT/VERSIONS/v0.8.md` §E).

## 6. Flujo de Creación de Proyecto

```
Home (Crear/Abrir) → Cuestionario (genérico + nivel + IA opcional) →
PROJECT.md + SPECS.md + CONTEXT.md + estructura (Project/, ToDo.md, CONTEXT/) →
Chat / Plan (ROADMAP.md + VERSIONS.md + VERSIONS/v0.x.md) → Work (orquestador) →
STACK (v0.9/v1.0) → release
```

Desde v0.7.3 el turno Work sigue el flujo de estabilidad:
`petición → analista (CONTEXT/specs/outlines) → planner → workers → auditor →
TEMP → loop fix hasta verde → commit (rama ARQHIA) → push (opcional)`.

## 7. Cuestionario

### 7.1 v0.5 — Base (histórico, Done)

Wizard de preguntas fijas que generaba un único `.md` de especificaciones
(`CONTEXT/ESPEC.md`). Queda como base; el diseño vigente es el de v0.8.

### 7.2 v0.8 — Genérico + por nivel + IA (base 🟢 Done; universal en v0.8.1, reorden + MVP en v0.8.2)

**Paso 0 — Nivel:** `Principiante | Intermedio | Avanzado`.

**Genéricas (siempre):** nombre, descripción, objetivo, características/funcionalidades.

**Por nivel (v0.8 base) + taxonomía universal (v0.8.1, reemplaza `TipoProyecto` por `Categoria → SysType` con 6 categorías/~24 tipos y preguntas por familia; ver `VERSIONS/v0.8.1.md`):**

*   **Principiante (diseño y decisiones, no código):** estilo visual con
    lista `{minimalista, gamer, corporativo, infantil, retro, oscuro,
    otro}` + referencia libre, plataforma **múltiple** `{web, escritorio,
    móvil iOS, móvil Android, servidor/nube, embebido/IoT,
    multiplataforma}`, facturación `{suscripciones, pago único, api, uso
    personal, publicidad, freemium, código abierto}`.
*   **Intermedio:** UI/UX, tipo de proyecto `{app web, API/backend,
    framework/librería, escritorio, móvil, CLI, juego, bot/agente, otro}`,
    plataforma múltiple, stack **múltiple** `{Rust, Python, JS/TS, Go,
    Java/Kotlin, C/C++, C#, PHP, Ruby, Swift, otro, a decidir}` + detalle,
    facturación.
*   **Avanzado:** como Intermedio + arquitectura con lista `{monolito, por
    capas, hexagonal, microservicios, eventos, serverless, a decidir}` +
    detalle.
*   Vacío en picks/checkboxes = "(sin especificar)": nunca bloquean.

**IA (opcional, saltable):** el provider activo propone 3–5 preguntas
adicionales adaptadas a lo respondido. Sin provider, el paso se deshabilita y
el flujo continúa.

**Salida:** `PROJECT.md` (visión/objetivo/alcance/usuario), `SPECS.md`
(funcionalidades + secciones por familia según v0.8.1 + plataforma + stack +
arquitectura + facturación o licencia según familia) y `CONTEXT.md`
(índice + estado vivo + origen desde-cero/import). Se crea la estructura
`Project/`, `ToDo.md`, `CONTEXT/`. Hasta v0.8.1, `ROADMAP.md`/`VERSIONS.md`/
`VERSIONS/v0.1.md` se generaban en el primer **Plan** manual (chat "Plan
inicial" con prompt precargado + Enviar); en v0.9 Track D esa
generación pasó a automática (el Finish disparaba solo el turno Plan con el
system de convenciones de la casa, sin Enviar manual). **Desde v0.8.2 el
Finish abre una pantalla de carga (`View::Generating`) que genera sin pasar
por el chat la serie MVP (`ROADMAP + VERSIONS + v0.1→v0.2→v0.3→v1.0 + ToDo`)
con el system especial `mvp_prompt` y marcadores `---FILE---`; sin API usa el
fallback determinista offline.** Al **Abrir** carpeta con código, el modo
Import pre-rellena y solo pregunta gaps (v0.8.1 §D). `ESPEC.md` se retira (migración con nota
legacy).

## 8. Chat & Agente

### 8.1 Chat v0.1 (sin agente)

*   Conexión nativa por provider (OpenAI, Anthropic, OpenRouter, Local LM Studio) con `reqwest` + streaming a `Subscription` Iced.
*   Config: `ProviderConfig { provider, api_key, base_url, model }` en `~/.config/arqhia/config.toml` + SQLite.
*   Historial persistido por chat. Sin tools, sin workspace.
*   System prompt (`llm::system_identity`): al inicio de cada conversación se envía quién es ARQHIA, sus capacidades (con/sin herramientas según haya workspace) y reglas de respuesta (idioma del usuario, Markdown). No se guarda en la DB. En Anthropic va en el parámetro `system`; en el resto como mensaje `system`.

### 8.2 Chat v0.2

*   Múltiples chats + agrupación por proyectos (lógico) + cambio de tema Dark/Light en vivo.

### 8.3 Agente simple v0.3

*   Tools: `read_file, write_file, edit_file, delete_file, list_dir, bash(cmd)` limitados a workspace asignado.
*   `bash` inicial allowlist: `ls, cat, echo, pwd, cargo --version, rustc --version, cargo check, cargo build, cargo test, cargo run` con timeout.
*   Chat con selector de modelo en la entrada, respuestas IA con Markdown y Log debajo del chat (ver §5.3).

### 8.4 Archivos subidos v0.7

*   Botón `Subir` por proyecto (sidebar) copia archivos a `{workspace}/uploads/` (tope 50 MB, sin sobrescribir: `nombre (2).ext`).
*   El agente los lee con las tools existentes (`read_file`, `search_files`, `list_dir uploads`).
*   Borrado individual desde Config → Proyectos, limitado a `uploads/`.

### 8.4 Orquestador v0.6

*   `Planner → 2x Worker (generadores de código) → Auditor` en paralelo con `tokio::join`.
*   Permisos: modal `Aprobar/Denegar` antes de `write/edit/delete/bash`.
*   Contexto: para esta parte se podrá revisar repositorios de otros agentes de código opensource (ej. aider, swe-agent, patrones claude-code) para diseño de prompts y loop auditor.
*   `AGENTS.md` gobierna tools y roles.

### 8.5 Modos Chat / Plan / Work (v0.7.1)

*   `Mode` por chat (persistido en `chats.mode`; **default: Chat**, migraciones NULL = Chat).
*   **Chat:** conversación directa sin tools. **Plan:** solo `plan_tasks` → `CONTEXT/PLAN.md` aprobable + botón "Ejecutar plan" (pasa a Work). **Work:** orquestador con permisos/límites.
*   Selector segmentado de modo en el composer; el system prompt declara modo y capacidades. El badge de modo del header se retiró (con el indicador del composer basta). También se limpiaron del header/dock los botones de `Cuestionario` y `TEMP` (el cuestionario abre al crear/abrir proyecto). `Salir` en el sidebar usa rojo outline para destacar.
*   Atajos: `Ctrl+N` nuevo chat, `Ctrl+1/2/3` modos, `Ctrl+O` abrir proyecto, `Ctrl+,` configuración, `Esc` cerrar (apartado Atajos en Config).

### 8.6 Permisos y límites (v0.7 Track B)

*   `Permissions { auto_read, auto_write, auto_bash, auto_net, auto_install, extra_paths }`: lo peligroso nace desactivado y siempre pide aprobación categorizada (`tool · objetivo · alcance`).
*   Tool `fetch_url` con allowlist de dominios; categoría `Install` para instaladores; guard `workspace ∪ extra_paths` (canonicalizado).
*   `Limits { max_iters, max_tasks, max_read_kb, bash_timeout_s, max_upload_mb }` con rangos validados. Planner sin tools (denegado por rol).
*   Políticas vinculantes en `CONTEXT/POLICIES.md` (uso, privacidad, propiedad, licencias, consentimiento).

### 8.7 Git (v0.7.2)

*   **Configuración** (`GitConfig` en `config.rs`): `enabled` (def true), `auto_init` (def true), `base_branch` (def `main`), `work_branch` (def `ARQHIA`), `branch_mode` (`Single`/`PerTask`), `autonomy` (`ReadOnly`/`CommitLocal`/`CommitAndPush`, def `CommitLocal`), `push_enabled` (def false), `remote` (def `origin`), `push_branch`, `protected` (def `main`/`master`), autor opcional.
*   **Módulo `git.rs`:** init del repo, rama de trabajo, estado (`status`, rama, remoto), `commit_all` y `push` (no interactivo + timeout). Guarda anti-sucio: no auto-commitea si el árbol venía sucio antes del turno.
*   **Política por comando** (`agent/tools.rs`): lectura auto; `init/add/commit/stash` con `autonomy ≥ CommitLocal`; `push/fetch/pull` solo con `CommitAndPush && push_enabled` (si no, panel de aprobación). Bloqueados siempre `push --force`, `reset --hard`, `clean`, `rebase`, `config`, `remote add/remove` y commits sobre `protected`.
*   **Flujo:** al crear/abrir proyecto se inicializa repo y rama `ARQHIA`; al cerrar un turno Work exitoso, commit `"ARQHIA: <resumen>"` **solo si** `cargo check`+`test`+`clippy` pasan. El auditor incluye `git diff --stat` en `TEMP.md`.
*   **Push a GitHub:** desactivado por defecto; al activarlo, el agente pide aprobación (categoría `GitPush`) y puede empujar a `push_branch` (def = `ARQHIA`).

### 8.8 Bucle de estabilidad (v0.7.3)

*   **Flujo pre-STACK:** `petición → analista → planner → workers → auditor → TEMP → loop fix hasta verde → commit → push (opcional)`.
*   **Analista dedicado:** `agent::analyze_workspace` (1 llamada sin tools) lee `CONTEXT/*.md` (ESPEC o PROJECT/SPECS/CONTEXT en v0.8), `VERSIONS.md`, `TEMP.md` previo y outlines de código, y produce un **brief** (`CONTEXT/ANALYSIS.md`) que consume el planner. Fallback a `context_block` si no hay provider/timeout.
*   **Loop sin tope:** `auditor → fix worker → re-auditoría` hasta que no haya issues. `Limits.max_fix_cycles` (def `0` = ilimitado) permite caparlo. Frenos reales: `Detener`, `max_tokens_turn` y parada temprana.
*   **Commit solo al cerrar verde:** el auto-commit del turno se mueve al final del bucle verde (auditor limpio + `check/test/clippy` OK). Detenido o con presupuesto agotado ⇒ sin commit.
*   **Push opcional:** `autonomy == CommitAndPush && push_enabled` + aprobación `GitPush`.

### 8.9 Sesiones de chat (v0.8)

*   Cada chat guarda un **`session_id` estable** (`chats.session_id`, migración aditiva) que se genera al primer turno y sobrevive a archivar/mover.
*   Se envía al provider que lo soporta: **OpenRouter** `session_id`, **OpenAI/Groq** `prompt_cache_key` (fallback `user`), **Anthropic** header `x-session-id` (trazabilidad); **Local** sin efecto. Helper `llm::session_body_fields`.
*   Acción `Reiniciar sesión` en el menú `⋯` del chat regenera el id. Mejora caché/coste y trazabilidad; sin provider con soporte no cambia nada.

### 8.11 BETA agentes (v0.9.1)

### 8.10 Chat UX + modelos con nombre (v0.7.4)

*   **Perfiles de modelo** con **nombre visible** (provider + `base_url` + API key + model + nivel): el nombre es lo que sale en el selector de Config y del composer. CRUD y migración desde el modelo activo.
*   **Utilidades de chat:** título por IA (fallback al primer mensaje), undo (`Ctrl+Z`, 1 paso), copiar chat, nuevo chat desde un mensaje (bifurcar), fecha/hora por mensaje (`messages.created_at`), reintento con icono `↻` y **citar fuentes** (`Fuentes` clicables) cuando el agente usa `fetch_url`.

### 8.11 BETA agentes (v0.9.1)

*   **5 roles explícitos** (`agent/roles.rs`): Orquestador, Analista, Planner,
    Worker y Auditor, cada uno con system prompt y tools permitidas propias.
    El Planner solo Net+Read (sin Write/Bash por construcción del body y por
    gate de rol en `exec_calls`); el Worker es el único con Write/Bash/Install;
    el Auditor y el Analista no ejecutan nada.
*   **Planner Net+Read:** flag `planner_net` en `Permissions` (OFF por defecto,
    con checkbox en Config → Permisos). Si el pedido menciona URLs sin cubrir,
    el planner pide permiso Net por el panel habitual antes de salir a la red
    (Aprobar = consulta con tope 8 KB por fetch y planifica; Denegar =
    planifica sin docs). Las rutas extra se leen con el permiso Read.
*   **Loop con re-análisis:** ante `VERDICT: ISSUES`, el analista revisa el
    TEMP.md (`AgentReanalyze` → `ANALYSIS.md` actualizada) antes del worker de
    fixes; luego se re-audita hasta verde o tope.
*   **Visibilidad:** Log `orquestador: fase X/5 — nombre` (analista, planner,
    workers, auditor, cierre) + línea `checklist: ☑/▶/☐` tras plan, workers y
    re-análisis; `PLAN.md` con criterio de aceptación por tarea.

## 9. STACK

### 9.1 STACK Local (v0.9)

*   SQLite + FTS5: `stack_items(id, title, code, tags, metadata_json, rating)`.
*   Metadatos: dependencias, versiones, ejecuciones, bugs, opiniones.
*   Legal desde el día 1 (v0.9 Track B): `author + license (MIT/Apache-2.0/Uso interno) + source`; consentimiento triple default OFF (`use_stack/share_local/share_cloud`); identidad local (nombre+email) para firmar.
*   Funciones completas: etiquetas, búsqueda con ranking, metadatos, `save/search/get/rate`.

### 9.2 STACK Nube (v1.0)

*   Cliente `stack_cloud.rs`: `push/pull/search_cloud` contra backend Axum+Postgres+S3 (o mock `~/.arqhia/cloud/` si no hay infra). Sync + fallback offline.

> El agente consulta el STACK en cada tarea (desde v0.9): `Revisa CONTEXT → Revisa código → Consulta STACK → Diseña tareas → Ejecuta`.

### 9.3 Skills (v0.9 locales, v1.0 nube)

*   Carpeta por skill con `SKILL.md` (frontmatter + instrucciones) + recursos/scripts, en `~/.local/share/arqhia/skills/`; ARQHIA trae embebidas (`commit-msg`, `ui-ux`, `code-review`, `test-qa`).
*   **Dominio (v0.9.2):** `ui-ux` → `CONTEXT/UI-REVIEW.md`, `code-review` → `CONTEXT/CODE-REVIEW.md`, `test-qa` → `CONTEXT/QA-REPORT.md`. El analista las incluye en el contexto del siguiente turno; el auditor sugiere `/skill test-qa` o `/skill code-review` tras ISSUES (sin auto-ejecutar).
*   Se invocan con `/skill nombre` en el chat (inyecta como contexto, el turno sigue normal) y se administran en Config → Skills.
*   En la nube son items del STACK con etiqueta `SKILL` (instalar/publicar con el mismo consentimiento que el código; `Uso interno` no sale de local).
*   Sin permisos propios: sus scripts pasan por el permiso Bash existente.

## 10. Resumen de Versiones (v0.1 → v1.2)

| Versión | Foco | Entregable clave |
|---|---|---|
| **v0.1** | Chat normal multi-provider | Iced shell + Chat + Config API nativa por provider, sin agente |
| **v0.2** | Sidebar + temas + proyectos | Múltiples chats, left sidebar 260px, Dark/Light, proyectos como agrupadores |
| **v0.3** | Agente simple | Workspace asignado, CRUD archivos, bash simple con allowlist |
| **v0.4** | Home page | Vistas Home/Chat/Config, Crear/Abrir proyecto, Config abajo |
| **v0.5** | Cuestionario simple | Wizard preguntas fijas → doc de especificaciones (`ESPEC.md`, histórico) |
| **v0.6** | Orquestador + permisos | Permisos modal, +tools, planner + 2 generadores + auditor |
| **v0.7** | Polish + seguridad | Tabs, uploads, permisos ampliados (todo peligroso OFF), límites, apariencia, POLICIES |
| **v0.7.1** | Agente eficiente + Modos | Matches/search v2, read paginado, outline, modos Chat/Plan/Work, atajos, presupuesto + badge tokens |
| **v0.7.2** | Git nativo + puerta de calidad | Repo por workspace + rama `ARQHIA` + pestaña Git + permisos por comando + auto-commit condicionado a `cargo check`/`test`/`clippy` |
| **v0.7.3** | Bucle de estabilidad | Analista dedicado (CONTEXT/specs/outlines) + loop `auditor → fix` hasta verde + commit al cerrar verde + push opcional |
| **v0.7.4** | Chat UX + modelos con nombre | Perfiles de modelo con nombre visible + título IA + undo + copiar/bifurcar chat + fecha/hora + reintento con icono + citar fuentes |
| **v0.8** | Cuestionario genérico + nivel + IA, estructura, onboarding y sesiones | Genéricas + por nivel + IA opcional → `PROJECT.md` + `SPECS.md` + `CONTEXT.md`; layout `Project/`/`CONTEXT/`/`ToDo.md`; onboarding de API; `session_id` estable por chat |
| **v0.8.1** | Cuestionario universal + import | `Categoria` ×6 + `SysType` ~24 + preguntas por familia + `Licencia`; import auto al Abrir (scan IA + gaps + prefill, merge sin borrar) |
| **v0.8.2** | Cuestionario reordenado + MVP | Cat→Tipo primero + genéricas contextualizadas + subtipos (lenguaje/OS/plugin/motor) + rama OSS + pantalla de carga con serie v0.1→v1.0 sin chat |
| **v0.9** | STACK local + legal + identidad + infra skills + contexto auto | Tags + FTS5 + author/license/consent + identidad local + skills infra (`SKILL.md`, `/skill`, pestaña Config) + Track D (post-cuestionario automático con plantilla de la casa) — instalador en v0.9.6 |
| **v0.9.1** | BETA agentes (5 roles + loop con re-análisis) | `agent/roles.rs` + Planner Net+Read (`planner_net` OFF) + re-análisis del TEMP + fases y checklist en Log + criterios en PLAN.md |
| **v0.9.2** | Skills de dominio (UI/UX, CodeReview, Test/QA) | 3 embebidas → `CONTEXT/UI-REVIEW.md`, `CODE-REVIEW.md`, `QA-REPORT.md`; manual + sugerida |
| **v0.9.3** | MCP mínimo (stdio + HTTP) | `mcp.rs` + `[mcp]` + enrutado `mcp__srv__tool` + aprobación Net/auto |
| **v0.9.4** | Hardening crítico (auditoría) | Anti-symlink + FKs/índices + tests en temp + `directories` + config 600 + anti-SSRF |
| **v0.9.5** | Calidad estructural (auditoría) | `ChatHistory` + git async + split handlers/views + README/docs/ADRs + `tracing` |
| **v0.9.6** | Revisión + icono + instalador (puerta de v1.0) | Higiene con backup + refactor + optimización (<30MB, <2s) + tests por rol + `assets/icon.svg/png` + `.deb/.tar.gz` + CI mínimo |
| **WEB** | Sitio del producto (dependencia, no versionado) | Astro + Cloudflare: descarga, precios, soporte, docs, legal, `updates/latest.json` |
| **v1.0** | STACK nube + auth + updater + workers async + release + skills nube | Push/pull/sync + auth + updater (notificar+descargar) + **workers async 2–3 (§F)** + Microsoft Store (Windows) + backup/export + skills etiqueta `SKILL` + icono v0.9.6 + MCP mínimo v0.9.3 + release estable Linux+Windows |
| **v1.0.1** | MCP avanzado | Pestaña MCP + resources/prompts + item STACK `MCP` + auth HTTP |
| **v1.1** | Pro (Linux+Windows) | Sandboxes + merge sobre el paralelo v1.0 + agentes/flujos custom + planes Trial/$5/$12/$20 (MCP ya en v0.9.3/v1.0.1) |
| **v1.2** | macOS (condicionado) | Firma Developer ID + notarización + `.dmg` Apple Silicon; se abre con 50–100 PRO de pago |

Detalle paso a paso por versión en `ROADMAP.md` y `VERSIONS/v0.x.md`. La web en `WEB.md`.

## 11. Monetización (v1.1)

*   **STACK gratis e ilimitado** en Free y Pro (efecto red: el valor crece con
    los usuarios que generan código). Nunca se gata por tier.
*   **Pro para indies:** Trial 30 días sin tarjeta → Mensual $5 → Trimestral
    $12 → Semestral $20. Anual ($40) y Lifetime ($129) definidos pero ocultos
    hasta v1.2+ según retención. Único coste real: nube del STACK (~$12-25/mes).
*   Pro = potencia: hasta 8 workers paralelos en sandboxes
    `{ws}/.arqhia/work/task-N/` con merge file-level (`MERGE.md` + auditor si
    hay conflicto), multi-workspace por tarea, agentes custom
    (`nombre/descripción/tools/flujo`) y flujos visuales en cadena
    (`Leer > Contexto > Editar > Refactorizar > Ejecutar > Auditar`).
    Base async (2–3 workers sobre `Project/`, sin sandbox) ya en v1.0 §F;
    v1.1 la aísla y escala.
*   Licencias offline `ed25519` en `~/.config/arqhia/license.toml` (600),
    pago vía proveedor externo, sin backend propio ni telemetría de código.
*   **Alcance de plataforma:** Pro se vende en Linux y Windows (v1.1). macOS
    (v1.2) solo se abre con **50–100 usuarios PRO de pago** como base.

## 12. Convenciones

*   **Idioma:** Español para docs de producto, inglés para código y commits.
*   **Target OS:** Linux y Windows prioritarios; macOS condicionado.
*   **Versionado:** SemVer `v0.x` → `v1.2`.
*   **Iced:** 0.13, `Theme::Dark` por defecto.
*   **Distribución:** `.deb`/`.tar.gz` + updater (Linux); Microsoft Store (Windows).
*   **Web:** dependencia aparte (`CONTEXT/WEB.md`), Astro + Cloudflare, previa a v1.0.

## 13. Próximo Paso

**v0.9.1 – v0.9.5 🟢 Done** (ver `CONTEXT/VERSIONS/`). Siguiente, en orden: **v0.9.6** (revisión + icono + instalador, puerta de v1.0) → WEB → **v1.0** (incluye **workers async §F**) → **v1.0.1** (MCP avanzado) → v1.1 (sandboxes + merge sobre el paralelo v1.0) → v1.2. Ver `HECHO.md` para estado actual.
