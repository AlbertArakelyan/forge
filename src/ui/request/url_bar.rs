use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::app_state::{AppState, RequestStatus};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::request_state::HttpMethod;
use super::super::layout::{ACCENT_BLUE, BORDER_INACTIVE};

const SPINNER_FRAMES: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

fn method_color(method: &HttpMethod) -> Color {
    match method {
        HttpMethod::Get => Color::Rgb(115, 218, 202),
        HttpMethod::Post => Color::Rgb(158, 206, 106),
        HttpMethod::Put => Color::Rgb(224, 175, 104),
        HttpMethod::Patch => Color::Rgb(187, 154, 247),
        HttpMethod::Delete => Color::Rgb(247, 118, 142),
        HttpMethod::Head => Color::Rgb(122, 162, 247),
        HttpMethod::Options => Color::Rgb(65, 72, 104),
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = matches!(state.focus, Focus::UrlBar);
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // [method 9] [│] [url flex] [│] [send 8]
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(9),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(8),
        ])
        .split(inner);

    // Method badge
    let mc = method_color(&state.request.method);
    let method_para = Paragraph::new(Line::from(Span::styled(
        state.request.method.as_str(),
        Style::default().fg(mc).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(method_para, chunks[0]);

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled("│", Style::default().fg(BORDER_INACTIVE))),
        chunks[1],
    );

    // URL input
    let url_line = build_url_line(state, focused);
    frame.render_widget(Paragraph::new(url_line), chunks[2]);

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled("│", Style::default().fg(BORDER_INACTIVE))),
        chunks[3],
    );

    // Send button
    let (send_text, send_style) = match &state.request_status {
        RequestStatus::Loading { spinner_tick } => {
            let idx = (*spinner_tick as usize) % SPINNER_FRAMES.len();
            (
                format!("{} ..", SPINNER_FRAMES[idx]),
                Style::default().fg(Color::Yellow),
            )
        }
        _ => ("Send ↵".to_string(), Style::default().fg(Color::Rgb(158, 206, 106))),
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(send_text, send_style))),
        chunks[4],
    );
}

fn build_url_line(state: &AppState, focused: bool) -> Line<'static> {
    if matches!(state.mode, Mode::Insert) && focused {
        let url = &state.request.url;
        let cursor = state.request.url_cursor;
        let before = url[..cursor].to_string();
        let (cursor_char, after) = if cursor < url.len() {
            let ch = url[cursor..].chars().next().unwrap();
            let next_byte = cursor + ch.len_utf8();
            (ch.to_string(), url[next_byte..].to_string())
        } else {
            (" ".to_string(), String::new())
        };
        Line::from(vec![
            Span::raw(before),
            Span::styled(cursor_char, Style::default().bg(Color::White).fg(Color::Black)),
            Span::raw(after),
        ])
    } else if state.request.url.is_empty() {
        Line::from(Span::styled(
            "Enter URL or paste text…",
            Style::default().fg(Color::Rgb(65, 72, 104)),
        ))
    } else {
        Line::from(Span::raw(state.request.url.clone()))
    }
}
