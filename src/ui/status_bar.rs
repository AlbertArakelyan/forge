use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::app_state::AppState;
use crate::state::mode::Mode;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let (mode_label, mode_color) = match state.mode {
        Mode::Normal => ("NORMAL", Color::Rgb(122, 162, 247)),   // blue
        Mode::Insert => ("INSERT", Color::Rgb(158, 206, 106)),   // green
        Mode::Command => ("COMMAND", Color::Rgb(224, 175, 104)), // orange
        Mode::Visual => ("VISUAL", Color::Rgb(187, 154, 247)),   // purple
    };

    let mode_span = Span::styled(
        format!(" {} ", mode_label),
        Style::default()
            .fg(Color::Black)
            .bg(mode_color)
            .add_modifier(Modifier::BOLD),
    );

    let hints = Span::styled(
        "  · ?:help · Ctrl+R:send · Ctrl+E:env · [ ]:method · Tab:focus · q:quit",
        Style::default().fg(Color::Rgb(65, 72, 104)),
    );

    let line = Line::from(vec![mode_span, hints]);
    frame.render_widget(Paragraph::new(line), area);
}
