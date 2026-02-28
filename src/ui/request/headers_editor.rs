use ratatui::{
    Frame,
    layout::{Alignment, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::app_state::AppState;
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::ui::layout::{ACCENT_BLUE, BORDER_INACTIVE};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let focused = state.focus == Focus::Editor;
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Headers ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 2 {
        return;
    }

    // Reserve bottom line for hint bar
    let hint_area = Rect { y: inner.y + inner.height - 1, height: 1, ..inner };
    let body_area = Rect { height: inner.height - 1, ..inner };

    // Hint bar
    let hint_spans = vec![
        Span::styled("a", Style::default().fg(Color::Rgb(169, 177, 214))),
        Span::styled(" add  ", Style::default().fg(Color::Rgb(100, 110, 140))),
        Span::styled("x", Style::default().fg(Color::Rgb(169, 177, 214))),
        Span::styled(" del  ", Style::default().fg(Color::Rgb(100, 110, 140))),
        Span::styled("Space", Style::default().fg(Color::Rgb(169, 177, 214))),
        Span::styled(" toggle  ", Style::default().fg(Color::Rgb(100, 110, 140))),
        Span::styled("←→", Style::default().fg(Color::Rgb(169, 177, 214))),
        Span::styled(" col  ", Style::default().fg(Color::Rgb(100, 110, 140))),
        Span::styled("i", Style::default().fg(Color::Rgb(169, 177, 214))),
        Span::styled(" edit", Style::default().fg(Color::Rgb(100, 110, 140))),
    ];
    let hint = Paragraph::new(Line::from(hint_spans))
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(hint, hint_area);

    let Some(tab) = state.active_tab() else {
        return;
    };
    let request = &tab.request;

    // Placeholder when no headers
    if request.headers.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            "Press a to add a header",
            Style::default()
                .fg(Color::Rgb(86, 95, 137))
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(placeholder, body_area);
        return;
    }

    // Column layout: [checkbox=4] [key=rest/2] [sep=1] [value=rest-key]
    let checkbox_w: u16 = 4;
    let sep_w: u16 = 1;
    let rest = body_area.width.saturating_sub(checkbox_w + sep_w);
    let key_w = rest / 2;
    let val_w = rest - key_w;

    let sel_row = request.headers_row;
    let sel_col = request.headers_col;

    for (i, pair) in request.headers.iter().enumerate() {
        let row_y = body_area.y + i as u16;
        if row_y >= body_area.y + body_area.height {
            break;
        }

        let is_selected = i == sel_row;
        let row_bg = if is_selected { Color::Rgb(41, 45, 62) } else { Color::Reset };
        let row_style = Style::default().bg(row_bg);

        // Checkbox
        let (check_str, check_fg) = if pair.enabled {
            ("[✓] ", Color::Green)
        } else {
            ("[ ] ", Color::Rgb(100, 110, 140))
        };
        let check_rect = Rect { x: body_area.x, y: row_y, width: checkbox_w, height: 1 };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                check_str,
                Style::default().fg(check_fg).bg(row_bg),
            ))),
            check_rect,
        );

        // Key column
        let key_active = is_selected && sel_col == 0;
        let key_fg = if focused && key_active { Color::White } else { Color::Rgb(169, 177, 214) };
        let key_rect = Rect { x: body_area.x + checkbox_w, y: row_y, width: key_w, height: 1 };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                pair.key.as_str(),
                Style::default().fg(key_fg),
            )))
            .style(row_style),
            key_rect,
        );

        // Separator
        let sep_rect = Rect {
            x: body_area.x + checkbox_w + key_w,
            y: row_y,
            width: sep_w,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "│",
                Style::default().fg(BORDER_INACTIVE).bg(row_bg),
            ))),
            sep_rect,
        );

        // Value column
        let val_active = is_selected && sel_col == 1;
        let val_fg = if focused && val_active { Color::White } else { Color::Rgb(169, 177, 214) };
        let val_rect = Rect {
            x: body_area.x + checkbox_w + key_w + sep_w,
            y: row_y,
            width: val_w,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                pair.value.as_str(),
                Style::default().fg(val_fg),
            )))
            .style(row_style),
            val_rect,
        );
    }

    // Cursor in Insert mode
    if focused && state.mode == Mode::Insert {
        if let Some(pair) = request.headers.get(sel_row) {
            let cursor = request.headers_cursor;
            let (cell_x, text) = if sel_col == 0 {
                (body_area.x + checkbox_w, pair.key.as_str())
            } else {
                (body_area.x + checkbox_w + key_w + sep_w, pair.value.as_str())
            };
            let col_offset = text[..cursor.min(text.len())].chars().count() as u16;
            let row_y = body_area.y + sel_row as u16;
            if row_y < body_area.y + body_area.height {
                frame.set_cursor_position(Position {
                    x: cell_x + col_offset,
                    y: row_y,
                });
            }
        }
    }
}
