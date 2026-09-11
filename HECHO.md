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
- v0.7 en curso: CONTEXT reescrito (config por pestañas, uploads, borrado); Config con columna `API|Apariencia|Permisos|Proyectos`; pestaña Proyectos con borrado a papelera (reusa `remove_project_everywhere`) y lista `uploads/` con borrado individual (guard anti-escape); botón `Subir` por proyecto (`rfd::pick_files` en `spawn_blocking` → `{ws}/uploads/`, tope 50 MB, `nombre (2).ext`); `workspace::{upload_files,list_uploads,delete_upload}` + tests. `cargo test` 32/32 OK, sin warnings.
- Fix crash pestaña Proyectos: `scrollable(list).height(Fill)` anidado dentro del scrollable de Config (panic "must not fill its vertical scrolling axis"). El tab devuelve la columna y el scroll lo pone el contenedor. Test headless `config_tabs_build_without_panic` que construye las 4 pestañas. `cargo test` 33/33 OK.
- Estilo general: `⏻ Salir` al fondo del sidebar (`iced::exit()`); botones compactos en toda la app (helpers `icon_btn/primary_btn/danger_btn`, padding 4–14); burbujas de chat, tarjetas de proyecto, modal de permisos y paneles con `rounded_box`; CTAs en primary, borrados en danger; pickers con ancho fijo anti-desborde; `cargo clippy` a cero warnings (`--fix` + ajustes manuales). `cargo test` 33/33 OK, binario reconstruido.
- System prompt (`llm::system_identity`): cada conversación arranca con identidad de ARQHIA, capacidades (con/sin tools) y reglas (idioma, Markdown, modelo en uso). Nuevo `Role::System`; Anthropic lo manda en su parámetro `system`, el resto como mensaje `system`. No se persiste en DB. `cargo test` 35/35 OK.
- Fix Volver: Config guarda `config_from` y `← Volver` regresa al origen (Chat o Home), ya no siempre a Home.
- Flujo: el cuestionario abre inmediatamente al crear proyecto (Crear o Abrir con ruta nueva, nombre prellenado); Abrir ruta ya registrada va directo al Chat. CONTEXT (§6, v0.4, v0.5) actualizado.
- Pulido UI (sin versiones en textos): título "ARQHIA", subtítulo `provider · modelo`, sidebar con secciones ("Conversaciones", nombre + contador por proyecto), workspace legible, estados vacíos amables, mensajes con más aire y etiquetas en gris, header con "⌂ Inicio / 📋 Cuestionario / ⚙", Home con tagline y consejo, footer ESPEC sin versión. `cargo check/test` OK.
- Sidebar v2: secciones `Chats` y `Proyectos` separadas con regla, proyectos plegables (`▸/▾` por proyecto), crear proyecto inline con `+` en la cabecera (adiós formulario fijo abajo; el click derecho no existe en Iced, el `+` es su equivalente), iconos/botones compactos, entrada con más aire respecto al selector de modelo. Borrar proyecto pide confirmación y elimina chats + carpeta del disco (avisa en el Log si la carpeta no se pudo borrar). `cargo test` 22/22 OK.

# Siguiente paso

- Prueba manual v0.7.1 (GUI): `pkill arqhia_p; cargo run`. Chat nuevo → badge dice Chat aunque haya workspace; `Ctrl+2` → Plan, "revisa el proyecto" → `CONTEXT/PLAN.md` existe, disco intacto, 1 llamada; Ejecutar plan → pasa a Work y orquesta; Work sobre repo grande → Log con `🪙` y `target/` ignorado; `max_tokens_turn=5000` + tarea larga → para al 100% con mensaje. Si OK, marcar v0.7.1 🟢 y seguir v0.8 (cuestionario 3 niveles). Nota: v0.7 sigue pendiente de su prueba manual (ver arriba); puede probarse junta.

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
- v0.7.1 implementada (código, pendiente prueba GUI): modos `Chat/Plan/Work` por chat (`db::Mode`, migración `chats.mode`, default Chat, badge en header + segmento en composer, `ModePicked` persiste); Plan = 1 llamada sin tools → `CONTEXT/PLAN.md` + checklist + `Ejecutar plan` (pasa a Work); Work = orquestador (Chat sin workspace = directo); atajos `Ctrl+N/1/2/3/O/,` + `Esc` (suscripción teclado, sin robar input) + pestaña Atajos; `search_files` v2 (bloques con 3 líneas ctx, máx 20, ranking nombre>contenido, ignora `target/.git/node_modules/*.lock`); `read_file` paginado (`offset/limit`, `líneas X–Y de Z`); `get_file_outline` (firmas); ventana historial configurable (def 20, con marcador) en chat plano y workers; ESPEC/AGENTS una vez como mensaje de contexto (system lean); colapso progresivo de outputs (últimos 4) + al 80% del presupuesto; caché de lecturas por turno (`📦`); parada temprana (calls idénticas 2×); presupuesto `max_tokens_turn` (0 = ilimitado) + badge `🪙` reescrito en Log (estimador `chars/4` ±30%); `busy_timeout` 5s en SQLite (adiós flake de tests paralelos). `cargo test` 64/64 OK, `cargo clippy --all-targets` cero warnings, binario reconstruido.
