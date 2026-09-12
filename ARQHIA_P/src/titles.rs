//! Títulos de chats: truncado seguro para cabeceras y filas. Puro, con tests.

/// Limpia un título propuesto por la IA (v0.7.4): 1 línea, sin comillas,
/// máx 40 chars; vacío → fallback al primer mensaje.
pub fn sanitize_ai_title(raw: &str, fallback_msg: &str) -> String {
    let one: String = raw
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(['"', '\'', '«', '»', '.', '!', '¡', '?', '¿'])
        .trim()
        .to_string();
    let short: String = one.chars().take(40).collect();
    if short.trim().is_empty() {
        title_for(fallback_msg)
    } else {
        short
    }
}

pub fn title_for(first_msg: &str) -> String {
    let t = first_msg.trim();
    if t.is_empty() {
        return "Nuevo chat".to_string();
    }
    let short: String = t.chars().take(30).collect();
    if t.chars().count() > 30 {
        format!("{short}…")
    } else {
        short
    }
}

pub fn chat_label(title: &str) -> String {
    let short: String = title.chars().take(28).collect();
    if title.chars().count() > 28 {
        format!("{short}…")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_truncates_to_30() {
        assert_eq!(title_for(""), "Nuevo chat");
        assert_eq!(title_for("hola"), "hola");
        let long = "abcdefghijklmnopqrstuvwxyz0123456789EXTRA";
        let t = title_for(long);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), 31);
    }

    #[test]
    fn ai_title_sanitizes_and_falls_back() {
        assert_eq!(sanitize_ai_title("\"Mi proyecto web\"\nsegunda línea", "hola"), "Mi proyecto web");
        assert_eq!(sanitize_ai_title("   ", "hola mundo"), "hola mundo");
        assert_eq!(sanitize_ai_title("", ""), "Nuevo chat");
        let long = "a".repeat(100);
        assert_eq!(sanitize_ai_title(&long, "x").chars().count(), 40);
    }
}
