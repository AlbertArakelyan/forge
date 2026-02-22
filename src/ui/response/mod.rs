pub mod tab_bar;
pub mod body_viewer;
pub mod headers_viewer;
pub mod cookies_viewer;
pub mod timing_viewer;

use ratatui::{Frame, layout::Rect};
use crate::state::app_state::AppState;

pub fn render_meta(frame: &mut Frame, area: Rect, state: &AppState) {
    body_viewer::render_meta(frame, area, state);
}
