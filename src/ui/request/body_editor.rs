// Request body editor (JSON, form, raw, binary)
use ratatui::{Frame, layout::{Alignment, Rect}, style::{Modifier, Style}, widgets::Paragraph};
use crate::state::app_state::AppState;

pub fn render(frame: &mut Frame, area: Rect, _state: &AppState) {
    let p = Paragraph::new("Body")
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(p, area);
}
