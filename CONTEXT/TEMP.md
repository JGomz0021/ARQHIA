## Auditoría ARQHIA

SIN ISSUES

## UI polish + tokens + models.dev (ronda actual)

- Layout: mensajes centrados (880px); cabecera/stats/dock a todo el ancho. Dock inferior (composer + log) con fondo elevado y sombra (`design::dock`). Entrada 16px/padding `[10,8]`; log 14px ampliable 12↔40 líneas y 170↔460px.
- Contexto de la barra superior ahora incluye tools/lecturas (`App.context_tokens`, actualizado en `account_tokens` con `system + raw`), no solo mensajes.
- Tokens: `Usage { input, output, cached }`; parseo real OpenAI (`stream_options.include_usage`, `prompt_tokens_details.cached_tokens`) y Anthropic (`message_start`/`message_delta`, cache read/creation). `in/out/cache` bajo cada mensaje y en el Log.
- Precios: `pricing.rs` descarga/cachea `models.dev/api.json` (200 providers, miles de modelos) con input/output/cache write, contexto y capacidades. Coste por mensaje con fallback `llm::estimate_cost_usd`.
- Navegador de modelos (Config → API): botón `Buscar modelo`, búsqueda por id/nombre/familia, precios y badges; exige API key del proveedor (o LM Studio `/v1/models` para Local). Mapeo por `base_url` (Groq/Google/etc.).
- Chat más ancho (`1080px`); navegador con filtro de precio (`Todos/Gratis/≤$1/≤$5/≤$15`), `solo tools`, `ordenar por precio`, contador y panel de 900px.
- Densidad operativa: `design::gap/pad` reduce espaciados y paddings reales en chat, sidebar y cards de Config.
- Chat a 1400px en ventana 1560×880; composer reorganizado (input solo arriba; modos/modelo debajo).
- Limpieza de iconos: fuera botón `Cuestionario` del header, botón `TEMP` del log, badge de modo del header (y se eliminó el visor TEMP/message asociado). `Salir` ahora es rojo outline.
- Nivel de razonamiento por modelo desde `models.dev` (`reasoning_options`): selector en Config y en el composer; se persiste y se envía al provider (`reasoning_effort` / `reasoning.effort` / `thinking.budget_tokens`). Width de burbujas a 1180/1000 y sidebar 248 para que el ensanchado se note.
- Correcciones ronda: chat a todo el ancho (quitado el centrado con spacers, que repartía en tercios); burbujas `width(Fill)`. Log oculto en modo Chat. Selector de razonamiento siempre visible + autoload del catálogo al arrancar. Coste usa el reportado por el proveedor (`usage.cost`) cuando existe. Barra de contexto en formato `264.000 tokens / 26% used`.
- Ajuste fino: chat centrado con ancho por densidad (cómoda 1020 / compacta 860); texto de mensajes a 14px; pickers de modelo/razonamiento con borde de acento y razonamiento más estrecho (110); sin mensaje de estado al cambiar el nivel; quitado el botón `"<"` del header.
- Ronda Config/contexto: lookup **por provider** en el catálogo (evita colisiones que daban `on/auto` y contexto de 1M); etiqueta "Nivel" en el composer; aviso al ≥90% de contexto (sigue, olvidando lo más antiguo); sin guiones `—` al final de las filas del navegador; Config con tabs de 220px, cards lado a lado (Proveedor/Credenciales, Permisos/Red) y tooltips en Límites. Backlog Config/UI (v0.8–v1.0) añadido a ROADMAP y PROJECT.
- Backlog ampliado: guardar varios modelos/perfiles (API key + base_url + provider propios) y mejora/revisión del lector de Markdown (cobertura de sintaxis + aspecto visual).
- Backlog ampliado 2: repos guía (opencode, OpenHands, SWE-agent) y división de chats en sesiones con id por provider (OpenRouter/OpenAI/Anthropic/Groq…).

## cargo check / test / clippy

- `cargo check`: OK (sin warnings).
- `cargo test`: 70/70 OK (3 ignorados; uno valida el api.json real de models.dev).
- `cargo clippy --all-targets`: 0 warnings.
