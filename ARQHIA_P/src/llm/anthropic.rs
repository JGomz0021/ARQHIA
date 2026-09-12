use crate::config::ProviderConfig;
use crate::llm::{ChatMsg, parse_anthropic_chunk};
use futures::StreamExt;
use serde_json::json;

fn endpoint(cfg: &ProviderConfig) -> String {
    // Permite base_url custom, por defecto https://api.anthropic.com
    format!(
        "{}/v1/messages",
        crate::llm::normalize_base_url(&cfg.base_url)
    )
}

pub async fn chat_stream(
    history: Vec<ChatMsg>,
    cfg: ProviderConfig,
    on_chunk: &mut impl FnMut(String),
) -> Result<crate::llm::Usage, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("Falta API key de Anthropic. Ve a Configuración.".to_string());
    }
    let input_estimate: u32 = history
        .iter()
        .map(|m| crate::llm::estimate_tokens_text(&m.content))
        .sum();
    let client = crate::llm::http_stream_client();
    // System va en parámetro propio de Anthropic; el resto en messages.
    // Anthropic exige turnos user/assistant alternados; colapsamos vacíos.
    let system: String = history
        .iter()
        .filter(|m| m.role == crate::llm::Role::System)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let messages: Vec<_> = history
        .iter()
        .filter(|m| m.role != crate::llm::Role::System && !m.content.trim().is_empty())
        .map(|m| {
            let role = match m.role {
                crate::llm::Role::Assistant => "assistant",
                crate::llm::Role::System => "user", // inalcanzable por el filtro
                crate::llm::Role::User => "user",
            };
            json!({"role": role, "content": m.content})
        })
        .collect();
    if messages.is_empty() {
        return Err("Mensaje vacío".to_string());
    }
    let mut body = json!({
        "model": cfg.model,
        "max_tokens": 1024,
        "stream": true,
        "messages": messages,
    });
    if !system.trim().is_empty() {
        body["system"] = json!(system);
    }
    // Nivel de razonamiento -> presupuesto de "extended thinking".
    let effort = cfg.reasoning_effort.trim();
    if !effort.is_empty() {
        let budget: u64 = match effort {
            "minimal" | "low" => 2_000,
            "medium" => 8_000,
            "high" => 16_000,
            "max" => 24_000,
            _ => 8_000, // "on"
        };
        body["thinking"] = json!({ "type": "enabled", "budget_tokens": budget });
        if body["max_tokens"].as_u64().unwrap_or(1024) < budget + 1024 {
            body["max_tokens"] = json!(budget + 1024);
        }
    }
    let resp = client
        .post(endpoint(&cfg))
        .header("x-api-key", cfg.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red Anthropic: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(400).collect();
        return Err(format!("Anthropic {status}: {short}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut out_chars = 0usize;
    let mut input_tokens: Option<u32> = None;
    let mut output_tokens: Option<u32> = None;
    let mut cached_tokens: u32 = 0;
    while let Some(item) = stream.next().await {
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                // Sin texto aún es un fallo real; con texto, cierre parcial.
                if out_chars == 0 {
                    return Err(format!("Corte de stream: {e}"));
                }
                on_chunk("\n\n_(se cortó la conexión; respuesta parcial)_".to_string());
                break;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if let Some(u) = crate::llm::parse_anthropic_start_usage(data) {
                input_tokens = Some(u.input);
                cached_tokens = u.cached;
            }
            if let Some(n) = crate::llm::parse_anthropic_output_usage(data) {
                output_tokens = Some(n);
            }
            if let Some(text) = parse_anthropic_chunk(data)
                && !text.is_empty() {
                    out_chars += text.chars().count();
                    on_chunk(text);
                }
        }
    }
    Ok(crate::llm::Usage {
        input: input_tokens.unwrap_or(input_estimate),
        output: output_tokens.unwrap_or(crate::llm::estimate_tokens_chars(out_chars)),
        cached: cached_tokens,
        cost: None,
    })
}

pub async fn test_connection(cfg: ProviderConfig) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("API key vacía".to_string());
    }
    // Anthropic no tiene GET /models público; hacemos un POST mínimo de 1 token.
    let client = crate::llm::http_client();
    let body = json!({
        "model": cfg.model,
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "hi"}],
    });
    let resp = client
        .post(endpoint(&cfg))
        .header("x-api-key", cfg.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    if resp.status().is_success() {
        Ok("Conexión Anthropic OK".to_string())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(300).collect();
        Err(format!("Anthropic {status}: {short}"))
    }
}
