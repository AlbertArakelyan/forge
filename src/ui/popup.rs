use ratatui::layout::Rect;

/// Returns a centered `Rect` that is `percent_x`% wide and `percent_y`% tall
/// relative to `area`. Minimum 1×1.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_w = (area.width as u32 * percent_x as u32 / 100).max(1) as u16;
    let popup_h = (area.height as u32 * percent_y as u32 / 100).max(1) as u16;

    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;

    Rect { x, y, width: popup_w.min(area.width), height: popup_h.min(area.height) }
}
