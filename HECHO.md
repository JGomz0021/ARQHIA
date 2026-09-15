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

# v0.8.2 — Cuestionario reordenado + matriz profunda + pantalla de carga MVP (🟢 Done)

- `levels.rs`: orden `Nivel → Cat → Tipo → genéricas contextualizadas → familia → OSS → IA` (`generic_questions` por familia, `Q_CAT/Q_SYS` primero); Sistema partido (`LenguajeRuntime`: sintaxis+toolchain+arch+compat; `OS/Kernel/VM`: arch+arranque+syscalls+compat; resto sin syscalls); Librería partida (`PluginExtension`: host_api; `MotorEngine`: ui_ux+estilo+plataformas); `oss_questions()` (repo+gobierno+contrib) + `is_oss()`; `GENERIC` retirado; tests de orden y matriz.
- `mod.rs`: 7 campos nuevos (`sintaxis, toolchain, syscalls, host_api, oss_repo, oss_gobierno, oss_contrib`) + extras/contenido; test de pasos 1-2 picks / 3-6 genéricas.
- `events.rs`/`state.rs`/`main.rs`: `View::Generating` + `GenDone(run, files)`/`GenCancel` + `gen_*` (fase/progreso/error/run) + dispatch y vista.
- `templates.rs`: `PROJECT.md` con Tipo+Gaps; `SPECS.md` con Stack/Plataforma siempre + secciones por subtipo + `## Open source`; `or_dash` sin placeholders mudos (`Pendiente de definir: completar en Plan/MVP.`).
- `planning.rs`: `gaps()` con subtipos+OSS; `mvp_prompt/mvp_files/parse_mvp_files/write_mvp_docs/deterministic_mvp` (serie v0.1→v0.2→v0.3→v1.0 + ROADMAP/VERSIONS/ToDo, validación por versión con fallback+aviso); tests MVP.
- `handlers/questionnaire.rs`: Finish → Generating (1 llamada IA sin chat) u offline determinista sin API; GenDone escribe serie + abre `Plan inicial` con resumen; GenCancel invalida por run y vuelve al paso IA. Retirados `begin_plan_turn_auto` (chat.rs) y el turno Plan automático.
- `views/`: `view_generating` (fase+barra+Detener) + test sin panic.
- Docs: `CONTEXT/VERSIONS/v0.8.2.md` (nueva), `VERSIONS.md`, `ROADMAP.md`, `PROJECT.md` (§7.2, §10).
- Fix sin versión: el paso `nombre` se omite si ya se ingresó al crear/abrir (`Answers.nombre_locked`; la cabecera del wizard muestra el nombre). `cargo test` 161/161 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.

# Siguiente paso (actualizado)

- **v0.7.2, v0.7.3, v0.7.4, v0.8, v0.8.1, v0.9, v0.9.1 y v0.9.2 🟢 Done.** v0.9.2 verificada (152/152 tests, clippy 0); pendiente prueba en GUI (ver `VERSIONS/v0.9.2.md`).
- Tweak UX sin versión (2026-09-13): **skip cuestionario** (`Crear` vs `Sin cuestionario` en Home y sidebar + `ARQHIA_HOME` aislado para tests); **sidebar 330px (+10%)** + colapso chats >8 con `... ver más`; **`/ ` compacto** (1 línea, 160px, scroll vertical); limpieza de textos sin valor técnico (`Chat vacío`, `Sin workspace`, `Mensaje…`, `Contexto al 90%…`, `Define…`). Limpieza de 54 chats tmp/sess de la DB del usuario (2 vigentes, backup `arqhia-pre-cleanup-*.db.bak`); home aislado del agente en `/tmp/arqhia-agent-home` (`ARQHIA_HOME/ARQHIA_DB/ARQHIA_CONFIG`). 152/152 tests, clippy 0.
- Tweak UX+fix sin versión (2026-09-14): **bug mensaje inválido** (Plan sin workspace se guardaba y sumaba contexto → ahora se valida ANTES de persistir, input conservado, test `invalid_plan_message_never_reaches_db_or_context`); **sidebar Proyectos arriba** colapsable por sección + Conversaciones debajo; **`/ ` en ventana flotante** sobre los mensajes (`stack!` overlay, tarjeta ≤210px con scroll, composer intacto); **skill `revisar-codigo` retirada** (la cubre `code-review`; 4 embebidas); docs de **límites API por tiempo/gasto** en ROADMAP §6 + PROJECT §5.4. 153/153 tests, clippy 0.
- Tweak cuestionario+chat sin versión (2026-09-14): **facturación `Código abierto`** (7ª opción, SPECS `sin cobro; ver Licencia`); **Tab/Shift+Tab** entre respuestas IA (`qai-N` + `QAiFocusCycle`, solo en cuestionario); **`/ ` centrada** (tarjeta 560px, fuente 12–14) sobre los mensajes; **entrar al chat desde el final** (`snap_to` End al seleccionar/navegar). Tests `qai_tab_cycles_focus_with_wrap`, Tab en `keyboard_shortcuts`, `option_lists_are_broad`, `open_source_billing_renders_without_charge`. 155/155 tests, clippy 0.
- Después, en orden: v0.9.3 (MCP mínimo) → v0.9.4 (hardening crítico) → v0.9.5 (calidad estructural) → v0.9.6 (revisión + icono + instalador, puerta de v1.0) → WEB → v1.0 → v1.0.1 (MCP avanzado) → v1.1 → v1.2.

