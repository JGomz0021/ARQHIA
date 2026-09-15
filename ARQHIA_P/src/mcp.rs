//! MCP mínimo v0.9.3: stdio + HTTP, solo tools/*.
//! Sin crates nuevos. JSON-RPC a mano sobre tokio/reqwest.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::config::{McpServerConfig, McpTransport};

/// Tool MCP descubierta vía tools/list.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Prefijo para enrutar tools MCP : mcp__{server}__{tool}
pub fn mcp_tool_name(server: &str, tool: &str) -> String {
    format!("mcp__{}__{}", sanitize_server(server), tool)
}

fn sanitize_server(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Parse mcp__srv__tool -> (srv, tool)
pub fn parse_mcp_tool(full: &str) -> Option<(String, String)> {
    let rest = full.strip_prefix("mcp__")?;
    let idx = rest.find("__")?;
    let srv = &rest[..idx];
    let tool = &rest[idx + 2..];
    if srv.is_empty() || tool.is_empty() {
        return None;
    }
    Some((srv.to_string(), tool.to_string()))
}

pub fn is_mcp_tool(name: &str) -> bool {
    parse_mcp_tool(name).is_some()
}

/// Schema size cap 4KB (spec).
const SCHEMA_CAP: usize = 4096;

/// Convierte McpTool a schema OpenAI-compatible, con truncado si >4KB.
pub fn tool_to_openai_schema(server: &str, tool: &McpTool) -> Value {
    let mut desc = format!("[MCP {}] {}", server, tool.description);
    let mut schema = tool.input_schema.clone();
    let size = schema.to_string().len();
    if size > SCHEMA_CAP {
        desc.push_str(&format!(" (schema truncado: {size} > 4096)"));
        // Trunca a objeto vacío con aviso para no tragar contexto.
        schema =
            json!({"type":"object","properties":{},"description":"schema truncado por tamaño"});
    }
    // Anthropic/OpenAI both need properties.
    let params = if schema.is_object() && schema.get("type").is_some() {
        schema
    } else {
        json!({"type":"object","properties": schema.as_object().cloned().unwrap_or_default()})
    };
    json!({
        "type": "function",
        "function": {
            "name": mcp_tool_name(server, &tool.name),
            "description": desc.chars().take(1024).collect::<String>(),
            "parameters": params
        }
    })
}

pub fn tool_to_anthropic_schema(server: &str, tool: &McpTool) -> Value {
    let mut desc = format!("[MCP {}] {}", server, tool.description);
    let mut schema = tool.input_schema.clone();
    if schema.to_string().len() > SCHEMA_CAP {
        desc.push_str(" (schema truncado)");
        schema = json!({"type":"object","properties":{}});
    }
    let input_schema = if schema.is_object() && schema.get("type").is_some() {
        schema
    } else {
        json!({"type":"object","properties": schema})
    };
    json!({
        "name": mcp_tool_name(server, &tool.name),
        "description": desc.chars().take(1024).collect::<String>(),
        "input_schema": input_schema
    })
}

/// Cache 5 min para tools/list por servidor.
#[derive(Debug, Clone)]
struct CacheEntry {
    tools: Vec<McpTool>,
    at: Instant,
}
static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
fn cache_lock() -> &'static Mutex<HashMap<String, CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
fn cache_get(server: &str) -> Option<Vec<McpTool>> {
    let m = cache_lock().lock().ok()?;
    if let Some(e) = m.get(server)
        && e.at.elapsed() < Duration::from_secs(300)
    {
        return Some(e.tools.clone());
    }
    None
}
fn cache_put(server: &str, tools: Vec<McpTool>) {
    if let Ok(mut m) = cache_lock().lock() {
        m.insert(
            server.to_string(),
            CacheEntry {
                tools,
                at: Instant::now(),
            },
        );
    }
}
#[cfg(test)]
pub fn clear_cache() {
    if let Ok(mut m) = cache_lock().lock() {
        m.clear();
    }
}

// ---------------------------------------------------------------------------
// Stdio JSON-RPC helpers
// ---------------------------------------------------------------------------
fn rpc_req(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string() + "\n"
}
fn rpc_notify(method: &str, params: Value) -> String {
    json!({"jsonrpc":"2.0","method":method,"params":params}).to_string() + "\n"
}

fn parse_tools_from_result(v: &Value) -> Vec<McpTool> {
    let arr = v
        .get("tools")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    arr.into_iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?.to_string();
            if name.trim().is_empty() {
                return None;
            }
            let desc = t
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let schema = t
                .get("inputSchema")
                .or_else(|| t.get("input_schema"))
                .cloned()
                .unwrap_or(json!({"type":"object"}));
            Some(McpTool {
                name,
                description: desc,
                input_schema: schema,
            })
        })
        .collect()
}

