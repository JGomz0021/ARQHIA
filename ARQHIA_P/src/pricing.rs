//! Catálogo de modelos y precios (models.dev).
//!
//! Fuente: `https://models.dev/api.json` (público, sin key). Incluye ~200
//! providers y miles de modelos con precio por 1M tokens (input/output y
//! cache read/write), ventana de contexto y capacidades (tools/reasoning).
//!
//! Se cachea el JSON crudo en disco para no depender de la red. Los precios
//! se consultan por id de modelo (con match por último segmento), de modo que
//! sirven tanto para el navegador de modelos como para el coste por mensaje.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Precio en USD por 1M tokens.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    pub cost: Option<Cost>,
    pub context: Option<u64>,
    pub tool_call: bool,
    pub reasoning: bool,
    /// Valores del modo `effort` (p.ej. low/medium/high/max) si el modelo los expone.
    pub reasoning_efforts: Vec<String>,
    /// El modelo expone un toggle de razonamiento on/off.
    pub reasoning_toggle: bool,
}

/// Vista resumida de un modelo para consultas planas (coste/contexto/niveles).
#[derive(Debug, Clone, Default)]
pub struct ModelLookup {
    pub cost: Option<Cost>,
    pub context: Option<u64>,
    pub efforts: Vec<String>,
    pub toggle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub api: Option<String>,
    pub env: Vec<String>,
    pub models: Vec<ModelInfo>,
}

/// Catálogo completo + índice plano para consultas por modelo.
#[derive(Debug, Clone, Default)]
pub struct Pricing {
    providers: Vec<ProviderEntry>,
    /// id (o último segmento) en minúsculas -> datos del modelo.
    flat: HashMap<String, ModelLookup>,
}

pub fn cache_path() -> PathBuf {
    let home = std::env::var("ARQHIA_HOME")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("arqhia")
        .join("models.dev.json")
}

const REMOTE_URL: &str = "https://models.dev/api.json";

/// Normaliza un id de modelo (quita `:free` y espacios, minúsculas).
fn normalize_model(model: &str) -> String {
    model.trim().split(':').next().unwrap_or("").trim().to_lowercase()
}

/// Coste en USD aplicando precio de cache read a los tokens cacheados.
fn usage_cost(cost: Cost, usage: crate::llm::Usage) -> f64 {
    let cached = usage.cached.min(usage.input) as f64;
    let fresh = usage.input.saturating_sub(usage.cached) as f64;
    let cache_price = cost.cache_read.unwrap_or(cost.input);
    fresh / 1_000_000.0 * cost.input
        + cached / 1_000_000.0 * cache_price
        + usage.output as f64 / 1_000_000.0 * cost.output
}

// --- Deserialización del JSON de models.dev ---------------------------------

#[derive(Deserialize)]
struct RawProvider {
    #[serde(default)]
    name: String,
    #[serde(default)]
    env: Vec<String>,
    #[serde(default)]
    api: Option<String>,
    #[serde(default)]
    models: HashMap<String, RawModel>,
}

#[derive(Deserialize)]
struct RawModel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    tool_call: bool,
    #[serde(default)]
    reasoning: bool,
    #[serde(default)]
    reasoning_options: Vec<RawReasoningOption>,
    #[serde(default)]
    cost: Option<RawCost>,
    #[serde(default)]
    limit: Option<RawLimit>,
}

#[derive(Deserialize)]
struct RawReasoningOption {
    #[serde(rename = "type")]
    kind: String,
    /// models.dev a veces trae `null` en la lista (p.ej. `[null,"low","high"]`).
    #[serde(default)]
    values: Vec<Option<String>>,
}

#[derive(Deserialize)]
struct RawCost {
    #[serde(default)]
    input: f64,
    #[serde(default)]
    output: f64,
    #[serde(default)]
    cache_read: Option<f64>,
    #[serde(default)]
    cache_write: Option<f64>,
}

#[derive(Deserialize)]
struct RawLimit {
    #[serde(default)]
    context: Option<u64>,
}