# v0.8.1 — Cuestionario universal (🟢 Done)

- `levels.rs`: `Categoria` ×6 + `SysType` ×29 (`for_cat` con cascada Cat→Tipo, `categoria()` inversa); `Licencia` ×4 (aviso GPL, `Uso interno` nunca a nube), `ApiStyle`/`AuthKind`/`Trigger`/`SemverOpt`, `ArchOpt` ×3 multi, `has_ui` (estilo solo con UI), `uses_facturacion` (App/Servicio; resto licencia), `default_plataformas` (API→Servidor/Nube, editable); `TipoProyecto` y `ArqPreset` retirados (`grep` limpio).
- Matriz condicional por familia: App (ui_ux + estilo si UI + plataformas + stack si no Principiante + facturación), Servicio (endpoints + api_style + auth + escala + stack + facturación), Librería (api Pública + semver + lenguaje target + ejemplos + licencia), Sistema (arch + arranque + compat + lenguaje implementación + licencia), Automatización (trigger + inputs + idempotencia + dónde + stack + licencia), Datos/IA (dataset + pipeline + modelo/eval + stack + licencia). Plataformas ×9 (escritorio desglosado en Windows/macOS/Linux).
- Wizard dinámico `total = 1 + gen(4) + cat + sys + N(familia) + 1(IA)`; al cambiar Cat se resetea Sys al primero + defaults.
- `import.rs` nuevo + `workspace::{is_nonempty_dir,scan_import}`: Abrir carpeta con código → modo Import (banner, prefill nombre/stack/descripción, gaps precargados, fallback estático sin provider, `import_gap_prompt` con IA); Finish hace merge sin borrar `Project/`; Cancel conserva carpeta con código.
- `templates.rs`: SPECS por familia + `## Tipo de sistema` (cat + tipo) + cierre facturación/licencia con avisos; CONTEXT anota origen (desde cero vs importado).
- `cargo test` 112/112 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Pendiente en GUI: Intermedio + Framework/Librería → API pública + semver + licencia sin plataformas de app; Avanzado + Driver/Firmware → arch + arranque + GPL con aviso sin facturación; Abrir carpeta Rust → banner import → docs sin borrar archivos; Crear desde cero → wizard sin banner.

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

# Reorg v0.8→v1.2 + icono pre-release (CONTEXT actualizado)

