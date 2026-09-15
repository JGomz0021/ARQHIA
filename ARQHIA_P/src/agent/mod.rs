pub mod roles;
pub mod tools;

use crate::config::{Provider, ProviderConfig};
use crate::llm::{ChatMsg, Role};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Resultado de un turno autocontenido. La UI v0.6 usa el driver paso a paso;
// esto queda como API programática (y lo usa el E2E `agent_e2e_groq`).
#[allow(dead_code)]
pub struct AgentResult {
    pub answer: String,
    pub logs: Vec<String>,
}

#[cfg(test)]
mod live_tests {
    /// E2E manual contra la API real con la config guardada del usuario.
    /// `cargo test agent_e2e_groq -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn agent_e2e_groq() {
        let cfg_all = crate::config::AppConfig::load();
        let cfg = cfg_all.active_config();
        assert!(
            cfg.is_configured_for(cfg_all.active),
            "Configura API key antes del E2E (Configuración -> Guardar)"
        );
        let ws = std::path::PathBuf::from("/tmp/arqhia-e2e-ws");
        std::fs::create_dir_all(&ws).unwrap();
        let history = vec![crate::llm::ChatMsg {
            role: crate::llm::Role::User,
            content: "crea el archivo hola.txt con el texto HOLA-AGENTE y nada más".to_string(),
        }];
        let res = super::run_turn(cfg_all.active, history, cfg, ws.clone())
            .await
            .expect("run_turn falló");
        tracing::info!("ANSWER: {}", res.answer);
        for l in &res.logs {
            tracing::info!("LOG: {l}");
        }
        let content = std::fs::read_to_string(ws.join("hola.txt")).expect("hola.txt no creado");
        assert!(content.contains("HOLA-AGENTE"), "contenido: {content}");
        let _ = std::fs::remove_dir_all(&ws);
    }

    /// E2E del orquestador: planner -> workers -> auditor + TEMP.md.
    /// `cargo test orchestrator_e2e -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn orchestrator_e2e() {
        let cfg_all = crate::config::AppConfig::load();
        // E2E contra Groq (slot OpenAI), no contra el provider activo
        let provider = crate::config::Provider::OpenAI;
        let cfg = cfg_all.openai.clone();
        assert!(cfg.is_configured_for(provider), "Configura API key primero");
        let ws = std::path::PathBuf::from("/tmp/arqhia-orch-ws");
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();

        // 1) Planner
        let tasks = super::plan_tasks(
            provider,
            "crea el archivo a.txt con AAA y el archivo b.txt con BBB",
            "Workspace: /tmp/arqhia-orch-ws\nArchivos: (vacío)",
            None,
            None,
            &cfg,
            "Work",
            None,
        )
        .await
        .expect("planner falló");
        tracing::info!("TASKS: {tasks:?}");
        assert!(!tasks.is_empty() && tasks.len() <= 3);

        // 2) Workers secuenciales (todo auto, como la UI con permisos full)
        for (i, task) in tasks.iter().take(2).enumerate() {
            let system = super::worker_system(&ws, task);
            let seed =
                serde_json::json!({"role": "user", "content": format!("TAREA: {}", task.desc)});
            let mut raw = vec![seed];
            for step in 1..=super::MAX_ITERS {
                match super::llm_step(provider, &cfg, &system, &raw)
                    .await
                    .expect("llm_step")
                {
                    super::StepOutcome::Final(a) => {
                        tracing::info!("WORKER{i} ANSWER: {a}");
                        break;
                    }
                    super::StepOutcome::Calls {
                        calls,
                        assistant_msg,
                    } => {
                        raw.push(assistant_msg);
                        let (append, logs) = super::exec_calls(
                            provider,
                            &ws,
                            &[],
                            step,
                            &calls,
                            &super::tools::ExecPolicy::default(),
                            super::roles::Role::Worker,
                        )
                        .await;
                        for l in &logs {
                            tracing::info!("WORKER{i} LOG: {l}");
                        }
                        raw.extend(append);
                    }
                }
            }
        }
        assert!(ws.join("a.txt").exists(), "a.txt no creado");
        assert!(ws.join("b.txt").exists(), "b.txt no creado");

        // 3) Auditor + TEMP.md
        let (temp, logs, verify_ok) = super::audit_workspace(provider, &cfg, &ws, 0)
            .await
            .expect("auditor falló");
        for l in &logs {
            tracing::info!("AUDIT LOG: {l}");
        }
        tracing::info!("VERIFY_OK: {verify_ok}");
        tracing::info!("TEMP:\n{temp}");
        assert!(temp.contains("## Auditoría"), "TEMP.md sin formato");
        let _ = std::fs::remove_dir_all(&ws);
    }
}

/// Tope de rondas LLM->tools por turno. Existe para evitar loops infinitos
/// (un modelo confundido pediría tools sin parar) y coste descontrolado.
/// El turno termina antes si el modelo responde sin pedir más tools.
pub const MAX_ITERS: usize = 10;

fn system_prompt(workspace: &std::path::Path) -> String {
    format!(
        "Eres ARQHIA, un agente de código con acceso real al workspace.\n\
        {}\n\
        Reglas:\n\
        - Flujo de lectura EFICIENTE (en este orden): `get_file_outline` para decidir QUÉ leer -> `search_files` para localizar (máx 20 bloques con contexto) -> `read_file` POR PÁGINAS (200 líneas; pide más con offset). Nunca pidas archivos enteros de golpe.\n\
        - Usa las tools para leer/crear/editar/borrar archivos y ejecutar comandos. Rutas RELATIVAS al workspace (ej. src/main.rs).\n\
        - Antes de editar, lee la página exacta. `edit_file` exige que `old` aparezca EXACTAMENTE 1 vez.\n\
        - `bash` solo permite: ls, cat, echo, pwd, cargo --version, rustc --version, cargo check, cargo build, cargo test, cargo run, cargo clippy, cargo fmt, `git` (status/diff/log/add/commit/stash; push con permiso; destructivos bloqueados) (+instaladores solo con permiso Install).\n- `fetch_url` descarga http(s) como texto; dominios no autorizados piden permiso.\n\
        - Cuando termines los cambios, resume en 1-3 líneas qué hiciste. Si no necesitas tools, responde directo.",
        crate::workspace::context_block(workspace)
    )
}

/// System LEAN del worker (v0.7.1): tarea + reglas, SIN SPECS/AGENTS.
/// Esos viajan UNA vez como mensaje de contexto (ver `worker_context_block`),
/// no repetidos en cada step.
pub fn worker_system(workspace: &Path, task: &WTask) -> String {
    format!(
        "{}\n\nTAREA ASIGNADA (enfócate solo en esto):\n{}\nArchivos previstos: {}\n\
        No toques archivos fuera de lo necesario para esta tarea.",
        system_prompt(workspace),
        task.desc,
        if task.files.is_empty() {
            "(los que necesites del workspace)".to_string()
        } else {
            task.files.join(", ")
        },
    )
}