fn parse_call_result(v: &Value) -> String {
    // MCP tools/call returns {content:[{type:"text",text:"..."}], isError?}
    if let Some(content) = v.get("content").and_then(|c| c.as_array()) {
        let parts: Vec<String> = content
            .iter()
            .filter_map(|b| {
                b.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        b.get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string())
                    })
            })
            .collect();
        if !parts.is_empty() {
            return parts.join("\n");
        }
    }
    if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
        return t.to_string();
    }
    v.to_string()
}

/// Lista tools vía stdio: spawn -> initialize -> tools/list -> kill.
/// Timeout por servidor.
pub async fn list_tools_stdio(server: &str, cfg: &McpServerConfig) -> Result<Vec<McpTool>, String> {
    if let Some(cached) = cache_get(server) {
        return Ok(cached);
    }
    if cfg.command.trim().is_empty() {
        return Err("MCP stdio: command vacío".to_string());
    }
    let timeout = Duration::from_secs(cfg.timeout_s.clamp(5, 120));
    let res = tokio::time::timeout(timeout, list_tools_stdio_inner(cfg)).await;
    match res {
        Ok(Ok(tools)) => {
            cache_put(server, tools.clone());
            Ok(tools)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!(
            "MCP {server}: timeout tras {}s (tools/list)",
            cfg.timeout_s
        )),
    }
}

async fn list_tools_stdio_inner(cfg: &McpServerConfig) -> Result<Vec<McpTool>, String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let mut child = tokio::process::Command::new(&cfg.command)
        .args(&cfg.args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo lanzar {}: {e}", cfg.command))?;

    let mut stdin = child.stdin.take().ok_or("sin stdin")?;
    let stdout = child.stdout.take().ok_or("sin stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    // initialize
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name":"arqhia","version":"0.9.3"}
    });
    stdin
        .write_all(rpc_req(1, "initialize", init_params).as_bytes())
        .await
        .map_err(|e| format!("write initialize: {e}"))?;
    stdin.flush().await.map_err(|e| e.to_string())?;

    // read initialize response (id 1) – ignora notificaciones sin id
    let mut v: Value = Value::Null;
    for _ in 0..5 {
        let line = tokio::time::timeout(Duration::from_secs(8), reader.next_line())
            .await
            .map_err(|_| "timeout initialize".to_string())?
            .map_err(|e| e.to_string())?
            .ok_or("MCP cerró stdout en initialize")?;
        let cand: Value =
            serde_json::from_str(&line).map_err(|e| format!("JSON initialize: {e}"))?;
        if cand.get("id").and_then(|x| x.as_u64()) == Some(1) {
            v = cand;
            break;
        }
    }
    if v.is_null() {
        let _ = child.kill().await;
        return Err("MCP initialize: sin respuesta id 1".to_string());
    }
    if v.get("error").is_some() {
        let _ = child.kill().await;
        return Err(format!("MCP initialize error: {}", v["error"]));
    }
    // notifications/initialized
    let _ = stdin
        .write_all(rpc_notify("notifications/initialized", json!({})).as_bytes())
        .await;
    let _ = stdin.flush().await;

    // tools/list
    stdin
        .write_all(rpc_req(2, "tools/list", json!({})).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())?;

    let mut v2: Value = Value::Null;
    for _ in 0..5 {
        let line2 = tokio::time::timeout(Duration::from_secs(8), reader.next_line())
            .await
            .map_err(|_| "timeout tools/list".to_string())?
            .map_err(|e| e.to_string())?
            .ok_or("MCP cerró stdout en tools/list")?;
        let cand: Value =
            serde_json::from_str(&line2).map_err(|e| format!("JSON tools/list: {e}"))?;
        if cand.get("id").and_then(|x| x.as_u64()) == Some(2) {
            v2 = cand;
            break;
        }
    }
    if v2.is_null() {
        let _ = child.kill().await;
        return Err("MCP tools/list: sin respuesta id 2".to_string());
    }
    if let Some(err) = v2.get("error") {
        let _ = child.kill().await;
        return Err(format!("tools/list error: {err}"));
    }
    let result = v2.get("result").cloned().unwrap_or(v2.clone());
    let tools = parse_tools_from_result(&result);
    let _ = child.kill().await;
    // wait a bit to reap
    let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
    Ok(tools)
}

