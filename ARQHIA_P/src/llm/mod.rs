pub mod anthropic;
pub mod local;
pub mod openai;
pub mod openrouter;

use crate::config::{Provider, ProviderConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => Role::User,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMsg {
    pub role: Role,
    pub content: String,
}

/// Uso de tokens de una llamada (input = prompt, output = completion).
/// `cached` es el subconjunto de `input` servido desde cache (si el
/// proveedor lo reporta), que suele costar menos. `cost` es el coste real
/// reportado por el proveedor (OpenRouter, algunos OpenAI-compat), si lo hay.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Usage {
    pub input: u32,
    pub output: u32,
    pub cached: u32,
    pub cost: Option<f64>,
}

/// Estimador de tokens por caracteres (chars/4, redondeado hacia arriba).
/// Se usa cuando el proveedor no reporta `usage` en el stream.
pub fn estimate_tokens_chars(n: usize) -> u32 {
    n.div_ceil(4) as u32
}

pub fn estimate_tokens_text(s: &str) -> u32 {
    estimate_tokens_chars(s.chars().count())
}

/// Ventana de contexto orientativa por provider/modelo (tokens).
pub fn context_window(provider: Provider, model: &str) -> u64 {
    if provider == Provider::Local {
        return 32_768;
    }
    let m = model.to_lowercase();
    if m.contains("claude") {
        200_000
    } else if m.contains("gpt-4o") || m.contains("gpt-4.1") || m.contains("o3") {
        128_000
    } else if m.contains("gemini") {
        1_000_000
    } else {
        128_000
    }
}

/// Coste aproximado en USD a partir de una tabla de precios por 1M tokens.
/// Devuelve `None` si no conocemos el modelo (no inventamos cifras).
pub fn estimate_cost_usd(provider: Provider, model: &str, usage: Usage) -> Option<f64> {
    if provider == Provider::Local {
        return Some(0.0); // local: sin coste de API
    }
    let m = model.to_lowercase();
    let (input, output): (f64, f64) = if m.contains("gpt-4o-mini") {
        (0.15, 0.60)
    } else if m.contains("gpt-4o") {
        (2.50, 10.00)
    } else if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") {
        (3.00, 15.00)
    } else if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") {
        (0.80, 4.00)
    } else if m.contains("claude-3-opus") {
        (15.00, 75.00)
    } else {
        return None;
    };
    Some((usage.input as f64 / 1_000_000.0) * input + (usage.output as f64 / 1_000_000.0) * output)
}

/// Coste legible; `None` => "—".
pub fn format_cost(cost: Option<f64>) -> String {
    match cost {
        None => "—".to_string(),
        Some(c) if c < 0.01 => format!("${c:.5}"),
        Some(c) => format!("${c:.4}"),
    }
}

/// Formatea un contador de tokens de forma compacta (`1.2k`).
pub fn format_tokens(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.0}k", n as f64 / 1000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

/// Prompt system de ARQHIA: quién es, qué puede hacer y cómo responder.
/// Se envía al inicio de cada conversación (chat plano y agente).
/// `model_desc` ej. "llama-3.1-8b-instant (OpenAI)".
pub fn system_identity(model_desc: &str, has_tools: bool) -> String {
    let tools_block = if has_tools {
        "En este chat TIENES HERRAMIENTAS sobre un workspace asignado: leer, crear, editar y borrar archivos, listar carpetas, buscar texto y ejecutar comandos permitidos (incluido cargo). Úsalas en vez de pedirle al usuario que pegue código."
    } else {
        "En este chat NO tienes herramientas: no puedes ver ni tocar archivos. Si el usuario quiere que trabajes sobre su código, pídele que asigne un workspace al proyecto."
    };
    format!(
        "Eres ARQHIA, el asistente de IA integrado en la app de escritorio ARQHIA (nativa, Rust + Iced).\n\
        \n\
        Qué es ARQHIA: acompaña al usuario desde la idea hasta el código. Flujo: crear/abrir proyecto -> cuestionario guiado que genera ESPEC.md -> chat -> agente de código con workspace, planificador, workers generadores y auditor que deja hallazgos en CONTEXT/TEMP.md.\n\
        \n\
        {tools_block}\n\
        \n\
        Cómo responder:\n\
        - Idioma del usuario (por defecto español).\n\
        - Markdown siempre que ayude: encabezados, listas y bloques de código con lenguaje.\n\
        - Directo y útil; si falta información clave para actuar, pregunta antes de suponer.\n\
        - Estás corriendo como: {model_desc}."
    )
}

/// Normaliza base_url: quita sufijos que el usuario suele pegar de más
/// ("/v1/chat/completions", "/v1/models", "/chat/completions") y "/" final.
/// Así "https://api.groq.com/openai/v1/models" -> "https://api.groq.com/openai".
pub fn normalize_base_url(url: &str) -> String {
    let mut s = url.trim().trim_end_matches('/').to_string();
    for suffix in [
        "/api/v1/chat/completions",
        "/api/v1/models",
        "/v1/chat/completions",
        "/v1/models",
        "/chat/completions",
        "/models",
        "/api/v1",
        "/v1",
    ] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
            break;
        }
    }
    s.trim_end_matches('/').to_string()
}

