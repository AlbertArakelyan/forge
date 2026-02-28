// Request body editor — multiline text editor with JSON syntax highlighting
use ratatui::{
    Frame,
    layout::{Alignment, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::app_state::AppState;
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::request_state::RequestBody;
use crate::ui::highlight::highlight_text;
use crate::ui::layout::{ACCENT_BLUE, BORDER_INACTIVE};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let focused = state.focus == Focus::Editor;
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Body ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some(tab) = state.active_tab() else {
        return;
    };
    let request = &tab.request;

    let (text, lang) = match &request.body {
        RequestBody::Json(s) => (s.as_str(), "json"),
        RequestBody::Text(s) => (s.as_str(), "txt"),
        RequestBody::None | RequestBody::Form(_) | RequestBody::Binary(_) => ("", "json"),
    };

    let scroll = request.body_scroll_offset;
    let cursor = request.body_cursor;

    if text.is_empty() && state.mode != Mode::Insert {
        // Show placeholder when empty and not editing
        let placeholder = Paragraph::new(
            Line::from(Span::styled(
                "Press i to start editing…",
                Style::default()
                    .fg(Color::Rgb(86, 95, 137))
                    .add_modifier(Modifier::DIM),
            ))
        )
        .alignment(Alignment::Center);
        frame.render_widget(placeholder, inner);
        return;
    }

    let highlighted = highlight_text(text, lang);
    let para = Paragraph::new(highlighted).scroll((scroll, 0));
    frame.render_widget(para, inner);

    // Show cursor when focused
    if focused {
        let (cursor_row, cursor_col) = cursor_row_col(text, cursor);
        let visible_row = cursor_row.saturating_sub(scroll as usize);
        if visible_row < inner.height as usize {
            frame.set_cursor_position(Position {
                x: inner.x + cursor_col as u16,
                y: inner.y + visible_row as u16,
            });
        }
    }
}

/// Returns (row, col) for a byte offset in text, both 0-indexed.
pub fn cursor_row_col(text: &str, cursor: usize) -> (usize, usize) {
    let clamped = cursor.min(text.len());
    let before = &text[..clamped];
    let row = before.bytes().filter(|&b| b == b'\n').count();
    let col = match before.rfind('\n') {
        Some(i) => before[i + 1..].chars().count(),
        None => before.chars().count(),
    };
    (row, col)
}
