use std::io::{self, stdout, Write};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use zero_core::AgentEvent;

use crate::{
    app::App,
    event::{poll_event, AppAction},
    ui,
};

/// Run the TUI application.
/// Returns user messages via `user_tx`, receives agent events via `agent_rx`.
pub async fn run_tui(
    model_name: String,
    mut agent_rx: mpsc::Receiver<AgentEvent>,
    user_tx: mpsc::Sender<String>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(model_name);

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        while let Ok(event) = agent_rx.try_recv() {
            match event {
                AgentEvent::TextDelta(t) => app.add_text_delta(&t),
                AgentEvent::ToolStart { id, name } => app.tool_start(id, name),
                AgentEvent::ToolEnd {
                    id,
                    result,
                    is_error,
                } => app.tool_end(&id, &result, is_error),
                AgentEvent::TurnComplete {
                    usage_input,
                    usage_output,
                } => {
                    app.update_usage(usage_input, usage_output);
                    app.is_processing = false;
                }
                AgentEvent::ThinkingDelta(_) => {}
                AgentEvent::Error(e) => {
                    app.status_text = format!("Error: {e}");
                    app.is_processing = false;
                }
            }
        }

        let action = tokio::task::block_in_place(|| {
            poll_event(std::time::Duration::from_millis(16))
        })?;

        match action {
            AppAction::Input(c) => {
                app.input.insert(app.input_cursor, c);
                app.input_cursor += c.len_utf8();
            }
            AppAction::Submit => {
                if !app.input.is_empty() && !app.is_processing {
                    let msg = app.input.clone();
                    app.add_user_message(&msg);
                    app.input.clear();
                    app.input_cursor = 0;
                    app.is_processing = true;
                    app.status_text.clear();
                    let _ = user_tx.send(msg).await;
                }
            }
            AppAction::Backspace => {
                if app.input_cursor > 0 {
                    let prev = app.input[..app.input_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    app.input.remove(prev);
                    app.input_cursor = prev;
                }
            }
            AppAction::Delete => {
                if app.input_cursor < app.input.len() && app.input.is_char_boundary(app.input_cursor) {
                    app.input.remove(app.input_cursor);
                }
            }
            AppAction::CursorLeft => {
                if app.input_cursor > 0 {
                    app.input_cursor = app.input[..app.input_cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            AppAction::CursorRight => {
                if app.input_cursor < app.input.len() {
                    app.input_cursor = app.input[app.input_cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| app.input_cursor + i)
                        .unwrap_or(app.input.len());
                }
            }
            AppAction::CursorHome => {
                app.input_cursor = 0;
            }
            AppAction::CursorEnd => {
                app.input_cursor = app.input.len();
            }
            AppAction::ScrollUp => {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
            AppAction::ScrollDown => {
                app.scroll_offset = app.scroll_offset.saturating_add(1);
            }
            AppAction::Quit => {
                app.should_quit = true;
                break;
            }
            AppAction::Clear => {
                app.messages.clear();
                app.scroll_offset = 0;
            }
            AppAction::None => {}
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    Ok(())
}
