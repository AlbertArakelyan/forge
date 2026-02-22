use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

use crate::state::app_state::AppState;
use crate::state::focus::Focus;
use super::layout::{ACCENT_BLUE, BORDER_INACTIVE};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = matches!(state.focus, Focus::Sidebar);
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let block = Block::default()
        .title("forge ⚒")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let p = Paragraph::new("Collections (Round 3)").block(block);
    frame.render_widget(p, area);
}
