# HECHO

- Plan redefinido a Iced 0.13 nativo (sin Tauri) para v0.1 → v1.0.
- `CONTEXT/PROJECT.md` actualizado: stack Iced 0.13, providers nativos, nuevo resumen de versiones.
- `CONTEXT/ROADMAP.md` y `CONTEXT/VERSIONS.md` reescritos con nuevo orden (chat → sidebar → agente simple → home → cuestionario → orquestador → polish → niveles → STACK local → nube).
- `CONTEXT/VERSIONS/v0.1.md` a `v1.0.md` reescritos con pasos concretos (widgets, structs, archivos, tablas, criterios Done).
- Carpeta `ARQHIA_P/` creada con `cargo init --bin`, dependencias Iced 0.13 + Tokio + reqwest + rusqlite + rfd + minijinja, esqueleto `main.rs` + stubs `config/chat/db/sidebar/workspace/llm/agent/questionnaire/stack/views/ui`.
- v0.1 implementada: `config.rs` (Provider OpenAI/Anthropic/OpenRouter + toml en `~/.config/arqhia/config.toml`), `db.rs` (SQLite `~/.local/share/arqhia/arqhia.db` tabla `messages`), `llm/{mod,openai,anthropic,openrouter}.rs` nativos con streaming SSE (`parse_openai_chunk`, `parse_anthropic_chunk`), `main.rs` con vistas Chat/Config, `Task::stream(channel)` que emite `StreamChunk/StreamDone/StreamError`, pantalla Config con pick_list + Guardar + Probar conexión. `cargo check` limpio, `cargo test` 9/9 OK, `cargo clippy` solo warnings de estilo.
- Fix test conexión: `reqwest` ahora con timeout 20s (`llm::http_client`) en los 3 providers para no quedarse en "Probando..." infinito; botón muestra `⏳ Probando...` y se bloquea mientras prueba; valida key vacía antes de lanzar.
- Fix base_url Groq: `normalize_base_url()` quita sufijos pegados (`/v1/models`, `/v1/chat/completions`, etc.); config corregida a `https://api.groq.com/openai`; verificado `GET /models -> 200` con la key guardada. Binario reconstruido.
- Fix crítico GUI sin red: Iced 0.13 no traía executor tokio → `reqwest`/`tokio::spawn` morían en silencio (botones muertos). `Cargo.toml`: `iced = { version="0.13", features=["tokio"] }`, binario reconstruido. Probe headless confirmó que la key guardada actual da `401 Invalid API Key` → hay que generar una nueva.
- v0.1 probada en GUI por el usuario: chat funciona. v0.1 🟢 Done.
- v0.2 probada: sidebar + temas + proyectos OK. v0.2 🟢 Done. Botón `+` por proyecto para crear chats dentro.
- v0.3 implementada: `projects.path` (migración SQLite) + `workspace.rs` (validate/list_top/context_block); `agent/tools.rs` con 6 tools (guard léxico anti-`../`, allowlist bash, timeout 30s, truncado 4000); `agent/mod.rs` loop function-calling OpenAI-compat + Anthropic (máx 5 iters, system prompt con top-20 archivos); `main.rs` con campo workspace por proyecto (Asignar/Quitar), modo agente automático cuando el chat tiene workspace, Log inferior 130px, placeholder "🤖 ejecutando tools...". E2E real contra Groq OK: "crea hola.txt" → `🔧 write_file → ✅ write hola.txt (11 bytes)` en 1s. `cargo test` 17/17 OK (+1 E2E ignored).
- CONTEXT ampliado (v0.3+): selector de modelo en la entrada del chat (`pick_list` con providers configurados, cambio instantáneo + persistencia), respuestas IA con Markdown (`iced::widget::markdown`, feature `markdown`, links vía `xdg-open`, mensajes de usuario en plano), Log movido debajo de la entrada del chat (últimas 8 líneas). `cargo check/test` OK, binario reconstruido.
- Límite del agente 5→10: el tope existe para evitar loops infinitos y coste descontrolado (`MAX_ITERS` en `agent/mod.rs`); el turno termina antes si el modelo responde sin tools. Ahora con progreso `🔧 [paso N/10]` en el Log y mensaje final que invita a continuar. Documentado en `CONTEXT/VERSIONS/v0.3.md`.
- Duplicados: `db::project_name_exists` (insensible a mayúsculas) bloquea crear/Abrir-manual con nombre repetido ("Ya existe un proyecto llamado 'X'"); Abrir carpeta con nombre en choque auto-sufija ("X (2)") vía `db::free_project_name`. Tus dos `ARQHIA` existentes se quedan como están. `cargo test` 19/19 OK.
- `cargo build` y `cargo test` añadidos a la allowlist del agente (`tools::is_allowed`, con test de política). Bloqueados siguen `cargo publish/install/login`, `rm`, `curl`, etc. System prompt, schemas y CONTEXT actualizados.
- v0.2 implementada: `db.rs` con tablas `chats`/`projects` + `messages.chat_id` y migración v0.1→v0.2 (11 mensajes movidos a "General", 0 huérfanos); `config.rs` con `ThemeMode{Dark,Light}` y `#[serde(default)]` para no romper configs v0.1; `main.rs` con layout `sidebar 260px + chat`, `+ Nuevo chat`, borrado con confirmación inline, proyectos como agrupadores (crear/borrar/asignar vía pick_list), auto-título 30 chars, tema Dark/Light en vivo en Config. `cargo check` limpio, `cargo test` 12/12 OK.