- v0.8 marcada 🟢 Done en `CONTEXT/VERSIONS.md` (+ ROADMAP/PROJECT/HECHO).
- v0.8.1 reescrita: taxonomía `Categoria`×6 + `SysType`~24 tras Nivel (retira `TipoProyecto`); preguntas por familia (App/Servicio/Librería/Sistema/Automatización/Datos-IA); dedup Tipo↔Plataforma; `Licencia` donde no aplica facturación; import auto al Abrir (scan IA + gaps + prefill, merge sin borrar). Spec en `CONTEXT/VERSIONS/v0.8.1.md`.
- v0.9 recortada: instalador movido a v0.9.6 (nota en Track B + Done/Test).
- Nuevas: `v0.9.1.md` (BETA 5 roles Orquestador/Analista/Planner Net+Read/Workers/Auditor + loop con re-análisis), `v0.9.2.md` (skills ui-ux/code-review/test-qa → `CONTEXT/*.md`, manual+sugerida), `v0.9.3.md` (MCP mínimo, ex-v1.0.1, bloquea v1.0), `v0.9.6.md` (revisión + backup obligatorio antes de tocar unwrap + refactor + optimización + tests + icono + instalador Linux + CI, puerta de v1.0).
- MCP movido: mínimo → v0.9.3, avanzado → `v1.0.1.md` reescrita (pestaña/resources/prompts/item MCP/auth); v1.1 limpia (sin MCP, nota Licencia Pro vs Licencia código).
- v1.0 exige v0.9.6 verde + icono + MCP mínimo; E2E v0.1→v0.9.6.
- Icono pre-release: `assets/icon.svg` + PNGs + cableado .desktop/ventana en v0.9.6 (ROADMAP §6 + v0.9.6 §E + v1.0 §H).
- Índices: `VERSIONS.md` (v0.8 🟢, v0.8.1 🟡 ampliada, v0.9.x nuevas, v1.0.1 avanzado), `ROADMAP.md` (tabla + §6), `PROJECT.md` (§3 árbol mcp, §7.2, §10, §13).

# Sugerencias `/skill` + STACK con llamada al agente (post-v0.9)

- Escribir `/` en el chat abre sugerencias con nombre + descripción + preview del contenido (máx 5, filtran por prefijo, con o sin verbo `skill`); clic completa a `/skill nombre ` y Enter lo envía (prefijo único como `/rev` se completa solo; ambiguo o vacío avisa con candidatas, sin turno). Placeholder del composer lo anuncia (`/ para skills`).
- STACK con 14 seeds (nuevos: `iced-task-perform`, `rusqlite-column-migration`, `toml-config-roundtrip`, `fts5-match-rank`); siembra por título (`seed_missing`, no duplica DBs viejas de 10).
- Llamada al agente: botón `Pedir al agente` en el preview deja `Usa el snippet #N «título» del STACK para: ` en el chat; nombrar el STACK en el pedido vale como consentimiento puntual (`mentions_stack`) aunque `use_stack` esté off (log `pedido explícito`).
- `cargo test` 142/142 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Fix de flake: `copy_chat`/`branch_chat` usan transacción IMMEDIATE (dos DEFERRED con SELECT+INSERT en tests paralelos se bloqueaban con `database is locked`; 20/20 verdes tras el cambio).
- Pendiente en GUI: caja de sugerencias al escribir `/`, flujo Pedir al agente → Work con hits.

# v0.9 — STACK local + legal + skills + contexto auto (🟢 Done)

- Track A (`src/stack/{mod,seed}.rs`, `db.rs::migrate_stack`, `views/stack.rs`, `handlers/stack.rs`, botón STACK en sidebar): `stack_items` (+author/license/source) + `stack_meta` + `stack_fts` (FTS5 verificado en `bundled`); `save/search/get/rate/record_execution/report_bug` con scoring `tag_match*2.0 + fts + rating*0.5` (tope 20, preview 200 chars); 10 seeds MIT (`seed_if_empty` idempotente); panel con buscar/tags/preview 15 líneas + meta + guardar/valorar/reportar/copiar a workspace; el planner consulta (`consult` + `keywords_from_request`) y loguea `STACK: N hits`.
- Track B (`config.rs::StackConsent/Identity`, `db.rs::usage_stats/usage_tools`): 3 switches OFF por defecto + persisten; identidad local con validación (`author_line`); `Uso interno` rechazado por `can_share_to_cloud`; sin `use_stack` el planner ni consulta (ni Log); sin `share_local` el worker lleva nota de no proponer guardar + UI bloquea; contadores turnos/tools (total + por categoría read/write/bash/install/net/git) + 429s (`is_rate_limit_error`) con panel en Config → STACK.
- Track C (`src/skills.rs` + `skills/embebidas/commit-msg.md` + `revisar-codigo.md`): `skills_dir/list/load/delete/parse_skill_message/context_block`, frontmatter exigido, tope 8 KB reales con aviso, slug anti-`../`; `/skill nombre [texto]` inyecta UNA vez como contexto y sigue el turno (inexistente = error + lista, sin turno); pestaña Skills con badge embebida/local + Recargar + borrado (embebida se restaura); scripts bajo permiso Bash existente.
- Track D (`questionnaire/planning.rs` + `templates.rs` criterios + `CONTEXT_T`): `plan_prompt` (convenciones + resumen + gaps explícitos), `validate_version_md` (8 secciones + ≥3 pasos + Done), `write_auto_docs` (ROADMAP + VERSIONS + v0.1 + ToDo, idempotente, 1 reintento); SPECS con criterios de aceptación por funcionalidad (verbos por familia, cero placeholders); Finish dispara el Plan solo (`begin_plan_turn_auto`, fallback a precargado sin API); `Detener` cancela sin escribir; commit solo verde (sin cambios).
- `cargo test` 137/137 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Pendiente en GUI: panel STACK end-to-end, `/skill` en chat real, wizard → plan auto visible, consentimiento + uso en Config → STACK.
- Docs: `VERSIONS.md` (v0.9 🟢), `VERSIONS/v0.9.md` (Estado Done), `PROJECT.md` (§3 árbol, §13 siguiente v0.9.1).

