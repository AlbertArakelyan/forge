# Round 3 — Collections & Workspaces: Step-by-Step Guide

## Overview
Implement collections, workspaces, sidebar tree, open-tabs bar, and all related popups.
Build in phases so each step compiles independently.

---

## Phase 1 — State Types

### Step 1 — `src/state/collection.rs`
New file. Defines `Collection`, `Folder`, `CollectionItem`.

### Step 2 — `src/state/workspace.rs`
Add `WorkspaceFile`, `WorkspaceState`, `RequestTab`.
`RequestTab` bundles `request + response + active_tab + response_tab + is_dirty`.

### Step 3 — `src/state/app_state.rs`
- Add `SidebarState` (cursor, collapsed_ids, search_mode, search_query, scroll_offset)
- Add `WorkspaceSwitcherState` (selected, search, search_cursor, naming, new_name, new_name_cursor)
- Add `NamingTarget` enum (NewCollection, NewFolder{collection_id}, NewRequest{collection_id, folder_id}, Rename{id, old_name})
- Add `NamingState` (target, input, cursor)
- Add `ConfirmDeleteState` (message, target_id)
- Extend `ActivePopup`: add WorkspaceSwitcher, CollectionNaming, ConfirmDelete
- Migrate `AppState`: replace `request`/`response`/`active_tab`/`response_tab`/`environments`/`active_env_idx`
  with `workspace: WorkspaceState`, `all_workspaces: Vec<String>`, `sidebar: SidebarState`,
  `naming: NamingState`, `confirm_delete: ConfirmDeleteState`, `ws_switcher: WorkspaceSwitcherState`
- Add `active_tab() -> Option<&RequestTab>` and `active_tab_mut() -> Option<&mut RequestTab>` helpers

### Step 4 — `src/state/mod.rs`
Add `pub mod collection;`

---

## Phase 2 — Storage Layer

### Step 5 — `src/storage/collection.rs`
Implement:
- `data_dir(ws_name) -> PathBuf` → `<data>/forge/workspaces/<ws>/collections/`
- `load_all_collections(ws_name) -> Vec<Collection>`
- `save_collection_meta(ws_name, col) -> Result<()>`
- `delete_collection(ws_name, col_name) -> Result<()>`

### Step 6 — `src/storage/workspace.rs`
Implement:
- `workspaces_dir() -> PathBuf` → `<data>/forge/workspaces/`
- `list_workspaces() -> Vec<String>`
- `load_workspace(name) -> WorkspaceFile`
- `save_workspace(ws) -> Result<()>`
- `load_workspace_full(name) -> WorkspaceState`

### Step 7 — `src/storage/environment.rs`
Add workspace-scoped variants alongside the existing global ones:
- `save_ws(ws_name, env)`
- `delete_ws(ws_name, id)`
- `load_all_ws(ws_name) -> Vec<Environment>`

---

## Phase 3 — Sidebar Tree UI

### Step 8 — `src/ui/sidebar.rs` (full rewrite)
Define `SidebarNode { depth, kind (Collection|Folder|Request), id, label, is_collapsed, method }`.
`flatten_collections(state)` builds the flat list with proper indent.
`render()` iterates the flat list, draws:
  - Collections: bold label + `▶`/`▼` arrow
  - Folders: indent + arrow + label
  - Requests: indent + method badge + label
  - Highlight active cursor row with ACCENT_BLUE bg
  - Show search bar at bottom when `state.sidebar.search_mode`

---

## Phase 4 — New UI Components

### Step 9 — `src/ui/request_tabs.rs`
Render the open-tabs row between URL bar and request tab bar.
Each tab: `[METHOD  NAME*]` — `*` if dirty, active tab in ACCENT_BLUE bold.

### Step 10 — `src/ui/naming_popup.rs`
Centered popup. Shows prompt based on `NamingTarget`, text input with cursor, Enter=confirm, Esc=cancel.

### Step 11 — `src/ui/confirm_delete.rs`
Centered popup. Shows message. `[y/Enter] Yes  [n/Esc] No`. Highlights based on confirm state.

### Step 12 — `src/ui/workspace_switcher.rs`
Centered popup. Search bar at top, workspace list with `●` for active, `Alt+N` to start naming new workspace, Enter=switch.

---

## Phase 5 — Layout Integration

### Step 13 — `src/ui/layout.rs`
- Add `request_tabs` row (Length 1) above URL bar in right panel
- Import and call new popup renderers inside the `match &state.active_popup` block
- Update layout constraint array to account for new row

### Step 14 — `src/ui/mod.rs`
Add `pub mod request_tabs; pub mod naming_popup; pub mod confirm_delete; pub mod workspace_switcher;`

---

## Phase 6 — Keybinding Handlers

### Step 15 — `src/app.rs` state migration
Update all `state.request`, `state.response`, `state.active_tab`, `state.response_tab`,
`state.environments`, `state.active_env_idx` references to use:
- `state.active_tab()`, `state.active_tab_mut()`
- `state.workspace.environments`, `state.workspace.active_environment_idx`

### Step 16 — Sidebar keys in `app.rs`
`handle_sidebar_key()`: j/k cursor, h/l collapse/expand, Enter open/toggle, `/` enter search,
in search mode chars build query, Esc exits search.

### Step 17 — Sidebar CRUD in `app.rs`
N=new collection→NamingPopup(NewCollection), n=new request→NamingPopup(NewRequest),
f=new folder→NamingPopup(NewFolder), r=rename→NamingPopup(Rename), d=delete→ConfirmDelete, D=duplicate.

### Step 18 — NamingPopup handler
char input, Backspace, Enter dispatch create/rename action + close popup, Esc cancel.

### Step 19 — ConfirmDelete handler
y/Enter dispatch delete + close, n/Esc cancel.

### Step 20 — WorkspaceSwitcher handler
Ctrl+W open; inside: j/k navigate, `/` search, Alt+N naming mode, Enter switch workspace, Esc close.

### Step 21 — Tab management
`[`/`]` when focus ≠ UrlBar → cycle open tabs; Alt+1–9 → jump to tab N; Alt+W → close active tab.

### Step 22 — Dirty flag
Set `active_tab_mut().is_dirty = true` on any request field edit.

---

## Phase 7 — Startup & Auto-save

### Step 23 — `main.rs` / `App::new()` startup
Call `load_workspace_full("default")` or last-used workspace; call `list_workspaces()`.
Initialize `AppState.workspace` and `AppState.all_workspaces`.

### Step 24 — Auto-save
On tab close or workspace switch, if tab `is_dirty`, save request TOML.

---

## Verification Checklist
- [ ] `cargo check` — zero errors
- [ ] `cargo test` — all tests pass
- [ ] Launch: sidebar shows correctly
- [ ] Ctrl+W opens workspace switcher
- [ ] N creates collection, n creates request
- [ ] Enter opens request in new tab
- [ ] `[`/`]` cycles between open tabs
- [ ] Ctrl+E / Ctrl+W env and workspace popups work
