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
use super::super::layout::{ACCENT_BLUE, BORDER_INACTIVE};
use super::super::highlight::highlight_text;

const SPINNER_FRAMES: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = matches!(state.focus, Focus::ResponseViewer);
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    match &state.request_status {
        RequestStatus::Loading { spinner_tick } => {
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
        RequestStatus::Error(msg) => {
            let text = Line::from(Span::styled(
                format!("  Error: {}", msg),
                Style::default().fg(Color::Red),
            ));
            frame.render_widget(Paragraph::new(text), area);
        }
        RequestStatus::Idle => {
            match &state.response {
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
                            let lang = detect_lang(text);
                            highlight_text(text, lang)
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
    let line = match &state.response {
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

fn detect_lang(text: &str) -> &'static str {
    let t = text.trim_start();
    if t.starts_with('{') || t.starts_with('[') {
        "json"
    } else if t.starts_with('<') {
        "xml"
    } else {
        "txt"
    }
}
