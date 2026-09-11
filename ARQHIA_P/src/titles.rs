//! Títulos de chats: truncado seguro para cabeceras y filas. Puro, con tests.
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
}