/// Llama a una tool vía stdio.
pub async fn call_tool_stdio(
    cfg: &McpServerConfig,
    tool: &str,
    args: Value,
) -> Result<String, String> {
    if cfg.command.trim().is_empty() {
        return Err("MCP stdio: command vacío".to_string());
    }
    let timeout = Duration::from_secs(cfg.timeout_s.clamp(5, 120));
    let res = tokio::time::timeout(timeout, call_tool_stdio_inner(cfg, tool, args)).await;
    match res {
        Ok(r) => r,
        Err(_) => Err(format!("MCP tool {tool}: timeout tras {}s", cfg.timeout_s)),
    }
}

async fn call_tool_stdio_inner(
    cfg: &McpServerConfig,
    tool: &str,
    args: Value,
) -> Result<String, String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    // Para tools/call repetimos initialize (stateless per call para simplicidad).
    let mut child = tokio::process::Command::new(&cfg.command)
        .args(&cfg.args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo lanzar {}: {e}", cfg.command))?;

    let mut stdin = child.stdin.take().ok_or("sin stdin")?;
    let stdout = child.stdout.take().ok_or("sin stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    // initialize
    let init_params = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name":"arqhia","version":"0.9.3"}
    });
    stdin
        .write_all(rpc_req(1, "initialize", init_params).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())?;
    // consume initialize response (id 1) ignoring others
    for _ in 0..5 {
        if let Ok(Ok(Some(line))) =
            tokio::time::timeout(Duration::from_secs(8), reader.next_line()).await
        {
            if let Ok(v) = serde_json::from_str::<Value>(&line)
                && v.get("id").and_then(|x| x.as_u64()) == Some(1)
            {
                break;
            }
        } else {
            break;
        }
    }
    let _ = stdin
        .write_all(rpc_notify("notifications/initialized", json!({})).as_bytes())
        .await;
    let _ = stdin.flush().await;

    // tools/call
    let params = json!({"name": tool, "arguments": args});
    stdin
        .write_all(rpc_req(2, "tools/call", params).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())?;

    let mut v: Value = Value::Null;
    for _ in 0..5 {
        let line = tokio::time::timeout(Duration::from_secs(15), reader.next_line())
            .await
            .map_err(|_| format!("timeout tools/call {tool}"))?
            .map_err(|e| e.to_string())?
            .ok_or(format!("MCP cerró stdout en tools/call {tool}"))?;
        let cand: Value = serde_json::from_str(&line).map_err(|e| format!("JSON call: {e}"))?;
        if cand.get("id").and_then(|x| x.as_u64()) == Some(2) {
            v = cand;
            break;
        }
    }
    if v.is_null() {
        let _ = child.kill().await;
        return Err(format!("MCP tools/call {tool}: sin respuesta id 2"));
    }
    if let Some(err) = v.get("error") {
        let _ = child.kill().await;
        return Err(format!("tools/call error: {err}"));
    }
    let result = v.get("result").cloned().unwrap_or(v.clone());
    // MCP resources/prompts → error claro
    if result.get("resource").is_some() || result.get("prompt").is_some() {
        let _ = child.kill().await;
        return Err("resources/prompts hasta v1.0.1".to_string());
    }
    let out = parse_call_result(&result);
    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
    Ok(out)
}

