//! Terminal UI for the 0-code agent (ratatui).

mod app;
mod event;
mod run;
mod ui;

pub use app::{ActiveTool, App, DisplayMessage, MessageRole};
pub use event::{poll_event, AppAction};
pub use run::run_tui;
pub use run::run_tui as run;
