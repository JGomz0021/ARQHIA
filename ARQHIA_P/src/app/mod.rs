//! Estado, eventos y lógica de aplicación (sin Iced views).
//!
//! - `events`: Message / View / ConfigTab (solo datos).
//! - `state`: struct App + carga inicial + helpers puros.
//! - `projects`: crear / papelera / borrado total (SQLite + FS).
//! - `orchestrator`: driver planner -> workers -> auditor.
//! - `handlers`: brazos de update por dominio (chat, proyectos, agente… config, cuestionario, navegación).

pub mod events;
pub mod handlers;
pub mod history;
pub mod orchestrator;
pub mod projects;
pub mod state;

pub use events::{ConfigTab, Message, View};
pub use state::App;