- v0.3 probada por el usuario. v0.3 🟢 Done.
- v0.4 probada por el usuario. v0.4 🟢 Done. Extra: chats perezosos (Crear/Abrir navegan al Chat vacío con `pending_project`; el chat nace al enviar el 1er mensaje, sin "Nuevo chat" fantasma; borrar el último deja vacío) + botón "Ir al chat →" en Home (el `⌂` ya existía en el Chat).
- v0.5 probada. v0.5 🟢 Done.
- Fixes: sin chat "General" auto al entrar (init solo lo crea si hay huérfanos v0.1; arranque puede empezar vacío); divisores del sidebar condicionales; crear en sidebar con carpeta (default `~/ARQHIA/projects/{slug}` o escrita, helper compartido `create_project_with_dir`, abre cuestionario igual que Home).
- v0.6 implementada: `Permissions{auto_read,auto_write,auto_bash}` + checkboxes en Config; panel `🔐 Permitir/Denegar` por lote; `search_files` + `ToolCat`; orquestador planner→workers→auditor→TEMP.md con checklist `☑/▶/☐`, `AGENTS.md` auto, 1 pasada de fixes (workers secuenciales, paralelo pendiente). E2E Groq OK (2 tareas, 2 archivos, auditor SIN ISSUES). `cargo check` sin warnings, `cargo test` 27/27 OK.
- Provider Local (LM Studio): 4º provider OpenAI-compatible (`llm/local.rs`), default `http://localhost:1234`, sin key obligatoria (`requires_key`/`is_configured_for`), modelo = id exacto cargado en LM Studio, `Probar conexión` sin auth lista modelos, hints propios en Config, visible siempre en el selector rápido. `normalize_base_url` ahora también recorta `/v1` y `/api/v1` pegados. `cargo test` 28/28 OK.
- Cierre v0.6: ESPEC.md inyectado en planner + workers (~2000 chars); retry sin tools ante 400 con "tool" (modelos locales); botón `⏹ Detener` con generaciones (ignora resultados tardíos); higiene (`orchestrator.rs` stub y handler `AgentDone` eliminados); papelera `~/.local/share/arqhia/papelera/` al borrar proyecto (con fallback copiar+borrar); `Denegar y no preguntar más` con memoria por turno; visor `📝 TEMP` en el chat. Tests: inyección ESPEC, papelera mueve archivos, `cargo test` 31/31 OK, `cargo check` sin warnings, binario reconstruido.
- v0.6 probada por el usuario. v0.6 🟢 Done.
- v0.7 🟢 Done (probada y revisada): CONTEXT reescrito (config por pestañas, uploads, borrado); Config con columna `API|Apariencia|Permisos|Proyectos`; pestaña Proyectos con borrado a papelera (reusa `remove_project_everywhere`) y lista `uploads/` con borrado individual (guard anti-escape); botón `Subir` por proyecto (`rfd::pick_files` en `spawn_blocking` → `{ws}/uploads/`, tope 50 MB, `nombre (2).ext`); `workspace::{upload_files,list_uploads,delete_upload}` + tests. `cargo test` 32/32 OK, sin warnings.
- Fix crash pestaña Proyectos: `scrollable(list).height(Fill)` anidado dentro del scrollable de Config (panic "must not fill its vertical scrolling axis"). El tab devuelve la columna y el scroll lo pone el contenedor. Test headless `config_tabs_build_without_panic` que construye las 4 pestañas. `cargo test` 33/33 OK.
- Estilo general: `⏻ Salir` al fondo del sidebar (`iced::exit()`); botones compactos en toda la app (helpers `icon_btn/primary_btn/danger_btn`, padding 4–14); burbujas de chat, tarjetas de proyecto, modal de permisos y paneles con `rounded_box`; CTAs en primary, borrados en danger; pickers con ancho fijo anti-desborde; `cargo clippy` a cero warnings (`--fix` + ajustes manuales). `cargo test` 33/33 OK, binario reconstruido.
- System prompt (`llm::system_identity`): cada conversación arranca con identidad de ARQHIA, capacidades (con/sin tools) y reglas (idioma, Markdown, modelo en uso). Nuevo `Role::System`; Anthropic lo manda en su parámetro `system`, el resto como mensaje `system`. No se persiste en DB. `cargo test` 35/35 OK.
- Fix Volver: Config guarda `config_from` y `← Volver` regresa al origen (Chat o Home), ya no siempre a Home.
- Flujo: el cuestionario abre inmediatamente al crear proyecto (Crear o Abrir con ruta nueva, nombre prellenado); Abrir ruta ya registrada va directo al Chat. CONTEXT (§6, v0.4, v0.5) actualizado.
- Pulido UI (sin versiones en textos): título "ARQHIA", subtítulo `provider · modelo`, sidebar con secciones ("Conversaciones", nombre + contador por proyecto), workspace legible, estados vacíos amables, mensajes con más aire y etiquetas en gris, header con "⌂ Inicio / 📋 Cuestionario / ⚙", Home con tagline y consejo, footer ESPEC sin versión. `cargo check/test` OK.
- Sidebar v2: secciones `Chats` y `Proyectos` separadas con regla, proyectos plegables (`▸/▾` por proyecto), crear proyecto inline con `+` en la cabecera (adiós formulario fijo abajo; el click derecho no existe en Iced, el `+` es su equivalente), iconos/botones compactos, entrada con más aire respecto al selector de modelo. Borrar proyecto pide confirmación y elimina chats + carpeta del disco (avisa en el Log si la carpeta no se pudo borrar). `cargo test` 22/22 OK.