/// Bloque de contexto del worker (v0.7.1 + v0.7.3): AGENTS.md + SPECS.md +
/// brief del analista (ANALYSIS.md) truncados, para enviar UNA vez como
/// mensaje de contexto al abrir el worker.
pub fn worker_context_block(workspace: &Path, agents_md: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(md) = agents_md {
        let cut: String = md.chars().take(2000).collect();
        parts.push(format!("Reglas del proyecto (AGENTS.md):\n{cut}"));
    }
    if let Some(espec) = read_espec_md(workspace) {
        parts.push(format!("Especificación del proyecto (SPECS.md):\n{espec}"));
    }
    if let Some(brief) = read_analysis_md(workspace) {
        parts.push(format!(
            "Brief del analista (CONTEXT/ANALYSIS.md):\n{brief}"
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Lee la especificación del workspace (v0.8: `SPECS.md`, con fallback al
/// `ESPEC.md` legacy de v0.5). Truncado a 2000 chars o None.
pub fn read_espec_md(workspace: &Path) -> Option<String> {
    for name in ["SPECS.md", "ESPEC.md"] {
        if let Ok(s) = std::fs::read_to_string(workspace.join("CONTEXT").join(name)) {
            return Some(s.chars().take(2000).collect());
        }
    }
    None
}

/// Lee `CONTEXT/ANALYSIS.md` (brief del analista, v0.7.3) truncado o None.
pub fn read_analysis_md(workspace: &Path) -> Option<String> {
    std::fs::read_to_string(workspace.join("CONTEXT").join("ANALYSIS.md"))
        .ok()
        .map(|s| s.chars().take(2000).collect())
}

/// Lee los docs reales de `CONTEXT/*.md` (v0.7.3) con tope por archivo.
/// Genérico: sirve para ESPEC.md (v0.5-v0.7) y PROJECT/SPECS/CONTEXT.md (v0.8).
/// `skip_analysis` evita realimentar el propio brief.
pub fn read_context_docs(workspace: &Path) -> String {
    const NAMES: [&str; 11] = [
        "PROJECT.md",
        "SPECS.md",
        "ESPEC.md",
        "CONTEXT.md",
        "VERSIONS.md",
        "ROADMAP.md",
        "PLAN.md",
        "TEMP.md",
        // v0.9.2: reportes de skills de dominio (los lee el Orquestador
        // en el siguiente turno).
        "UI-REVIEW.md",
        "CODE-REVIEW.md",
        "QA-REPORT.md",
    ];
    let dir = workspace.join("CONTEXT");
    let mut out = String::new();
    for name in NAMES {
        let Ok(raw) = std::fs::read_to_string(dir.join(name)) else {
            continue;
        };
        let cut: String = raw.trim().chars().take(1500).collect();
        if cut.is_empty() {
            continue;
        }
        out.push_str(&format!("\n### {name}\n{cut}\n"));
    }
    if out.is_empty() {
        "(sin docs en CONTEXT/)".to_string()
    } else {
        out
    }
}

/// Outlines (solo firmas) de las fuentes principales del workspace (v0.7.3).
/// Máx 12 archivos y 60 firmas por archivo vía `tools::outline_of`.
pub fn code_outlines(workspace: &Path) -> String {
    const EXTS: [&str; 7] = [".rs", ".py", ".ts", ".js", ".go", ".java", ".c"];
    let mut out = String::new();
    let mut n = 0;
    let walker = walkdir::WalkDir::new(workspace)
        .max_depth(6)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !matches!(name.as_ref(), "target" | ".git" | "node_modules")
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());
    for entry in walker {
        let path = entry.path();
        let rel = path
            .strip_prefix(workspace)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if !EXTS.iter().any(|x| rel.ends_with(x)) || tools::is_ignored(&rel, &[]) {
            continue;
        }
        let sigs = tools::outline_of(path);
        if sigs.is_empty() {
            continue;
        }
        out.push_str(&format!("\n### {rel}\n{sigs}\n"));
        n += 1;
        if n >= 12 {
            break;
        }
    }
    if out.is_empty() {
        "(sin fuentes con firmas)".to_string()
    } else {
        out
    }
}

/// `code_outlines` en background (v0.9.5): el walk de 6 niveles no bloquea
/// ni al executor ni a la UI.
pub async fn code_outlines_async(workspace: &Path) -> String {
    let ws = workspace.to_path_buf();
    tokio::task::spawn_blocking(move || code_outlines(&ws))
        .await
        .unwrap_or_else(|_| "(outlines no disponibles)".to_string())
}

/// `read_context_docs` en background (v0.9.5).
pub async fn read_context_docs_async(workspace: &Path) -> String {
    let ws = workspace.to_path_buf();
    tokio::task::spawn_blocking(move || read_context_docs(&ws))
        .await
        .unwrap_or_else(|_| "(sin docs en CONTEXT/)".to_string())
}

/// Analista dedicado (v0.7.3): UNA llamada sin tools que produce un brief
/// (estado, restricciones, archivos, riesgos) a partir de CONTEXT/, outlines
/// y el pedido. El llamador decide el fallback si falla.
/// v0.9.5: las lecturas de FS van por `spawn_blocking` (no bloquean workers).
pub async fn analyze_workspace(
    provider: Provider,
    cfg: &ProviderConfig,
    workspace: &Path,
    pedido: &str,
) -> Result<String, String> {
    let system = roles::Role::Analista.system_prompt();
    let ws = workspace.to_path_buf();
    // FS bloqueante fuera del executor: las 3 lecturas en background.
    let (tree, docs, outlines) = tokio::join!(
        crate::workspace::context_block_async(&ws),
        read_context_docs_async(&ws),
        code_outlines_async(&ws),
    );
    let user = format!(
        "Pedido del usuario:\n{pedido}\n\n## Árbol del workspace\n{tree}\n\n## Docs de CONTEXT/\n{docs}\n\n## Firmas del código\n{outlines}",
    );
    simple_chat(provider, system, &user, cfg).await
}

/// Recorta a `max` chars con … (seguro en UTF-8: nunca corta a mitad de
/// carácter como haría un slice por bytes).
fn short(s: &str, max: usize) -> String {
    let one: String = s.replace('\n', " ").chars().take(max + 1).collect();
    if one.chars().count() > max {
        format!("{}…", one.chars().take(max).collect::<String>())
    } else {
        one
    }
}

// ---------------------------------------------------------------------------
// Utilidades v0.7.1: presupuesto, ventana, colapso, parada temprana, caché.
// ---------------------------------------------------------------------------

/// Estimador de tokens (v0.7.1): `chars/4`, aproximado ±30% según tokenizer.
/// Documentado como aproximado: el código denso subestima. Nunca promete
/// exactitud; sirve para el badge del Log y el presupuesto configurable.
/// Fuente única en `llm::estimate_tokens_text` (aquí en u64 para el presupuesto).
pub fn estimate_tokens(text: &str) -> u64 {
    crate::llm::estimate_tokens_text(text) as u64
}

/// Suma estimada de un historial crudo (los `Value` se serializan).
pub fn estimate_raw_tokens(raw: &[Value]) -> u64 {
    raw.iter().map(|v| estimate_tokens(&v.to_string())).sum()
}

/// `~12k` para el badge del Log. Discreto, sin panel.
pub fn format_tokens(n: u64) -> String {
    if n < 1000 {
        format!("{n}")
    } else {
        format!("~{}k", n / 1000)
    }
}

/// Ventana de historial (v0.7.1): últimos `limit` mensajes.
/// Devuelve (ventana, colapsados): si se recortó, el llamador antepone un
/// marcador para que el modelo sepa que hay contexto previo omitido.
pub fn window_history<T: Clone>(history: &[T], limit: usize) -> (Vec<T>, usize) {
    let n = limit.max(1);
    if history.len() <= n {
        (history.to_vec(), 0)
    } else {
        let cut = history.len() - n;
        (history[cut..].to_vec(), cut)
    }
}

/// Colapsa outputs viejos de tools en `raw` (v0.7.1): solo los últimos
/// `keep` resultados quedan intactos, el resto -> `[omitido]`.
/// Cubre ambos formatos (OpenAI `role=tool`, Anthropic `tool_result`).
pub fn collapse_old_tool_outputs(raw: &mut [Value], keep: usize) {
    // Índices de mensajes con resultados, en orden de aparición.
    let mut idx: Vec<usize> = Vec::new();
    for (i, m) in raw.iter().enumerate() {
        let role = m["role"].as_str().unwrap_or("");
        if role == "tool" {
            idx.push(i);
        } else if role == "user"
            && let Some(blocks) = m["content"].as_array()
            && blocks.iter().any(|b| b["type"] == "tool_result")
        {
            idx.push(i);
        }
    }
    if idx.len() <= keep {
        return;
    }
    let collapse_until = idx.len() - keep;
    for &i in &idx[..collapse_until] {
        if raw[i]["role"] == "tool" {
            raw[i]["content"] =
                json!("[omitido: resultado anterior colapsado para ahorrar contexto]");
        } else if let Some(blocks) = raw[i]["content"].as_array_mut() {
            for b in blocks.iter_mut() {
                if b["type"] == "tool_result" {
                    b["content"] = json!("[omitido]");
                }
            }
        }
    }
}

/// Firma de un lote de calls (v0.7.1): 2 steps con la misma firma o sin
/// cambios -> parada temprana del worker (evita loops que queman tokens).
pub fn calls_signature(calls: &[PendingCall]) -> String {
    let mut parts: Vec<String> = calls
        .iter()
        .map(|c| format!("{}:{}", c.name, c.args))
        .collect();
    parts.sort();
    parts.join("\n")
}

/// Clave de caché de lecturas por turno (v0.7.1): mismo archivo+parámetros
/// no se relee del disco dos veces. Solo lecturas puras (read/outline).
/// `None` = no cacheable (escrituras, bash, búsquedas con contexto vivo).
pub fn read_cache_key(call: &PendingCall) -> Option<String> {
    match call.name.as_str() {
        "read_file" => {
            let p = call
                .args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let o = call
                .args
                .get("offset")
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            let l = call
                .args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(200);
            Some(format!("read:{p}:{o}:{l}"))
        }
        "get_file_outline" => {
            let p = call
                .args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Some(format!("outline:{p}"))
        }
        _ => None,
    }
}

/// Mensaje de resultado servido desde caché, en el formato de cada provider.
pub fn cached_result_msg(provider: Provider, call: &PendingCall, output: &str) -> Value {
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => json!({
            "role": "tool",
            "tool_call_id": call.id,
            "content": output,
        }),
        Provider::Anthropic => json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": output,
            }],
        }),
    }
}

