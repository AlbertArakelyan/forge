use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::app_state::AppState;
use crate::ui::layout::ACCENT_BLUE;

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);

/// Render the open-tabs bar (1 row height) showing all open request tabs.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.workspace.open_tabs.is_empty() {
        let hint = Paragraph::new(Line::from(Span::styled(
            "No open tabs",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::DIM),
        )));
        frame.render_widget(hint, area);
        return;
    }

    let mut spans: Vec<Span<'static>> = Vec::new();

    for (i, tab) in state.workspace.open_tabs.iter().enumerate() {
        let is_active = i == state.workspace.active_tab_idx;
        let method = tab.request.method.as_str();
        let name = if tab.request.name.is_empty() {
            "Untitled".to_string()
        } else {
            tab.request.name.clone()
        };
        let dirty = if tab.is_dirty { "*" } else { "" };

        let tab_label = format!(" {} {}{} ", method, name, dirty);

        let style = if is_active {
            Style::default()
                .fg(ACCENT_BLUE)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };

        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(TEXT_MUTED)));
        }
        spans.push(Span::styled(tab_label, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
