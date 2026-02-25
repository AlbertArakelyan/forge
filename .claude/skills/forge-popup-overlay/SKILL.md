---
name: forge-popup-overlay
description: >
  How to add a new modal popup overlay in forge (Ratatui + crossterm).
  Use when implementing any new popup: collection browser, command palette,
  history viewer, import dialog, auth picker, theme picker, etc.
---

# forge Popup Overlay — Implementation Pattern

## 1. Add variant to `ActivePopup` (`src/state/app_state.rs`)

```rust
pub enum ActivePopup {
    None,
    EnvSwitcher,
    EnvEditor,
    MyNewPopup,   // ← add here
}
```

## 2. Add state struct (same file or new file in `src/state/`)

```rust
#[derive(Debug, Clone, Default)]
pub struct MyNewPopupState {
    pub selected: usize,
    pub search: String,
    // ...
}
```

Add the field to `AppState`:
```rust
pub my_new_popup: MyNewPopupState,
```

## 3. Create render function (`src/ui/my_new_popup.rs`)

```rust
use ratatui::widgets::Clear;
use crate::ui::popup_utils::centered_rect;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(60, 50, area);  // % width, % height
    frame.render_widget(Clear, popup_area);         // ← always Clear first
    // render your block + content into popup_area
}
```

`centered_rect` is defined in `src/ui/env_editor.rs` (or extract to a shared util).

## 4. Wire into layout renderer (`src/ui/layout.rs`)

```rust
match &state.active_popup {
    ActivePopup::None => {}
    ActivePopup::EnvSwitcher => env_editor::render_switcher(frame, area, state),
    ActivePopup::EnvEditor   => env_editor::render_editor(frame, area, state),
    ActivePopup::MyNewPopup  => my_new_popup::render(frame, area, state),
}
```

Popups are rendered **last** so they draw on top of everything else.

## 5. Wire key handler (`src/app.rs`)

```rust
fn handle_popup_key(&mut self, key: KeyEvent) {
    match self.state.active_popup {
        ActivePopup::EnvSwitcher => self.handle_env_switcher_key(key),
        ActivePopup::EnvEditor   => self.handle_env_editor_key(key),
        ActivePopup::MyNewPopup  => self.handle_my_new_popup_key(key),
        ActivePopup::None => {}
    }
}
```

`handle_popup_key` is called from the normal-key handler before any global key
processing, so popup keys shadow global ones cleanly.

## 6. Keybinding rule — CRITICAL

**Never bind bare letter keys as popup actions when the popup also has a search/text field.**

If the popup has a free-text input (search box, name entry, etc.), bare `a`–`z`
will be consumed by the text field and must NOT trigger actions.

Use `Alt+key` for destructive/navigational actions inside popups with text input:
```rust
// CORRECT — won't conflict with typing in search box
KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::ALT) => { … }
KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => { … }
KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => { … }
```

Update the hint bar to show `Alt+e`, `Alt+n`, `Alt+d` so the user can discover them.

Safe bare-letter actions: only if the popup has NO text input at all (pure navigation list).

## 7. Hint bar convention

Bottom row of the popup block shows all keys. Pattern:
```rust
Line::from(vec![
    Span::styled("Enter", Style::default().fg(TEXT_PRIMARY)),
    Span::styled(" select  ", Style::default().fg(TEXT_MUTED)),
    Span::styled("Alt+e", Style::default().fg(TEXT_PRIMARY)),
    Span::styled(" edit  ", Style::default().fg(TEXT_MUTED)),
    Span::styled("Esc", Style::default().fg(TEXT_PRIMARY)),
    Span::styled(" close", Style::default().fg(TEXT_MUTED)),
])
```

Render with `.add_modifier(Modifier::DIM)` on the paragraph for visual subtlety.

## 8. Auto-activate new items

When a user creates a new item inside a popup (e.g. new environment, new collection),
immediately make it the active/selected item in AppState:
```rust
self.state.environments.push(new_env);
let i = self.state.environments.len() - 1;
self.state.active_env_idx = Some(i);  // ← select it immediately
```

Also: on startup, auto-select index 0 if a default active item exists, so users
don't have to re-select after every restart.

## Checklist

- [ ] Add variant to `ActivePopup`
- [ ] Add state struct + field in `AppState`
- [ ] Create `src/ui/my_new_popup.rs` with `render(frame, area, state)`
- [ ] Call `frame.render_widget(Clear, popup_area)` before drawing
- [ ] Add arm to `match state.active_popup` in `layout.rs`
- [ ] Add arm to `handle_popup_key` in `app.rs`
- [ ] Implement `handle_my_new_popup_key` — use `Alt+key` for actions if there's a text field
- [ ] Add hint bar at bottom of popup showing all keybindings
- [ ] If user creates items: auto-activate the new item immediately
