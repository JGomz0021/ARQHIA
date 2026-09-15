use crate::config::ProviderConfig;
use crate::llm::{ChatMsg, parse_openai_chunk};
use futures::StreamExt;
use serde_json::json;

fn endpoint(cfg: &ProviderConfig) -> String {
    format!(
        "{}/v1/chat/completions",
        crate::llm::normalize_base_url(&cfg.base_url)
    )
}

fn models_url(cfg: &ProviderConfig) -> String {
    format!(
        "{}/v1/models",
        crate::llm::normalize_base_url(&cfg.base_url)
    )
}

pub async fn chat_stream(
    history: Vec<ChatMsg>,
    cfg: ProviderConfig,
    session: Option<&str>,
    on_chunk: &mut impl FnMut(String),
) -> Result<crate::llm::Usage, String> {
    // v0.8: la sesión no tiene efecto en Local (se acepta y se ignora).
    let _ = session;
    // LM Studio no exige key: si está vacía se omite el header.
    let input_estimate: u32 = history
        .iter()
        .map(|m| crate::llm::estimate_tokens_text(&m.content))
        .sum();
    let client = crate::llm::http_stream_client();
    let messages: Vec<_> = history
        .iter()
        .map(|m| json!({"role": m.role.as_str(), "content": m.content}))
        .collect();
    // Sin `stream_options`: algunos builds de LM Studio lo rechazan. Si el
    // server reporta `usage` igualmente, se usa; si no, se estima.
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
    });
    if !cfg.reasoning_effort.trim().is_empty() {
        body["reasoning_effort"] = json!(cfg.reasoning_effort.trim());
    }
    let mut req = client
        .post(endpoint(&cfg))
        .header("Content-Type", "application/json");
    if !cfg.api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.api_key.trim()));
    }
    let resp = req.json(&body).send().await.map_err(|e| {
        format!(
            "No se pudo contactar LM Studio en {}: {e}. ¿Está el servidor iniciado?",
            cfg.base_url
        )
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(400).collect();
        return Err(format!("LM Studio {status}: {short}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
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
        buf.extend_from_slice(&bytes);
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim();
            if !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" {
                return Ok(usage.unwrap_or(crate::llm::Usage {
                    input: input_estimate,
                    output: crate::llm::estimate_tokens_chars(out_chars),
                    cached: 0,
                    cost: None,
                }));
            }
            if let Some(u) = crate::llm::parse_openai_usage(data) {
                usage = Some(u);
            }
            if let Some(text) = parse_openai_chunk(data)
                && !text.is_empty()
            {
                out_chars += text.chars().count();
                on_chunk(text);
            }
        }
    }
    Ok(usage.unwrap_or(crate::llm::Usage {
        input: input_estimate,
        output: crate::llm::estimate_tokens_chars(out_chars),
        cached: 0,
        cost: None,
    }))
}

pub async fn test_connection(cfg: ProviderConfig) -> Result<String, String> {
    let ids = list_models(cfg).await?;
    if ids.is_empty() {
        Ok("Local OK (sin modelos cargados: carga uno en LM Studio)".to_string())
    } else {
        let shown: String = ids.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        Ok(format!("Local OK ({} modelos: {shown})", ids.len()))
    }
}

/// Lista los ids de modelos cargados en LM Studio (`GET /v1/models`).
pub async fn list_models(cfg: ProviderConfig) -> Result<Vec<String>, String> {
    let client = crate::llm::http_client();
    let mut req = client.get(models_url(&cfg));
    if !cfg.api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", cfg.api_key.trim()));
    }
    let resp = req.send().await.map_err(|e| {
        format!(
            "No se pudo contactar LM Studio en {}: {e}. Inicia el servidor local primero.",
            cfg.base_url
        )
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short: String = text.chars().take(300).collect();
        return Err(format!("LM Studio {status}: {short}"));
    }
    let v: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
    Ok(v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}
