use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::app_state::AppState;
use crate::state::environment::VarType;
use crate::ui::popup::centered_rect;
use crate::ui::layout::ACCENT_BLUE;

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const SURFACE: Color = Color::Rgb(36, 40, 59);
const BG: Color = Color::Rgb(26, 27, 38);

/// Render the environment switcher popup (~50% wide × 40% tall).
pub fn render_switcher(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_BLUE))
        .title(" Environments (Ctrl+E) ")
        .style(Style::default().bg(BG));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height < 3 {
        return;
    }

    // Layout: [search_row=1] [list=rest-1] [hint=1]
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Search row
    let search = &state.env_switcher.search;
    let search_line = if search.is_empty() {
        Line::from(Span::styled("Search…", Style::default().fg(TEXT_MUTED)))
    } else {
        Line::from(vec![
            Span::styled("/ ", Style::default().fg(ACCENT_BLUE)),
            Span::raw(search.clone()),
        ])
    };
    frame.render_widget(Paragraph::new(search_line), chunks[0]);

    // Filtered environment list
    let filter = search.to_lowercase();
    let envs_filtered: Vec<(usize, &str)> = state
        .environments
        .iter()
        .enumerate()
        .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
        .map(|(i, e)| (i, e.name.as_str()))
        .collect();

    let list_area = chunks[1];
    for (row, &(orig_idx, name)) in envs_filtered.iter().enumerate() {
        let y = list_area.y + row as u16;
        if y >= list_area.y + list_area.height {
            break;
        }
        let is_active = state.active_env_idx == Some(orig_idx);
        let is_selected = row == state.env_switcher.selected;
        let marker = if is_active { "● " } else { "○ " };
        let marker_color = if is_active { Color::Rgb(158, 206, 106) } else { TEXT_MUTED };
        let name_style = if is_selected {
            Style::default().fg(Color::White).bg(SURFACE).add_modifier(Modifier::BOLD)
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
    let hint = Line::from(vec![
        Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" select  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("e", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" edit  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("n", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" new  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("d", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" del  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" close", Style::default().fg(TEXT_MUTED)),
    ]);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
        chunks[2],
    );
}

/// Render the full environment editor popup (~70% wide × 70% tall).
pub fn render_editor(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup_area);

    let env = state.environments.get(state.env_editor.env_idx);
    let env_name = env.map(|e| e.name.as_str()).unwrap_or("(none)");

    let title = format!(" Environment: {} ", env_name);
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

    // Layout: [header=1] [rows=rest-1] [hint=1]
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Column widths: [check=4] [key=%30] [value=%30] [desc=rest-4] [type=8]
    let w = inner.width;
    let check_w: u16 = 4;
    let type_w: u16 = 8;
    let rest = w.saturating_sub(check_w + type_w + 2); // 2 for separators
    let key_w = rest * 30 / 100;
    let val_w = rest * 30 / 100;
    let desc_w = rest.saturating_sub(key_w + val_w);

    // Header row
    let header_line = Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(pad_right("Key", key_w as usize), Style::default().fg(Color::Yellow)),
        Span::styled(pad_right("Value", val_w as usize), Style::default().fg(Color::Yellow)),
        Span::styled(pad_right("Description", desc_w as usize), Style::default().fg(Color::Yellow)),
        Span::styled("Type    ", Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(Paragraph::new(header_line), chunks[0]);

    // Variable rows
    let body_area = chunks[1];
    let sel_row = state.env_editor.row;
    let sel_col = state.env_editor.col;

    if let Some(env) = env {
        for (i, var) in env.variables.iter().enumerate() {
            let y = body_area.y + i as u16;
            if y >= body_area.y + body_area.height {
                break;
            }
            let is_selected = i == sel_row;
            let row_bg = if is_selected { SURFACE } else { BG };

            let check_str = if var.enabled { "[✓] " } else { "[ ] " };
            let check_fg = if var.enabled { Color::Rgb(158, 206, 106) } else { TEXT_MUTED };

            let is_secret = var.var_type == VarType::Secret;
            let display_value = if is_secret && !state.env_editor.show_secret {
                "••••••••".to_string()
            } else {
                var.value.clone()
            };

            let type_str = if is_secret { "Secret  " } else { "Text    " };
            let type_fg = if is_secret { Color::Rgb(187, 154, 247) } else { TEXT_MUTED };

            let col_fg = |col: u8| {
                if is_selected && sel_col == col {
                    Color::White
                } else {
                    TEXT_PRIMARY
                }
            };

            let line = Line::from(vec![
                Span::styled(check_str, Style::default().fg(check_fg).bg(row_bg)),
                Span::styled(pad_right(&var.key, key_w as usize), Style::default().fg(col_fg(0)).bg(row_bg)),
                Span::styled(pad_right(&display_value, val_w as usize), Style::default().fg(col_fg(1)).bg(row_bg)),
                Span::styled(pad_right(&var.description, desc_w as usize), Style::default().fg(col_fg(2)).bg(row_bg)),
                Span::styled(type_str, Style::default().fg(type_fg).bg(row_bg)),
            ]);
            let row_area = Rect { y, height: 1, ..body_area };
            frame.render_widget(Paragraph::new(line), row_area);
        }
    }

    // Cursor when editing
    if state.env_editor.editing {
        if let Some(env) = env {
            let cursor = state.env_editor.cursor;
            let row = state.env_editor.row;
            let col = state.env_editor.col;
            if let Some(var) = env.variables.get(row) {
                let row_y = body_area.y + row as u16;
                if row_y < body_area.y + body_area.height {
                    let (cell_x, text): (u16, &str) = match col {
                        0 => (body_area.x + check_w, var.key.as_str()),
                        1 => (body_area.x + check_w + key_w, var.value.as_str()),
                        _ => (body_area.x + check_w + key_w + val_w, var.description.as_str()),
                    };
                    let col_offset = text[..cursor.min(text.len())].chars().count() as u16;
                    frame.set_cursor_position(Position { x: cell_x + col_offset, y: row_y });
                }
            }
        }
    }

    // Hint bar
    let hint = Line::from(vec![
        Span::styled("a", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" add  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("d", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" del  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("i/Enter", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" edit  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("Space", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" toggle  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
        Span::styled(" save+close", Style::default().fg(TEXT_MUTED)),
    ]);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM)),
        chunks[2],
    );
}

/// Pad or truncate a string to exactly `width` chars (ASCII-safe for column alignment).
fn pad_right(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count >= width {
        s.chars().take(width).collect()
    } else {
        let mut out = s.to_string();
        for _ in 0..width - char_count {
            out.push(' ');
        }
        out
    }
}