# v0.9.2 — Skills de dominio (🟢 Done)

- `src/skills/embebidas/{ui-ux,code-review,test-qa}.md` (nuevas): mismo formato Track C (frontmatter + cuerpo ≤8KB, MIT); cada una cierra escribiendo su reporte en `CONTEXT/` (`UI-REVIEW.md`, `CODE-REVIEW.md`, `QA-REPORT.md`) o devolviéndolo en el chat si no hay tools. Registro en `EMBEDDED` (5 embebidas; instalación/restaurado automáticos).
- `agent::read_context_docs` suma los 3 reportes (tope 1500 c/u) → el siguiente turno del Orquestador los incluye; `CONTEXT.md` generado los indexa.
- Auditor: tras ISSUES, Log con `Prueba /skill test-qa|code-review` (`roles::suggest_skill_for_issues`: VERIFY FAIL → test-qa, si no → code-review; pura + test). Sin auto-ejecución.
- `/skill ui-ux|code-review|test-qa` pasa por el parseo genérico (sin rama nueva); pestaña Skills las muestra como embebidas restaurables sin cambios.
- `cargo test` 152/152 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings, `cargo build` OK.
- Pendiente en GUI: `/skill ui-ux` → 3 mejoras; `code-review` con `unwrap` → 1 Alta con archivo:línea; `test-qa` tras romper test → FAIL + fix; auditor con ISSUES → sugerencia en Log.

# Renumerado v0.9.4–v0.9.6 (deploy al final)

- El deploy (revisión + icono + instalador, puerta de v1.0) estaba numerado v0.9.4 pero se ejecutaba último. Rotación para orden secuencial: **v0.9.4** = hardening crítico (antes v0.9.5), **v0.9.5** = calidad estructural (antes v0.9.6), **v0.9.6** = deploy/puerta de v1.0 (antes v0.9.4).
- Solo docs (sin código): `git mv` de los 3 archivos + títulos/dependencias internas + `VERSIONS.md`, `ROADMAP.md` (filas + deps + §6), `PROJECT.md` (§10, §13), `HECHO.md`, `v1.0.md`, `WEB.md`, `v0.9.md`, `v0.9.2.md`, `v1.1.md`.

# v0.9.1 — BETA agentes (🟢 Done)

- `agent/roles.rs` (nuevo): 5 roles explícitos (Orquestador/Analista/Planner/Worker/Auditor) con system prompt propio + `allows_tool` por rol (Planner solo Net+Read, Worker todo, Auditor solo Read, Analista/Orquestador nada) + `FixDecision` + `planner_urls_needing_permission` + `extra_paths_block` + `checklist_line`. Tests de roles.
- `agent/mod.rs`: `WTask.accept` (criterio de aceptación; el planner lo pide en el JSON) + `plan_tasks(..., extra_ctx)` con contexto Net+Read + `fetch_planner_net_block` (tope 8 KB por URL) + `exec_calls(..., role)` con gate por rol (denegado sin ejecutar) + prompts de Analista/Planner/Auditor desde `roles` + `AGENTS_TEMPLATE` con los 5 roles y el loop con re-análisis.
- `config.rs`: `Permissions.planner_net` (default OFF, `#[serde(default)]`, checkbox en Config → Permisos).
- Orquestador: `PendingPlanner`, `spawn_planner`/`resume_planner_with_net`, fases visibles `orquestador: fase X/5` + `checklist: ☑/▶/☐` en el Log, `PLAN.md` con criterios de aceptación.
- Loop con re-análisis: `AgentReanalyze` — ante ISSUES el analista revisa el TEMP.md (actualiza `ANALYSIS.md`) antes del worker de fixes; Aprobar/Denegar reanudan el planner pendiente (con red / sin red); `Detener` cancela planner y re-análisis pendientes sin commit.
- `cargo test` 149/149 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.
- Pendiente en GUI: "crea a.txt y b.txt" → CLEAN + commit; romper a.txt → re-análisis → CLEAN en ciclo 2; planner con Net OFF + URL → panel de permiso; `Detener` a mitad → sin commit.

