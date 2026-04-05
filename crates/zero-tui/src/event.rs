use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

pub enum AppAction {
    Input(char),
    Submit,
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    ScrollUp,
    ScrollDown,
    Clear,
    Quit,
    None,
}

pub fn poll_event(timeout: Duration) -> std::io::Result<AppAction> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            return Ok(match key.code {
                KeyCode::Enter => AppAction::Submit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    AppAction::Quit
                }
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    AppAction::Clear
                }
                KeyCode::Char(c) => AppAction::Input(c),
                KeyCode::Backspace => AppAction::Backspace,
                KeyCode::Delete => AppAction::Delete,
                KeyCode::Left => AppAction::CursorLeft,
                KeyCode::Right => AppAction::CursorRight,
                KeyCode::Home => AppAction::CursorHome,
                KeyCode::End => AppAction::CursorEnd,
                KeyCode::Up => AppAction::ScrollUp,
                KeyCode::Down => AppAction::ScrollDown,
                KeyCode::Esc => AppAction::Quit,
                _ => AppAction::None,
            });
        }
    }
    Ok(AppAction::None)
}
