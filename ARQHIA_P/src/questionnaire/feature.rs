#![allow(dead_code)]
//! Incorporación de nuevas funcionalidades (§3 de la reestructuración).
//!
//! Cuando el usuario quiere añadir una funcionalidad después de crear el
//! proyecto, **no se modifica directamente el CONTEXT**.
//! Flujo obligatorio:
//! 1. Conversación específica para comprender la funcionalidad.
//! 2. Análisis de impacto.
//! 3. Decisión de ubicación (versión existente vs incremental).
//! 4. Actualización coherente de todo el CONTEXT.

/// Datos recogidos en la conversación de nueva funcionalidad (§3).
#[derive(Debug, Clone, Default)]
pub struct FeatureRequest {
    pub que_hace: String,
    pub para_quien: String,
    pub como_se_utilizara: String,
    pub por_que: String,
    pub problema_resuelve: String,
    pub requisitos_funcionales: String,
    pub requisitos_tecnicos: String,
    pub dependencias: String,
    pub integracion_existentes: String,
    pub impacto_arquitectura: String,
    pub impacto_flujos: String,
    pub impacto_otras_versiones: String,
    pub tests_qa: String,
}

impl FeatureRequest {
    pub fn is_complete(&self) -> bool {
        !self.que_hace.trim().is_empty()
            && !self.para_quien.trim().is_empty()
            && !self.requisitos_funcionales.trim().is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Funcionalidad: {}\nPara: {}\nUso: {}\nPor qué: {}\nProblema: {}\nFuncionales: {}\nTécnicos: {}\nDep: {}\nIntegración: {}\nImpacto arq: {}\nImpacto flujos: {}\nImpacto versiones: {}\nTests/QA: {}",
            none(&self.que_hace),
            none(&self.para_quien),
            none(&self.como_se_utilizara),
            none(&self.por_que),
            none(&self.problema_resuelve),
            none(&self.requisitos_funcionales),
            none(&self.requisitos_tecnicos),
            none(&self.dependencias),
            none(&self.integracion_existentes),
            none(&self.impacto_arquitectura),
            none(&self.impacto_flujos),
            none(&self.impacto_otras_versiones),
            none(&self.tests_qa),
        )
    }
}

fn none(s: &str) -> String {
    if s.trim().is_empty() {
        "(pendiente)".to_string()
    } else {
        s.trim().to_string()
    }
}

/// Decisión de ubicación de la funcionalidad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionPlacement {
    /// Cabe en una versión existente (ej. v0.3 aún no cerrada).
    Existing { version: String },
    /// Requiere versión incremental (ej. v0.6.1, v0.8.3).
    Incremental { version: String },
}

impl VersionPlacement {
    /// Sugiere versión incremental si no encaja en existentes.
    /// `existing` = lista de versiones ya definidas (ej. ["v0.1","v0.2","v0.3"]).
    /// Si `fits_existing` es false, crea `v{major}.{minor+1}` o `v{major}.{minor}.1`.
    pub fn decide(fits_existing: bool, target: &str, existing: &[String]) -> Self {
        if fits_existing && existing.contains(&target.to_string()) {
            VersionPlacement::Existing { version: target.to_string() }
        } else {
            // incremental: si target existe, añade .1; si no, usa target
            let inc = if existing.contains(&target.to_string()) {
                format!("{target}.1")
            } else {
                target.to_string()
            };
            VersionPlacement::Incremental { version: inc }
        }
    }

    pub fn version(&self) -> &str {
        match self {
            VersionPlacement::Existing { version } => version,
            VersionPlacement::Incremental { version } => version,
        }
    }
}

/// Documentos del CONTEXT que deben revisarse al incorporar una funcionalidad (§3).
pub fn context_docs_to_update(placement: &VersionPlacement) -> Vec<&'static str> {
    let mut docs = vec!["PROJECT.md", "SPECS.md", "ROADMAP.md", "VERSIONS.md"];
    match placement {
        VersionPlacement::Existing { .. } => docs.push("VERSIONS/{version}.md (existente)"),
        VersionPlacement::Incremental { .. } => docs.push("VERSIONS/{new_version}.md (nuevo)"),
    }
    docs.push("CONTEXT.md (índice + siguiente paso)");
    docs
}

/// Principio fundamental del flujo (§4):
/// Usuario → Cuestionario → Preguntas IA → CONTEXT → Plan → Implementación → Auditoría → Actualización CONTEXT
pub fn flow_principle() -> &'static str {
    "Usuario → Cuestionario → Preguntas IA (5–10) → CONTEXT (SSOT) → Plan → Implementación → Auditoría → Actualización del CONTEXT"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_completeness() {
        let mut f = FeatureRequest::default();
        assert!(!f.is_complete());
        f.que_hace = "Chat en tiempo real".to_string();
        f.para_quien = "usuarios finales".to_string();
        f.requisitos_funcionales = "enviar/recibir mensajes".to_string();
        assert!(f.is_complete());
    }

    #[test]
    fn placement_logic() {
        let existing = vec!["v0.1".to_string(), "v0.2".to_string()];
        let p = VersionPlacement::decide(true, "v0.2", &existing);
        assert_eq!(p, VersionPlacement::Existing { version: "v0.2".to_string() });
        let p2 = VersionPlacement::decide(false, "v0.2", &existing);
        assert_eq!(p2.version(), "v0.2.1");
        let p3 = VersionPlacement::decide(false, "v0.3", &existing);
        assert_eq!(p3.version(), "v0.3");
    }

    #[test]
    fn docs_list_covers_context() {
        let p = VersionPlacement::Existing { version: "v0.3".to_string() };
        let docs = context_docs_to_update(&p);
        assert!(docs.contains(&"PROJECT.md"));
        assert!(docs.contains(&"SPECS.md"));
        assert!(docs.contains(&"ROADMAP.md"));
    }

    #[test]
    fn flow_principle_is_chain() {
        assert!(flow_principle().contains("CONTEXT"));
        assert!(flow_principle().contains("Auditoría"));
    }
}