// ---------------------------------------------------------------------------
// Primitivas paso a paso (v0.6): un LLM-step no ejecuta nada, el driver
// (UI) decide permisos y luego llama a exec_calls. run_turn las reutiliza.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PendingCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub enum StepOutcome {
    Final(String),
    Calls {
        calls: Vec<PendingCall>,
        assistant_msg: Value,
    },
}

fn openai_chat_url(cfg: &ProviderConfig) -> String {
    let base = crate::llm::normalize_base_url(&cfg.base_url);
    // OpenRouter usa /api/v1, el resto /v1
    if base.contains("openrouter") {
        format!("{base}/api/v1/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

pub(crate) fn history_to_openai(history: &[ChatMsg]) -> Vec<Value> {
    history
        .iter()
        .map(|m| json!({"role": m.role.as_str(), "content": m.content}))
        .collect()
}

pub(crate) fn history_to_anthropic(history: &[ChatMsg]) -> Vec<Value> {
    let mut messages: Vec<Value> = history
        .iter()
        .filter(|m| {
            m.role != Role::System && (!m.content.trim().is_empty() || m.role == Role::User)
        })
        .map(|m| {
            let role = match m.role {
                Role::Assistant => "assistant",
                // System ya filtrado arriba; User por defecto
                _ => "user",
            };
            json!({"role": role, "content": m.content})
        })
        .collect();
    // Anthropic exige primer mensaje user
    while messages
        .first()
        .map(|m| m["role"] == "assistant")
        .unwrap_or(false)
    {
        messages.remove(0);
    }
    messages
}

/// Un solo paso LLM con tools declaradas. No ejecuta nada.
pub async fn llm_step(
    provider: Provider,
    cfg: &ProviderConfig,
    system: &str,
    raw: &[Value],
) -> Result<StepOutcome, String> {
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => {
            llm_step_openai(cfg, system, raw).await
        }
        Provider::Anthropic => llm_step_anthropic(cfg, system, raw).await,
    }
}

async fn llm_step_openai(
    cfg: &ProviderConfig,
    system: &str,
    raw: &[Value],
) -> Result<StepOutcome, String> {
    crate::llm::ensure_secure_endpoint(cfg)?;
    let url = openai_chat_url(cfg);
    let client = crate::llm::http_client();
    let mut messages = vec![json!({"role": "system", "content": system})];
    messages.extend(raw.iter().cloned());
    // v0.9.3: anexa tools MCP si hay servidores configurados (cache 5min, no bloquea si cae)
    let mcp_servers = crate::config::AppConfig::load().mcp.servers;
    let tools_val = if mcp_servers.is_empty() {
        tools::openai_schemas()
    } else {
        let collected = crate::mcp::collect_mcp_tools(&mcp_servers).await;
        if collected.is_empty() {
            tools::openai_schemas()
        } else {
            tools::openai_schemas_with_mcp(&collected)
        }
    };
    let body = json!({
        "model": cfg.model,
        "messages": messages,
        "tools": tools_val,
        "tool_choice": "auto",
        "temperature": 0.2,
    });
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://arqhia.local")
        .header("X-Title", "ARQHIA")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        // Modelos locales sin function calling rechazan `tools` con 400:
        // degradar a chat plano en vez de romper el turno.
        if status.as_u16() == 400 && text.to_lowercase().contains("tool") {
            let plain = plain_chat_openai(cfg, system, raw).await?;
            return Ok(StepOutcome::Final(format!(
                "{plain}\n\n_(nota: este modelo no soporta tools, respondí sin ejecutar acciones)_"
            )));
        }
        return Err(format!("{}: {}", status, short(&text, 300)));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida: {e}"))?;
    let msg = &v["choices"][0]["message"];
    let content = msg["content"].as_str().unwrap_or("").to_string();
    let raw_calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
    if raw_calls.is_empty() {
        let answer = if content.trim().is_empty() {
            "(el agente no devolvió texto)".to_string()
        } else {
            content
        };
        return Ok(StepOutcome::Final(answer));
    }
    let calls = raw_calls
        .iter()
        .map(|call| PendingCall {
            id: call["id"].as_str().unwrap_or("").to_string(),
            name: call["function"]["name"].as_str().unwrap_or("?").to_string(),
            args: call["function"]["arguments"]
                .as_str()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(json!({})),
        })
        .collect();
    Ok(StepOutcome::Calls {
        calls,
        assistant_msg: json!({
            "role": "assistant",
            "content": content,
            "tool_calls": raw_calls,
        }),
    })
}

/// Chat plano OpenAI-compatible sin tools (fallback para modelos sin
/// function calling + base de simple_chat local si hiciera falta).
async fn plain_chat_openai(
    cfg: &ProviderConfig,
    system: &str,
    raw: &[Value],
) -> Result<String, String> {
    let url = openai_chat_url(cfg);
    let client = crate::llm::http_client();
    let mut messages = vec![json!({"role": "system", "content": system})];
    messages.extend(raw.iter().cloned());
    let body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.2,
    });
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://arqhia.local")
        .header("X-Title", "ARQHIA");
    if !cfg.api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.api_key.trim()));
    }
    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, short(&text, 300)));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida: {e}"))?;
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(sin respuesta)")
        .to_string())
}