// ---------------------------------------------------------------------------
// HTTP (Streamable HTTP POST per call)
// ---------------------------------------------------------------------------
/// Lista tools vía HTTP POST con Accept: application/json, text/event-stream.
pub async fn list_tools_http(server: &str, cfg: &McpServerConfig) -> Result<Vec<McpTool>, String> {
    if let Some(cached) = cache_get(server) {
        return Ok(cached);
    }
    if cfg.url.trim().is_empty() {
        return Err("MCP http: url vacía".to_string());
    }
    let timeout = Duration::from_secs(cfg.timeout_s.clamp(5, 120));
    let res = tokio::time::timeout(timeout, list_tools_http_inner(cfg)).await;
    match res {
        Ok(Ok(tools)) => {
            cache_put(server, tools.clone());
            Ok(tools)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!(
            "MCP {server} http: timeout tras {}s",
            cfg.timeout_s
        )),
    }
}

async fn list_tools_http_inner(cfg: &McpServerConfig) -> Result<Vec<McpTool>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_s.clamp(5, 120)))
        .build()
        .map_err(|e| e.to_string())?;
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}});
    let resp = client
        .post(cfg.url.trim())
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("MCP http: {e}"))?;
    if !resp.status().is_success() {
        let st = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(format!(
            "MCP http {st}: {}",
            txt.chars().take(300).collect::<String>()
        ));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    // Puede venir como JSON directo o SSE `data: {...}`
    let json_str = extract_json_from_sse(&text);
    let v: Value = serde_json::from_str(&json_str).map_err(|e| format!("JSON http: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("tools/list error: {err}"));
    }
    let result = v.get("result").cloned().unwrap_or(v.clone());
    if result.get("resource").is_some() {
        return Err("resources/prompts hasta v1.0.1".to_string());
    }
    Ok(parse_tools_from_result(&result))
}

pub async fn call_tool_http(
    cfg: &McpServerConfig,
    tool: &str,
    args: Value,
) -> Result<String, String> {
    if cfg.url.trim().is_empty() {
        return Err("MCP http: url vacía".to_string());
    }
    let timeout = Duration::from_secs(cfg.timeout_s.clamp(5, 120));
    let res = tokio::time::timeout(timeout, call_tool_http_inner(cfg, tool, args)).await;
    match res {
        Ok(r) => r,
        Err(_) => Err(format!(
            "MCP tool {tool} http: timeout tras {}s",
            cfg.timeout_s
        )),
    }
}

async fn call_tool_http_inner(
    cfg: &McpServerConfig,
    tool: &str,
    args: Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_s.clamp(5, 120)))
        .build()
        .map_err(|e| e.to_string())?;
    // MCP spec: resources/* y prompts/* no soportados en v0.9.3
    if tool.starts_with("resources/") || tool.starts_with("prompts/") {
        return Err("resources/prompts hasta v1.0.1".to_string());
    }
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name": tool, "arguments": args}});
    let resp = client
        .post(cfg.url.trim())
        .header("Accept", "application/json, text/event-stream")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("MCP http: {e}"))?;
    if !resp.status().is_success() {
        let st = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(format!(
            "MCP http {st}: {}",
            txt.chars().take(300).collect::<String>()
        ));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let json_str = extract_json_from_sse(&text);
    let v: Value = serde_json::from_str(&json_str).map_err(|e| format!("JSON http: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("tools/call error: {err}"));
    }
    let result = v.get("result").cloned().unwrap_or(v.clone());
    Ok(parse_call_result(&result))
}

fn extract_json_from_sse(text: &str) -> String {
    let t = text.trim();
    if t.starts_with("data:") {
        // SSE: lines starting with data:
        for line in t.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("data:") {
                let rest = rest.trim();
                if rest.starts_with('{') {
                    return rest.to_string();
                }
            }
        }
    }
    t.to_string()
}

