use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::app_state::{ActiveTab, AppState};
use crate::state::focus::Focus;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let tabs = [
        ("Headers", ActiveTab::Headers),
        ("Body", ActiveTab::Body),
        ("Auth", ActiveTab::Auth),
        ("Params", ActiveTab::Params),
        ("Scripts", ActiveTab::Scripts),
    ];

    let tab_focused = state.focus == Focus::TabBar;
    let active_tab = state.active_tab().map(|t| &t.active_tab);

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (name, tab)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let is_active = active_tab == Some(tab);
        let style = if is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::Rgb(65, 72, 104))
        };
        let label: String = if is_active && tab_focused {
            format!("[{name}]")
        } else {
            name.to_string()
        };
        spans.push(Span::styled(label, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
