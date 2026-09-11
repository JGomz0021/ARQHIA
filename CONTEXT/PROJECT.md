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
*   **LLM:** Clientes **nativos por provider** (`llm/openai.rs`, `llm/anthropic.rs`, `llm/openrouter.rs`, `llm/local.rs`) con trait común `Provider { chat_stream }`. Keys configurables en pantalla Config. Cada provider con su `base_url`, `api_key`, `model` y su formato de auth/cuerpo propio.
*   **Local (LM Studio):** provider `Local (LM Studio)`, OpenAI-compatible contra `http://localhost:1234` (normaliza `/v1` pegado). No exige API key (LM Studio la ignora); sí exige el id exacto del modelo cargado (se ve en la app de LM Studio). `Probar conexión` lista modelos vía `GET /v1/models` sin auth.
*   **Catálogo de modelos/precios:** `pricing.rs` usa `https://models.dev/api.json` (público) para precios por 1M tokens (input/output/cache), contexto, capacidades (tools/reasoning) y **niveles de razonamiento por modelo** (`reasoning_options`). Cache en disco; `RefreshPricing` lo actualiza. Alimenta el coste por mensaje, la ventana de contexto, el navegador de modelos y el selector de nivel.
*   **Build Target:** Linux en v0.1–v0.9, multiplataforma en v1.0 si da tiempo (Iced lo permite sin cambios mayores).

## 4. Arquitectura Workspace

Cada proyecto creado por ARQHIA contiene:

```
{proyecto}/
├── Proyecto/          # Código generado del proyecto del usuario (desde v0.3 es el workspace asignado)
└── CONTEXT/           # Fuente de verdad del proyecto
    ├── ESPEC.md       # Generado por cuestionario v0.5+ (nombre, descripción, ubicación, objetivo, público, interfaz)
    ├── CONCEPTO.md    # Idea, visión, objetivos (heredado / futuro)
    ├── PROJECT.md     # Copia de este archivo + adaptaciones
    ├── VERSIONS.md    # Índice de versiones
    ├── VERSIONS/      # Un .md por versión
    └── TEMP.md        # Errores de auditoría (efímero, desde v0.3/v0.6)
```

En v0.2 `Proyecto` es solo agrupador lógico de chats (sin path en disco). Desde v0.3 cada proyecto tiene `path: PathBuf` asignado como workspace real con guards `canonicalize + starts_with`.

*   `AGENTS.md:1` (desde v0.6) — Define herramientas permitidas, permisos y roles orquestador/worker/auditor.
*   `CONTEXT/` es leído en cada petición del agente antes de revisar código.

Código fuente de ARQHIA vive en `ARQHIA_P/` en la raíz del repo:

```
ARQHIA_P/
├── Cargo.toml
└── src/
    ├── main.rs            # shell: update/view guards + iced::application
    ├── app/
    │   ├── events.rs      # Message / View / ConfigTab (solo datos)
    │   ├── state.rs       # App + carga inicial + helpers puros
    │   ├── projects.rs    # crear / papelera / borrado total (SQLite + FS)
    │   ├── orchestrator.rs# driver planner → workers → auditor
    │   └── handlers/      # brazos de update por dominio
    │       └── {navigation,chat,projects,agent,config,questionnaire}.rs
    ├── views/             # render puro (home, sidebar, chat, questionnaire, config_view)
    ├── ui/                # design tokens + componentes (design.rs, components.rs)
    ├── config.rs          # providers + tema + permisos (toml)
    ├── db.rs              # SQLite (chats + archived, projects, messages)
    ├── sidebar.rs         # helpers de títulos (puro)
    ├── workspace.rs       # guards + uploads + context_block
    ├── questionnaire/     # modelo + validación + plantilla ESPEC.md
    ├── llm/{mod.rs, openai.rs, anthropic.rs, openrouter.rs, local.rs}
    ├── pricing.rs         # catálogo de modelos/precios (models.dev) + cache
    └── agent/{mod.rs, tools.rs}
```

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
*   `Configuración` abajo → API + tema.
*   Desde v0.7 la Configuración va por pestañas: `API | Apariencia | Permisos | Proyectos` (columna izquierda + contenido). La pestaña Proyectos lista workspaces, borra proyectos (a papelera) y gestiona archivos subidos. Desde v0.7.1 hay apartado **Atajos** (lista de solo lectura: `Ctrl+N` nuevo chat, `Ctrl+1/2/3` modos, `Ctrl+O` abrir proyecto, `Ctrl+,` configuración, `Esc` cerrar).

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
*   Opciones de propiedad del código para el STACK (licencia/consentimiento).
*   Control de datos: archivos, proyectos, caché y contexto (ver/exportar/limpiar).
*   Especificaciones claras en cada límite (no solo historial del modelo), con **tooltip flotante** al pasar el cursor (ya iniciado en Límites).
*   Config con sidebar de pestañas más ancha y bloques pequeños lado a lado (iniciado).
*   **Guardar modelos/perfiles:** además de un modelo por provider, guardar varios modelos con su propia API key, `base_url` y provider, y alternar entre ellos rápidamente.
*   **Lector de Markdown del chat:** verificar cobertura de sintaxis completa (tablas, listas anidadas, blockquotes, checkboxes, imágenes, HTML, LaTeX, código con lenguaje) y mejorar el aspecto visual (tipografía, espaciado, bloques de código, enlaces, tablas).
*   **Repos guía:** usar los repositorios de **opencode**, **OpenHands** y **SWE-agent** como referencia para el orquestador, tools, prompts y flujo de agente.
*   **Sesiones de chat:** separar los chats en **sesiones** con un id estable cuando el provider lo permita (OpenRouter, OpenAI, Anthropic, Groq…), para que lo detecten y mejoren caché/coste y trazabilidad (revisar doc de cada provider).

