use crate::config::ProviderConfig;
use crate::llm::{ChatMsg, parse_openai_chunk};
use futures::StreamExt;
use serde_json::json;

fn endpoint(cfg: &ProviderConfig) -> String {
    format!(
        "{}/api/v1/chat/completions",
        crate::llm::normalize_base_url(&cfg.base_url)
    )
}

pub async fn chat_stream(
    history: Vec<ChatMsg>,
    cfg: ProviderConfig,
    session: Option<&str>,
    on_chunk: &mut impl FnMut(String),
) -> Result<crate::llm::Usage, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("Falta API key de OpenRouter. Ve a Configuración.".to_string());
    }
    let input_estimate: u32 = history
        .iter()
        .map(|m| crate::llm::estimate_tokens_text(&m.content))
        .sum();
    let client = crate::llm::http_stream_client();
    let messages: Vec<_> = history
        .iter()
        .map(|m| json!({"role": m.role.as_str(), "content": m.content}))
        .collect();
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
        "stream_options": { "include_usage": true },
    });
    if !cfg.reasoning_effort.trim().is_empty() {
        body["reasoning"] = json!({ "effort": cfg.reasoning_effort.trim() });
    }
    // v0.8: id estable de sesión (caché/trazabilidad del provider).
    for (k, v) in crate::llm::session_body_fields(
        crate::config::Provider::OpenRouter,
        session.unwrap_or(""),
    ) {
        body[k] = json!(v);
    }
    let resp = client
        .post(endpoint(&cfg))
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://arqhia.local")
        .header("X-Title", "ARQHIA")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red OpenRouter: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(400).collect();
        return Err(format!("OpenRouter {status}: {short}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut out_chars = 0usize;
    let mut usage: Option<crate::llm::Usage> = None;
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
            if data == "[DONE]" {
                return Ok(usage.unwrap_or(crate::llm::Usage {
                    input: input_estimate,
                    output: crate::llm::estimate_tokens_chars(out_chars), cached: 0, cost: None,
                }));
            }
            if let Some(u) = crate::llm::parse_openai_usage(data) {
                usage = Some(u);
            }
            if let Some(text) = parse_openai_chunk(data)
                && !text.is_empty() {
                    out_chars += text.chars().count();
                    on_chunk(text);
                }
        }
    }
    Ok(usage.unwrap_or(crate::llm::Usage {
        input: input_estimate,
        output: crate::llm::estimate_tokens_chars(out_chars), cached: 0, cost: None,
    }))
}

pub async fn test_connection(cfg: ProviderConfig) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("API key vacía".to_string());
    }
    let client = crate::llm::http_client();
    let url = format!(
        "{}/api/v1/models",
        cfg.base_url.trim_end_matches('/')
    );
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    if resp.status().is_success() {
        Ok("Conexión OpenRouter OK".to_string())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(300).collect();
        Err(format!("OpenRouter {status}: {short}"))
    }
}