# v0.9.3 — MCP mínimo (🟢 Done)

- `config.rs`: `McpTransport{Stdio,Http}` + `McpServerConfig{transport,command,args,url,auto,timeout_s}` + `McpConfig{servers}` con `validated()` (clamp 5–120) y `is_install()` (npx/uvx/npm/pip/curl → Install); `[mcp.servers.xxx]` en `AppConfig` con `#[serde(default)]` para migración; test `mcp_config_defaults_and_roundtrip`.
- `src/mcp.rs` (nuevo, sin crates nuevos): `McpTool{name,description,input_schema}` + `mcp_tool_name/parse_mcp_tool/is_mcp_tool` (`mcp__srv__tool`) + `tool_to_openai/anthropic_schema` (trunca >4KB con aviso) + stdio (spawn `initialize` + `tools/list` con cache 5min, `tools/call` por llamada, kill + timeout) + HTTP (POST `Accept: application/json, text/event-stream`, JSON o SSE) + `collect_mcp_tools` (no bloquea si cae) + `extract_json_from_sse`; solo `tools/*` (resources/prompts → "hasta v1.0.1"); tests con stub Python stdio y stub HTTP (list+call+cache+SSE+timeout).
- `agent/tools.rs`: `ExecPolicy.mcp_servers` + `policy_for_with_mcp` + `category_of_mcp_call` (Install si npx, Read si auto=true, Net resto) + `execute` enruta `mcp__*` a `mcp::call_*` (Install pide `allow_install`) + `mcp_openai/anthropic_schemas` + `openai/anthropic_schemas_with_mcp`; tests `mcp_category_and_schemas` + `mcp_e2e_via_execute_with_stub` (list → execute OK, npx sin Install → error).
- `agent/mod.rs`: `llm_step_openai/anthropic` anexan schemas MCP si hay servidores (collect con cache, fallback a nativos si cae); `AGENTS_TEMPLATE` menciona `mcp__*`.
- `app/orchestrator.rs`: `call_needs_approval` para MCP (Read auto, Install → auto_install, Net → pide) + `spawn_exec_calls` usa `policy_for_with_mcp`.
- `cargo test` 172/172 OK (3 ignorados), `cargo clippy --all-targets` 0 warnings.

# v0.9.4 — Hardening crítico (🟢 Done)

- `agent/tools.rs`: `resolve` anti-symlink (canonical + ancestro, externo → `⛔ Fuera del workspace (symlink)`); `search` salta symlinks fugados; `bash` sin shell (`split_argv` + `contains_shell_metachars`, `exec` directo por argv[0]); `fetch_url` anti-SSRF + redirects máx 3; tests de symlink/inyección/SSRF.
- `db.rs`: `PRAGMA foreign_keys=ON`, FKs en DDL nuevo, `idx_messages_chat/idx_chats_project/idx_projects_name_unique NOCASE` (dedup previo), `meta.schema_version='1'`, limpieza de huérfanos, `delete_project` = suelta (SET NULL), `add_column` tolerante a carreras.
- `stack/mod.rs::save` en tx IMMEDIATE.
- `paths.rs` nuevo (`directories::ProjectDirs` + `ARQHIA_HOME` override); migran `db/config/pricing/skills/papelera/projects_base` + `~/`; `config.save()` con `600` en Unix (test).
- `import.rs`: `has_build_rs` + aviso en banner + test.
- Tests aislados: `db::test_guard::with_test_db` (lock + tempdir); migrados `db/stack/skills/seed/handlers`; guarda `tests_do_not_touch_real_db`.
- `cargo test` 189/189 OK (3 ignorados) ×2 seguidas, `cargo clippy --all-targets` 0 warnings.
- Docs: `VERSIONS.md` v0.9.4 🟢, `VERSIONS/v0.9.4.md` (Estado Done), `PROJECT.md` (§3 `paths.rs`, §13 siguiente v0.9.5).

