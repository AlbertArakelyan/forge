# forge — Build Progress

## Rounds

- [x] Round 1 — Core Request Engine
- [x] Round 2 — Environment Variables (partly - [more details here](SPEC.md#implementation-tasks-1))
- [x] Round 3 — Collections & Workspaces
- [ ] Round 4 — Authentication
- [ ] Round 5 — Request Headers & Query Params (Done Partly)
- [ ] Round 6 — Request Body Editor (Done Partly)
- [ ] Round 7 — Response Viewer
- [ ] Round 8 — History
- [ ] Round 9 — Scripting (Pre/Post Hooks)
- [ ] Round 10 — Streaming & SSE
- [ ] Round 11 — Import & Export
- [ ] Round 12 — Configuration & Theming
- [ ] Round 13 — Polish, Performance & Release

---

## Round 1 — Core Request Engine ✓

### Files Implemented (22 total)

| Layer | Files |
|---|---|
| Entry | `main.rs` |
| App | `app.rs` |
| Events | `event.rs` |
| Terminal | `terminal.rs` |
| Errors | `error.rs` |
| State | `state/app_state.rs`, `state/focus.rs`, `state/mode.rs`, `state/request_state.rs`, `state/response_state.rs` |
| HTTP | `http/client.rs`, `http/builder.rs`, `http/executor.rs` |
| UI | `ui/layout.rs`, `ui/sidebar.rs`, `ui/highlight.rs`, `ui/status_bar.rs` |
| UI/Request | `ui/request/url_bar.rs`, `ui/request/tab_bar.rs` |
| UI/Response | `ui/response/mod.rs`, `ui/response/tab_bar.rs`, `ui/response/body_viewer.rs` |

### Architecture

- **Event loop**: background thread feeds `crossterm` events + `Tick` into `mpsc::UnboundedChannel<Event>`
- **HTTP**: `tokio::spawn` + `tokio::select!` with `CancellationToken` for cancellation; result sent back as `Response` event
- **Rendering**: all `ui/` functions are pure — take `&AppState` + `Frame`, never mutate
- **Syntax highlighting**: `syntect` via `LazyLock`-initialized `SyntaxSet`/`ThemeSet`

### Gotchas & Fixes

- `reqwest` 0.12: `Response::cookies()` removed — cookies parsed from `Set-Cookie` headers manually
- `tokio-util` 0.7: `CancellationToken` lives under feature `"rt"`, not `"sync"` (doesn't exist)
- Mouse wheel scroll wired up in Normal mode via `MouseEventKind::ScrollUp/Down`
- Response scroll offset clamped to prevent over-scrolling past content

---

## Round 2 — Environment Variables (Done Partly)

### Files Implemented (10 total)

| Layer | Files |
|---|---|
| Env Core | `src/env/mod.rs`, `src/env/resolver.rs`, `src/env/interpolator.rs` |
| Storage | `src/storage/environment.rs` |
| State | `src/state/environment.rs`, `src/state/app_state.rs` (additions) |
| UI | `src/ui/env_editor.rs`, `src/ui/request/url_bar.rs` (additions) |
| App Logic | `src/app.rs` (additions), `src/ui/layout.rs` (additions) |

### What's Implemented

**Data Structures** (`src/state/environment.rs`):
- `Environment { id, name, color, variables }` — each env stored as TOML by UUID filename
- `EnvVariable { key, value, var_type, enabled, description }` — individually toggleable rows
- `VarType::Text | VarType::Secret` — secrets masked as `••••••••` in UI, sent as real value

**`{{variable}}` Parser** (`src/env/interpolator.rs`):
- `parse_vars(input) → Vec<(start, end, name)>` — byte-offset–aware, skips empty/unclosed braces
- Handles multiple variables per string: `{{scheme}}://{{host}}/path`

**EnvResolver — Layered Resolution** (`src/env/resolver.rs`):
- `resolver_from_state(state)` builds resolver with priority layers: active env → OS env vars
- `resolve(&str) → ResolvedString` — for display: secrets masked, unresolved kept as `{{name}}`, `VarSpan` list for UI highlighting
- `resolve_for_send(&str) → String` — for HTTP: secrets sent as real value; unresolved kept as-is
- UTF-8 byte-boundary aware throughout; unit-tested

**Variable Highlighting in URL Bar** (`src/ui/request/url_bar.rs`):
- Resolved vars highlighted cyan (`Rgb(42, 195, 222)`), unresolved red (`Rgb(247, 118, 142)`)
- Cursor mode: `build_highlighted_url_with_cursor()` renders cursor block inside variable spans
- Ghost text row 2: shows `→ resolved_url` (muted) so user sees what will be sent

**Variable Interpolation at Send-time** (`src/app.rs::send_request`):
- URL and enabled request headers resolved via `resolve_for_send()` before HTTP dispatch
- Resolver built from current `AppState` at send-time → live env switching without re-typing

**Environment Switcher Popup** (`src/ui/env_editor.rs::render_switcher`):
- `Ctrl+E` toggles popup; centered, ~50%×40%, `Clear` overlay darkens background
- List with active env (green `●`), search filter, selection with `j`/`k`
- `Enter` activate, `Alt+e` open editor, `Alt+n` new env (naming mode), `Alt+d` delete
- Active env first-selected on startup (index 0 if any exist)

**Environment Editor Popup** (`src/ui/env_editor.rs::render_editor`):
- Full table: key/value/description/type columns with per-column widths
- Row + column cursor `(row, col)`, inline Insert mode per cell
- `Tab` → next cell, auto-creates new row at end of last row; `r` renames env name
- `Space` context-aware: col 0 = toggle enabled, col 1 = toggle show_secret, col 3 = toggle var_type
- Secrets masked by default; `Space` on value col reveals plaintext

**TOML Persistence** (`src/storage/environment.rs`):
- `save(env)` writes `{id}.toml`; `load_all()` loads all `*.toml` on startup; `delete(id)` removes file
- Data dir: `%APPDATA%\forge\environments\` (Windows) / XDG / macOS equivalents
- Files named by UUID → rename-safe

### Missing / Not Yet Implemented

| Feature | Status |
|---|---|
| Variable interpolation in request body & auth fields | Not started |
| Secret variable encryption (AES-256-GCM, machine-local key) | Not started (plaintext storage) |
| "Unresolved variable" persistent warning indicator | Not started (red highlight works; no badge/warning) |

### Gotchas & Fixes

- All cursor positions tracked in **bytes** (not chars); `prev_char_boundary_of()` / `next_char_boundary_of()` helpers guard UTF-8 safety
- `resolve()` vs `resolve_for_send()` split keeps display masking decoupled from HTTP transmission
- `filtered_env_count()` prevents selection overflow when search narrows the list
- Environment file IDs are UUIDs → files survive renames without path changes

---

## Round 3 — Collections & Workspaces ✓

### Files Implemented (13 new/major files)

| Layer | Files |
|---|---|
| State | `state/collection.rs`, `state/workspace.rs`, `state/app_state.rs` (major migration) |
| Storage | `storage/workspace.rs`, `storage/collection.rs` |
| UI | `ui/sidebar.rs` (full rewrite), `ui/request_tabs.rs`, `ui/naming_popup.rs`, `ui/confirm_delete.rs`, `ui/workspace_switcher.rs` |
| App Logic | `app.rs` (sidebar CRUD, tab management, workspace switching) |

### Architecture

- **AppState migration**: `state.request`/`state.response` → `state.workspace.open_tabs[active_tab_idx]`; accessed via `state.active_tab()` / `state.active_tab_mut()`
- **Environments moved**: `state.environments` → `state.workspace.environments`; `state.active_env_idx` → `state.workspace.active_environment_idx`
- **Storage path**: `%APPDATA%/forge/workspaces/<ws-name>/` (Windows) / XDG / macOS equivalents
- **Sidebar tree**: `SidebarNode` enum flattened to a list via `flatten_collections()`; collapsed node IDs tracked in `sidebar.collapsed_ids: HashSet<Uuid>`
- **Tabs**: `WorkspaceState.open_tabs: Vec<RequestTab>`; `active_tab_idx` tracks focus; tabs persist to `workspace.toml`

### Keybindings Added

| Key | Action |
|---|---|
| `Ctrl+W` | Workspace switcher popup |
| `Ctrl+n` (Sidebar) | New collection |
| `n` (Sidebar) | New request |
| `f` (Sidebar) | New folder |
| `r` (Sidebar) | Rename selected item |
| `d` (Sidebar) | Delete selected item |
| `D` (Sidebar) | Duplicate selected item |
| `h` / `l` (Sidebar) | Collapse / expand node |
| `/` (Sidebar) | Toggle search mode |
| `Alt+1–9` | Switch to tab N |
| `Alt+w` | Close active tab |
| `[` / `]` | Cycle open tabs (non-UrlBar focus) |

### Gotchas & Fixes

- Sidebar search runs inline at the footer row (repurposed hint row); `NamingState` carries the HTTP method for new requests so method persists through the naming popup flow
- `active_tab()` returns `Option<&RequestTab>` — all render functions must handle `None` gracefully
- Workspace save triggered on every tab/request mutation via `dirty` flag; debounced via the Tick event
- `flatten_collections()` recurses into folders and respects `collapsed_ids` to hide children

---

## Round 5 — Request Headers & Query Params (Done Partly)

### What's Implemented

**Key-Value Editor** — generic reusable component built for the Headers tab:
- Add / remove key-value rows
- Toggle individual rows enabled/disabled
- Navigate rows with `j`/`k`, edit cells in Insert mode
- Used in `ui/request/headers_editor.rs` (or equivalent); can be re-used for Query Params, form body, etc.

### Missing / Not Yet Implemented

| Feature | Status |
|---|---|
| Query Params tab | Not started |
| `{{variable}}` interpolation in header values | Not started |
| Bidirectional URL ↔ Query Params sync | Blocked on Query Params |

---

## Round 6 — Request Body Editor (Done Partly)

### What's Implemented

**Raw JSON Body Editor** — basic body editing for JSON requests:
- Raw text editor for JSON body content
- Wired into the Body tab of the request panel

### Missing / Not Yet Implemented

| Feature | Status |
|---|---|
| Body type selector (JSON / Form / Multipart / GraphQL / Raw / Binary) | Not started |
| Form URL-encoded body (key-value editor) | Not started |
| Multipart form body | Not started |
| GraphQL body (query + variables) | Not started |
| Raw body (plain text, XML, etc.) | Not started |
| Binary file upload | Not started |