# Siguiente paso

- **v0.7, v0.7.1 y v0.7.2 marcadas 🟢 Done.** Se dan por cerradas: tabs/uploads/permisos/límites/apariencia/POLICIES (v0.7), agente eficiente + Modos Chat/Plan/Work (v0.7.1) y Git nativo + puerta de calidad (v0.7.2).
- Siguiente: **v0.7.3 — Bucle de estabilidad** (spec en `CONTEXT/VERSIONS/v0.7.3.md`; ver "v0.7.3 — Bucle de estabilidad (diseño)" más abajo).

# UI polish + tokens (ronda actual)

- Iconos no fiables retirados/reemplazados por ASCII en sidebar, Home, feed y checklist. Arrows `→` → `->`.
- Chat: burbujas de usuario alineadas a la izquierda (ya no se salen), sin etiqueta ARQHIA por mensaje, sin modelo duplicado en header.
- Layout centrado (max 960px) + separador con sombra mensajes/composer.
- Uso de tokens: parseo real (OpenAI `include_usage`, Anthropic) con fallback; `in/out/coste` bajo cada mensaje, barra superior de contexto/%/gasto API y línea en el Log.
- Log 13px con botón `Ampliar/Reducir` (8↔40 líneas). `cargo test` 66/66, clippy 0 warnings.

# Models.dev + layout dock + contexto real

- `pricing.rs`: catálogo `models.dev/api.json` cacheado (precios input/output/cache, contexto, tools/reasoning). Navegador de modelos en Config → API con búsqueda, precios y badges; exige API key del proveedor (Local usa `/v1/models`).
- Coste por mensaje y ventana de contexto usan el catálogo (fallback `llm::estimate_cost_usd`/`llm::context_window`).
- `Usage.cached` + parseo de cache (OpenAI/Anthropic); `in/out/cache` bajo cada mensaje.
- Contexto de la barra superior incluye tools/lecturas (`App.context_tokens`).
- Layout: mensajes centrados 880px; cabecera/stats/dock a todo el ancho; dock con fondo + sombra; entrada 16px; log 14px ampliable (12↔40, 170↔460). `cargo test` 70/70, clippy 0 warnings.

# Ajustes UI (chat ancho, filtros de modelo, densidad)

- Chat más ancho (1080px).
- Navegador de modelos: filtro de precio, `solo tools`, `ordenar por precio`, contador de resultados y panel de 900px.
- Densidad ahora aplica de verdad (`design::gap/pad`): chat, burbujas, dock, sidebar y cards de Config.
- Peso release: ~33 MB sin strip / ~23 MB stripped; 12.244 líneas Rust; 14 deps directas.

# Chat ancho + composer en dos filas + limpieza de iconos

- Chat 1400px de ancho de lectura; ventana inicial 1560×880.
- Composer: caja de entrada sola arriba, modos/modelo/datos justo debajo.
- Limpieza: fuera botón Cuestionario (header), botón TEMP (log) y badge de modo (header). Se eliminó el visor TEMP y su `Message::ToggleTemp`.
- `Salir` en sidebar en rojo outline (`design::danger_outline`). `cargo test` 70/70, clippy 0 warnings.

# Nivel de razonamiento por modelo + ancho visible

- `pricing.rs` parsea `reasoning_options` de models.dev (effort/toggle) y expone `effort_choices`.
- Selector de nivel en Config → API y en el composer; persiste en `ProviderConfig.reasoning_effort`; se envía al provider.
- Burbujas 1180/1000px y sidebar 248px para que el chat se vea más ancho. `cargo test` 70/70, clippy 0 warnings.

# Fixes ronda UI

- Chat a todo el ancho (el centrado con spacers lo dejaba en tercios); burbujas `width(Fill)`.
- Log oculto en modo Chat (solo Plan/Work).
- Selector de nivel de razonamiento siempre visible; catálogo models.dev se autodescarga al arrancar si falta.
- Coste: usa `usage.cost` reportado por el proveedor (OpenRouter) antes que la estimación.
- Contexto superior: `264.000 tokens / 26% used` (miles con punto). `cargo test` 70/70, clippy 0 warnings.

# Ajuste fino UI

- Chat centrado con ancho por densidad: cómoda 1020px / compacta 860px (~20% menos).
- Texto de mensajes (usuario y modelo) unificado a 14px.
- Pickers de modelo/razonamiento con borde de acento; razonamiento más estrecho; sin mensaje al cambiar nivel.
- Quitado el botón `"<"` del header del chat (Inicio ya está en el sidebar). `cargo test` 70/70, clippy 0 warnings.