async fn llm_step_anthropic(
    cfg: &ProviderConfig,
    system: &str,
    raw: &[Value],
) -> Result<StepOutcome, String> {
    crate::llm::ensure_secure_endpoint(cfg)?;
    let base = crate::llm::normalize_base_url(&cfg.base_url);
    let url = format!("{base}/v1/messages");
    let client = crate::llm::http_client();
    let mcp_servers = crate::config::AppConfig::load().mcp.servers;
    let tools_val = if mcp_servers.is_empty() {
        tools::anthropic_schemas()
    } else {
        let collected = crate::mcp::collect_mcp_tools(&mcp_servers).await;
        if collected.is_empty() {
            tools::anthropic_schemas()
        } else {
            tools::anthropic_schemas_with_mcp(&collected)
        }
    };
    let body = json!({
        "model": cfg.model,
        "max_tokens": 2048,
        "system": system,
        "tools": tools_val,
        "messages": raw,
    });
    let resp = client
        .post(&url)
        .header("x-api-key", cfg.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic {status}: {}", short(&text, 300)));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida: {e}"))?;
    let blocks = v["content"].as_array().cloned().unwrap_or_default();
    let mut text_out = String::new();
    let mut calls = Vec::new();
    for b in &blocks {
        match b["type"].as_str().unwrap_or("") {
            "text" => {
                if let Some(t) = b["text"].as_str() {
                    text_out.push_str(t);
                }
            }
            "tool_use" => calls.push(PendingCall {
                id: b["id"].as_str().unwrap_or("").to_string(),
                name: b["name"].as_str().unwrap_or("?").to_string(),
                args: b.get("input").cloned().unwrap_or(json!({})),
            }),
            _ => {}
        }
    }
    if calls.is_empty() {
        let answer = if text_out.trim().is_empty() {
            "(el agente no devolvió texto)".to_string()
        } else {
            text_out
        };
        return Ok(StepOutcome::Final(answer));
    }
    Ok(StepOutcome::Calls {
        calls,
        assistant_msg: json!({"role": "assistant", "content": blocks}),
    })
}

/// Ejecuta llamadas ya aprobadas. Devuelve (mensajes a anexar al historial, logs).
/// `role` es el gate POR ROL (v0.9.1): el Planner nunca escribe aunque el
/// lote venga aprobado; solo el Worker ejecuta Write/Bash/Install.
pub async fn exec_calls(
    provider: Provider,
    workspace: &Path,
    extra: &[PathBuf],
    step: usize,
    calls: &[PendingCall],
    policy: &tools::ExecPolicy,
    role: roles::Role,
) -> (Vec<Value>, Vec<String>) {
    let mut append = Vec::new();
    let mut logs = Vec::new();
    // Gate por rol (v0.9.1): lo que el rol no permite ni se ejecuta.
    let mut allowed: Vec<&PendingCall> = Vec::new();
    for call in calls {
        if role.allows_tool(&call.name, &call.args) {
            allowed.push(call);
        } else {
            logs.push(format!(
                "⛔ {} denegado por rol {}",
                call.name,
                role.label()
            ));
            append.push(role_denial_msg(provider, call, role));
        }
    }
    let calls = allowed;
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => {
            for call in calls {
                let outcome =
                    tools::execute(workspace, extra, &call.name, &call.args, policy).await;
                logs.push(format!(
                    "🔧 [paso {step}/{MAX_ITERS}] {} {} -> {}",
                    call.name,
                    short(&call.args.to_string(), 80),
                    short(&outcome, 120)
                ));
                append.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": outcome,
                }));
            }
        }
        Provider::Anthropic => {
            let mut results = Vec::new();
            for call in calls {
                let outcome =
                    tools::execute(workspace, extra, &call.name, &call.args, policy).await;
                logs.push(format!(
                    "🔧 [paso {step}/{MAX_ITERS}] {} {} -> {}",
                    call.name,
                    short(&call.args.to_string(), 80),
                    short(&outcome, 120)
                ));
                results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": call.id,
                    "content": outcome,
                }));
            }
            append.push(json!({"role": "user", "content": results}));
        }
    }
    (append, logs)
}

/// Mensaje de "denegado por rol" para UNA llamada (v0.9.1), en el formato
/// de cada provider.
pub fn role_denial_msg(provider: Provider, call: &PendingCall, role: roles::Role) -> Value {
    let text = format!(
        "⛔ El rol {} no puede usar `{}`. Continúa sin ella o propón una alternativa.",
        role.label(),
        call.name,
    );
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => json!({
            "role": "tool",
            "tool_call_id": call.id,
            "content": text,
        }),
        Provider::Anthropic => json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": text,
                "is_error": true,
            }],
        }),
    }
}

/// Mensajes de "denegado por el usuario" con el formato de cada provider.
pub fn denial_msgs(provider: Provider, calls: &[PendingCall]) -> Vec<Value> {
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => calls
            .iter()
            .map(|call| {
                json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": "⛔ El usuario denegó esta acción. Continúa sin ella o propón una alternativa.",
                })
            })
            .collect(),
        Provider::Anthropic => vec![json!({
            "role": "user",
            "content": calls.iter().map(|call| {
                json!({
                    "type": "tool_result",
                    "tool_use_id": call.id,
                    "content": "⛔ El usuario denegó esta acción. Continúa sin ella o propón una alternativa.",
                    "is_error": true,
                })
            }).collect::<Vec<_>>(),
        })],
    }
}

