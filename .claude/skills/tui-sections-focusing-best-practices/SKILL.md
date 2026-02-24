---
name: tui-sections-focusing-best-practices
description: >
  Best practices for implementing focusable sections in a Ratatui/crossterm TUI app.
  Use when adding a new focusable widget, wiring a focus cycle, showing focus indicators,
  or deciding how keybindings should be scoped to focused sections.
---

# TUI Section Focus — Best Practices

## Core model

Focus is a property of `AppState`, not of individual widgets.
One enum variant per focusable section (e.g. `Sidebar`, `UrlBar`, `TabBar`, `Editor`, `ResponseViewer`).

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Focus {
    Sidebar,
    #[default]
    UrlBar,
    TabBar,
    Editor,
    ResponseViewer,
}
```

`AppState.focus: Focus` is the single source of truth.
Widgets read it; they never write it.

---

## Focus cycle

Add `next()` and `prev()` methods to `Focus`.
Document the order in the comment — it's the canonical reference.

```rust
/// Cycle: Sidebar → UrlBar → TabBar → Editor → ResponseViewer → Sidebar
pub fn next(&self) -> Focus { ... }
pub fn prev(&self)  -> Focus { ... }
```

- `Tab` calls `self.state.focus = self.state.focus.next()`.
- `BackTab` calls `.prev()`.
- Numeric shortcuts (`1`–`N`) jump directly to a section.

When inserting a new section, update *both* `next()` and `prev()`, update
the doc comment, and add a direct shortcut if the section is frequently used.

---

## Keybindings scoped to focus

Guard every section-specific keybinding with a focus check.
Use a `if` guard on the match arm — never rely on mode alone.

```rust
KeyCode::Left if self.state.focus == Focus::TabBar => {
    self.state.active_tab = self.state.active_tab.prev();
}
KeyCode::Right if self.state.focus == Focus::TabBar => {
    self.state.active_tab = self.state.active_tab.next();
}
```

This prevents key collisions between sections that share the same key
(e.g. `Left`/`Right` in UrlBar for cursor vs. TabBar for tab switch).

---

## Visual focus indicator

Each widget is responsible for rendering its own focus state.
Receive `state: &AppState` and check `state.focus == Focus::MySection`.

**Border colour** (most common):
```rust
let border_style = if state.focus == Focus::Editor {
    Style::default().fg(ACCENT_BLUE)
} else {
    Style::default().fg(BORDER_INACTIVE)
};
Block::default().borders(Borders::ALL).border_style(border_style)
```

**Bracket label** (for tab bars / inline items):
```rust
let label = if is_active && state.focus == Focus::TabBar {
    format!("[{name}]")
} else {
    name.to_string()
};
```

Avoid flashing: indicator should be based entirely on `state.focus` — no
animation or timer needed.

---

## Sub-selection within a section

When a section contains multiple selectable items (tabs, list rows, etc.),
store the selection in `AppState` alongside `focus`.

```rust
pub active_tab: ActiveTab,   // which tab is selected
pub focus: Focus,            // which section owns keyboard input
```

Provide `next()`/`prev()` on the sub-selection enum too.
Switch sub-selection only when `focus == Focus::ThatSection`.

---

## Dirty flag discipline

Set `AppState.dirty = true` only when visible state actually changes.

- Key events: set dirty in the event dispatcher (always safe).
- Sub-selection change: already covered by the key event path.
- Focus change: covered by Tab/BackTab handler.
- Tick: set dirty **only** during loading spinner (`RequestStatus::Loading`).

---

## Checklist for a new focusable section

- [ ] Add variant to `Focus` enum
- [ ] Update `Focus::next()` and `Focus::prev()`
- [ ] Update doc comment on `next()`
- [ ] Add direct numeric shortcut in `handle_normal_key()` if needed
- [ ] Render focus indicator inside the widget's `render()` function
- [ ] Guard section-specific keybindings with `focus == Focus::MySection`
- [ ] If section has sub-selection: add sub-enum + `next()`/`prev()` + store in `AppState`
