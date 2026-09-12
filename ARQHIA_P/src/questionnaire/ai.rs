//! Preguntas opcionales generadas por IA (v0.8, bloque saltable).
//!
//! Puro: prompt + parseo. La llamada al provider vive en el handler
//! (usa el provider activo; sin provider el paso se deshabilita).

use super::levels::Level;

/// Prompt para proponer 3–5 preguntas adaptadas a lo respondido.
pub fn ai_prompt(level: Level, summary: &str) -> String {
    format!(
        "Estoy definiendo un proyecto con un asistente. Nivel del usuario: {level}.\n\
        Lo respondido hasta ahora:\n{summary}\n\n\
        Propón entre 3 y 5 preguntas ADICIONALES pertinentes para afinar la \
        especificación (una por línea, sin numerar con paréntesis raros, sin \
        explicaciones). Solo las preguntas, nada más."
    )
}

/// Resumen de una línea por campo para alimentar el prompt.
pub fn answers_summary(
    nombre: &str,
    descripcion: &str,
    objetivo: &str,
    funcionalidades: &str,
    extras: &[(String, String)],
) -> String {
    let mut out = format!(
        "- Nombre: {nombre}\n- Descripción: {descripcion}\n- Objetivo: {objetivo}\n\
        - Funcionalidades: {funcionalidades}"
    );
    for (k, v) in extras {
        if !v.trim().is_empty() {
            out.push_str(&format!("\n- {k}: {v}"));
        }
    }
    out
}

/// Parsea la respuesta del modelo a 3–5 preguntas limpias.
/// Quita numeración (`1.`, `1)`, `-`, `*`), ignora vacías, tope 5.
pub fn parse_ai_questions(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let mut t = line.trim();
        // Quita bullets comunes.
        t = t.trim_start_matches(['-', '*', '•', '>']).trim();
        // Quita numeración "1." / "1)" / "1:".
        let mut chars = t.chars();
        let digits: String = chars.by_ref().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            t = chars.as_str().trim_start_matches(['.', ')', ':', '-']).trim();
        }
        if t.chars().count() >= 4 {
            out.push(t.to_string());
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbered_and_bulleted_questions() {
        let raw = "1. ¿Quién lo usará?\n- ¿Offline o nube?\n* Precio\n\nxx\n";
        let qs = parse_ai_questions(raw);
        assert_eq!(qs.len(), 3);
        assert_eq!(qs[0], "¿Quién lo usará?");
        assert_eq!(qs[1], "¿Offline o nube?");
        assert_eq!(qs[2], "Precio");
    }

    #[test]
    fn caps_at_five_and_skips_short() {
        let raw = (1..=8).map(|i| format!("{i}. Pregunta número {i}")).collect::<Vec<_>>().join("\n");
        assert_eq!(parse_ai_questions(&raw).len(), 5);
        assert!(!parse_ai_questions("hola\n\n  \nok, ¿y el logo?").is_empty());
    }

    #[test]
    fn prompt_mentions_level_and_summary() {
        let p = ai_prompt(Level::Avanzado, "- Nombre: X");
        assert!(p.contains("Avanzado") && p.contains("X"));
    }
}