## 6. Flujo de Creación de Proyecto

```
Home (Crear/Abrir) → Cuestionario inmediato → ESPEC.md → Chat (v0.1-v0.4) → Agente simple (v0.3+) → Orquestador (v0.6) → STACK (v0.9/v1.0)
```

## 7. Cuestionario

### 7.1 v0.5 — Simple (único nivel)

Texto libre: `nombre, descripción, ubicación en equipo (path), objetivo`.
Opción múltiple: `público_objetivo[dev-indie, pequeño-equipo, empresa, estudiantes, otro]`, `interfaz[nativa, web, CLI, móvil, backend]`.
Salida: `{workspace}/CONTEXT/ESPEC.md` vía plantilla `minijinja`.

### 7.2 v0.8 — Tres niveles

*   **Principiante:** diseño y decisiones de proyecto, no código (problema, usuario, pantallas clave, prioridades).
*   **Intermedio:** stack tecnológico y alcance (lenguaje, web/nativo/CLI, BD, MVP, auth).
*   **Avanzado:** alcance, escalabilidad y arquitectura (módulos, escalado, multi-dispositivo, seguridad, CI/CD).

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

## 9. STACK

### 9.1 STACK Local (v0.9)

*   SQLite + FTS5: `stack_items(id, title, code, tags, metadata_json, rating)`.
*   Metadatos: dependencias, versiones, ejecuciones, bugs, opiniones.
*   Legal desde el día 1 (v0.9 Track B): `author + license (MIT/Apache-2.0/Uso interno) + source`; consentimiento triple default OFF (`use_stack/share_local/share_cloud`); identidad local (nombre+email) para firmar.
*   Funciones completas: etiquetas, búsqueda con ranking, metadatos, `save/search/get/rate`.

### 9.2 STACK Nube (v1.0)

*   Cliente `stack_cloud.rs`: `push/pull/search_cloud` contra backend Axum+Postgres+S3 (o mock `~/.arqhia/cloud/` si no hay infra). Sync + fallback offline.

> El agente consulta el STACK en cada tarea (desde v0.9): `Revisa CONTEXT → Revisa código → Consulta STACK → Diseña tareas → Ejecuta`.

## 10. Resumen de Versiones (v0.1 → v1.0)

| Versión | Foco | Entregable clave |
|---|---|---|
| **v0.1** | Chat normal multi-provider | Iced shell + Chat + Config API nativa por provider, sin agente |
| **v0.2** | Sidebar + temas + proyectos | Múltiples chats, left sidebar 260px, Dark/Light, proyectos como agrupadores |
| **v0.3** | Agente simple | Workspace asignado, CRUD archivos, bash simple con allowlist |
| **v0.4** | Home page | Vistas Home/Chat/Config, Crear/Abrir proyecto, Config abajo |
| **v0.5** | Cuestionario simple | Wizard 6 preguntas → ESPEC.md |
| **v0.6** | Orquestador + permisos | Permisos modal, +tools, planner + 2 generadores + auditor |
| **v0.7** | Polish + seguridad | Tabs, uploads, permisos ampliados (todo peligroso OFF), límites, apariencia, POLICIES |
| **v0.7.1** | Agente eficiente + Modos | Matches/search v2, read paginado, outline, modos Chat/Plan/Work, atajos, presupuesto + badge tokens |
| **v0.8** | Cuestionario 3 niveles + modos | Principiante/Intermedio/Avanzado + Chat/Plan/Work + PLAN.md + atajos |
| **v0.9** | STACK local + legal + instalador | Tags + FTS5 + author/license/consent + identidad local + .deb + CI mínimo |
| **v1.0** | STACK nube + release | Push/pull/sync + auth + Windows + backup/export + release estable |
| **v1.1** | Pro + macOS + licencias | Multi-agent paralelo + sandboxes + merge + agentes/flujos custom + planes Trial/$5/$12/$20 + `.dmg` Apple Silicon |

Detalle paso a paso por versión en `ROADMAP.md` y `VERSIONS/v0.x.md`.

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
*   Licencias offline `ed25519` en `~/.config/arqhia/license.toml` (600),
    pago vía proveedor externo, sin backend propio ni telemetría de código.

## 12. Convenciones

*   **Idioma:** Español para docs de producto, inglés para código y commits.
*   **Target OS:** Linux prioritario.
*   **Versionado:** SemVer `v0.x` → `v1.0`.
*   **Iced:** 0.13, `Theme::Dark` por defecto.

## 13. Próximo Paso

Implementar v0.1 en `ARQHIA_P/`: shell Iced + `config.rs` + `llm/` nativo + chat con streaming. Ver `HECHO.md` en raíz para estado actual.