# Fix fantasmas de tests + borrado imborrable (sin cambio de versión)

- Causa: ~20 tests llamaban a `App::default()` (→ `db::init/connect`) sin
  guarda; con `ARQHIA_HOME` sin fijar caían en la DB real y dejaban filas
  (`qgen-tmp-cancel`, `qcancel-tmp-xyz`). Además `remove_project_everywhere`
  abortaba si la papelera fallaba (carpeta de `/tmp` ya borrada) y el
  fantasma no se podía borrar desde la UI.
- Arreglo:
  - `paths.rs`: en builds de test, sin override, todo va a
    `temp/arqhia-cargo-test-{pid}` (`data/config/projects_base/home_dir`);
    ningún test —con o sin guarda— toca el home real. Verificado: `cargo
    test` deja `projects` de la DB real intacta (3 filas antes y después).
  - `app/projects.rs`: si la carpeta no existe, la DB se borra igual
    (`sin carpeta`); `trash_dir` avisa `ya no existe`. Los 2 fantasmas
    actuales ya se pueden borrar desde la UI con este binario.
  - Tests: `ghost_project_without_folder_deletes_from_db` +
    `trash_missing_dir_reports_clearly`; `test_guard::lock()` compartido
    con el test de `paths`.
- `cargo test` 192/192 OK (3 ignorados) ×2, `cargo clippy --all-targets` 0 warnings.

# v0.9.5 — Calidad estructural (🟢 Done)

- `app/history.rs` (nuevo): `ChatHistory` puro con push/pop/clear/truncate
  atómicos + tests (10 ciclos undo/rama alineados); `App::history_*`
  delegan (push del turno, placeholder y ExecutePlan migrados).
- `orchestrator.rs`: `run_fix_cycle`/`count_temp_issues`/`reanalyze_prompt`/
  `fix_task_desc` puros + test CLEAN/Fix/CapReached; `handlers/agent.rs`
  solo despacha (TEMP rotativo con test: dos auditorías, solo queda la última).
- Split sin comportamiento: `handlers/chat.rs` → `chat_stream.rs` +
  `chat_history.rs`; `views/config_view.rs` → `config_api.rs` + `config_git.rs`.
  `dispatch_routes_every_group` verde.
- Git async: `workspace_status_async`/`commit_all_async`/`diff_stat_async`/
  `current_branch_async` (`tokio::process`); `GitStatusFetched`/`GitCommitDone`
  en background (cierre encadena push); auditor y policy usan async; sync
  redundantes eliminados. Test repo 1000 archivos async==sync acotado.
- Uso: `usage_stats_id`/`usage_tools_id` + backfill + vista compat
  `usage_stats_names`; `record_tool_call_cat` y `usage_bump` en tx (nombre+id).
- FS pesado a `spawn_blocking`: `code_outlines_async`,
  `read_context_docs_async`, `context_block_async` (analista con `join!`),
  `scan_import_async` + test de equivalencia.
- Docs: `README.md` útil (quickstart, rutas, `cargo test`), `ARQHIA_P/docs/`
  (ARCHITECTURE/CONFIG/SECURITY + 4 ADRs); `PROJECT.md §3` solo archivos
  reales (futuros a ROADMAP).
- `tracing` mínimo (panic guard + E2E a `info/warn`, subscriber en `main`);
  `tokio` slim (`rt-multi-thread, macros, fs, process, time, sync, io-*`).
- `cargo test` 200/200 OK (3 ignorados), `cargo clippy --all-targets` 0
  warnings, `cargo fmt --check` OK.
- Pendiente en GUI: Work con repo grande responde durante status/commit;
  `/skill code-review` cita ChatHistory/symlink (test de inyección).

# Siguiente paso (actualizado)

- **v0.9.5 🟢 Done.** Siguiente: **v0.9.6** (revisión + icono + instalador,
  puerta de v1.0) → WEB → v1.0 → v1.0.1 → v1.1 → v1.2.

# Rondas UI/Config pre-v0.9.6 (histórico movido desde TEMP)