# Config/contexto + backlog

- Catálogo por provider (`lookup_in`) para niveles/contexto/coste correctos; etiqueta "Nivel" en el composer.
- Aviso al ≥90% de contexto (continúa olvidando lo más antiguo, no se detiene); sin `—` al final de filas del navegador.
- Config: tabs 220px, cards lado a lado, tooltips en Límites.
- Backlog Config/UI (v0.8–v1.0) en ROADMAP/PROJECT. `cargo test` 70/70, clippy 0 warnings.

# Backlog añadido

- Guardar varios modelos/perfiles (API key + base_url + provider propios) y mejora/revisión del lector de Markdown (cobertura de sintaxis + aspecto visual), en `CONTEXT/ROADMAP.md §6` y `CONTEXT/PROJECT.md §5.4`.
- Repos guía (opencode, OpenHands, SWE-agent) y división de chats en sesiones con id por provider, en `CONTEXT/ROADMAP.md §6` y `CONTEXT/PROJECT.md §5.4`.

# Refactor arquitectónico (sin cambio de versión)

- `main.rs` 3750 → ~150 líneas: solo shell (`update`/`view` guards, `view_main`, `main`, tests de papelera).
- Nuevo `src/app/`: `events.rs` (Message/View/ConfigTab), `state.rs` (App + carga inicial), `projects.rs` (crear/papelera/borrado), `orchestrator.rs` (driver planner→workers→auditor), `handlers/{navigation,chat,projects,agent,config,questionnaire}.rs` (brazos de update; dispatch total verificado por el compilador + test `dispatch_routes_every_group`).
- Nuevo `src/views/{sidebar,chat,questionnaire,config_view}.rs` (render puro; `views::app_theme` compartido). Botones a `ui/components.rs`. Stubs muertos eliminados (`chat.rs`, `stack.rs`, `views/config.rs`, `views/stack.rs`, `ui/markdown.rs`).
- DB: `chats.archived` con migración + `set_archived`; tests de vistas movidos a `views/tests`; `friendly_error` para 429/401/402/404/timeout.
- Producción sin `unwrap`/`expect` (quedaban solo en tests + 1 en `worker_seed`, eliminado). Sin nuevas dependencias, sin cambios de comportamiento. `cargo test` 40/40, `cargo clippy --all-targets` cero warnings.
- v0.7.1 implementada, probada en GUI y revisada 🟢 Done: modos `Chat/Plan/Work` por chat (`db::Mode`, migración `chats.mode`, default Chat, badge en header + segmento en composer, `ModePicked` persiste); Plan = 1 llamada sin tools → `CONTEXT/PLAN.md` + checklist + `Ejecutar plan` (pasa a Work); Work = orquestador (Chat sin workspace = directo); atajos `Ctrl+N/1/2/3/O/,` + `Esc` (suscripción teclado, sin robar input) + pestaña Atajos; `search_files` v2 (bloques con 3 líneas ctx, máx 20, ranking nombre>contenido, ignora `target/.git/node_modules/*.lock`); `read_file` paginado (`offset/limit`, `líneas X–Y de Z`); `get_file_outline` (firmas); ventana historial configurable (def 20, con marcador) en chat plano y workers; ESPEC/AGENTS una vez como mensaje de contexto (system lean); colapso progresivo de outputs (últimos 4) + al 80% del presupuesto; caché de lecturas por turno (`📦`); parada temprana (calls idénticas 2×); presupuesto `max_tokens_turn` (0 = ilimitado) + badge `🪙` reescrito en Log (estimador `chars/4` ±30%); `busy_timeout` 5s en SQLite (adiós flake de tests paralelos). `cargo test` 64/64 OK, `cargo clippy --all-targets` cero warnings, binario reconstruido.

# v0.7.2 — Git nativo + puerta de calidad (🟢 Done)

- Contexto escrito: `CONTEXT/VERSIONS/v0.7.2.md` (spec completa), `VERSIONS.md` (🟢), `ROADMAP.md` (tabla + stack), `PROJECT.md` (§3.2, §4, §5.2, §8.7, §10, §13).
- Decisiones: repo por workspace con rama de trabajo fija `ARQHIA` (base `main` protegida); autonomía por defecto `CommitLocal`; push a GitHub **OFF + aprobación**; auto-commit tras cada tarea exitosa.
- Implementado:
  - `config.rs`: `GitConfig` (+ `BranchMode`, `GitAutonomy`), defaults (`enabled`/`auto_init` true, `base_branch` main, `work_branch` ARQHIA, `protected` main/master) y `validated()` (recorta ramas/remoto, evita `work_branch` protegida). Migración por `#[serde(default)]`.
  - `git.rs` (nuevo): `is_repo`, `current_branch`, `remotes`, `status_short`, `is_clean`, `init_repo`, `ensure_work_branch`, `commit_all` (guarda anti-sucio), `push` (async, `GIT_TERMINAL_PROMPT=0` + timeout 60s), `diff_stat`, `workspace_status`. 3 tests con repo temporal.
  - `agent/tools.rs`: `classify_git` (Read/Write/Net/Blocked), `GitKind`, categorías `ToolCat::Git`/`GitPush`, `ExecPolicy` con autonomía/push/protegidas, allowlist `cargo clippy`/`cargo fmt` + `git`, bloqueo de `push --force`/`reset --hard`/`clean`/`rebase`/`config`/`remote add-remove`/`-C`, commit sobre rama protegida. Tests de política.
  - UI: pestaña `ConfigTab::Git` con 3 cards (Repositorio, Autonomía, Estado del workspace) + `GitInitWorkspace` async; mensajes `Git*`, staging en `state.rs`.
  - `app/projects.rs` + `handlers/projects.rs`: `init_workspace_git` al crear/abrir (init + rama ARQHIA).
  - `orchestrator.rs`: `prepare_git_turn` (rama + árbol limpio) y `close_git_turn` (auto-commit `ARQHIA: <resumen>` solo si verde y árbol limpio; push si `CommitAndPush && push_enabled`).
  - Auditor: `cargo check` + `cargo test` + `cargo clippy --all-targets` (timeout 600s), `TEMP.md` con secciones Verificación/Estado, `VERIFY: OK|FAIL`; auto-commit solo con `git_verify_ok` + `git diff --stat`.