impl Pricing {
    /// Carga la cache local del JSON; vacío si no existe o está corrupto.
    pub fn load() -> Self {
        match std::fs::read_to_string(cache_path()) {
            Ok(s) => Self::parse(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.flat.len()
    }

    pub fn providers(&self) -> &[ProviderEntry] {
        &self.providers
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// Parsea el JSON de models.dev.
    pub fn parse(json: &str) -> Result<Self, String> {
        let raw: HashMap<String, RawProvider> =
            serde_json::from_str(json).map_err(|e| format!("JSON de modelos inválido: {e}"))?;
        let mut providers = Vec::with_capacity(raw.len());
        let mut flat: HashMap<String, ModelLookup> = HashMap::new();
        for (pid, p) in raw {
            let mut models: Vec<ModelInfo> = p
                .models
                .iter()
                .map(|(mid, m)| {
                    let cost = m.cost.as_ref().map(|c| Cost {
                        input: c.input,
                        output: c.output,
                        cache_read: c.cache_read,
                        cache_write: c.cache_write,
                    });
                    let context = m.limit.as_ref().and_then(|l| l.context).filter(|c| *c > 0);
                    let mut reasoning_efforts = Vec::new();
                    let mut reasoning_toggle = false;
                    for o in &m.reasoning_options {
                        match o.kind.as_str() {
                            "effort" if o.values.iter().any(|v| v.is_some()) => {
                                reasoning_efforts = o
                                    .values
                                    .iter()
                                    .filter_map(|v| v.clone())
                                    .collect();
                            }
                            "toggle" => reasoning_toggle = true,
                            _ => {}
                        }
                    }
                    ModelInfo {
                        id: mid.clone(),
                        name: if m.name.is_empty() { mid.clone() } else { m.name.clone() },
                        family: m.family.clone(),
                        cost,
                        context,
                        tool_call: m.tool_call,
                        reasoning: m.reasoning,
                        reasoning_efforts,
                        reasoning_toggle,
                    }
                })
                .collect();
            models.sort_by(|a, b| a.id.cmp(&b.id));
            // Índice plano: id completo y último segmento.
            for m in &models {
                let entry = ModelLookup {
                    cost: m.cost,
                    context: m.context,
                    efforts: m.reasoning_efforts.clone(),
                    toggle: m.reasoning_toggle,
                };
                let key = m.id.to_lowercase();
                flat.entry(key.clone()).or_insert_with(|| entry.clone());
                if let Some(last) = key.rsplit('/').next() {
                    flat.entry(last.to_string()).or_insert_with(|| entry.clone());
                }
            }
            providers.push(ProviderEntry {
                id: pid.clone(),
                name: if p.name.is_empty() { pid } else { p.name },
                api: p.api,
                env: p.env,
                models,
            });
        }
        providers.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self { providers, flat })
    }

    pub fn save_raw(json: &str) -> Result<(), String> {
        let path = cache_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Datos planos de un modelo (id completo o último segmento).
    pub fn lookup(&self, model: &str) -> Option<&ModelLookup> {
        let q = normalize_model(model);
        if q.is_empty() {
            return None;
        }
        if let Some(v) = self.flat.get(&q) {
            return Some(v);
        }
        let last = q.rsplit('/').next().unwrap_or(q.as_str()).to_string();
        if let Some(v) = self.flat.get(&last) {
            return Some(v);
        }
        self.flat.iter().find(|(k, _)| k.contains(&last)).map(|(_, v)| v)
    }

    #[allow(dead_code)]
    pub fn context_window(&self, model: &str) -> Option<u64> {
        self.lookup(model).and_then(|m| m.context)
    }

    /// Niveles de razonamiento que expone el modelo: (valores effort, toggle).
    #[allow(dead_code)]
    pub fn reasoning(&self, model: &str) -> (Vec<String>, bool) {
        match self.lookup(model) {
            Some(m) => (m.efforts.clone(), m.toggle),
            None => (Vec::new(), false),
        }
    }

    /// Opciones para el selector de nivel: `auto` + niveles del modelo.
    /// Si el modelo solo expone toggle, se ofrece `auto`/`on`.
    #[allow(dead_code)]
    pub fn effort_choices(&self, model: &str) -> Vec<String> {
        let (efforts, toggle) = self.reasoning(model);
        let mut v = vec!["auto".to_string()];
        if !efforts.is_empty() {
            v.extend(efforts);
        } else if toggle {
            v.push("on".to_string());
        }
        v
    }

    /// Modelo dentro de un provider concreto (evita colisiones de nombre entre
    /// providers: p.ej. `gpt-4o` existe en varios).
    pub fn model_in(&self, provider_id: Option<&str>, model: &str) -> Option<&ModelInfo> {
        let q = normalize_model(model);
        if q.is_empty() {
            return None;
        }
        let pid = provider_id?;
        let p = self.provider(pid)?;
        if let Some(m) = p.models.iter().find(|m| m.id.to_lowercase() == q) {
            return Some(m);
        }
        let last = q.rsplit('/').next().unwrap_or(q.as_str());
        p.models
            .iter()
            .find(|m| m.id.to_lowercase().rsplit('/').next() == Some(last))
    }

    /// Datos por modelo, priorizando el provider correcto y cayendo al índice global.
    pub fn lookup_in(&self, provider_id: Option<&str>, model: &str) -> Option<ModelLookup> {
        if let Some(m) = self.model_in(provider_id, model) {
            return Some(ModelLookup {
                cost: m.cost,
                context: m.context,
                efforts: m.reasoning_efforts.clone(),
                toggle: m.reasoning_toggle,
            });
        }
        self.lookup(model).cloned()
    }

    /// Opciones de nivel priorizando el provider del modelo.
    pub fn effort_choices_in(&self, provider_id: Option<&str>, model: &str) -> Vec<String> {
        let (efforts, toggle) = match self.lookup_in(provider_id, model) {
            Some(m) => (m.efforts, m.toggle),
            None => (Vec::new(), false),
        };
        let mut v = vec!["auto".to_string()];
        if !efforts.is_empty() {
            v.extend(efforts);
        } else if toggle {
            v.push("on".to_string());
        }
        v
    }

    pub fn context_window_in(&self, provider_id: Option<&str>, model: &str) -> Option<u64> {
        self.lookup_in(provider_id, model).and_then(|m| m.context)
    }

    pub fn cost_in(
        &self,
        provider_id: Option<&str>,
        model: &str,
        usage: crate::llm::Usage,
    ) -> Option<f64> {
        let cost = self.lookup_in(provider_id, model)?.cost?;
        Some(usage_cost(cost, usage))
    }

    /// Coste en USD, considerando cache read si el proveedor la reporta.
    #[allow(dead_code)]
    pub fn cost(&self, model: &str, usage: crate::llm::Usage) -> Option<f64> {
        Some(usage_cost(self.lookup(model)?.cost?, usage))
    }

    /// Descarga la tabla y la cachea. Devuelve el número de modelos.
    pub async fn fetch() -> Result<(Self, usize), String> {
        let resp = crate::llm::http_client()
            .get(REMOTE_URL)
            .send()
            .await
            .map_err(|e| format!("No se pudo descargar models.dev: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("models.dev: HTTP {}", resp.status()));
        }
        let text = resp.text().await.map_err(|e| e.to_string())?;
        let pricing = Self::parse(&text)?;
        if pricing.is_empty() {
            return Err("models.dev devolvió un catálogo vacío.".to_string());
        }
        let n = pricing.len();
        Self::save_raw(&text)?;
        Ok((pricing, n))
    }
}

/// Id de provider en models.dev a partir del provider de ARQHIA y su base_url
/// (permite reconocer Groq/Google/etc. cuando se usa el provider OpenAI con
/// una URL custom).
pub fn provider_id_for(provider: crate::config::Provider, base_url: &str) -> Option<String> {
    let host = base_url.to_lowercase();
    for (needle, id) in [
        ("groq", "groq"),
        ("openrouter", "openrouter"),
        ("anthropic", "anthropic"),
        ("openai", "openai"),
        ("mistral", "mistral"),
        ("deepseek", "deepseek"),
        ("google", "google"),
        ("generativelanguage", "google"),
        ("xai", "xai"),
        ("together", "togetherai"),
        ("fireworks", "fireworks-ai"),
        ("cerebras", "cerebras"),
        ("perplexity", "perplexity"),
    ] {
        if host.contains(needle) {
            return Some(id.to_string());
        }
    }
    match provider {
        crate::config::Provider::OpenAI => Some("openai".to_string()),
        crate::config::Provider::Anthropic => Some("anthropic".to_string()),
        crate::config::Provider::OpenRouter => Some("openrouter".to_string()),
        crate::config::Provider::Local => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Usage;

    const SAMPLE: &str = r#"{
        "openai": {
            "name": "OpenAI",
            "env": ["OPENAI_API_KEY"],
            "models": {
                "gpt-4o": {
                    "name": "GPT-4o",
                    "family": "gpt",
                    "tool_call": true,
                    "reasoning": true,
                    "reasoning_options": [{"type":"effort","values":["low","medium","high"]}],
                    "cost": {"input": 2.5, "output": 10.0, "cache_read": 1.25},
                    "limit": {"context": 128000, "output": 16384}
                }
            }
        },
        "groq": {
            "name": "Groq",
            "env": ["GROQ_API_KEY"],
            "models": {
                "groq/llama-3.1-8b-instant": {
                    "name": "Llama 3.1 8B",
                    "tool_call": true,
                    "cost": {"input": 0.05, "output": 0.08},
                    "limit": {"context": 131072}
                }
            }
        }
    }"#;

    #[test]
    fn parses_catalog_and_flat_lookup() {
        let p = Pricing::parse(SAMPLE).unwrap();
        assert_eq!(p.providers().len(), 2);
        assert!(p.provider("openai").is_some());
        // lookup por id y por último segmento (sin provider ni :free).
        assert!(p.lookup("gpt-4o").is_some());
        assert!(p.lookup("openai/gpt-4o").is_some());
        assert!(p.lookup("gpt-4o:free").is_some());
        assert!(p.lookup("llama-3.1-8b-instant").is_some());
        assert_eq!(p.context_window("gpt-4o"), Some(128_000));
        let (efforts, toggle) = p.reasoning("gpt-4o");
        assert_eq!(efforts, vec!["low", "medium", "high"]);
        assert!(!toggle);
        assert!(p.reasoning("modelo-raro").0.is_empty());
    }

    #[test]
    fn cost_accounts_for_cache_read() {
        let p = Pricing::parse(SAMPLE).unwrap();
        let usage = Usage { input: 2000, output: 500, cached: 1000, cost: None };
        let expected = 1000.0 / 1e6 * 2.5 + 1000.0 / 1e6 * 1.25 + 500.0 / 1e6 * 10.0;
        assert!((p.cost("gpt-4o", usage).unwrap() - expected).abs() < 1e-12);
        assert!(p.cost("modelo-raro", usage).is_none());
    }

    #[test]
    fn provider_mapping_uses_base_url() {
        use crate::config::Provider;
        assert_eq!(
            provider_id_for(Provider::OpenAI, "https://api.groq.com/openai").as_deref(),
            Some("groq")
        );
        assert_eq!(
            provider_id_for(Provider::OpenAI, "https://api.openai.com").as_deref(),
            Some("openai")
        );
        assert_eq!(provider_id_for(Provider::Local, "http://localhost:1234"), None);
    }

    /// Parsea el api.json real de models.dev si está en /tmp (se descarga con
    /// `curl -s https://models.dev/api.json -o /tmp/opencode/modelsdev.json`).
    #[test]
    #[ignore = "requiere /tmp/opencode/modelsdev.json"]
    fn parses_real_modelsdev_catalog() {
        let Ok(json) = std::fs::read_to_string("/tmp/opencode/modelsdev.json") else {
            return;
        };
        let p = Pricing::parse(&json).unwrap();
        assert!(p.len() > 3_000, "modelos indexados: {}", p.len());
        assert!(p.providers().len() > 100, "providers: {}", p.providers().len());
        assert!(p.lookup("gpt-4o").is_some());
        assert!(p.lookup("claude-sonnet-4-6").is_some());
    }
}