/// Cliente HTTP compartido con timeout para no dejar la UI colgada.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Envía chunks de texto por el callback. Retorna el uso de tokens si se pudo
/// determinar (real del proveedor o estimado `chars/4`).
pub async fn chat_stream(
    provider: Provider,
    history: Vec<ChatMsg>,
    cfg: ProviderConfig,
    mut on_chunk: impl FnMut(String),
) -> Result<Usage, String> {
    match provider {
        Provider::OpenAI => openai::chat_stream(history, cfg, &mut on_chunk).await,
        Provider::Anthropic => anthropic::chat_stream(history, cfg, &mut on_chunk).await,
        Provider::OpenRouter => openrouter::chat_stream(history, cfg, &mut on_chunk).await,
        Provider::Local => local::chat_stream(history, cfg, &mut on_chunk).await,
    }
}

pub async fn test_connection(provider: Provider, cfg: ProviderConfig) -> Result<String, String> {
    match provider {
        Provider::OpenAI => openai::test_connection(cfg).await,
        Provider::Anthropic => anthropic::test_connection(cfg).await,
        Provider::OpenRouter => openrouter::test_connection(cfg).await,
        Provider::Local => local::test_connection(cfg).await,
    }
}

/// Modelos disponibles en un servidor local OpenAI-compatible (LM Studio).
pub async fn list_local_models(cfg: ProviderConfig) -> Result<Vec<String>, String> {
    local::list_models(cfg).await
}

/// Extrae `content` de un chunk SSE OpenAI-compatible:
/// `data: {"choices":[{"delta":{"content":"hola"}}]}`
pub fn parse_openai_chunk(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("choices")?
        .as_array()?
        .first()?
        .get("delta")?
        .get("content")?
        .as_str()
        .map(|s| s.to_string())
}

/// Extrae `usage` de un chunk SSE OpenAI-compatible (con
/// `stream_options.include_usage`, el último chunk trae prompt/completion).
pub fn parse_openai_usage(json: &str) -> Option<Usage> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let u = v.get("usage")?;
    let input = u.get("prompt_tokens")?.as_u64()? as u32;
    let output = u.get("completion_tokens")?.as_u64()? as u32;
    let cached = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;
    // Coste real si el proveedor lo reporta (OpenRouter: `cost`; otros: `total_cost`).
    let cost = u
        .get("cost")
        .and_then(|c| c.as_f64())
        .or_else(|| u.get("total_cost").and_then(|c| c.as_f64()));
    Some(Usage { input, output, cached, cost })
}

/// Anthropic `message_start`: `message.usage` con input, cache read/creation.
/// Devuelve el input total (incluye cache) y cuánto vino de cache.
pub fn parse_anthropic_start_usage(json: &str) -> Option<Usage> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let u = v.get("message")?.get("usage")?;
    let fresh = u.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let cache_read = u
        .get("cache_read_input_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let cache_write = u
        .get("cache_creation_input_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    Some(Usage {
        input: (fresh + cache_read + cache_write) as u32,
        output: 0,
        cached: cache_read as u32,
        cost: None,
    })
}

/// Anthropic `message_delta`: `usage.output_tokens`.
pub fn parse_anthropic_output_usage(json: &str) -> Option<u32> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    Some(v.get("usage")?.get("output_tokens")?.as_u64()? as u32)
}

/// Extrae texto de un chunk SSE Anthropic:
/// `data: {"type":"content_block_delta","delta":{"text":"hola"}}`
pub fn parse_anthropic_chunk(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v.get("type")?.as_str()? != "content_block_delta" {
        return None;
    }
    v.get("delta")?.get("text")?.as_str().map(|s| s.to_string())
}