- `cargo test` 78/78 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.

# v0.7.3 — Bucle de estabilidad (🟢 Done)

- Implementado:
  - `config.rs`: `Limits.max_fix_cycles` (0 = ilimitado, default; clamp 1–20 si >0) + `unlimited_fix_cycles()`.
  - `agent/mod.rs`: `read_context_docs` (PROJECT/SPECS/ESPEC/CONTEXT/VERSIONS/ROADMAP/PLAN/TEMP con tope 1500), `code_outlines` (firmas `tools::outline_of`, máx 12 archivos, ignora target/.git/node_modules), `analyze_workspace` (1 llamada sin tools → brief) y `read_analysis_md`.
  - `plan_tasks` recibe `brief: Option<&str>`; `worker_context_block` inyecta `CONTEXT/ANALYSIS.md` una vez por worker.
  - `agent/tools.rs`: `outline_of` reutilizable.
  - `handlers/chat.rs`: Plan y Work llaman al analista antes del planner (cadena `AgentAnalyze`); fallback a `context_block` si falla; guarda `CONTEXT/ANALYSIS.md`.
  - `handlers/agent.rs`: loop `auditor → fix` sin tope (salvo `max_fix_cycles`), `FixDecision` puro (`Clean`/`Fix`/`CapReached`), log `↻ ciclo N: M issues` y `✅ estable tras N ciclos`; al tope con issues: sin commit.
  - Auditor: `VERDICT: CLEAN|ISSUES` (reseña LLM + check/test/clippy) y contador de ciclo en `TEMP.md`; si el LLM cae, no bloquea (manda la puerta de calidad).
  - `orchestrator.rs`: `git_turn_interrupted` (presupuesto/parada) bloquea el commit; commit solo con turno verde.
  - UI: límite `Ciclos de fix` en Config → Permisos.
- `cargo test` 81/81 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.

# Fix navegación de Config (sin cambio de versión)

- `Esc` en Config ahora vuelve al origen (misma acción que `Volver`), además de cerrar menús/paneles (`CloseOverlays`).
- `Ctrl+,` estando ya en Config no pisa el origen: reabrir `Ajustes` con el atajo no deja atrapado el botón `Volver`.
- Test `config_back_and_esc_return_to_origin` (Home→Config→Esc=Home; Chat→Config→Volver=Chat; Ctrl+, repetido no atrapa). `cargo test` 82/82 OK, clippy 0 warnings.

# Fix sidebar y confirmación de borrado (sin cambio de versión)

- Sidebar izquierdo 248 → **300 px** (un poco más ancho).
- Confirmación de borrar chat (`... → Borrar`): antes era una fila con texto + 2 botones que se aplastaba; ahora el texto va arriba y los botones (`Sí`/`No`) debajo, con `width(Fill)`. `cargo test` 82/82, clippy 0 warnings.
- Chat: padding derecho de 14px en el contenido del scrollable de mensajes para separar la barra de deslizamiento del texto (a la derecha).

# Fix "Corte de stream: error decoding response body" (sin cambio de versión)

- Causa: `http_client()` aplicaba un timeout total de 20 s también al streaming SSE; una generación larga (p. ej. cuento con OpenRouter/DeepSeek) se cortaba a mitad y reqwest lo reportaba como `Kind::Decode` ("error decoding response body").
- Arreglo:
  - `llm/mod.rs`: nuevo `http_stream_client()` (solo `connect_timeout` 15 s, sin timeout total) para los 4 `chat_stream`; `http_client()` one-shot sube a 120 s (cubre `agent::llm_step`/`simple_chat`/auditor/pricing).
  - `openai/openrouter/anthropic/local.rs`: cierre de stream tolerante: error antes de texto → error real; error después de texto → cierra como parcial con aviso `_(se cortó la conexión; respuesta parcial)_`.
  - `Cargo.toml`: reqwest con `gzip`, `brotli`, `deflate`, `zstd`.
  - `friendly_error`: mapea `error decoding response body` / `connection closed` / `unexpected eof` / `stream error` / `reset by peer` / `broken pipe` / `corte de stream` a un mensaje claro en español.
