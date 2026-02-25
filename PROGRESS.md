# forge — Build Progress

## Rounds

- [x] Round 1 — Core Request Engine
- [ ] Round 2 — Environment Variables
- [ ] Round 3 — Collections & Workspaces
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