/// Turno de agente autocontenido (todo permitido; lo usa el E2E y futuros batch).
/// La UI v0.6 usa el driver paso a paso (llm_step + permisos + exec_calls).
#[allow(dead_code)]
pub async fn run_turn(
    provider: Provider,
    history: Vec<ChatMsg>,
    cfg: ProviderConfig,
    workspace: PathBuf,
) -> Result<AgentResult, String> {
    let system = system_prompt(&workspace);
    let mut raw: Vec<Value> = match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => history_to_openai(&history),
        Provider::Anthropic => {
            let r = history_to_anthropic(&history);
            if r.is_empty() {
                return Err("Mensaje vacío".to_string());
            }
            r
        }
    };
    let mut logs = Vec::new();
    for step in 1..=MAX_ITERS {
        match llm_step(provider, &cfg, &system, &raw).await? {
            StepOutcome::Final(answer) => return Ok(AgentResult { answer, logs }),
            StepOutcome::Calls {
                calls,
                assistant_msg,
            } => {
                raw.push(assistant_msg);
                let (append, step_logs) = exec_calls(
                    provider,
                    &workspace,
                    &[],
                    step,
                    &calls,
                    &tools::ExecPolicy::default(),
                    roles::Role::Worker,
                )
                .await;
                logs.extend(step_logs);
                raw.extend(append);
            }
        }
    }
    Ok(AgentResult {
        answer: format!(
            "Hice cambios pero llegué al límite de {MAX_ITERS} pasos. Revisa el Log y pídeme que continúe."
        ),
        logs,
    })
}

// ---------------------------------------------------------------------------
// Orquestador v0.6: planner -> workers (generadores) -> auditor -> TEMP.md
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WTask {
    pub desc: String,
    pub files: Vec<String>,
    /// Criterio de aceptación verificable (v0.9.1): qué debe cumplirse
    /// para dar la tarea por hecha. Vacío = sin criterio explícito.
    #[allow(dead_code)]
    pub accept: String,
}

/// Planner: 1 llamada sin tools que divide el pedido en 2-3 tareas disjuntas.
/// Si el JSON no parsea, fallback a 1 tarea con el pedido tal cual.
/// Cuerpo de la petición del planner (v0.7 Track B).
/// El planner es solo-lectura POR CONSTRUCCIÓN: este body nunca incluye
/// `tools`, así que el modelo no puede pedir ejecuciones en esta fase.
pub fn planner_request_body(cfg: &ProviderConfig, system: &str, user: &str) -> Value {
    json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": 0.2,
    })
}

/// `extra_ctx` (v0.9.1): contexto Net+Read ya autorizado (docs + rutas extra).
#[allow(clippy::too_many_arguments)]
pub async fn plan_tasks(
    provider: Provider,
    user_text: &str,
    context: &str,
    espec: Option<&str>,
    brief: Option<&str>,
    cfg: &ProviderConfig,
    mode_label: &str,
    extra_ctx: Option<&str>,
) -> Result<Vec<WTask>, String> {
    let system = format!(
        "{}\n\nDivide el pedido del usuario en 2 o 3 subtareas de código DISJUNTAS (archivos distintos, sin solaparse), cada una con 1 criterio de aceptación verificable. Responde SOLO con un array JSON, sin markdown ni texto extra, con este formato exacto: [{{\"desc\": \"...\", \"files\": [\"src/a.rs\"], \"accept\": \"...\"}}]. Si el pedido es trivial, responde con 1 sola tarea. Modo actual: {mode_label} (solo planificas: nunca ejecutas herramientas).",
        roles::Role::Planner.system_prompt()
    );
    let mut user =
        format!("Contexto del workspace:\n{context}\n\nPedido del usuario:\n{user_text}");
    if let Some(b) = brief {
        let cut: String = b.chars().take(2000).collect();
        user.push_str(&format!("\n\nBrief del analista (v0.7.3):\n{cut}"));
    }
    if let Some(e) = espec {
        let cut: String = e.chars().take(2000).collect();
        user.push_str(&format!(
            "\n\nEspecificación del proyecto (SPECS.md):\n{cut}"
        ));
    }
    // v0.9.1: contexto Net+Read del planner (docs externos + rutas extra).
    if let Some(x) = extra_ctx {
        let cut: String = x.chars().take(9000).collect();
        if !cut.trim().is_empty() {
            user.push_str(&format!(
                "\n\nContexto extra del planner (Net+Read):\n{cut}"
            ));
        }
    }
    // El body OpenAI-compat sale del constructor auditado; Anthropic va por
    // simple_chat (body propio también sin tools).
    let answer = if provider == Provider::Anthropic {
        simple_chat(provider, &system, &user, cfg).await?
    } else {
        let body = planner_request_body(cfg, &system, &user);
        simple_chat_with_body(provider, &body, &system, &user, cfg).await?
    };
    if let Some(tasks) = parse_tasks(&answer)
        && !tasks.is_empty()
    {
        return Ok(tasks.into_iter().take(3).collect());
    }
    Ok(vec![WTask {
        desc: user_text.to_string(),
        files: Vec::new(),
        accept: String::new(),
    }])
}