- `cargo test` 82/82 OK, `cargo clippy --all-targets` 0 warnings, `cargo build` OK.

# Detener agente + scrollbar (sin cambio de versión)

- Botón **Detener** que reemplaza a **Enviar** durante la ejecución (stream o agente), en el mismo sitio del composer (`views/chat.rs`).
- **Doble Esc** detiene el turno en curso (ventana 600 ms); un solo Esc sigue cerrando menús / saliendo de Configuración.
- `StopAgent` ahora corta también el stream de chat plano: nueva generación `stream_gen` en `Message::Stream{Chunk,Usage,Done,Error}` para ignorar chunks tardíos; conserva el texto parcial y añade `_(detenido por el usuario)_`.
- Scrollbar del chat: padding derecho 14 → **28 px** para separarlo más del texto.
- El `provider · modelo` activo se movió de la caja de entrada (composer) a la **cabecera** del chat, junto al proyecto y el nombre del chat.
- Atajos (Config): añadida fila `Doble Esc — Detener el turno en curso`.
- Test `double_esc_stops_running_turn`. `cargo test` 83/83 OK, clippy 0 warnings, `cargo build` OK.

# Roadmap: sesiones de chat asignadas a v0.8

- La feature de **sesiones** (id estable por chat reutilizado por el provider) deja de ser backlog sin versión y pasa a **v0.8**.
- Spec ampliada en `CONTEXT/VERSIONS/v0.8.md` (§E): `chats.session_id` + migración, `ChatMeta.session_id`, `llm::session_body_fields` (OpenRouter `session_id`, OpenAI/Groq `prompt_cache_key`, Anthropic header `x-session-id`), acción `Reiniciar sesión`, tests.
- Actualizados: `ROADMAP.md` (fila v0.8 + backlog §6), `PROJECT.md` (§5.4, §8.9, §10, §13), `VERSIONS.md`.

# Roadmap: nueva v0.7.4 (Chat UX + modelos con nombre)

- **Decisión:** crear versión intermedia **v0.7.4** para no sobrecargar v0.8 (que ya carga cuestionario + estructura + onboarding + sesiones).
- Contenido (§A/§B de `CONTEXT/VERSIONS/v0.7.4.md`):
  - **Perfiles de modelo con nombre visible** (provider + base_url + API key + model + nivel), selector por nombre en Config y composer; CRUD + migración.
  - **Utilidades de chat:** título por IA (fallback), undo (`Ctrl+Z`), copiar chat, nuevo chat desde un mensaje (bifurcar), fecha/hora por mensaje (`messages.created_at`), reintento con icono `↻`, y **citar fuentes** (`Fuentes` clicables) tras `fetch_url`.
- Docs: `VERSIONS/v0.7.4.md` (nueva), `VERSIONS.md`, `ROADMAP.md` (tabla + §6), `PROJECT.md` (§5.4, §8.10, §10, §13).
- Las **sesiones** siguen en v0.8 §E.

# Siguiente paso

- **v0.7, v0.7.1, v0.7.2 y v0.7.3 🟢 Done.** Flujo pre-STACK completo implementado; ARQHIA puede dogfoodear su propio código.
- Siguiente, en orden: **v0.7.4** (chat UX + perfiles de modelo) → **v0.8** (cuestionario + estructura + onboarding + sesiones) → **v0.9** (STACK local + `.deb` + CI) → **WEB** (dependencia) → **v1.0** → v1.1 → v1.2.

# v0.7.3 — Bucle de estabilidad (diseño)

Flujo pre-STACK acordado: `petición → orquestador → analista (contexto/specs/código)
→ planner → workers → auditor → TEMP → loop fix hasta verde → commit → push (opcional)`.

- Spec escrita: `CONTEXT/VERSIONS/v0.7.3.md`; fila en `VERSIONS.md` (🟢) y `ROADMAP.md`.
- Decisiones: commit **1 por turno con el loop verde**; análisis como **llamada
  dedicada** (brief `CONTEXT/ANALYSIS.md` consumido por el planner, con fallback);
  bucle **ilimitado hasta verde** (freno: `Detener`, `max_tokens_turn`, parada
  temprana; `Limits.max_fix_cycles` def 0 = ilimitado); push OFF + aprobación.
- v0.7.3 reubica el auto-commit de v0.7.2 (de "fin de turno" a "fin de bucle verde").
- Docs actualizados: `CONCEPTO.md` (flujo del agente), `PROJECT.md` (§6, §8.8, §10, §13),
  `ROADMAP.md` (stack + tabla), `VERSIONS.md`, `VERSIONS/v0.7.2.md` (nota).

# v0.7.4 — Chat UX + modelos con nombre (🟢 Done)

