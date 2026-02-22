use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
};

use crate::state::app_state::AppState;
use super::{
    sidebar,
    status_bar,
    request::{url_bar, tab_bar as req_tab_bar},
    response::{render_meta, body_viewer, tab_bar as resp_tab_bar},
};

// TokyoNight palette
pub const ACCENT_BLUE: Color = Color::Rgb(122, 162, 247);  // #7aa2f7
pub const BORDER_INACTIVE: Color = Color::Rgb(65, 72, 104); // #414868
pub const BG: Color = Color::Rgb(26, 27, 38);               // #1a1b26

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Split off status bar at bottom
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_area = vertical[1];

    // Optional sidebar
    let right_area = if state.sidebar_visible {
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(main_area);
        sidebar::render(frame, horiz[0], state);
        horiz[1]
    } else {
        main_area
    };

    // Right panel vertical split
    let editor_h = right_area.height.saturating_sub(3 + 1 + 1 + 1 + 1);
    let editor_h = ((editor_h as u32 * 35 / 100) as u16).max(3);
    let viewer_h = right_area
        .height
        .saturating_sub(3 + 1 + editor_h + 1 + 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),          // url bar
            Constraint::Length(1),          // request tab bar
            Constraint::Length(editor_h),   // request editor (future)
            Constraint::Length(1),          // response meta line
            Constraint::Length(1),          // response tab bar
            Constraint::Min(viewer_h),      // response viewer
        ])
        .split(right_area);

    url_bar::render(frame, chunks[0], state);
    req_tab_bar::render(frame, chunks[1], state);
    // chunks[2] — request editor body (future rounds)
    render_meta(frame, chunks[3], state);
    resp_tab_bar::render(frame, chunks[4], state);
    body_viewer::render(frame, chunks[5], state);

    status_bar::render(frame, status_area, state);
}

/// Helper used by sub-widgets to decide whether a rect is visible
#[allow(dead_code)]
pub fn is_visible(rect: Rect) -> bool {
    rect.width > 0 && rect.height > 0
}