/// Recolecta todos los schemas MCP para un config (con cache), sin bloquear si cae.
/// Usado por el orquestador al iniciar turno: lista + anexa al LLM.
pub async fn collect_mcp_tools(
    servers: &HashMap<String, McpServerConfig>,
) -> HashMap<String, Vec<McpTool>> {
    let mut out = HashMap::new();
    for (name, cfg) in servers {
        let tools = match cfg.transport {
            McpTransport::Stdio => list_tools_stdio(name, cfg).await,
            McpTransport::Http => list_tools_http(name, cfg).await,
        };
        match tools {
            Ok(t) => {
                out.insert(name.clone(), t);
            }
            Err(_) => {
                // servidor caído → no bloquea turno (spec)
            }
        }
    }
    out
}

/// Busca un McpTool por nombre en el collect.
#[allow(dead_code)]
pub fn find_mcp_tool<'a>(
    collected: &'a HashMap<String, Vec<McpTool>>,
    server: &str,
    tool: &str,
) -> Option<&'a McpTool> {
    collected.get(server)?.iter().find(|t| t.name == tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpTransport;

    #[test]
    fn mcp_names_parse_and_format() {
        assert_eq!(mcp_tool_name("mi-docs", "echo"), "mcp__mi-docs__echo");
        assert_eq!(
            parse_mcp_tool("mcp__mi-docs__echo"),
            Some(("mi-docs".to_string(), "echo".to_string()))
        );
        assert_eq!(
            parse_mcp_tool("mcp__srv__a__b"),
            Some(("srv".to_string(), "a__b".to_string()))
        );
        assert!(is_mcp_tool("mcp__x__y"));
        assert!(!is_mcp_tool("write_file"));
        assert!(parse_mcp_tool("mcp__x").is_none());
        assert!(parse_mcp_tool("x__y__z").is_none());
    }

    #[test]
    fn schema_truncated_when_large() {
        let big = json!({"type":"object","properties": {"a": {"type":"string","description": "x".repeat(5000)}}});
        let tool = McpTool {
            name: "big".to_string(),
            description: "desc".to_string(),
            input_schema: big,
        };
        let s = tool_to_openai_schema("srv", &tool);
        let desc = s["function"]["description"].as_str().unwrap();
        assert!(desc.contains("truncado"), "{desc}");
        // params should be truncated minimal
        let params_str = s["function"]["parameters"].to_string();
        assert!(params_str.len() < 500, "{params_str}");
    }

    #[test]
    fn small_schema_not_truncated() {
        let small = json!({"type":"object","properties":{"text":{"type":"string"}}});
        let tool = McpTool {
            name: "echo".to_string(),
            description: "echo tool".to_string(),
            input_schema: small,
        };
        let s = tool_to_openai_schema("srv", &tool);
        let desc = s["function"]["description"].as_str().unwrap();
        assert!(!desc.contains("truncado"), "{desc}");
        assert!(s["function"]["parameters"]["properties"]["text"].is_object());
    }

    #[test]
    fn parse_tools_from_result_ok() {
        let v = json!({"tools":[{"name":"echo","description":"hi","inputSchema":{"type":"object","properties":{"text":{"type":"string"}}}}]});
        let tools = parse_tools_from_result(&v);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn extract_sse_json() {
        let sse = "data: {\"jsonrpc\":\"2.0\",\"result\":{\"tools\":[]}}\n\n";
        assert!(extract_json_from_sse(sse).contains("tools"));
        let plain = r#"{"jsonrpc":"2.0","result":{"tools":[]}}"#;
        assert_eq!(extract_json_from_sse(plain), plain);
    }

    #[tokio::test]
    async fn stdio_stub_list_and_call() {
        clear_cache();
        // Stub Python MCP server: reads lines, replies JSON-RPC
        let dir = std::env::temp_dir().join("arqhia-mcp-stub-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("stub.py");
        std::fs::write(
            &script,
            r#"import sys, json
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try:
        req=json.loads(line)
    except: continue
    mid=req.get("id")
    method=req.get("method")
    if method=="initialize":
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"protocolVersion":"2024-11-05","capabilities":{}}})+"\n"); sys.stdout.flush()
    elif method=="notifications/initialized":
        continue
    elif method=="tools/list":
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"tools":[{"name":"echo","description":"echo text","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}}]}})+"\n"); sys.stdout.flush()
    elif method=="tools/call":
        name=req.get("params",{}).get("name")
        args=req.get("params",{}).get("arguments",{})
        txt=args.get("text","")
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"content":[{"type":"text","text":f"echo:{txt}"}]}})+"\n"); sys.stdout.flush()
    else:
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"error":{"code":-32601,"message":"not found"}})+"\n"); sys.stdout.flush()
"#,
        )
        .unwrap();
        let cfg = McpServerConfig {
            transport: McpTransport::Stdio,
            command: "python3".to_string(),
            args: vec![script.to_string_lossy().to_string()],
            url: String::new(),
            auto: false,
            timeout_s: 10,
        };
        let tools = list_tools_stdio("stub", &cfg).await.expect("list");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        let out = call_tool_stdio(&cfg, "echo", json!({"text":"hola"}))
            .await
            .expect("call");
        assert_eq!(out, "echo:hola");
        // cache hit: second list should come from cache even if script deleted
        let _ = std::fs::remove_file(&script);
        let cached = list_tools_stdio("stub", &cfg).await.expect("cached");
        assert_eq!(cached.len(), 1);
        clear_cache();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn http_stub_list_and_call() {
        clear_cache();
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        // Tiny HTTP server stub (one thread, handles 2 requests)
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for _ in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                    let body = &req[body_start..];
                    // extract method
                    let is_list = body.contains("tools/list");
                    let resp_body = if is_list {
                        r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"echo","description":"echo http","inputSchema":{"type":"object","properties":{"text":{"type":"string"}}}}]}}"#
                    } else {
                        r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"echo:http-hola"}]}}"#
                    };
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    let _ = stream.write_all(resp.as_bytes());
                }
            }
        });
        // wait a bit for listener
        tokio::time::sleep(Duration::from_millis(50)).await;
        let cfg = McpServerConfig {
            transport: McpTransport::Http,
            command: String::new(),
            args: Vec::new(),
            url: format!("http://{addr}/mcp"),
            auto: false,
            timeout_s: 10,
        };
        let tools = list_tools_http("http-stub", &cfg).await.expect("list http");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        let out = call_tool_http(&cfg, "echo", json!({"text":"hi"}))
            .await
            .expect("call http");
        assert_eq!(out, "echo:http-hola");
        clear_cache();
    }

    #[tokio::test]
    async fn http_down_returns_error() {
        clear_cache();
        let cfg = McpServerConfig {
            transport: McpTransport::Http,
            command: String::new(),
            args: Vec::new(),
            url: "http://127.0.0.1:1/mcp".to_string(),
            auto: false,
            timeout_s: 2,
        };
        let err = list_tools_http("down", &cfg).await.unwrap_err();
        assert!(
            err.contains("http") || err.contains("timeout") || err.contains("MCP"),
            "{err}"
        );
        clear_cache();
    }
}