fn parse_tasks(answer: &str) -> Option<Vec<WTask>> {
    // Acepta JSON directo o envuelto en ```json fences
    let mut s = answer.trim();
    if let Some(start) = s.find("```") {
        let after = &s[start..];
        if let Some(nl) = after.find('\n') {
            s = after[nl + 1..].trim();
        }
        if let Some(end) = s.find("```") {
            s = s[..end].trim();
        }
    }
    let v: Value = serde_json::from_str(s).ok()?;
    let arr = v.as_array()?;
    let mut out = Vec::new();
    for item in arr {
        let desc = item["desc"].as_str().unwrap_or("").trim();
        if desc.is_empty() {
            continue;
        }
        let files = item["files"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|f| f.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        // v0.9.1: criterio de aceptación por tarea (opcional en el JSON).
        let accept = item["accept"]
            .as_str()
            .unwrap_or("")
            .trim()
            .chars()
            .take(200)
            .collect();
        out.push(WTask {
            desc: desc.to_string(),
            files,
            accept,
        });
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Chat simple sin tools (planner, auditor).
pub async fn simple_chat(
    provider: Provider,
    system: &str,
    user: &str,
    cfg: &ProviderConfig,
) -> Result<String, String> {
    let body = planner_request_body(cfg, system, user);
    simple_chat_with_body(provider, &body, system, user, cfg).await
}

/// Envía un body OpenAI-compat ya construido (planner) o el de Anthropic.
/// `body` solo se usa en la rama OpenAI-compat; Anthropic lo reconstruye
/// (también sin tools) porque su formato difiere.
async fn simple_chat_with_body(
    provider: Provider,
    body: &Value,
    system: &str,
    user: &str,
    cfg: &ProviderConfig,
) -> Result<String, String> {
    crate::llm::ensure_secure_endpoint(cfg)?;
    let client = crate::llm::http_client();
    match provider {
        Provider::OpenAI | Provider::OpenRouter | Provider::Local => {
            let resp = client
                .post(openai_chat_url(cfg))
                .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://arqhia.local")
                .header("X-Title", "ARQHIA")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Error de red: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("{}: {}", status, short(&text, 300)));
            }
            let v: Value = resp
                .json()
                .await
                .map_err(|e| format!("Respuesta inválida: {e}"))?;
            Ok(v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string())
        }
        Provider::Anthropic => {
            let body = json!({
                "model": cfg.model,
                "max_tokens": 1024,
                "system": system,
                "messages": [{"role": "user", "content": user}],
            });
            let base = crate::llm::normalize_base_url(&cfg.base_url);
            let resp = client
                .post(format!("{base}/v1/messages"))
                .header("x-api-key", cfg.api_key.trim())
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Error de red: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("Anthropic {status}: {}", short(&text, 300)));
            }
            let v: Value = resp
                .json()
                .await
                .map_err(|e| format!("Respuesta inválida: {e}"))?;
            Ok(v["content"]
                .as_array()
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter_map(|b| {
                            if b["type"] == "text" {
                                b["text"].as_str()
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default())
        }
    }
}

/// Título corto por IA (v0.7.4): 1 llamada sin tools, 3–5 palabras.
/// Si falla o viene vacío, el llamador cae a `titles::title_for`.
pub async fn ai_title(
    provider: Provider,
    cfg: &ProviderConfig,
    first_msg: &str,
) -> Result<String, String> {
    let out = simple_chat(
        provider,
        "Eres el titulador de chats de ARQHIA. Devuelve SOLO un título de 3 a 5 palabras, sin comillas ni punto final, que resuma el mensaje del usuario.",
        first_msg,
        cfg,
    )
    .await?;
    Ok(crate::titles::sanitize_ai_title(&out, first_msg))
}

/// Descarga las URLs del planner (v0.9.1, Net) con la política ya aprobada
/// y las recorta a `roles::PLANNER_FETCH_CAP` chars cada una. El llamador
/// decide el permiso ANTES (ver `roles::planner_urls_needing_permission`):
/// aquí solo se sale a la red con permiso concedido.
pub async fn fetch_planner_net_block(
    urls: &[String],
    policy: &tools::ExecPolicy,
) -> Option<String> {
    let mut parts = Vec::new();
    for u in urls.iter().take(3) {
        match tools::fetch_url(u, policy).await {
            Ok(text) => {
                let cut: String = text.chars().take(roles::PLANNER_FETCH_CAP).collect();
                parts.push(format!("### Doc externa: {u}\n{cut}"));
            }
            Err(e) => parts.push(format!("### Doc externa: {u}\n(no se pudo descargar: {e})")),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Extrae URLs http(s) de un texto (v0.7.4 Fuentes): para citar fuentes tras fetch_url.
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        let t = tok.trim_matches(|c: char| "<>()[]{}\"'.,;:!?".contains(c));
        if (t.starts_with("http://") || t.starts_with("https://"))
            && t.len() > 10
            && !out.contains(&t.to_string())
        {
            out.push(t.to_string());
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

/// Auditor (solo lectura): revisa con el LLM + puerta de calidad
/// (`cargo check` + `cargo test` + `cargo clippy --all-targets`) si hay
/// Cargo.toml. Devuelve (contenido TEMP.md, logs, clean).
/// `clean == true` solo si la reseña quedó limpia Y la verificación pasó
/// (v0.7.3): el auto-commit queda bloqueado si no. Si el LLM no está
/// disponible, la reseña no bloquea (solo manda la puerta de calidad).
/// `cycle` es el contador `auditor → fix` que se refleja en TEMP.md.
pub async fn audit_workspace(
    provider: Provider,
    cfg: &ProviderConfig,
    workspace: &Path,
    cycle: usize,
) -> Result<(String, Vec<String>, bool), String> {
    let mut logs = vec![format!("🕵️ auditor revisando… (ciclo {cycle})")];
    let files = crate::workspace::list_top(workspace, 30).join("\n");
    let review_raw = simple_chat(
        provider,
        roles::Role::Auditor.system_prompt(),
        &format!("Archivos del workspace:\n{files}\n\nÚltimos cambios: revisa coherencia general."),
        cfg,
    )
    .await;
    // LLM caído o timeout: no bloquea el commit por sí solo (mandan los checks).
    let review_available = review_raw.is_ok();
    let review = review_raw.unwrap_or_else(|e| {
        format!("(auditor LLM no disponible: {e}; se aplica solo la puerta de calidad)")
    });
    let review_clean = review.contains("SIN ISSUES") || !review_available;

    let mut verify_ok = true;
    let mut check_block = String::new();
    if workspace.join("Cargo.toml").exists() {
        // Timeout ampliado: test/clippy pueden tardar más que un comando normal.
        let policy = tools::ExecPolicy {
            timeout_s: 600,
            ..tools::ExecPolicy::default()
        };
        for (label, cmd) in [
            ("cargo check", "cargo check"),
            ("cargo test", "cargo test"),
            ("cargo clippy --all-targets", "cargo clippy --all-targets"),
        ] {
            match tools::bash(workspace, &[], cmd, &policy).await {
                Ok(out) => {
                    let failed = out.contains("[exit ");
                    let tail = tail_chars(&out, 1500);
                    if failed {
                        verify_ok = false;
                        logs.push(format!("❌ {label} falló"));
                    } else {
                        logs.push(format!("🔧 {label} OK"));
                    }
                    check_block.push_str(&format!("### {label}\n```\n{tail}\n```\n"));
                }
                Err(e) => {
                    verify_ok = false;
                    logs.push(format!("❌ {label}: {e}"));
                    check_block.push_str(&format!("### {label} no ejecutable\n```\n{e}\n```\n"));
                }
            }
        }
    } else {
        check_block = "(sin Cargo.toml, no se verificó compilación)\n".to_string();
    }

    // Si la verificación falla, la reseña del LLM no puede ocultarlo.
    let review_final = if verify_ok {
        review
    } else if review.contains("SIN ISSUES") {
        "(la verificación automática falló; revisa la sección Verificación)".to_string()
    } else {
        review
    };
    let clean = verify_ok && review_clean;
    // v0.9.5: diff async (no bloquea al auditor en repos grandes).
    let diff = crate::git::diff_stat_async(workspace).await;
    let diff_block = if diff.is_empty() {
        "(sin cambios detectados)".to_string()
    } else {
        format!("```\n{diff}\n```")
    };
    let verdict = if clean {
        "VERDICT: CLEAN"
    } else {
        "VERDICT: ISSUES"
    };
    let verify_line = if verify_ok {
        "VERIFY: OK"
    } else {
        "VERIFY: FAIL"
    };
    let temp = format!(
        "## Auditoría ARQHIA\n\n{review_final}\n\n## Verificación\n\n{check_block}\n## git diff --stat\n\n{diff_block}\n\n## Ciclo y estado\n\n- Ciclo: {cycle}\n- {verify_line}\n- {verdict}\n"
    );
    Ok((temp, logs, clean))
}

/// Últimos `max` chars de un texto (respetando UTF-8).
fn tail_chars(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        s.chars().skip(n - max).collect()
    }
}

/// true si el TEMP.md contiene issues reales (v0.7.3: marcador VERDICT).
pub fn temp_has_issues(temp: &str) -> bool {
    if temp.contains("VERDICT: CLEAN") {
        return false;
    }
    if temp.contains("VERDICT: ISSUES") {
        return true;
    }
    !(temp.contains("SIN ISSUES") || temp.trim().len() < 60)
}

pub const AGENTS_TEMPLATE: &str = r#"# AGENTS.md — reglas del orquestador ARQHIA

## Roles (v0.9.1: 5 roles explícitos)
- orquestador: recibe el objetivo, decide plan directo vs work, lanza fases, escribe checklist y cierra (commit solo en verde). No ejecuta tools.
- analista: 1 llamada sin tools → `CONTEXT/ANALYSIS.md` (Estado, Restricciones, Archivos, Riesgos; ≤200 palabras). En cada ciclo de fix RE-ANALIZA `TEMP.md` antes del arreglo.
- planner: divide el pedido en 2–3 subtareas disjuntas (archivos distintos) con criterio de aceptación → `CONTEXT/PLAN.md`. Solo Net+Read (fetch_url con permiso Net + lectura de rutas extra); NUNCA Write/Bash por construcción.
- worker (generador): ejecuta UNA subtarea con read/write/edit/delete/list/bash/search/fetch_url. Único con Write/Bash/Install (según permisos).
- auditor: solo lee y reporta a `CONTEXT/TEMP.md` (`VERDICT: CLEAN|ISSUES` + `VERIFY: OK|FAIL` + `git diff --stat`). Nunca escribe código.

## Flujo
Orquestador > Analista (ANALYSIS.md) > Planner (PLAN.md) > Workers > Auditor (TEMP.md) > CLEAN fin + commit > ISSUES Analista revisa > Workers fix > Auditor > loop.

## Tools permitidas
read_file (paginado: offset/limit), get_file_outline, write_file, edit_file, delete_file, list_dir, search_files (bloques con contexto), bash (allowlist + git con política por subcomando), fetch_url (dominios), mcp__* (tools MCP stdio/HTTP, categoría Net/auto, npx pide Install).

## Reglas
- Rutas relativas al workspace. Nunca escribir fuera (guard estricto).
- `edit_file` exige 1 coincidencia exacta de `old`.
- Pasos LLM->tools y tareas del plan según Límites de config; installs y red piden permiso (Install/Net; el planner pide Net la 1ª vez).
- Git (v0.7.2): se trabaja en la rama `ARQHIA`; `main`/`master` protegidas. Push con aprobación; destructivos bloqueados.
- Tras workers, el auditor revisa (código + `cargo check`/`test`/`clippy`) y escribe CONTEXT/TEMP.md; el bucle `auditor -> analista -> fix` repite hasta quedar verde (tope `Limits.max_fix_cycles`, 0 = ilimitado). El auto-commit solo con el turno verde.
"#;

/// Crea AGENTS.md en el workspace si no existe. Devuelve línea de log o None.
pub fn ensure_agents_md(workspace: &Path) -> Option<String> {
    let file = workspace.join("AGENTS.md");
    if file.exists() {
        return None;
    }
    match std::fs::write(&file, AGENTS_TEMPLATE) {
        Ok(()) => Some("📄 AGENTS.md creado".to_string()),
        Err(_) => None,
    }
}

/// Lee AGENTS.md (truncado) para inyectar en prompts, o None si no existe.
pub fn read_agents_md(workspace: &Path) -> Option<String> {
    std::fs::read_to_string(workspace.join("AGENTS.md"))
        .ok()
        .map(|s| s.chars().take(2000).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v0.9.5: los wrappers `spawn_blocking` devuelven lo mismo que las sync.
    #[tokio::test]
    async fn blocking_wrappers_match_sync() {
        let dir = std::env::temp_dir().join("arqhia-blocking-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("CONTEXT")).unwrap();
        std::fs::write(dir.join("CONTEXT").join("SPECS.md"), "# Demo").unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn f() {}\n").unwrap();
        assert_eq!(code_outlines_async(&dir).await, code_outlines(&dir));
        assert_eq!(read_context_docs_async(&dir).await, read_context_docs(&dir));
        assert_eq!(
            crate::workspace::context_block_async(&dir).await,
            crate::workspace::context_block(&dir)
        );
        assert_eq!(
            crate::workspace::scan_import_async(&dir).await,
            crate::workspace::scan_import(&dir)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn planner_body_declares_no_tools() {
        // v0.7 Track B: el planner es solo-lectura por construcción.
        let cfg = ProviderConfig {
            api_key: String::new(),
            base_url: String::new(),
            model: "m".to_string(),
            reasoning_effort: String::new(),
        };
        let body = super::planner_request_body(&cfg, "sys", "haz X");
        assert!(
            body.get("tools").is_none(),
            "el planner no debe declarar tools"
        );
        assert!(body.get("tool_choice").is_none());
        assert_eq!(body["model"], serde_json::json!("m"));
    }

    #[test]
    fn parse_tasks_plain_and_fenced() {
        let plain = r#"[{"desc": "Crear a.rs", "files": ["src/a.rs"], "accept": "compila"}, {"desc": "Crear b.rs", "files": ["src/b.rs"]}]"#;
        let tasks = parse_tasks(plain).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].files, vec!["src/a.rs"]);
        assert_eq!(tasks[0].accept, "compila");
        assert!(tasks[1].accept.is_empty(), "accept es opcional");
        let fenced = format!("Aquí va:\n```json\n{plain}\n```\nlisto");
        assert_eq!(parse_tasks(&fenced).unwrap().len(), 2);
        assert!(parse_tasks("no hay json aquí").is_none());
        assert!(parse_tasks("[{\"desc\": \"\", \"files\": []}]").is_none());
    }

    #[test]
    fn temp_issue_detection() {
        assert!(!temp_has_issues("## Auditoría\n\nSIN ISSUES\n"));
        assert!(temp_has_issues(
            "## Auditoría\n\n- src/main.rs línea 3: falta punto y coma, el binario no compila por el módulo roto"
        ));
        // v0.7.3: marcador explícito manda sobre el texto.
        assert!(!temp_has_issues(
            "## Auditoría\n\nrevisión\n\nVERDICT: CLEAN\n"
        ));
        assert!(temp_has_issues(
            "## Auditoría\n\nrevisión\n\nVERDICT: ISSUES\n"
        ));
    }

    #[test]
    fn analyst_context_docs_and_outlines_compose() {
        let dir = std::env::temp_dir().join("arqhia-analyst-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("CONTEXT")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("CONTEXT").join("PROJECT.md"), "# MiApp\nVisión X").unwrap();
        std::fs::write(
            dir.join("src").join("main.rs"),
            "pub fn arranque() {}\nstruct Motor {}\n",
        )
        .unwrap();
        let docs = read_context_docs(&dir);
        assert!(docs.contains("PROJECT.md"), "{docs}");
        assert!(docs.contains("MiApp"), "{docs}");
        let outs = code_outlines(&dir);
        assert!(outs.contains("src/main.rs"), "{outs}");
        assert!(outs.contains("fn arranque"), "{outs}");
        // El brief del analista viaja UNA vez en el contexto del worker.
        std::fs::write(dir.join("CONTEXT").join("ANALYSIS.md"), "Estado: inicial").unwrap();
        let ctx = worker_context_block(&dir, None).expect("contexto");
        assert!(ctx.contains("Brief del analista"), "{ctx}");
        assert!(ctx.contains("Estado: inicial"), "{ctx}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn context_docs_include_skill_reports() {
        // v0.9.2: el siguiente turno del Orquestador lee los reportes de
        // skills de dominio vía `read_context_docs`.
        let dir = std::env::temp_dir().join("arqhia-skill-reports-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("CONTEXT")).unwrap();
        std::fs::write(
            dir.join("CONTEXT").join("UI-REVIEW.md"),
            "## Chat\n- [Media] Aumentar el espaciado del composer",
        )
        .unwrap();
        std::fs::write(
            dir.join("CONTEXT").join("CODE-REVIEW.md"),
            "- [Alta] src/main.rs:10 — unwrap en producción",
        )
        .unwrap();
        std::fs::write(
            dir.join("CONTEXT").join("QA-REPORT.md"),
            "QA: PASS — check/test/clippy verdes",
        )
        .unwrap();
        let docs = read_context_docs(&dir);
        assert!(docs.contains("UI-REVIEW.md"), "{docs}");
        assert!(docs.contains("CODE-REVIEW.md"), "{docs}");
        assert!(docs.contains("QA-REPORT.md"), "{docs}");
        assert!(docs.contains("unwrap en producción"), "{docs}");
        assert!(docs.contains("QA: PASS"), "{docs}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn worker_system_mentions_task() {
        let t = WTask {
            desc: "Crear hola".to_string(),
            files: vec!["hola.txt".to_string()],
            accept: "hola.txt existe con el texto".to_string(),
        };
        let s = worker_system(Path::new("/tmp"), &t);
        assert!(s.contains("Crear hola"));
        assert!(s.contains("hola.txt"));
        // v0.7.1: el system es lean (ESPEC va como mensaje de contexto).
        assert!(!s.contains("Especificación del proyecto"));
    }

    /// v0.9.1: el gate por rol se ejecuta de verdad — el Worker escribe y
    /// el Planner es denegado sin tocar el disco.
    #[tokio::test]
    async fn role_gate_blocks_planner_writes() {
        let dir = std::env::temp_dir().join("arqhia-role-gate-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let call = PendingCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            args: serde_json::json!({"path": "nota.txt", "content": "hola"}),
        };
        let policy = tools::ExecPolicy::default();
        // Worker: ejecuta.
        let (append, logs) = exec_calls(
            Provider::OpenAI,
            &dir,
            &[],
            1,
            std::slice::from_ref(&call),
            &policy,
            roles::Role::Worker,
        )
        .await;
        assert!(dir.join("nota.txt").exists(), "el worker debe escribir");
        assert!(logs.iter().any(|l| l.contains("write_file")), "{logs:?}");
        assert_eq!(append.len(), 1);
        let _ = std::fs::remove_file(dir.join("nota.txt"));
        // Planner: denegado por rol, sin tocar el disco.
        let (append2, logs2) = exec_calls(
            Provider::OpenAI,
            &dir,
            &[],
            1,
            &[call],
            &policy,
            roles::Role::Planner,
        )
        .await;
        assert!(
            !dir.join("nota.txt").exists(),
            "el planner no debe escribir"
        );
        assert!(
            logs2.iter().any(|l| l.contains("denegado por rol Planner")),
            "{logs2:?}"
        );
        assert_eq!(append2.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn worker_context_block_injects_espec_once() {
        let dir = std::env::temp_dir().join("arqhia-espec-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("CONTEXT")).unwrap();
        std::fs::write(
            dir.join("CONTEXT").join("ESPEC.md"),
            "# MiApp\n\n## Objetivo\nX",
        )
        .unwrap();
        let ctx = worker_context_block(&dir, None).expect("debería haber contexto");
        assert!(ctx.contains("MiApp"), "ESPEC no inyectado");
        // Sin ESPEC ni AGENTS no hay bloque (el worker arranca sin lastre).
        let empty = std::env::temp_dir().join("arqhia-espec-empty");
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        assert!(worker_context_block(&empty, None).is_none());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn token_estimator_is_chars_over_four() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(12_000), "~12k");
    }

    #[test]
    fn history_window_keeps_last_n() {
        let h: Vec<i32> = (0..30).collect();
        let (w, cut) = window_history(&h, 20);
        assert_eq!(w.len(), 20);
        assert_eq!(cut, 10);
        assert_eq!(w[0], 10);
        let (w2, cut2) = window_history(&h, 50);
        assert_eq!(w2.len(), 30);
        assert_eq!(cut2, 0);
    }

    #[test]
    fn collapse_keeps_only_last_tool_outputs() {
        let mut raw = vec![
            json!({"role": "user", "content": "hola"}),
            json!({"role": "tool", "tool_call_id": "1", "content": "contenido viejo 1"}),
            json!({"role": "tool", "tool_call_id": "2", "content": "contenido viejo 2"}),
            json!({"role": "tool", "tool_call_id": "3", "content": "reciente"}),
        ];
        collapse_old_tool_outputs(&mut raw, 1);
        assert!(raw[1]["content"].as_str().unwrap().contains("omitido"));
        assert!(raw[2]["content"].as_str().unwrap().contains("omitido"));
        assert_eq!(raw[3]["content"], json!("reciente"));
        // Formato Anthropic también colapsa.
        let mut raw2 = vec![
            json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "a", "content": "viejo"}]}),
            json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "b", "content": "nuevo"}]}),
        ];
        collapse_old_tool_outputs(&mut raw2, 1);
        assert_eq!(raw2[1]["content"][0]["content"], json!("nuevo"));
        assert_eq!(raw2[0]["content"][0]["content"], json!("[omitido]"));
    }

    #[test]
    fn early_stop_signature_is_order_stable() {
        let a = vec![
            PendingCall {
                id: "1".to_string(),
                name: "read_file".to_string(),
                args: json!({"path": "a.rs"}),
            },
            PendingCall {
                id: "2".to_string(),
                name: "bash".to_string(),
                args: json!({"cmd": "ls"}),
            },
        ];
        let mut b = a.clone();
        b.reverse();
        assert_eq!(calls_signature(&a), calls_signature(&b));
        let c = vec![PendingCall {
            id: "1".to_string(),
            name: "read_file".to_string(),
            args: json!({"path": "b.rs"}),
        }];
        assert_ne!(calls_signature(&a), calls_signature(&c));
    }

    #[test]
    fn extract_urls_finds_http_links_once() {
        let t = "Mira https://example.com/a y (https://docs.rs/crate,) más http://x.test/q.";
        let urls = extract_urls(t);
        assert_eq!(
            urls,
            vec![
                "https://example.com/a",
                "https://docs.rs/crate",
                "http://x.test/q"
            ]
        );
        assert!(extract_urls("sin enlaces").is_empty());
        assert!(extract_urls("https://a.test https://a.test").len() == 1);
    }

    #[test]
    fn read_cache_keys_only_pure_reads() {
        let read = PendingCall {
            id: "1".to_string(),
            name: "read_file".to_string(),
            args: json!({"path": "a.rs"}),
        };
        assert_eq!(read_cache_key(&read).as_deref(), Some("read:a.rs:1:200"));
        let paged = PendingCall {
            id: "2".to_string(),
            name: "read_file".to_string(),
            args: json!({"path": "a.rs", "offset": 201, "limit": 200}),
        };
        assert_ne!(read_cache_key(&read), read_cache_key(&paged));
        let outline = PendingCall {
            id: "3".to_string(),
            name: "get_file_outline".to_string(),
            args: json!({"path": "a.rs"}),
        };
        assert!(read_cache_key(&outline).is_some());
        let write = PendingCall {
            id: "4".to_string(),
            name: "write_file".to_string(),
            args: json!({"path": "a.rs"}),
        };
        assert!(read_cache_key(&write).is_none());
        let search = PendingCall {
            id: "5".to_string(),
            name: "search_files".to_string(),
            args: json!({"query": "x"}),
        };
        assert!(read_cache_key(&search).is_none());
    }
}
