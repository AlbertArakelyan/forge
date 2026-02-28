use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
};

use crate::state::app_state::{ActivePopup, ActiveTab, AppState};
use super::{
    confirm_delete,
    env_editor,
    naming_popup,
    request_tabs,
    sidebar,
    status_bar,
    workspace_switcher,
    request::{
        url_bar, tab_bar as req_tab_bar,
        headers_editor, body_editor, auth_editor, params_editor, scripts_editor,
    },
    response::{render_meta, body_viewer, tab_bar as resp_tab_bar},
};

// TokyoNight palette
pub const ACCENT_BLUE: Color = Color::Rgb(122, 162, 247);  // #7aa2f7
pub const BORDER_INACTIVE: Color = Color::Rgb(65, 72, 104); // #414868
pub const BG: Color = Color::Rgb(26, 27, 38);               // #1a1b26

pub const SPINNER_FRAMES: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

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
            .constraints([Constraint::Length(28), Constraint::Min(0)])
            .split(main_area);
        sidebar::render(frame, horiz[0], state);
        horiz[1]
    } else {
        main_area
    };

    // Right panel vertical split
    // chunks[0] = open-tabs row (Length 1)
    // chunks[1] = url bar (Length 3)
    // chunks[2] = request tab bar (Length 1)
    // chunks[3] = request editor (flexible)
    // chunks[4] = response meta (Length 1)
    // chunks[5] = response tab bar (Length 1)
    // chunks[6] = response viewer (flexible)
    let total_fixed: u16 = 1 + 3 + 1 + 1 + 1; // 7 rows fixed
    let remaining = right_area.height.saturating_sub(total_fixed);
    let editor_h = ((remaining as u32 * 35 / 100) as u16).max(3);
    let viewer_h = remaining.saturating_sub(editor_h).max(3);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),          // open tabs bar
            Constraint::Length(3),          // url bar
            Constraint::Length(1),          // request tab bar
            Constraint::Length(editor_h),   // request editor
            Constraint::Length(1),          // response meta line
            Constraint::Length(1),          // response tab bar
            Constraint::Min(viewer_h),      // response viewer
        ])
        .split(right_area);

    request_tabs::render(frame, chunks[0], state);
    url_bar::render(frame, chunks[1], state);
    req_tab_bar::render(frame, chunks[2], state);

    let active_tab = state.active_tab().map(|t| &t.active_tab);
    match active_tab.unwrap_or(&ActiveTab::Headers) {
        ActiveTab::Headers => headers_editor::render(frame, chunks[3], state),
        ActiveTab::Body    => body_editor::render(frame, chunks[3], state),
        ActiveTab::Auth    => auth_editor::render(frame, chunks[3], state),
        ActiveTab::Params  => params_editor::render(frame, chunks[3], state),
        ActiveTab::Scripts => scripts_editor::render(frame, chunks[3], state),
    }

    render_meta(frame, chunks[4], state);
    resp_tab_bar::render(frame, chunks[5], state);
    body_viewer::render(frame, chunks[6], state);

    status_bar::render(frame, status_area, state);

    // Overlay popups — rendered last so they appear on top
    match &state.active_popup {
        ActivePopup::None => {}
        ActivePopup::EnvSwitcher => env_editor::render_switcher(frame, area, state),
        ActivePopup::EnvEditor => env_editor::render_editor(frame, area, state),
        ActivePopup::WorkspaceSwitcher => workspace_switcher::render(frame, area, state),
        ActivePopup::CollectionNaming => naming_popup::render(frame, area, state),
        ActivePopup::ConfirmDelete => confirm_delete::render(frame, area, state),
    }
}

/// Helper used by sub-widgets to decide whether a rect is visible
#[allow(dead_code)]
pub fn is_visible(rect: Rect) -> bool {
    rect.width > 0 && rect.height > 0
}
