use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::app_state::{AppState, NamingTarget};
use crate::ui::layout::ACCENT_BLUE;
use crate::ui::popup::centered_rect;

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const BG: Color = Color::Rgb(26, 27, 38);

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_new_request = matches!(state.naming.target, NamingTarget::NewRequest { .. });

    let popup_area = centered_rect(50, 30, area);
    let popup_area = Rect {
        height: if is_new_request {
            popup_area.height.min(9).max(6)
        } else {
            popup_area.height.min(7).max(5)
        },
        ..popup_area
    };

    frame.render_widget(Clear, popup_area);

    let title = match &state.naming.target {
        NamingTarget::NewCollection => " New Collection ",
        NamingTarget::NewFolder { .. } => " New Folder ",
        NamingTarget::NewRequest { .. } => " New Request ",
        NamingTarget::Rename { .. } => " Rename ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .title(title)
        .style(Style::default().bg(BG));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height < 3 {
        return;
    }

    let constraints = if is_new_request {
        vec![
            Constraint::Length(1), // input
            Constraint::Length(1), // method row
            Constraint::Length(1), // separator
            Constraint::Length(1), // footer
        ]
    } else {
        vec![
            Constraint::Min(1),    // input
            Constraint::Length(1), // separator
            Constraint::Length(1), // footer
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Input field
    let input = &state.naming.input;
    let cursor = state.naming.cursor;

    let (before, cursor_char, after) = if cursor < input.len() {
        let ch = input[cursor..].chars().next().unwrap_or(' ');
        let next = cursor + ch.len_utf8();
        (
            input[..cursor].to_string(),
            ch.to_string(),
            input[next..].to_string(),
        )
    } else {
        (input.clone(), "_".to_string(), String::new())
    };

    let input_line = Line::from(vec![
        Span::styled(before, Style::default().fg(TEXT_PRIMARY)),
        Span::styled(cursor_char, Style::default().bg(Color::White).fg(Color::Black)),
        Span::styled(after, Style::default().fg(TEXT_PRIMARY)),
    ]);

    frame.render_widget(Paragraph::new(input_line), chunks[0]);

    // Set actual terminal cursor
    let col_offset = input[..cursor.min(input.len())].chars().count() as u16;
    frame.set_cursor_position(Position {
        x: chunks[0].x + col_offset,
        y: chunks[0].y,
    });

    if is_new_request {
        // Method row
        let method_line = Line::from(vec![
            Span::styled("◀ ", Style::default().fg(TEXT_MUTED)),
            Span::styled(
                state.naming.method.clone(),
                Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ▶", Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(Paragraph::new(method_line), chunks[1]);

        // Separator
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(TEXT_MUTED),
            ))),
            chunks[2],
        );

        // Footer hints (with Tab method)
        let hint = Line::from(vec![
            Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" confirm  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Tab", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" method  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" cancel", Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
            chunks[3],
        );
    } else {
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
            Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" confirm  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" cancel", Style::default().fg(TEXT_MUTED)),
        ]);
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
            chunks[2],
        );
    }
}