/// Convierte un error crudo del proveedor (a menudo JSON volcado) en un
/// mensaje legible y corto para la barra de estado. El detalle técnico
/// completo sigue yendo al Log.
pub fn friendly_error(raw: &str) -> String {
    let low = raw.to_lowercase();
    if low.contains("429") || (low.contains("rate") && low.contains("limit")) {
        return "Proveedor saturado (límite 429): espera unos segundos y reintenta. \
            En OpenRouter los modelos gratuitos tienen cuota baja; si persiste, \
            usa otro modelo o añade tu propia key."
            .to_string();
    }
    if low.contains("401") || low.contains("invalid api key") || low.contains("unauthorized") {
        return "API key rechazada (401): revísala en Configuración.".to_string();
    }
    if low.contains("402") || low.contains("credit") || low.contains("payment") {
        return "Sin crédito en el proveedor (402): revisa tu saldo.".to_string();
    }
    if low.contains("404") || low.contains("not found") || low.contains("no endpoints") {
        return "Modelo no disponible (404): revisa el nombre del modelo en Configuración."
            .to_string();
    }
    if low.contains("timeout") || low.contains("timed out") {
        return "Tiempo de espera agotado: el proveedor no respondió. Reintenta.".to_string();
    }
    // Genérico: corta el volcado a una línea legible.
    let one: String = raw.replace('\n', " ").chars().take(220).collect();
    if raw.chars().count() > 220 {
        format!("{one}…")
    } else {
        one
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_delta() {
        let j = r#"{"choices":[{"delta":{"content":"hola"}}]}"#;
        assert_eq!(parse_openai_chunk(j).as_deref(), Some("hola"));
    }

    #[test]
    fn parse_openai_ignores_role_only() {
        let j = r#"{"choices":[{"delta":{"role":"assistant"}}]}"#;
        assert_eq!(parse_openai_chunk(j), None);
    }

    #[test]
    fn parse_anthropic_delta() {
        let j = r#"{"type":"content_block_delta","delta":{"text":"mundo"}}"#;
        assert_eq!(parse_anthropic_chunk(j).as_deref(), Some("mundo"));
    }

    #[test]
    fn parse_anthropic_ignores_other_types() {
        let j = r#"{"type":"message_start","message":{}}"#;
        assert_eq!(parse_anthropic_chunk(j), None);
    }

    #[test]
    fn role_system_roundtrip() {
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::from_str("system"), Role::System);
        assert_eq!(Role::from_str("user"), Role::User);
    }

    #[test]
    fn system_identity_mentions_arqhia_and_model() {
        let plain = system_identity("qwen3-8b (Local (LM Studio))", false);
        assert!(plain.contains("ARQHIA"));
        assert!(plain.contains("qwen3-8b"));
        assert!(plain.contains("NO tienes herramientas"));
        let agent = system_identity("gpt-4o-mini (OpenAI)", true);
        assert!(agent.contains("HERRAMIENTAS"));
        assert!(!agent.contains("NO tienes herramientas"));
    }

    #[test]
    fn friendly_error_maps_statuses() {
        let r429 = "429 Too Many Requests: {\"error\":{\"message\":\"Provider returned error\",\"code\":429}}";
        let f = friendly_error(r429);
        assert!(f.contains("429"), "429 debe mencionarse: {f}");
        assert!(!f.contains('{'), "no debe volcar JSON: {f}");
        assert!(friendly_error("401 Unauthorized: bad key").contains("401"));
        assert!(friendly_error("OpenRouter 404: No endpoints found").contains("404"));
        assert!(friendly_error("boom").contains("boom"));
    }

    #[test]
    fn normalize_strips_pasted_suffixes() {
        assert_eq!(
            normalize_base_url("https://api.groq.com/openai/v1/models"),
            "https://api.groq.com/openai"
        );
        assert_eq!(
            normalize_base_url("https://api.groq.com/openai/"),
            "https://api.groq.com/openai"
        );
        assert_eq!(
            normalize_base_url("https://api.openai.com"),
            "https://api.openai.com"
        );
        assert_eq!(
            normalize_base_url("https://openrouter.ai/api/v1/models"),
            "https://openrouter.ai"
        );
    }

    #[test]
    fn usage_parsing_and_estimates() {
        assert_eq!(
            parse_openai_usage(r#"{"usage":{"prompt_tokens":10,"completion_tokens":20}}"#),
            Some(Usage { input: 10, output: 20, cached: 0, cost: None })
        );
        assert_eq!(
            parse_openai_usage(
                r#"{"usage":{"prompt_tokens":10,"completion_tokens":20,"prompt_tokens_details":{"cached_tokens":6}}}"#
            ),
            Some(Usage { input: 10, output: 20, cached: 6, cost: None })
        );
        assert_eq!(parse_openai_usage(r#"{"choices":[]}"#), None);
        assert_eq!(
            parse_anthropic_start_usage(
                r#"{"type":"message_start","message":{"usage":{"input_tokens":7,"cache_read_input_tokens":3}}}"#
            ),
            Some(Usage { input: 10, output: 0, cached: 3, cost: None })
        );
        assert_eq!(
            parse_anthropic_output_usage(
                r#"{"type":"message_delta","usage":{"output_tokens":9}}"#
            ),
            Some(9)
        );
        assert_eq!(estimate_tokens_text(""), 0);
        assert_eq!(estimate_tokens_text("abcd"), 1);
        assert_eq!(estimate_tokens_chars(5), 2);
    }

    #[test]
    fn cost_and_context_helpers() {
        assert_eq!(context_window(Provider::Local, "x"), 32_768);
        assert_eq!(context_window(Provider::OpenAI, "claude-3-5-sonnet"), 200_000);
        assert_eq!(context_window(Provider::OpenAI, "gpt-4o-mini"), 128_000);
        let million = Usage { input: 1_000_000, output: 0, cached: 0, cost: None };
        assert_eq!(estimate_cost_usd(Provider::OpenAI, "gpt-4o", million), Some(2.5));
        assert_eq!(estimate_cost_usd(Provider::Local, "x", million), Some(0.0));
        assert_eq!(estimate_cost_usd(Provider::OpenAI, "modelo-raro", million), None);
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(12_000), "12k");
        assert_eq!(format_cost(None), "—");
    }
}