- `config.rs`: `ModelProfile{id,name,provider,base_url,api_key,model,reasoning_effort}` + `model_profiles/active_profile` + `ensure_profiles()` ("Perfil por defecto") + `apply_profile/sync_active_profile`; CRUD Guardar/Renombrar/Duplicar/Borrar/Activar; selector por nombre visible.
- `db.rs`: migración `messages.created_at` + `load_chat_history_full` + `copy_chat`/`branch_chat` transaccionales + `delete_last_messages`; test 3→copia 3→rama 2.
- Chat: timestamps `DD/MM HH:MM`, `Deshacer`(Ctrl+Z 1 paso, envío+borrado), `Copiar`, `Rama desde aquí`, `Título` (IA 1 llamada + fallback + Regenerar), reintento icono `↻`, bloque `Fuentes` clicables (fetch_url args + extract_urls, anexadas al cerrar turno).
- Fixes: perfil = unidad completa (provider+api+base+modelo+nivel); `Guardar` de provider ya no pisa perfiles, "Guardar como perfil" con mismo nombre actualiza; fecha/hora + "Rama desde aquí" al pie de cada burbuja; toolbar siempre visible `Deshacer (Ctrl+Z) | Copiar chat | Título por IA`; aire inferior 24px + separador 4px para que el composer no tape el último mensaje; Ctrl+Z en Atajos.
- Rediseño Config → API centrado en perfiles: formulario "Nuevo perfil" (nombre → provider → API key → URL base → modelo + Buscar → nivel → Guardar perfil/Probar); lista "Mis perfiles" con una caja por perfil (nombre + modelo, badge activo/provider, `···` con Borrar/Editar); overlay "Editar perfil" (nombre/provider/api/base/modelo+BUSCAR/nivel, Guardar cambios/Cancelar, Esc cierra). Buscador sin scroll horizontal (textos Fill + id truncado + padding 20px para la barra).
- Corrección chat: fuera botones superiores (Deshacer/Copiar/Título); título IA 100% automático y silencioso (fallback + 1 llamada, sin "generando título"); pie de cada mensaje con fecha + `Copiar` (portapapeles) + `···`; clic derecho o `···` abre `Deshacer hasta aquí` (aviso "Se borrará el resto. ¿Seguir?" Sí/No) y `Rama desde aquí`; Ctrl+Z recupera lo truncado (reinserta la cola en DB).
- `cargo test` 88/88 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.

# Refactor pre-v0.8 (debug + optimización + dedup)

- Helpers centrales en `App`: `pop_last_message()` (pops alineados) y `resync_msg_meta()` (relee ids/timestamps); eliminados `refresh_ids_times`/`now_ts` duplicados y el bloque inline de `finish_agent_answer`.
- `begin_analysis_turn()` unifica los arranques Plan/Work (eran ~50 líneas duplicadas); `spawn_chat()` unifica NewChat/NewChatInProject.
- Fuente única: `tools::default_ignores` → `config::default_ignores`; `agent::estimate_tokens` → `llm::estimate_tokens_text`; `short_preview` → `design::trunc_end`.
- Bugs: `agent::short` hacía slice por bytes (panic con UTF-8) → recorte por chars; Undo de envío borraba `added+1` filas fijas (podía llevarse mensajes viejos si el stream falló) → borrado por diferencia con `db::count_messages`; `ExecutePlan` y `Abrir proyecto` dejaban paralelos desalineados → pushes/clears completos; eliminado `sync_active_profile` muerto, shims `save_msg_legacy`/`load_chat_history`/`load_history` y const `UPLOAD_MAX_BYTES` sin uso (~80 líneas menos).
- `cargo clippy --all-targets` 0 warnings, `cargo test` 88/88 OK (3 ignorados), `cargo build` OK. 15.865 líneas Rust.

# Siguiente paso (actualizado)

- **v0.7.2, v0.7.3 y v0.7.4 🟢 Done + refactor verde.**
- **v0.8 + v0.8.1 implementadas, pendientes de prueba en GUI** (ver abajo). Sesiones verificadas por el usuario en OpenRouter ✅ (otros providers pendientes, sin cambios de código).
- Después: v0.9 (STACK + skills) → WEB → v1.0 → v1.1 → v1.2.

# v0.8.1 — Cuestionario profundo (implementada 🟡)

- `levels.rs`: `TipoProyecto` ×9 (Inter/Avanz), `Plataforma` ×7 múltiple, `StackOpt` ×12 múltiple, `ArqPreset` ×7 + detalle, `EstiloPreset` ×7 + referencia (Principiante), `Facturacion` ×6 (+Publicidad, +Freemium). Preguntas: Principiante 3 / Intermedio 5 / Avanzado 6; wizard 9/11/12 + IA.
- `Answers` multi (`plataformas`/`stacks: Vec` + `toggle_*`); vacío = "(sin especificar)", nunca bloquea. Mensajes nuevos (`QTipoPicked`, `QPlataformaToggled`, `QStackToggled`, `QEstiloPicked/Free`, `QArqPicked`); checkboxes en la vista.
- `templates.rs`: `## Tipo de proyecto`, plataforma/stack como listas, estilo = preset + referencia, arquitectura = preset + detalle; alcance con tipo + plataformas.
- Spec en `CONTEXT/VERSIONS/v0.8.1.md`; índices (`VERSIONS.md`, `ROADMAP.md`, `PROJECT.md` §7.2/§10).
- `cargo test` 104/104 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.
- Pendiente en GUI: Principiante "Blog" (estilo + 2 plataformas → SPECS sin arq) y Avanzado "API-bank" (tipo API/Backend, Rust+Python, Por capas, Freemium).

# v0.8 — fixes plegados (eran "v0.8.1", ahora parte de v0.8 🟡)

