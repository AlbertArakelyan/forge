use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::app_state::{AppState, ResponseTab};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let tabs = [
        ("Body", ResponseTab::Body),
        ("Headers", ResponseTab::Headers),
        ("Cookies", ResponseTab::Cookies),
        ("Timing", ResponseTab::Timing),
    ];

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (name, tab)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if *tab == state.response_tab {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::Rgb(65, 72, 104))
        };
        spans.push(Span::styled(name.to_string(), style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
