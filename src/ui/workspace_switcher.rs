use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::app_state::AppState;
use crate::ui::layout::ACCENT_BLUE;
use crate::ui::popup::centered_rect;

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const SURFACE: Color = Color::Rgb(36, 40, 59);
const BG: Color = Color::Rgb(26, 27, 38);

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .title(" Workspaces (Ctrl+W) ")
        .style(Style::default().bg(BG));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Search / naming row
    if state.ws_switcher.naming {
        let new_name = &state.ws_switcher.new_name;
        let name_line = Line::from(vec![
            Span::styled("Name: ", Style::default().fg(TEXT_MUTED)),
            Span::styled(new_name.clone(), Style::default().fg(TEXT_PRIMARY)),
        ]);
        frame.render_widget(Paragraph::new(name_line), chunks[0]);
        let col_offset = new_name[..state.ws_switcher.new_name_cursor.min(new_name.len())]
            .chars()
            .count() as u16;
        frame.set_cursor_position(Position {
            x: chunks[0].x + 6 + col_offset,
            y: chunks[0].y,
        });
    } else {
        let search = &state.ws_switcher.search;
        let search_line = if search.is_empty() {
            Line::from(Span::styled("Search…", Style::default().fg(TEXT_MUTED)))
        } else {
            Line::from(vec![
                Span::styled("/ ", Style::default().fg(ACCENT_BLUE)),
                Span::raw(search.clone()),
            ])
        };
        frame.render_widget(Paragraph::new(search_line), chunks[0]);
    }

    // Workspace list (filtered)
    let filter = state.ws_switcher.search.to_lowercase();
    let filtered: Vec<&str> = state
        .all_workspaces
        .iter()
        .filter(|w| filter.is_empty() || w.to_lowercase().contains(&filter))
        .map(|w| w.as_str())
        .collect();

    let list_area = chunks[1];
    for (row, &name) in filtered.iter().enumerate() {
        let y = list_area.y + row as u16;
        if y >= list_area.y + list_area.height {
            break;
        }
        let is_active = name == state.workspace.name;
        let is_selected = row == state.ws_switcher.selected;
        let marker = if is_active { "● " } else { "○ " };
        let marker_color = if is_active {
            Color::Rgb(158, 206, 106)
        } else {
            TEXT_MUTED
        };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(SURFACE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };
        let row_area = Rect { y, height: 1, ..list_area };
        let line = Line::from(vec![
            Span::styled(marker, Style::default().fg(marker_color)),
            Span::styled(name, name_style),
        ]);
        frame.render_widget(Paragraph::new(line), row_area);
    }

    // Hint bar
    let hint = if state.ws_switcher.naming {
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" confirm  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" cancel", Style::default().fg(TEXT_MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" switch  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Alt+n", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" new  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
            Span::styled(" close", Style::default().fg(TEXT_MUTED)),
        ])
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
        chunks[2],
    );
}