- **Post-cuestionario entra en modo Plan:** al terminar, abre chat "Plan inicial" en modo Plan con el prompt precargado (ROADMAP.md + VERSIONS.md + VERSIONS/v0.1.md + dependencias + ToDo; sin tocar código). El usuario revisa y pulsa Enviar; el turno NO se dispara solo.
- **Sin proyectos fantasma:** cancelar el cuestionario deshace la creación (fila DB + estado + carpeta solo si quedó vacía; con contenido del usuario la carpeta se conserva). Vuelve a Home si hubo rollback, al Chat si el proyecto era previo.
- **Flash del selector de modo:** al cambiar Chat/Plan/Work (click o Ctrl+1/2/3, mismo `Message::ModePicked`) el segmento se tiñe de acento y su padding pulsa 2→6→2 en ~8 ticks de 60 ms (`design::segmented_flash` + `ModeAnimTick` con generación anti-carreras).
- Tests: rollback (vacío se retira / con datos se conserva), Finish escribe docs rellenos + abre Plan sin disparar turno, animación por ticks y generación. `cargo test` 103/103 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Pendiente en GUI: terminar cuestionario → Plan precargado → Enviar genera ROADMAP/VERSIONS/ToDo; cancelar → sin proyecto; cambiar de modo → animación visible.

# v1.0.1 — MCP mínimo (spec 🟡)

- MCP tiene versión propia: `CONTEXT/VERSIONS/v1.0.1.md` (stdio + HTTP, `src/mcp.rs` sin crates nuevos, `[mcp]` en config, enrutado `mcp__{srv}__{tool}`, categoría Net/auto, npx pide Install, timeouts con kill).
- v1.1 suma §D MCP avanzado (pestaña, resources/prompts, item `MCP` en STACK, auth HTTP) + paso 4, archivos, Done y test 5; depende de v1.0.1.
- Índices: `VERSIONS.md`, `ROADMAP.md` (fila v1.0.1, v1.1 depende de v1.0.1), `PROJECT.md` (fila v1.0.1, v1.1 con MCP, `mcp.rs` en el árbol).

# Skills — spec añadida a v0.9 (Track C) y v1.0 (§G)

- `CONTEXT/VERSIONS/v0.9.md` Track C: skills locales (`SKILL.md` + recursos en `~/.local/share/arqhia/skills/`, embebidas `commit-msg` + `revisar-codigo`, `/skill nombre` que inyecta y sigue, pestaña Config → Skills, scripts bajo permiso Bash existente).
- `CONTEXT/VERSIONS/v1.0.md` §G: skills en la nube (etiqueta `SKILL`, instalar/publicar con consentimiento, `Uso interno` no sale, badge + botones, offline intacto) + paso 8, archivos, Done y test 5.
- Índices: `ROADMAP.md` (filas v0.9/v1.0), `PROJECT.md` (§9.3 + tabla §10).

# v0.8 — Cuestionario + estructura + onboarding + sesiones (implementada 🟡)

- `questionnaire/` reescrito: `levels.rs` (Nivel + genéricas 4 + propias por nivel: Principiante 3 / Intermedio 4 / Avanzado 5; wizard dinámico 9/10/11 pasos + paso IA final), `ai.rs` (prompt + parseo 3–5 preguntas, tope 5), `templates.rs` (PROJECT.md + SPECS.md con secciones por nivel + CONTEXT.md vía minijinja).
- Wizard con confirmación inline al cambiar de nivel con respuestas (2ª pulsación confirma y limpia el bloque de nivel); validación solo en genéricas + picks siempre válidos; IA saltable y deshabilitada sin API (no bloquea).
- `workspace.rs`: `ensure_project_layout` (Project/ + ToDo.md + CONTEXT/ + CONTEXT/VERSIONS/), `save_project_docs` (los 3 docs), `migrate_espec` (ESPEC→SPECS + legacy con nota, idempotente). El guard del agente ya cubría Project/CONTEXT/ToDo (todo dentro del workspace): sin cambios.
- Onboarding: Home muestra "Bienvenido → Configurar API / Configurar después" solo si ningún provider está listo (`AppConfig::has_any_api`, Local cuenta con solo modelo); "después" lo oculta en memoria, guardar la 1ª API lo quita del todo.
- Sesiones: `chats.session_id` + migración + `ensure_session_id` (estable, 1ª vez por chat) + `set_session_id`/`new_session_id`; copia conserva, rama estrena; `llm::session_body_fields` (OpenRouter `session_id`, OpenAI `prompt_cache_key`) + `session_header` (Anthropic `x-session-id`, Local nada); inyectado en los 4 `chat_stream` (firma con `session: Option<String>`); "Reiniciar sesión" en el menú ⋯ del chat activo.
- Agente: `read_espec_md` prefiere SPECS.md (fallback ESPEC legacy); `system_identity` y textos Home/wizard hablan de PROJECT.md + SPECS.md.
- `cargo test` 100/100 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Pendiente: prueba en GUI (los 6 pasos del Test de Funcionalidad de `CONTEXT/VERSIONS/v0.8.md`); al pasar, marcar v0.8 🟢 Done en `CONTEXT/VERSIONS.md`.
