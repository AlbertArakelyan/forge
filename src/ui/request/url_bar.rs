use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::env::interpolator::parse_vars;
use crate::env::resolver::resolver_from_state;
use crate::env::resolver::VarStatus;
use crate::state::app_state::{AppState, RequestStatus};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::request_state::HttpMethod;
use super::super::layout::{ACCENT_BLUE, BORDER_INACTIVE, SPINNER_FRAMES};

// TokyoNight colors for variable highlighting
const ENV_VAR_RESOLVED: Color = Color::Rgb(42, 195, 222);   // #2ac3de cyan
const ENV_VAR_UNRESOLVED: Color = Color::Rgb(247, 118, 142); // #f7768e red
const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);

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

    // Get request from active tab
    let Some(tab) = state.active_tab() else {
        return;
    };
    let request = &tab.request;
    let request_status = &tab.request_status;

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
    let mc = method_color(&request.method);
    let method_para = Paragraph::new(Line::from(Span::styled(
        request.method.as_str(),
        Style::default().fg(mc).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(method_para, chunks[0]);

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled("│", Style::default().fg(BORDER_INACTIVE))),
        chunks[1],
    );

    // URL input area — split vertically if there's room for ghost text
    let url_area = chunks[2];
    let has_vars = !parse_vars(&request.url).is_empty();
    if url_area.height >= 2 && has_vars {
        let url_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(url_area);
        let url_line = build_url_line(state, focused);
        frame.render_widget(Paragraph::new(url_line), url_chunks[0]);
        // Ghost resolved text
        let resolver = resolver_from_state(state);
        let resolved = resolver.resolve_for_send(&request.url);
        let ghost_line = Line::from(vec![
            Span::styled("→ ", Style::default().fg(TEXT_MUTED)),
            Span::styled(resolved, Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(Paragraph::new(ghost_line), url_chunks[1]);
    } else {
        let url_line = build_url_line(state, focused);
        frame.render_widget(Paragraph::new(url_line), url_area);
    }

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled("│", Style::default().fg(BORDER_INACTIVE))),
        chunks[3],
    );

    // Send button — rendered per-branch to avoid a heap allocation for the
    // common idle case where the label is a &'static str.
    match request_status {
        RequestStatus::Loading { spinner_tick } => {
            let idx = (*spinner_tick as usize) % SPINNER_FRAMES.len();
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("{} ..", SPINNER_FRAMES[idx]),
                    Style::default().fg(Color::Yellow),
                ))),
                chunks[4],
            );
        }
        _ => {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Send ↵",
                    Style::default().fg(Color::Rgb(158, 206, 106)),
                ))),
                chunks[4],
            );
        }
    }
}

fn build_url_line(state: &AppState, focused: bool) -> Line<'static> {
    let Some(tab) = state.active_tab() else {
        return Line::from(Span::styled(
            "No active tab",
            Style::default().fg(Color::Rgb(65, 72, 104)),
        ));
    };
    let url = &tab.request.url;
    let cursor = tab.request.url_cursor;

    if url.is_empty() {
        return Line::from(Span::styled(
            "Enter URL or paste text…",
            Style::default().fg(Color::Rgb(65, 72, 104)),
        ));
    }

    let var_spans = parse_vars(url);

    if matches!(state.mode, Mode::Insert) && focused {
        // Insert mode with cursor — show cursor block, and color variables
        if var_spans.is_empty() {
            // No variables: simple cursor rendering
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
        } else {
            build_highlighted_url_with_cursor(url, cursor, &var_spans, state)
        }
    } else {
        // Normal mode — color variables
        if var_spans.is_empty() {
            Line::from(Span::raw(url.clone()))
        } else {
            build_highlighted_url(url, &var_spans, state)
        }
    }
}

/// Build a highlighted URL line for normal mode (no cursor block).
fn build_highlighted_url(url: &str, var_spans: &[(usize, usize, String)], state: &AppState) -> Line<'static> {
    let resolver = resolver_from_state(state);
    let mut spans = Vec::new();
    let mut last = 0;

    for (start, end, name) in var_spans {
        if *start > last {
            spans.push(Span::raw(url[last..*start].to_string()));
        }
        let resolved = resolver.resolve(&url[*start..*end]);
        let is_resolved = resolved.spans.first().map(|s| !matches!(s.status, VarStatus::Unresolved)).unwrap_or(false);
        let final_color = if is_resolved { ENV_VAR_RESOLVED } else { ENV_VAR_UNRESOLVED };
        spans.push(Span::styled(
            format!("{{{{{}}}}}", name),
            Style::default().fg(final_color),
        ));
        last = *end;
    }
    if last < url.len() {
        spans.push(Span::raw(url[last..].to_string()));
    }

    Line::from(spans)
}

/// Build a highlighted URL line with cursor block in Insert mode.
fn build_highlighted_url_with_cursor(
    url: &str,
    cursor: usize,
    var_spans: &[(usize, usize, String)],
    state: &AppState,
) -> Line<'static> {
    let resolver = resolver_from_state(state);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut last = 0;
    let mut cursor_placed = false;

    // Helper closure to insert cursor within a plain text segment
    let place_cursor_in_segment = |text: &str, seg_start: usize, cursor: usize, spans: &mut Vec<Span<'static>>, placed: &mut bool| {
        if !*placed && cursor >= seg_start && cursor <= seg_start + text.len() {
            let local = cursor - seg_start;
            let before = text[..local].to_string();
            let (cur_char, after) = if local < text.len() {
                let ch = text[local..].chars().next().unwrap();
                let nb = local + ch.len_utf8();
                (ch.to_string(), text[nb..].to_string())
            } else {
                (" ".to_string(), String::new())
            };
            if !before.is_empty() { spans.push(Span::raw(before)); }
            spans.push(Span::styled(cur_char, Style::default().bg(Color::White).fg(Color::Black)));
            if !after.is_empty() { spans.push(Span::raw(after)); }
            *placed = true;
        } else {
            spans.push(Span::raw(text.to_string()));
        }
    };

    for (start, end, name) in var_spans {
        // Plain text before this variable span
        if *start > last {
            let seg = &url[last..*start];
            place_cursor_in_segment(seg, last, cursor, &mut spans, &mut cursor_placed);
        }

        // The variable span itself
        let is_resolved = {
            let resolved = resolver.resolve(&url[*start..*end]);
            resolved.spans.first().map(|s| !matches!(s.status, VarStatus::Unresolved)).unwrap_or(false)
        };
        let final_color = if is_resolved { ENV_VAR_RESOLVED } else { ENV_VAR_UNRESOLVED };

        // Check if cursor is inside the variable placeholder
        if !cursor_placed && cursor >= *start && cursor < *end {
            // Place cursor block on the opening `{`
            spans.push(Span::styled(
                format!("{{{{{}}}}}", name),
                Style::default().fg(final_color).bg(Color::Rgb(60, 60, 80)),
            ));
            cursor_placed = true;
        } else {
            spans.push(Span::styled(
                format!("{{{{{}}}}}", name),
                Style::default().fg(final_color),
            ));
        }

        last = *end;
    }

    // Remaining text after last variable
    if last < url.len() {
        let seg = &url[last..];
        place_cursor_in_segment(seg, last, cursor, &mut spans, &mut cursor_placed);
    }

    // If cursor is at the very end
    if !cursor_placed {
        spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
    }

    Line::from(spans)
}
