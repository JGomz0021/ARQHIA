//! Historial de chat alineado (v0.9.5).
//!
//! Los vectores paralelos (`messages`, `md`, `msg_usage`, `msg_times`,
//! `msg_ids`) deben moverse juntos: un push/clear/truncate parcial los
//! desalineaba (bugs de undo/rama). `ChatHistory` es el tipo puro y testeable;
//! `App` delega en él mediante `history_*` (ver `state.rs`).

use iced::widget::markdown;

use crate::llm::{ChatMsg, Usage};

/// Historial con sus paralelos siempre alineados por índice.
///
/// Tipo puro y testeable (v0.9.5): la app en producción usa los wrappers
/// atómicos `App::history_*` (mismo invariante); este tipo existe para
/// verificar el invariante en tests sin montar un `App` entero.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ChatHistory {
    pub messages: Vec<ChatMsg>,
    pub md: Vec<Vec<markdown::Item>>,
    pub msg_usage: Vec<Option<Usage>>,
    pub msg_times: Vec<String>,
    pub msg_ids: Vec<i64>,
}

#[allow(dead_code)]
impl ChatHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Invariante: los 5 vectores miden lo mismo.
    pub fn is_aligned(&self) -> bool {
        let n = self.messages.len();
        self.md.len() == n
            && self.msg_usage.len() == n
            && self.msg_times.len() == n
            && self.msg_ids.len() == n
    }

    fn debug_assert_aligned(&self) {
        debug_assert!(
            self.is_aligned(),
            "ChatHistory desalineado: messages={} md={} usage={} times={} ids={}",
            self.messages.len(),
            self.md.len(),
            self.msg_usage.len(),
            self.msg_times.len(),
            self.msg_ids.len()
        );
    }

    /// Push atómico de una entrada completa.
    pub fn push(
        &mut self,
        msg: ChatMsg,
        md_items: Vec<markdown::Item>,
        usage: Option<Usage>,
        time: String,
        id: i64,
    ) {
        self.messages.push(msg);
        self.md.push(md_items);
        self.msg_usage.push(usage);
        self.msg_times.push(time);
        self.msg_ids.push(id);
        self.debug_assert_aligned();
    }

    /// Pop atómico (None si vacío).
    pub fn pop(&mut self) -> Option<ChatMsg> {
        let msg = self.messages.pop()?;
        self.md.pop();
        self.msg_usage.pop();
        self.msg_times.pop();
        self.msg_ids.pop();
        self.debug_assert_aligned();
        Some(msg)
    }

    /// Clear atómico.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.md.clear();
        self.msg_usage.clear();
        self.msg_times.clear();
        self.msg_ids.clear();
        self.debug_assert_aligned();
    }

    /// Truncate atómico a `n` (si `n >= len`, no hace nada).
    pub fn truncate(&mut self, n: usize) {
        self.messages.truncate(n);
        self.md.truncate(n);
        self.msg_usage.truncate(n);
        self.msg_times.truncate(n);
        self.msg_ids.truncate(n);
        self.debug_assert_aligned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Role;

    fn msg(content: &str) -> ChatMsg {
        ChatMsg {
            role: Role::Assistant,
            content: content.to_string(),
        }
    }

    #[test]
    fn push_pop_truncate_stay_aligned() {
        let mut h = ChatHistory::new();
        for i in 0..10 {
            h.push(msg(&format!("m{i}")), Vec::new(), None, String::new(), 0);
            assert!(h.is_aligned());
        }
        assert_eq!(h.len(), 10);
        h.truncate(4);
        assert!(h.is_aligned());
        assert_eq!(h.len(), 4);
        assert_eq!(h.messages[3].content, "m3");
        // Truncate más allá del len: no-op alineado.
        h.truncate(99);
        assert!(h.is_aligned() && h.len() == 4);
        // Pop hasta vaciar: nunca desalinea.
        for _ in 0..4 {
            assert!(h.pop().is_some());
            assert!(h.is_aligned());
        }
        assert!(h.pop().is_none());
        assert!(h.is_aligned());
        h.clear();
        assert!(h.is_aligned() && h.is_empty());
    }

    #[test]
    fn undo_branch_cycles_keep_md_usage_aligned() {
        // Simula 10 ciclos enviar + deshacer-hasta-aquí + rama:
        // tras cada operación md/usage/times/ids miden lo mismo que messages.
        let mut h = ChatHistory::new();
        for cycle in 0..10 {
            h.push(
                msg(&format!("u{cycle}")),
                vec![],
                None,
                "t".to_string(),
                cycle,
            );
            h.push(
                msg(&format!("a{cycle}")),
                vec![],
                Some(Usage {
                    input: 1,
                    output: 1,
                    cached: 0,
                    cost: None,
                }),
                "t".to_string(),
                cycle + 100,
            );
            assert!(h.is_aligned(), "tras push ciclo {cycle}");
            // Deshacer hasta aquí: trunca la última (como UndoSnapshot).
            h.truncate(h.len() - 1);
            assert!(h.is_aligned(), "tras truncate ciclo {cycle}");
            assert_eq!(h.msg_usage.len(), h.messages.len());
            assert_eq!(h.md.len(), h.messages.len());
            // Rama: re-push de la respuesta rehecha.
            h.push(
                msg(&format!("a{cycle}-rama")),
                vec![],
                None,
                "t".to_string(),
                999,
            );
            assert!(h.is_aligned(), "tras rama ciclo {cycle}");
        }
        assert_eq!(h.messages.len(), 20);
        assert!(h.is_aligned());
    }
}
