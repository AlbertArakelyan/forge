use humansize::{format_size, DECIMAL};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::app_state::{AppState, RequestStatus};
use crate::state::response_state::ResponseBody;
use crate::state::focus::Focus;
use super::super::layout::{ACCENT_BLUE, BORDER_INACTIVE, SPINNER_FRAMES};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = matches!(state.focus, Focus::ResponseViewer);
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let request_status = state.active_tab().map(|t| &t.request_status);
    let response = state.active_tab().and_then(|t| t.response.as_ref());

    match request_status {
        Some(RequestStatus::Loading { spinner_tick }) => {
            let idx = (*spinner_tick as usize) % SPINNER_FRAMES.len();
            let text = Line::from(vec![
                Span::styled(
                    format!("  {} ", SPINNER_FRAMES[idx]),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    "Sending request…",
                    Style::default().fg(Color::Rgb(65, 72, 104)),
                ),
            ]);
            frame.render_widget(Paragraph::new(text), area);
        }
        Some(RequestStatus::Error(msg)) => {
            let msg = msg.clone();
            let text = Line::from(Span::styled(
                format!("  Error: {}", msg),
                Style::default().fg(Color::Red),
            ));
            frame.render_widget(Paragraph::new(text), area);
        }
        Some(RequestStatus::Idle) | None => {
            match response {
                None => {
                    let hint = Paragraph::new(Line::from(Span::styled(
                        "  Send a request to see the response",
                        Style::default().fg(Color::Rgb(65, 72, 104)),
                    )));
                    frame.render_widget(hint, area);
                }
                Some(resp) => {
                    let body_text = match &resp.body {
                        ResponseBody::Empty => {
                            ratatui::text::Text::raw("  (empty response body)")
                        }
                        ResponseBody::Binary(bytes) => {
                            ratatui::text::Text::raw(format!(
                                "  [Binary data: {} bytes]",
                                bytes.len()
                            ))
                        }
                        ResponseBody::Text(text) => {
                            // Use the pre-computed highlighted text; fall back to plain
                            // text only if the cache is somehow absent (e.g. after serde
                            // round-trip in a future history feature).
                            resp.highlighted_body
                                .clone()
                                .unwrap_or_else(|| ratatui::text::Text::raw(text.clone()))
                        }
                    };

                    let para = Paragraph::new(body_text)
                        .scroll((resp.scroll_offset, 0))
                        .style(Style::default().fg(if focused {
                            Color::Reset
                        } else {
                            Color::Reset
                        }));
                    // draw focus border hint via border color on the unused style field
                    let _ = border_color; // used for border styling in layout parent
                    frame.render_widget(para, area);
                }
            }
        }
    }
}

pub fn render_meta(frame: &mut Frame, area: Rect, state: &AppState) {
    let response = state.active_tab().and_then(|t| t.response.as_ref());
    let line = match response {
        None => Line::from(Span::styled("─", Style::default().fg(BORDER_INACTIVE))),
        Some(resp) => {
            let status_color = match resp.status {
                200..=299 => Color::Rgb(158, 206, 106), // green
                300..=399 => Color::Rgb(122, 162, 247), // blue
                400..=499 => Color::Rgb(224, 175, 104), // orange/yellow
                500..=599 => Color::Rgb(247, 118, 142), // red
                _ => Color::White,
            };
            let size_str = format_size(resp.size_bytes as u64, DECIMAL);
            Line::from(vec![
                Span::styled(
                    format!(" {} {}", resp.status, resp.status_text),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ·  {}ms  ·  {}", resp.timing.total_ms, size_str),
                    Style::default().fg(Color::Rgb(65, 72, 104)),
                ),
            ])
        }
    };
    frame.render_widget(Paragraph::new(line), area);
}
