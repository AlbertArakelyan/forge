use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::app_state::AppState;
use crate::ui::popup::centered_rect;

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const BG: Color = Color::Rgb(26, 27, 38);
const STATUS_ERR: Color = Color::Rgb(247, 118, 142);

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(40, 20, area);
    let popup_area = Rect {
        height: popup_area.height.min(5).max(5),
        ..popup_area
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(STATUS_ERR))
        .title(" Confirm Delete ")
        .style(Style::default().bg(BG));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Message
    let msg = &state.confirm_delete.message;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(TEXT_PRIMARY),
        ))),
        chunks[0],
    );

    // Separator
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(TEXT_MUTED),
        ))),
        chunks[1],
    );

    // Footer hints
    let hint = Line::from(vec![
        Span::styled("y/Enter", Style::default().fg(STATUS_ERR)),
        Span::styled(" Delete  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("n/Esc", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" Cancel", Style::default().fg(TEXT_MUTED)),
    ]);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
        chunks[2],
    );
}