- Layout: mensajes centrados (880px); cabecera/stats/dock a todo el ancho. Dock inferior (composer + log) con fondo elevado y sombra (`design::dock`). Entrada 16px/padding `[10,8]`; log 14px ampliable 12↔40 líneas y 170↔460px.
- Contexto de la barra superior incluye tools/lecturas (`App.context_tokens`), no solo mensajes.
- Tokens: `Usage { input, output, cached }`; parseo real OpenAI (`stream_options.include_usage`, `prompt_tokens_details.cached_tokens`) y Anthropic (`message_start`/`message_delta`). `in/out/cache` bajo cada mensaje y en el Log.
- Precios: `pricing.rs` descarga/cachea `models.dev/api.json` (input/output/cache write, contexto y capacidades). Navegador de modelos con filtro de precio, `solo tools`, orden por precio; mapeo por `base_url`.
- Densidad operativa (`design::gap/pad`); chat a 1400px en ventana 1560×880; composer reorganizado; limpieza de iconos (fuera `Cuestionario`/`TEMP`/badge de modo); `Salir` rojo outline.
- Nivel de razonamiento por modelo desde `models.dev` (`reasoning_options`): selector Config + composer; se envía al provider (`reasoning_effort` / `reasoning.effort` / `thinking.budget_tokens`).
- Lookup por provider en el catálogo (evita colisiones de contexto 1M); aviso ≥90% de contexto; Config con tabs 220px y cards lado a lado; tooltips en Límites.
- Backlog ampliado: varios modelos/perfiles; lector Markdown; repos guía (opencode/OpenHands/SWE-agent); sesiones con id por provider.
- Sugerencias `/skill` en composer (filtra por prefijo, `Enter` completa); 14 seeds de STACK con `mentions_stack` para la consulta puntual.

# v0.9.6 — Revisión + icono + instalador (🟢 Done, puerta de v1.0)

- **Gate backup/higiene:** rama `ARQHIA` al día con `origin/ARQHIA` (0/0);
  `rg 'unwrap\(\)|expect\('` productivo = 0 (los únicos hits son strings de
  seed del STACK), así que no se tocó ningún `unwrap` real: no hizo falta
  backup. `todo!`/`unimplemented!`/`panic!` productivos = 0.
- **Refactor/dedup:** ignores a fuente única (`config::IGNORE_DIRS` +
  `config::is_ignored_name`, que ahora también usa `workspace::scan_import`);
  `estimate_tokens`/`resync_msg_meta` ya eran de fuente única; `trunc*` se
  revisaron (semánticas distintas, todas UTF-8-safe). Test nuevo
  `config::tests::ignored_name_single_source`.
- **Optimización:** `[profile.release]` con `strip` + `lto="thin"` +
  `codegen-units=1` + `opt-level=3` (se mantiene `panic="unwind"` por el
  `catch_unwind` de `main.rs`). Release medido: **24.3 MB** (<30 MB). Arranque
  hasta el event loop: instantáneo (<2 s; GUI pendiente en máquina con display).
- **Icono:** `assets/icon.svg` (fuente propia) + `icon-{16,32,64,128,256}.png`
  + `icon.rgba` (embebido) generados por `packaging/make_icons.py`; ventana vía
  `iced::window::icon::from_rgba` (`main.rs::app_icon`) y Home con
  `ui/logo.rs` (canvas, misma geometría).
- **Instalador + CI:** `packaging/build_deb.sh` (dpkg-deb, sin cargo-deb) →
  `/usr/bin/arqhia` + iconos `hicolor` + `.desktop` + `metainfo`;
  `packaging/install.sh` (genérico, `~/.local` o `PREFIX`); `postinst` solo
  crea carpetas si faltan y **jamás** pisa `config.toml`/DB (verificado:
  config y DB sentinela intactas tras instalar). `.github/workflows/ci.yml`:
  `fmt --check` + `clippy -D warnings` + `test` + `build --release` (falla si
  el binario ≥30 MB) + job `.deb`.
- **Tests:** `cargo test` 203/203 OK (3 ignorados), `clippy --all-targets -D
  warnings` 0, `fmt --check` OK.
- Docs: `PROJECT.md` (§4 árbol + §10/§13), `POLICIES.md` §7 (instalador),
  `AGENTS.md` (5 roles), `README.md` (instalación Linux), `VERSIONS.md`.

# Siguiente paso (actualizado)

- **v0.9.6 🟢 Done (puerta de v1.0 cumplida).** Siguiente: **WEB**
  (`CONTEXT/WEB.md`) → v1.0 → v1.0.1 → v1.1 → v1.2.
