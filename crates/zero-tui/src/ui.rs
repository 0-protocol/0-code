use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use crate::app::{App, DisplayMessage, MessageRole};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);

    let messages_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(chunks[1]);

    let wrap_width = messages_chunks[0].width.saturating_sub(2);
    render_messages(frame, app, messages_chunks[0]);
    render_messages_scrollbar(frame, app, messages_chunks[1], wrap_width);

    render_input(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut tool_hint = String::new();
    if !app.active_tools.is_empty() {
        let names: Vec<_> = app.active_tools.iter().map(|t| t.name.as_str()).collect();
        tool_hint = format!(" | tools: {}", names.join(", "));
    }
    let (status_text, status_err) = if app.status_text.is_empty() {
        (
            if app.is_processing {
                "busy".to_string()
            } else {
                "ready".to_string()
            },
            false,
        )
    } else {
        let s = app.status_text.clone();
        let err = s.starts_with("Error:") || s.starts_with("Error ");
        (s, err)
    };
    let status_style = if status_err {
        Style::default()
            .fg(Color::Red)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    };
    let line = Line::from(vec![
        Span::styled(
            " zero-code ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " | model: {} | {} in / {} out{} | [",
                app.model_name, app.usage_input, app.usage_output, tool_hint,
            ),
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&status_text, status_style),
        Span::styled(
            "]",
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn message_lines(msg: &DisplayMessage) -> Vec<Line<'static>> {
    let ts = Span::styled(
        format!("[{}] ", msg.timestamp),
        Style::default().fg(Color::DarkGray),
    );
    let (prefix, base_style): (String, Style) = match &msg.role {
        MessageRole::User => (
            "You: ".to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        MessageRole::Assistant => (
            String::new(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        MessageRole::System => (
            "System: ".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        MessageRole::Tool(name) => (
            format!("Tool [{name}]: "),
            Style::default().fg(Color::Green),
        ),
    };

    let mut out = Vec::new();
    let first_prefix = Span::styled(prefix, base_style);
    let mut lines_iter = msg.content.split('\n');
    let first = lines_iter.next().unwrap_or("");
    out.push(Line::from(vec![
        ts,
        first_prefix,
        Span::styled(first.to_string(), base_style),
    ]));
    for line in lines_iter {
        out.push(Line::from(Span::styled(
            format!("      {line}"),
            base_style,
        )));
    }
    out
}

fn all_message_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for m in &app.messages {
        lines.extend(message_lines(m));
        lines.push(Line::from(""));
    }
    lines
}

fn render_messages(frame: &mut Frame, app: &App, area: Rect) {
    let lines = all_message_lines(app);
    let text = Text::from(lines);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Messages ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0))
        .style(Style::default().bg(Color::Reset));

    frame.render_widget(paragraph, inner);
}

fn render_messages_scrollbar(frame: &mut Frame, app: &App, area: Rect, wrap_width: u16) {
    let lines: Vec<Line<'static>> = all_message_lines(app);
    let mut line_count = 0u16;
    for line in &lines {
        line_count = line_count.saturating_add(1);
        line_count = line_count.saturating_add(extra_wrapped_lines(line, wrap_width));
    }
    let content_len = line_count.max(1) as usize;
    let mut state = ScrollbarState::new(content_len).position(app.scroll_offset as usize);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

/// Rough extra lines from wrapping (Paragraph wrap matches terminal width minus borders).
fn extra_wrapped_lines(line: &Line, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    let mut total = 0usize;
    for span in line.spans.iter() {
        let len = span.width();
        total = total.saturating_add(len);
    }
    if total == 0 {
        return 0;
    }
    ((total.saturating_sub(1)) / (width as usize)) as u16
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Input ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor = app.input_cursor.min(app.input.len());
    let cursor = if app.input.is_char_boundary(cursor) {
        cursor
    } else {
        app.input.floor_char_boundary(cursor)
    };
    let (before, after) = app.input.split_at(cursor);
    let line = Line::from(vec![
        Span::styled(before, Style::default().fg(Color::White)),
        Span::styled(
            "▏",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White),
        ),
        Span::styled(after, Style::default().fg(Color::White)),
    ]);

    let input_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };

    frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), input_area);
    let footer = Paragraph::new(Line::from(Span::styled(
        " Enter: send | Ctrl-C / Esc: quit | Ctrl-L: clear | ↑↓: scroll ",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(footer, footer_area);
}
