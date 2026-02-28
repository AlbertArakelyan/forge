# forge ⚒️

> A blazingly fast, terminal-native API client — Postman in your terminal, built in Rust.

---

## Table of Contents

1. [Project Vision](#project-vision)
2. [Architecture Overview](#architecture-overview)
3. [Project Structure](#project-structure)
4. [Tech Stack](#tech-stack)
5. [TUI Layout & Design System](#tui-layout--design-system)
6. [Keybinding System](#keybinding-system)
7. [File-Based Storage Format](#file-based-storage-format)
8. [Round 1 — Core Request Engine](#round-1--core-request-engine)
9. [Round 2 — Environment Variables](#round-2--environment-variables)
10. [Round 3 — Collections & Workspaces](#round-3--collections--workspaces)
11. [Round 4 — Authentication](#round-4--authentication)
12. [Round 5 — Request Headers & Query Params](#round-5--request-headers--query-params)
13. [Round 6 — Request Body Editor](#round-6--request-body-editor)
14. [Round 7 — Response Viewer](#round-7--response-viewer)
15. [Round 8 — History](#round-8--history)
16. [Round 9 — Scripting (Pre/Post Hooks)](#round-9--scripting-prepost-hooks)
17. [Round 10 — Streaming & SSE](#round-10--streaming--sse)
18. [Round 11 — Import & Export](#round-11--import--export)
19. [Round 12 — Configuration & Theming](#round-12--configuration--theming)
20. [Round 13 — Polish, Performance & Release](#round-13--polish-performance--release)
21. [Testing Strategy](#testing-strategy)
22. [Milestones Summary](#milestones-summary)

---

## Project Vision

**forge** is a terminal-native API client built in Rust using `ratatui`. It is the tool developers reach for when they want Postman's power without Postman's weight — no Electron, no account, no 300MB install, no cloud sync unless you want it.

### Why forge wins

| | Postman | Insomnia | Bruno | **forge** |
|---|---|---|---|---|
| Install size | ~300MB | ~250MB | ~120MB | **< 5MB** |
| Startup time | 3–8s | 2–5s | 1–3s | **< 50ms** |
| Requires account | Yes | Yes (new) | No | **No** |
| File-based storage | No | No | Yes | **Yes** |
| Git-friendly | No | No | Yes | **Yes** |
| Terminal native | No | No | No | **Yes** |
| Vim keybindings | No | No | No | **Yes** |
| Open source | No | Partial | Yes | **Yes** |
| Offline first | Partial | Partial | Yes | **Yes** |

### Strategic Relationship with fyr

forge is built **on top of fyr's HTTP engine**. The two projects share:
- The same request builder (`reqwest` + `tokio`)
- The same item syntax parser
- The same session/auth abstractions
- The same streaming infrastructure

fyr is the CLI knife. forge is the full workshop.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          forge TUI                              │
│                                                                 │
│  ┌──────────────┐  ┌──────────────────────────────────────┐    │
│  │  Sidebar     │  │           Main Panel                 │    │
│  │              │  │                                      │    │
│  │ Workspaces   │  │  ┌─────────────────────────────┐    │    │
│  │ Collections  │  │  │     Request Editor          │    │    │
│  │ Environments │  │  │  [Method] [URL Bar] [Send]  │    │    │
│  │ History      │  │  │  [Headers][Body][Auth][Params]   │    │
│  │              │  │  └─────────────────────────────┘    │    │
│  │              │  │  ┌─────────────────────────────┐    │    │
│  │              │  │  │     Response Viewer         │    │    │
│  │              │  │  │  Status | Time | Size       │    │    │
│  │              │  │  │  [Body] [Headers] [Cookies] │    │    │
│  │              │  │  └─────────────────────────────┘    │    │
│  └──────────────┘  └──────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Status Bar: mode | env | workspace | keyhints          │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### State Machine

forge is a pure event-driven state machine:

```
AppState
  ├── mode: Normal | Insert | Command | Visual
  ├── focus: Sidebar | UrlBar | TabBar | Editor | ResponseViewer
  ├── active_workspace: WorkspaceId
  ├── active_collection: Option<CollectionId>
  ├── active_request: Option<RequestId>
  ├── active_environment: Option<EnvironmentId>
  ├── active_tab: Headers | Body | Auth | Params | Scripts
  ├── response_tab: Body | Headers | Cookies | Timing
  └── pending_request: Option<JoinHandle<Response>>
```

All state transitions are pure functions: `(AppState, Event) → AppState`. No mutation outside of the reducer. This makes the app trivially testable and the TUI always in sync with reality.

---

## Project Structure

```
forge/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── LICENSE
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── release.yml
│   │   └── bench.yml
│   └── ISSUE_TEMPLATE/
├── src/
│   ├── main.rs                      # Entry: init terminal, run event loop
│   ├── app.rs                       # AppState struct + root reducer
│   ├── event.rs                     # Event enum: Key, Mouse, Tick, Response, Resize
│   ├── terminal.rs                  # Terminal setup/teardown (crossterm)
│   │
│   ├── ui/                          # Pure rendering functions (no state mutation)
│   │   ├── mod.rs
│   │   ├── layout.rs                # Root layout: sidebar + main + statusbar
│   │   ├── sidebar.rs               # Collections/workspaces/history tree
│   │   ├── request/
│   │   │   ├── mod.rs
│   │   │   ├── url_bar.rs           # Method selector + URL input + Send button
│   │   │   ├── tab_bar.rs           # Headers / Body / Auth / Params / Scripts tabs
│   │   │   ├── headers_editor.rs    # Key-value editor for headers
│   │   │   ├── params_editor.rs     # Key-value editor for query params
│   │   │   ├── body_editor.rs       # Body tab: raw/JSON/form/multipart
│   │   │   ├── auth_editor.rs       # Auth tab: None/Basic/Bearer/OAuth2/API Key
│   │   │   └── scripts_editor.rs    # Pre/post request script editor
│   │   ├── response/
│   │   │   ├── mod.rs
│   │   │   ├── tab_bar.rs           # Body / Headers / Cookies / Timing
│   │   │   ├── body_viewer.rs       # Syntax-highlighted body, scrollable
│   │   │   ├── headers_viewer.rs    # Response headers table
│   │   │   ├── cookies_viewer.rs    # Cookies table
│   │   │   └── timing_viewer.rs     # Waterfall timing breakdown
│   │   ├── env_editor.rs            # Environment variable editor popup
│   │   ├── status_bar.rs            # Bottom status bar
│   │   ├── command_palette.rs       # Fuzzy command palette (: mode)
│   │   ├── popup.rs                 # Generic modal popup wrapper
│   │   └── highlight.rs             # syntect integration for ratatui
│   │
│   ├── state/                       # All state types and reducers
│   │   ├── mod.rs
│   │   ├── app_state.rs             # Root AppState
│   │   ├── focus.rs                 # Focus enum and focus transitions
│   │   ├── mode.rs                  # Modal mode: Normal/Insert/Command/Visual
│   │   ├── workspace.rs             # Workspace state
│   │   ├── request_state.rs         # Active request being edited
│   │   └── response_state.rs        # Last received response
│   │
│   ├── actions/                     # All dispatchable actions
│   │   ├── mod.rs
│   │   ├── navigation.rs            # Focus changes, tab switches, sidebar nav
│   │   ├── request.rs               # Edit URL, method, headers, body, etc.
│   │   ├── response.rs              # Handle received response
│   │   ├── collection.rs            # CRUD for collections/requests
│   │   ├── environment.rs           # CRUD for environments/variables
│   │   ├── history.rs               # Add/clear history entries
│   │   └── workspace.rs             # Switch/create/delete workspaces
│   │
│   ├── http/                        # HTTP execution layer (shared with fyr)
│   │   ├── mod.rs
│   │   ├── client.rs                # reqwest client factory
│   │   ├── executor.rs              # Execute request, return Response future
│   │   ├── builder.rs               # Build reqwest::Request from RequestState
│   │   ├── auth.rs                  # Auth injection (basic, bearer, oauth2, api key)
│   │   └── stream.rs                # Streaming response handler
│   │
│   ├── storage/                     # File-based persistence layer
│   │   ├── mod.rs
│   │   ├── workspace.rs             # Read/write workspace.toml
│   │   ├── collection.rs            # Read/write collection files
│   │   ├── environment.rs           # Read/write environment files
│   │   ├── history.rs               # Read/write history.toml
│   │   └── config.rs                # Read/write config.toml
│   │
│   ├── env/                         # Environment variable resolution
│   │   ├── mod.rs
│   │   ├── resolver.rs              # Resolve {{variable}} in URL, headers, body
│   │   └── interpolator.rs          # String interpolation engine
│   │
│   ├── scripting/                   # Pre/post request hooks (Rhai engine)
│   │   ├── mod.rs
│   │   ├── engine.rs                # Rhai script engine setup
│   │   ├── context.rs               # Script context: request, response, env objects
│   │   └── stdlib.rs                # Built-in functions available in scripts
│   │
│   └── error.rs                     # Unified error types
│
├── tests/
│   ├── integration/
│   │   ├── request_execution.rs
│   │   ├── env_resolution.rs
│   │   ├── collection_storage.rs
│   │   ├── scripting.rs
│   │   └── import_export.rs
│   └── fixtures/
│       ├── collections/
│       ├── environments/
│       └── mock_server.rs
│
├── benches/
│   ├── startup.rs
│   └── large_response_render.rs
│
└── docs/
    ├── install.md
    ├── keybindings.md
    ├── collections.md
    ├── environments.md
    ├── scripting.md
    └── import-export.md
```

---

## Tech Stack

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# TUI framework
ratatui = "0.27"

# Terminal backend
crossterm = "0.27"

# HTTP client (shared with fyr)
reqwest = { version = "0.12", features = [
  "json", "stream", "multipart",
  "gzip", "brotli", "rustls-tls"
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Syntax highlighting (for response body)
syntect = "5"

# Fuzzy search (command palette + collection search)
fuzzy-matcher = "0.3"

# Scripting engine (pre/post hooks)
rhai = "1"

# Environment variable interpolation
# (custom implementation — see env/interpolator.rs)

# File watching (auto-reload changed collection files)
notify = "6"

# Error handling
thiserror = "1"
anyhow = "1"

# Platform config/data dirs
dirs = "5"

# UUID for request/collection IDs
uuid = { version = "1", features = ["v4"] }

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Human-readable sizes and durations
humansize = "2"
humantime = "2"

# URL parsing
url = "2"

# Mime type detection
mime = "0.3"
mime_guess = "2"

# Clipboard support
arboard = "3"

# Import: parse Postman/Insomnia/OpenAPI formats
serde_yaml = "0.9"            # OpenAPI/Insomnia import

[dev-dependencies]
wiremock = "0.6"
assert_cmd = "2"
tempfile = "3"
insta = { version = "1", features = ["json"] }
```

---

## TUI Layout & Design System

### Screen Regions

```
┌─────────────────────────────────────────────────────────────────┐  ← Row 0
│  ⚒  forge  [workspace: MyAPI ▾]  [env: development ▾]          │  ← Header bar (1 row)
├──────────────┬──────────────────────────────────────────────────┤  ← Row 1
│              │  GET  https://api.example.com/users    [Send ↵]  │  ← URL bar (3 rows)
│  COLLECTIONS │──────────────────────────────────────────────────│
│  ▼ MyAPI     │  Headers  Body  Auth  Params  Scripts            │  ← Tab bar (1 row)
│    ▶ Users   │──────────────────────────────────────────────────│
│    ▶ Auth    │                                                  │
│    ▶ Orders  │           [Request Editor Panel]                 │  ← Request editor
│              │                                                  │    (~40% height)
│  ENVIRONMENTS│──────────────────────────────────────────────────│
│  ● development│ 200 OK  ·  142ms  ·  1.2 KB                    │  ← Response meta (1 row)
│  ○ staging   │──────────────────────────────────────────────────│
│  ○ production│  Body  Headers  Cookies  Timing                  │  ← Response tab bar
│              │──────────────────────────────────────────────────│
│  HISTORY     │                                                  │
│  GET /users  │           [Response Viewer Panel]                │  ← Response viewer
│  POST /auth  │                                                  │    (~40% height)
│              │                                                  │
├──────────────┴──────────────────────────────────────────────────┤
│  NORMAL  ·  development  ·  MyAPI  ·  ?:help  /:search  q:quit  │  ← Status bar (1 row)
└─────────────────────────────────────────────────────────────────┘  ← Last row
```

### Color Palette (Default Dark Theme)

| Element | Color | Hex |
|---------|-------|-----|
| Background | Dark gray | `#1a1b26` |
| Surface | Slightly lighter | `#24283b` |
| Border (inactive) | Muted | `#414868` |
| Border (active/focus) | Accent blue | `#7aa2f7` |
| Text primary | Off-white | `#c0caf5` |
| Text muted | Gray | `#565f89` |
| GET method | Green | `#9ece6a` |
| POST method | Yellow | `#e0af68` |
| PUT method | Blue | `#7aa2f7` |
| DELETE method | Red | `#f7768e` |
| PATCH method | Purple | `#bb9af7` |
| Status 2xx | Green | `#9ece6a` |
| Status 4xx | Yellow/Orange | `#ff9e64` |
| Status 5xx | Red | `#f7768e` |
| Env variable | Cyan | `#2ac3de` |

### Focus Indicators

Focused panels get a colored border. All other panels use the muted border color. The status bar always shows the current focus region and mode.

---

## Keybinding System

### Modal Modes (Vim-inspired)

forge has four modes, displayed in the status bar:

| Mode | Trigger | Purpose |
|------|---------|---------|
| `NORMAL` | Default / `Esc` | Navigate between panels, execute commands |
| `INSERT` | `i` or `a` | Edit text fields |
| `COMMAND` | `:` | Command palette (fuzzy commands) |
| `VISUAL` | `v` | Select text in response body (for copy) |

### Global Keybindings (all modes)

| Key | Action |
|-----|--------|
| `Ctrl+q` | Quit forge |
| `Ctrl+s` | Save current request |
| `Ctrl+r` | Send current request |
| `Ctrl+p` / `:` | Open command palette |
| `F1` / `?` | Open help popup |
| `Ctrl+e` | Toggle environment selector |
| `Ctrl+w` | Toggle workspace selector |
| `Tab` | Cycle focus forward |
| `Shift+Tab` | Cycle focus backward |
| `Ctrl+/` | Toggle sidebar |

### Normal Mode Keybindings

| Key | Action |
|-----|--------|
| `h` `j` `k` `l` | Navigate (vim-style) |
| `↑` `↓` `←` `→` | Navigate (arrow keys) |
| `Enter` | Activate / open focused item |
| `i` | Enter Insert mode on focused editable field |
| `n` | New request |
| `N` | New collection |
| `d` | Delete focused item (with confirmation) |
| `r` | Rename focused item |
| `D` | Duplicate focused request |
| `y` | Yank (copy) response body to clipboard |
| `/` | Search/filter sidebar |
| `g g` | Go to top |
| `G` | Go to bottom |
| `Ctrl+u` | Scroll up half page |
| `Ctrl+d` | Scroll down half page |
| `1` | Focus: URL bar |
| `2` | Focus: Request editor |
| `3` | Focus: Response viewer |
| `4` | Focus: Sidebar |
| `[` / `]` | Previous / next request tab |
| `{` / `}` | Previous / next response tab |

### Insert Mode Keybindings

| Key | Action |
|-----|--------|
| `Esc` | Return to Normal mode, save field |
| `Enter` | Confirm and move to next field |
| `Ctrl+a` | Select all text in field |
| `Ctrl+u` | Clear field |
| `Tab` | Auto-complete (URL history, env variables) |

### Command Palette Commands

Access with `:` in Normal mode. Fuzzy-searchable.

| Command | Action |
|---------|--------|
| `new request` | Create new request |
| `new collection` | Create new collection |
| `new environment` | Create new environment |
| `new workspace` | Create new workspace |
| `import postman` | Import Postman collection |
| `import openapi` | Import OpenAPI/Swagger spec |
| `export collection` | Export current collection |
| `switch env <name>` | Switch active environment |
| `switch workspace <name>` | Switch active workspace |
| `clear history` | Clear request history |
| `copy response` | Copy response body to clipboard |
| `copy url` | Copy current request URL to clipboard |
| `set theme <name>` | Switch color theme |
| `set layout <name>` | Switch layout preset |
| `quit` | Quit forge |

---

## File-Based Storage Format

All forge data lives in plain files on disk. No database. No proprietary format. Everything is human-readable, diffable, and Git-friendly.

### Directory Layout

```
~/.local/share/forge/           (Linux)
~/Library/Application Support/forge/   (macOS)
%APPDATA%\forge\                (Windows)
│
├── config.toml                 # Global config
├── workspaces/
│   ├── default/
│   │   ├── workspace.toml      # Workspace metadata
│   │   ├── collections/
│   │   │   ├── myapi/
│   │   │   │   ├── collection.toml   # Collection metadata
│   │   │   │   ├── users/
│   │   │   │   │   ├── get-users.toml
│   │   │   │   │   ├── create-user.toml
│   │   │   │   │   └── delete-user.toml
│   │   │   │   └── auth/
│   │   │   │       ├── login.toml
│   │   │   │       └── refresh.toml
│   │   ├── environments/
│   │   │   ├── development.toml
│   │   │   ├── staging.toml
│   │   │   └── production.toml
│   │   └── history.toml
│   └── work/
│       └── ...
```

### Request File Format (`get-users.toml`)

```toml
[meta]
id = "req_01j8x..."
name = "Get Users"
description = "Fetch paginated list of users"
created = "2025-01-01T00:00:00Z"
updated = "2025-01-15T10:30:00Z"

[request]
method = "GET"
url = "{{base_url}}/users"

[request.params]
page = "1"
limit = "20"
search = ""         # empty params are included but empty

[request.headers]
Accept = "application/json"
X-API-Version = "2"

[request.auth]
type = "bearer"     # none | basic | bearer | api_key | oauth2
token = "{{auth_token}}"

[request.body]
type = "none"       # none | json | form | multipart | raw | graphql

[scripts]
pre_request = """
// Runs before request is sent
// Available: request, env, console
console.log("Sending request to: " + request.url);
"""

post_request = """
// Runs after response is received
// Available: request, response, env, console
if (response.status == 200) {
    env.set("last_user_count", response.json().total);
}
"""
```

### Collection File Format (`collection.toml`)

```toml
[meta]
id = "col_01j8x..."
name = "MyAPI"
description = "Main API collection"
version = "1.0.0"
created = "2025-01-01T00:00:00Z"

[auth]
# Default auth inherited by all requests in collection (can be overridden per-request)
type = "bearer"
token = "{{auth_token}}"

[variables]
# Collection-scoped variables (lower priority than environment variables)
api_version = "v2"
```

### Environment File Format (`development.toml`)

```toml
[meta]
id = "env_01j8x..."
name = "development"
color = "#9ece6a"   # Color indicator in UI

[variables]
base_url = "http://localhost:3000/api"
auth_token = "dev-token-abc123"
user_id = "42"

# Secret variables — stored encrypted, never shown in plain text in UI
[secrets]
# Values are AES-256-GCM encrypted using a machine-local key
db_password = "enc:AES256:base64encodedvalue=="
stripe_key = "enc:AES256:base64encodedvalue=="
```

### Workspace File Format (`workspace.toml`)

```toml
[meta]
id = "ws_01j8x..."
name = "MyAPI Workspace"
description = "Personal API workspace"
created = "2025-01-01T00:00:00Z"

[settings]
default_environment = "development"
default_collection = "myapi"
```

---

## Round 1 — Core Request Engine

**Goal:** A working TUI that can send HTTP requests and display responses. The foundation everything else builds on.

### Features

**URL Bar**
- Single-line text input for URL entry
- Dropdown method selector: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- "Send" button (also triggered by `Ctrl+R` or `Enter` when URL bar focused)
- URL is syntax-highlighted: scheme (gray), host (white), path (blue), query (cyan)
- Autocomplete from request history on `Tab`
- Shorthand support: `:3000/path` → `http://localhost:3000/path`

**Method Selector**
- Cycle through methods with `[` and `]` keys when URL bar focused
- Color-coded by method (GET=green, POST=yellow, PUT=blue, DELETE=red, PATCH=purple)
- Popup selector on `m` key showing all methods

**Request Execution**
- Requests run on a separate Tokio task so the TUI never blocks
- Loading spinner in the response area while request is in flight
- `Ctrl+C` (or `Esc` when request is pending) cancels an in-flight request
- Connection errors shown as styled error messages in response panel
- Timeout defaults to 30 seconds (configurable)

**Basic Response Viewing**
- Status code with color (2xx=green, 3xx=blue, 4xx=orange, 5xx=red)
- Response time in ms
- Response size in human-readable bytes
- Body content displayed in scrollable, syntax-highlighted panel
- Auto-detected content type drives highlighting (JSON, XML, HTML, plain text)
- JSON is automatically pretty-printed

### Data Structures

```rust
// src/state/request_state.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestState {
    pub id: Uuid,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValuePair>,
    pub params: Vec<KeyValuePair>,
    pub body: RequestBody,
    pub auth: AuthConfig,
    pub scripts: Scripts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get, Post, Put, Patch, Delete, Head, Options,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,     // Toggle rows without deleting them
    pub description: String,
}

// src/state/response_state.rs

#[derive(Debug, Clone)]
pub struct ResponseState {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: ResponseBody,
    pub cookies: Vec<Cookie>,
    pub timing: RequestTiming,
    pub size_bytes: usize,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RequestTiming {
    pub dns_lookup_ms: u64,
    pub tcp_connect_ms: u64,
    pub tls_handshake_ms: u64,
    pub time_to_first_byte_ms: u64,
    pub download_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone)]
pub enum ResponseBody {
    Text(String),
    Binary(Vec<u8>),
    Empty,
}
```

### Implementation Tasks

- [ ] Set up `ratatui` + `crossterm` event loop with clean terminal setup/teardown
- [ ] Implement root layout (sidebar + main panel + status bar)
- [ ] Implement URL bar widget with method selector
- [ ] Implement `reqwest` client factory in `http/client.rs`
- [ ] Implement request executor (async task, cancellation via `CancellationToken`)
- [ ] Implement response state hydration from `reqwest::Response`
- [ ] Implement response body viewer (scrollable, syntax highlighted)
- [ ] Implement status code + timing + size display
- [ ] Implement loading spinner during in-flight request
- [ ] Implement basic error display (network errors, timeouts)
- [ ] Implement status bar with mode + keyhints

---

## Round 2 — Environment Variables

**Goal:** Support `{{variable}}` interpolation in URLs, headers, and body. Allow switching between environments instantly.

### Features

**Variable Syntax**
- `{{variable_name}}` in any text field is interpolated at send time
- Variables highlighted in cyan in the URL bar and editors
- Unresolved variables highlighted in red with a warning indicator
- Hovering (or pressing `?`) on a variable shows its resolved value in a tooltip

**Environment Editor**
- Accessible via `Ctrl+E` or sidebar → Environments
- Table view: variable name, value, description, type (text/secret)
- Add row: `a` in Normal mode
- Delete row: `d`
- Edit cell: `Enter` or `i`
- Secret variables: value shown as `••••••••`, toggle with `Space`
- Active environment marked with `●` in sidebar, inactive with `○`

**Environment Switcher**
- Quick popup via `Ctrl+E` showing all environments
- Fuzzy search within popup
- Current environment shown in header bar
- Switching environment immediately re-resolves all visible variable previews

**Resolution Priority**

```
Request-level override (inline {{var=value}} syntax, future feature)
  └─▶ Active environment variables
        └─▶ Collection-level variables
              └─▶ Global variables (workspace-level)
                    └─▶ OS environment variables ($VAR syntax)
                          └─▶ Unresolved (shown in red)
```

**Variable Preview**
- URL bar shows resolved URL as a ghost text below the raw URL
- Example: raw `{{base_url}}/users` → resolved preview `http://localhost:3000/users`

### Data Structures

```rust
// src/state/environment.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub variables: Vec<EnvVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub key: String,
    pub value: String,          // Plain text (or encrypted blob for secrets)
    pub var_type: VarType,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VarType {
    Text,
    Secret,    // Stored encrypted, masked in UI
}

// src/env/resolver.rs

pub struct EnvResolver {
    layers: Vec<HashMap<String, String>>,   // Ordered by priority, highest first
}

impl EnvResolver {
    pub fn resolve(&self, input: &str) -> ResolvedString {
        // Returns the resolved string + list of variable spans with their status
        // (resolved, unresolved, secret) for UI highlighting
    }
}

pub struct ResolvedString {
    pub value: String,
    pub spans: Vec<VarSpan>,
}

pub struct VarSpan {
    pub start: usize,
    pub end: usize,
    pub variable_name: String,
    pub status: VarStatus,
}

pub enum VarStatus {
    Resolved(String),
    Unresolved,
    Secret,
}
```

### Implementation Tasks

- [x] Implement `EnvResolver` with layered variable resolution
- [x] Implement `{{variable}}` parser (handle nested braces, escaping)
- [ ] Implement variable interpolation in URL, headers, body, auth fields
- [x] Implement variable span highlighting in URL bar
- [x] Implement environment editor TUI widget (table, CRUD)
- [x] Implement environment switcher popup
- [ ] Implement secret variable encryption (AES-256-GCM, machine-local key)
- [ ] Implement "unresolved variable" warning indicator
- [x] Persist environment files to disk on change
- [x] Load environment files on startup

---

## Round 3 — Collections & Workspaces

**Goal:** Organize requests into collections and folders. Support multiple workspaces for separate projects.

### Features

**Sidebar Tree**
- Collapsible tree: Workspace → Collections → Folders → Requests
- Keyboard navigation: `j`/`k` to move, `Enter` to open, `h`/`l` or `←`/`→` to collapse/expand
- Request items show method badge (colored) + name
- Active request highlighted
- Unsaved changes indicator (`*`) on modified requests
- Drag-and-drop reordering (mouse support, `Round 12+`)

**Collection CRUD**
- `N` → New collection (prompts for name)
- `n` → New request in current collection (prompts for name)
- `r` → Rename focused item
- `d` → Delete focused item (confirmation popup)
- `D` → Duplicate focused request
- Folders: `f` → New folder inside current collection

**Workspace Switcher**
- Shown in header bar: `[workspace: MyAPI ▾]`
- `Ctrl+W` → open workspace switcher popup
- Creating a new workspace creates a new directory under `~/.local/share/forge/workspaces/`
- Each workspace has its own collections, environments, and history

**Request Tabs**
- Multiple requests open simultaneously as tabs in the main panel
- Switch tabs: `Alt+1` through `Alt+9`, or `[`/`]`
- Close tab: `Alt+W` or `x` on tab
- Middle-click to close (mouse support)
- Unsaved tab shows `*` in tab title

**Search**
- `/` in sidebar focuses search input
- Fuzzy search across all request names in current workspace
- Results highlight matching characters

### Data Structures

```rust
// src/state/workspace.rs

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub id: Uuid,
    pub name: String,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment_id: Option<Uuid>,
    pub open_tabs: Vec<RequestTab>,
    pub active_tab_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub auth: AuthConfig,      // Default auth inherited by requests
    pub variables: HashMap<String, String>,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectionItem {
    Request(RequestState),
    Folder(Folder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Clone)]
pub struct RequestTab {
    pub request: RequestState,
    pub response: Option<ResponseState>,
    pub is_loading: bool,
    pub is_dirty: bool,        // Unsaved changes
}
```

### Implementation Tasks

- [x] Implement sidebar tree widget with collapse/expand
- [x] Implement collection CRUD (new, rename, delete, duplicate)
- [x] Implement folder support inside collections
- [x] Implement request tabs in main panel
- [x] Implement workspace switcher popup
- [x] Implement sidebar fuzzy search
- [x] Implement file persistence for collections (TOML files)
- [x] Implement auto-save on request modification
- [x] Implement workspace-level CRUD
- [ ] Display unsaved indicator (`*`) on dirty requests/tabs

---

## Round 4 — Authentication

**Goal:** Full auth support across all common schemes, inheritable from collection level.

### Features

**Auth Types**

| Type | Fields |
|------|--------|
| None | — |
| Basic | Username, Password |
| Bearer Token | Token (supports `{{variables}}`) |
| API Key | Key name, Key value, Add to: Header / Query param |
| OAuth 2.0 | Grant type, Auth URL, Token URL, Client ID, Client Secret, Scopes |
| Digest | Username, Password |

**Inheritance**
- Requests default to "Inherit from Collection"
- Collection defaults to "Inherit from Workspace"
- Overriding at any level breaks the inheritance chain for that level
- UI shows effective auth type including inherited source: `Bearer (from Collection)`

**OAuth 2.0 Flow**
- Authorization Code: opens browser for user login, captures redirect
- Client Credentials: fully automated token fetch
- Tokens cached in session with expiry tracking
- Auto-refresh when token is within 60 seconds of expiry
- Manual "Get New Access Token" button

**Auth Editor UI**
- Auth tab in request editor
- Dropdown to select auth type
- Dynamic form fields based on selected type
- "Preview" button shows the raw header that will be sent
- Secret fields masked by default, toggle with eye icon or `Space`

### Data Structures

```rust
// src/http/auth.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    None,
    Inherit,
    Basic { username: String, password: String },
    Bearer { token: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
    OAuth2(OAuth2Config),
    Digest { username: String, password: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiKeyLocation {
    Header,
    QueryParam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    pub grant_type: OAuth2GrantType,
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,    // Stored encrypted
    pub scopes: Vec<String>,
    pub cached_token: Option<CachedToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}
```

### Implementation Tasks

- [ ] Implement auth type selector dropdown in Auth tab
- [ ] Implement Basic auth form + injection into request headers
- [ ] Implement Bearer token form + injection
- [ ] Implement API Key form + injection (header or query param)
- [ ] Implement auth inheritance (None → Inherit → Collection → Workspace)
- [ ] Implement OAuth2 Client Credentials flow
- [ ] Implement OAuth2 Authorization Code flow (browser open + local callback server)
- [ ] Implement token caching and expiry tracking
- [ ] Implement auto-refresh logic
- [ ] Implement Digest auth
- [ ] Display "Preview" of resulting auth header

---

## Round 5 — Request Headers & Query Params

**Goal:** Full-featured key-value editors for headers and query params, with smart features.

### Features

**Key-Value Editor (shared component for headers and params)**
- Table layout: `[✓] Key | Value | Description`
- Toggle row with `Space` (grays out disabled rows, they are not sent)
- Add row: `a`
- Delete row: `d`
- Duplicate row: `D`
- Edit cell: `Enter` or `i`
- Move row up/down: `Alt+↑` / `Alt+↓`
- Bulk operations in Visual mode: select multiple rows, delete/disable/enable all
- `{{variable}}` values resolved and highlighted inline

**Headers Editor Specific**
- Common header autocomplete: start typing and get suggestions for `Content-Type`, `Accept`, `Authorization`, `X-Request-ID`, etc.
- Common value autocomplete: typing in value field for `Content-Type` suggests `application/json`, `text/plain`, etc.
- Warning indicator for duplicate header keys
- Default headers shown but grayed out: `Content-Type: application/json` (added automatically when body type is JSON)

**Query Params Specific**
- Params are reflected live in the URL bar (shown appended as `?key=value&...`)
- Editing URL query string directly also updates the params table (bidirectional sync)
- URL-encoding handled automatically (values stored decoded, sent encoded)

**Bulk Edit Mode**
- `e` key in headers/params tab opens a raw text editor showing all headers/params as:
  ```
  Accept: application/json
  X-API-Key: {{api_key}}
  Content-Type: application/json
  ```
- Edit freely as text, save with `Ctrl+S`, parsed back into table format

### Implementation Tasks

- [ ] Implement generic `KeyValueEditor` widget (reusable for headers, params, form body)
- [ ] Implement row toggle (enable/disable)
- [ ] Implement row CRUD (add, delete, duplicate, reorder)
- [ ] Implement `{{variable}}` highlighting within cells
- [ ] Implement header key autocomplete
- [ ] Implement header value autocomplete (Content-Type, Accept values)
- [ ] Implement bidirectional URL ↔ params sync
- [ ] Implement duplicate key warning for headers
- [ ] Implement bulk text editor mode
- [ ] Implement Visual mode multi-row selection

---

## Round 6 — Request Body Editor

**Goal:** Support all common body types with a great editing experience.

### Features

**Body Types**

| Type | Description |
|------|-------------|
| None | No body (default for GET/HEAD) |
| JSON | JSON editor with syntax highlighting and validation |
| Form (URL-encoded) | Key-value editor, sent as `application/x-www-form-urlencoded` |
| Multipart | Key-value editor with file attachment support |
| Raw | Plain text editor, manual Content-Type selection |
| GraphQL | Query + Variables editors (JSON for variables) |
| Binary | File picker, sent as raw bytes |

**JSON Editor**
- Syntax highlighted (`syntect`)
- Real-time JSON validation: shows parse error with line number
- Auto-format (pretty-print) with `Ctrl+Shift+F`
- Auto-complete for `{{variables}}` in string values
- Line numbers shown
- Scrollable for large payloads

**Form Editor**
- Same `KeyValueEditor` component as headers/params
- File field: value starting with `@` treated as file path
- Multipart: additional "Content-Type" column per field

**GraphQL Editor**
- Two-pane: Query (top) and Variables (bottom)
- Query pane: basic GraphQL syntax highlighting
- Variables pane: JSON editor (same as JSON body editor)
- Sends as JSON: `{"query": "...", "variables": {...}}`

**Raw Editor**
- Free-text editor
- Manual Content-Type dropdown at top of panel

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestBody {
    None,
    Json(String),
    Form(Vec<KeyValuePair>),
    Multipart(Vec<MultipartField>),
    Raw { content: String, content_type: String },
    GraphQL { query: String, variables: String },
    Binary { file_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartField {
    pub key: String,
    pub value: MultipartValue,
    pub enabled: bool,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultipartValue {
    Text(String),
    File(String),    // File path
}
```

### Implementation Tasks

- [ ] Implement body type selector (tab bar or dropdown in Body tab)
- [ ] Implement JSON editor with syntax highlighting
- [ ] Implement real-time JSON validation with error display
- [ ] Implement JSON auto-format (`Ctrl+Shift+F`)
- [ ] Implement Form URL-encoded editor (reuse `KeyValueEditor`)
- [ ] Implement Multipart editor with file attachment
- [ ] Implement Raw editor with Content-Type selector
- [ ] Implement GraphQL two-pane editor
- [ ] Implement Binary file picker
- [ ] Auto-set `Content-Type` header based on body type
- [ ] Show body size estimate in tab label

---

## Round 7 — Response Viewer

**Goal:** A comprehensive, beautiful, navigable response viewer.

### Features

**Response Meta Bar**
- Status code (colored by category) + status text
- Total time in ms (colored: green <200ms, yellow <1s, red >1s)
- Response size in human-readable bytes
- "Copy Response" button (`y` keybinding)
- "Save to File" button (`s` keybinding)

**Body Tab**
- Syntax-highlighted based on `Content-Type`
- JSON: pretty-printed, collapsible nodes (`Tab` to fold/unfold, `za` vim-style)
- JSON path display: shows path to cursor position (e.g. `$.users[0].email`)
- XML: pretty-printed, collapsible
- HTML: syntax highlighted, not rendered
- Binary: hex dump view
- Images: ASCII art preview for small images (using `ratatui` image support)
- Search within response body: `/` to open search, `n`/`N` to navigate matches
- Wrap toggle: `w` wraps long lines

**Headers Tab**
- All response headers in a scrollable table
- `Copy` individual header value: `y` on focused row
- Filter headers: `/` to search

**Cookies Tab**
- Cookie name, value, domain, path, expires, secure, httpOnly
- Auto-saved to active session if session mode enabled

**Timing Tab**
- Visual waterfall chart (ASCII bar chart in TUI)
- DNS lookup, TCP connect, TLS handshake, TTFB, download
- Each phase shown as a colored bar with ms value

**Response History per Request**
- Last 10 responses for each request kept in memory
- Navigate with `Alt+←` / `Alt+→` to browse response history
- Useful for comparing "before and after" without re-sending

### Implementation Tasks

- [ ] Implement response meta bar (status, time, size)
- [ ] Implement body viewer with syntax highlighting
- [ ] Implement JSON pretty-print with collapsible nodes
- [ ] Implement JSON path indicator
- [ ] Implement XML pretty-print with collapsible nodes
- [ ] Implement hex dump view for binary responses
- [ ] Implement body search (`/` + `n`/`N`)
- [ ] Implement line wrap toggle
- [ ] Implement headers table with copy
- [ ] Implement cookies table
- [ ] Implement timing waterfall chart
- [ ] Implement response copy to clipboard
- [ ] Implement save response to file
- [ ] Implement in-memory response history per request (last 10)

---

## Round 8 — History

**Goal:** A searchable, persistent log of every request sent.

### Features

**History Panel**
- Shown in sidebar under "HISTORY" section
- Each entry: method badge + URL + status code + time + timestamp
- Most recent at top
- Grouped by day: "Today", "Yesterday", "January 15", etc.
- Click/Enter on history entry: loads request + response into main panel (read-only)

**History Search**
- `/` in history section opens fuzzy search
- Searches across URL, method, status code, response body snippet

**History Actions**
- `Enter` on entry: load as new request (pre-fills URL bar + body + headers)
- `y` on entry: copy request URL to clipboard
- `d` on entry: delete this history entry
- `D`: delete all history

**History Persistence**
- Saved to `workspace/history.toml`
- Configurable max entries (default: 500)
- Configurable max age (default: 30 days, older entries auto-pruned)

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub request: HistoryRequest,
    pub response: HistoryResponse,
    pub sent_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub workspace_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub request_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRequest {
    pub method: HttpMethod,
    pub url: String,              // Resolved URL (after variable interpolation)
    pub headers: Vec<(String, String)>,
    pub body_preview: Option<String>,   // First 200 chars of body
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body_preview: Option<String>,   // First 500 chars of body
    pub size_bytes: usize,
}
```

### Implementation Tasks

- [ ] Implement history entry creation on every request sent
- [ ] Implement history list in sidebar with grouping by day
- [ ] Implement history fuzzy search
- [ ] Implement "load as new request" from history entry
- [ ] Implement history entry deletion (single + all)
- [ ] Implement history persistence to `history.toml`
- [ ] Implement max-entries and max-age pruning
- [ ] Display method badge + status code badge in history list

---

## Round 9 — Scripting (Pre/Post Hooks)

**Goal:** Allow developers to run lightweight scripts before and after requests to automate workflows.

### Features

**Script Engine**
- Uses `Rhai` — a lightweight, safe, embedded scripting language with Rust-like syntax
- Two hooks per request: `pre_request` and `post_request`
- Scripts are stored in the request TOML file
- Script editor is the "Scripts" tab in the request editor

**Pre-Request Script**

Runs before the request is sent. Can modify the request:

```javascript
// Available objects: request, env, console

// Modify URL
request.url = request.url + "?ts=" + timestamp();

// Set a header
request.headers.set("X-Timestamp", timestamp().to_string());

// Set an environment variable
env.set("request_time", timestamp().to_string());

// Log to the forge console
console.log("Sending to: " + request.url);

// Abort the request
// request.abort("Condition not met");
```

**Post-Request Script**

Runs after the response is received. Can extract and store data:

```javascript
// Available objects: request, response, env, console

// Extract value from JSON response
let body = response.json();
env.set("auth_token", body["token"]);
env.set("user_id", body["user"]["id"].to_string());

// Conditional logic
if response.status == 200 {
    console.log("Success! Token saved.");
} else {
    console.log("Failed: " + response.status.to_string());
}

// Assert (throws error and fails test if false)
assert(response.status == 200, "Expected 200 OK");
```

**Script Console**
- A "Console" sub-panel appears below response when scripts are active
- Shows `console.log()` output from both hooks
- Shows script errors with line numbers
- Cleared on each new request

**Script API Reference**

```
request object:
  .method: string (read/write)
  .url: string (read/write)
  .headers.get(key): string | null
  .headers.set(key, value): void
  .headers.delete(key): void
  .body: string (read/write)

response object:
  .status: int (read)
  .headers.get(key): string | null
  .body: string (read)
  .json(): object (parse body as JSON)
  .size: int (read)
  .time_ms: int (read)

env object:
  .get(key): string | null
  .set(key, value): void
  .delete(key): void

console object:
  .log(...): void
  .warn(...): void
  .error(...): void

built-in functions:
  timestamp(): int (unix ms)
  uuid(): string
  base64_encode(s): string
  base64_decode(s): string
  hmac_sha256(key, data): string
  md5(s): string
  sha256(s): string
  assert(condition, message): void
```

### Implementation Tasks

- [ ] Integrate `rhai` scripting engine
- [ ] Implement `request`, `response`, `env`, `console` script objects
- [ ] Implement pre-request hook execution (before HTTP send)
- [ ] Implement post-request hook execution (after response received)
- [ ] Implement script editor in Scripts tab (syntax-highlighted, Rhai)
- [ ] Implement script console panel (log output + errors)
- [ ] Implement built-in script functions (timestamp, uuid, base64, hmac, assert)
- [ ] Implement request abort from pre-request script
- [ ] Implement timeout for scripts (max 5 seconds, configurable)
- [ ] Implement script error display with line numbers
- [ ] Test script isolation (scripts cannot access filesystem or network directly)

---

## Round 10 — Streaming & SSE

**Goal:** First-class support for Server-Sent Events and streaming HTTP responses.

### Features

**Stream Detection**
- Auto-detected from `Content-Type: text/event-stream`
- Also triggered by `--stream` flag or "Stream" toggle in UI
- Loading indicator replaced by live streaming indicator (`⚡ streaming...`)

**SSE Display**
- Events rendered in real-time as they arrive
- Each event shown with: `event` type, `data`, `id`, `retry` fields
- Color-coded by event type
- Scrollable — auto-scrolls to bottom as new events arrive
- Toggle auto-scroll with `f` (follow mode)
- Event counter in response meta bar: `47 events received`

**AI API Streaming**
- Special mode for `application/x-ndjson` and OpenAI/Anthropic-style streaming
- "Extract delta" toggle: when enabled, extracts text delta from each chunk and assembles into readable output
- Delta assembly shown in a separate "Assembled" sub-tab alongside raw chunks

**Stream Controls**
- `Ctrl+C` / `Esc`: stop streaming
- `c`: clear stream buffer
- `s`: save all streamed data to file
- `y`: copy all streamed data to clipboard
- Max buffer configurable (default: 10MB, older chunks dropped)

**Chunked Transfer**
- Chunk boundaries visible in verbose mode
- Chunk size shown in hex (matching curl `--trace` style)

### Implementation Tasks

- [ ] Implement streaming response handler using `reqwest` bytes stream
- [ ] Implement SSE parser (parse `event:`, `data:`, `id:`, `retry:` fields)
- [ ] Implement live streaming display in response body panel
- [ ] Implement auto-scroll with follow mode toggle
- [ ] Implement event counter
- [ ] Implement stream stop / cancel
- [ ] Implement NDJSON stream handler
- [ ] Implement AI delta extraction and assembly
- [ ] Implement stream buffer with configurable max size
- [ ] Implement save stream to file
- [ ] Implement chunked transfer visualization in verbose mode

---

## Round 11 — Import & Export

**Goal:** Seamless migration from Postman, Insomnia, Bruno, and OpenAPI.

### Features

**Import Formats**

| Format | Notes |
|--------|-------|
| Postman Collection v2.1 | Full support: requests, folders, auth, scripts |
| Postman Collection v2.0 | Full support |
| Insomnia v4 (JSON/YAML) | Requests, environments, workspaces |
| Bruno `.bru` files | Direct folder import |
| OpenAPI 3.x (JSON/YAML) | Generate requests from spec — endpoints → requests |
| OpenAPI 2.x (Swagger) | Same as above |
| cURL commands | Parse `curl` command string → forge request |
| HAR (HTTP Archive) | Import from browser DevTools export |

**Export Formats**

| Format | Notes |
|--------|-------|
| forge TOML | Native format (default) |
| Postman Collection v2.1 | Full fidelity |
| OpenAPI 3.x | Generate spec from collection |
| cURL commands | Export any request as `curl` command |
| HAR | Export all history as HAR |
| Markdown | Export collection as Markdown API docs |

**Import Flow**
- `Ctrl+P` → "import" → select format → pick file
- Preview of what will be imported (collection name, request count, environment count)
- Conflict resolution: "Skip", "Overwrite", "Rename" options when IDs clash
- Import progress shown for large collections

**cURL Import**
- Paste `curl` command directly in command palette: `: curl https://...`
- Parsed into full request including headers, body, auth
- Inverse of `fyr --offline` output

**OpenAPI Import**
- Generates one request per endpoint
- Groups by tag → folders
- Fills in path parameters as `{{param_name}}` variables
- Imports server URLs as environment variables

### Implementation Tasks

- [ ] Implement Postman v2.1 importer (serde_json)
- [ ] Implement Postman v2.0 importer
- [ ] Implement Insomnia v4 importer (serde_json/serde_yaml)
- [ ] Implement Bruno `.bru` file importer
- [ ] Implement OpenAPI 3.x importer (serde_yaml)
- [ ] Implement cURL command parser
- [ ] Implement HAR importer
- [ ] Implement conflict resolution UI
- [ ] Implement forge TOML exporter
- [ ] Implement Postman v2.1 exporter
- [ ] Implement cURL command exporter
- [ ] Implement Markdown docs exporter
- [ ] Implement HAR exporter
- [ ] Implement import preview popup
- [ ] Implement "paste curl" shortcut in command palette

---

## Round 12 — Configuration & Theming

**Goal:** Make forge deeply customizable so developers feel at home.

### Features

**Config File** (`~/.config/forge/config.toml`)

```toml
[ui]
theme = "tokyo-night"      # Built-in themes: tokyo-night, dracula, gruvbox,
                           # catppuccin, solarized-dark, solarized-light, nord, github-dark
sidebar_width = 30         # Percent of terminal width
sidebar_position = "left"  # left | right
show_sidebar = true
layout = "vertical"        # vertical (request top, response bottom) | horizontal

[editor]
vim_mode = true            # Modal editing (default: true)
tab_size = 2
word_wrap = true
line_numbers = true
auto_format_json = true    # Auto-pretty-print JSON body on tab switch

[request]
default_timeout = 30       # seconds
follow_redirects = true
max_redirects = 10
verify_ssl = true
default_content_type = "application/json"

[history]
max_entries = 500
max_age_days = 30
enabled = true

[scripting]
enabled = true
timeout_seconds = 5

[network]
# proxy = "http://localhost:8080"
# no_proxy = "localhost,127.0.0.1"

[keybindings]
# Override any keybinding
send_request = "ctrl+r"
focus_url_bar = "ctrl+l"
open_command_palette = "ctrl+p"
```

**Theme System**
- Themes defined as TOML files
- Built-in themes: `tokyo-night` (default), `dracula`, `gruvbox`, `catppuccin-mocha`, `nord`, `solarized-dark`, `solarized-light`, `github-dark`
- Custom themes: drop a `mytheme.toml` in `~/.config/forge/themes/`
- Switch theme live: `: set theme dracula`

**Custom Theme Format** (`~/.config/forge/themes/mytheme.toml`)

```toml
[meta]
name = "My Theme"
author = "Albert"
dark = true

[colors]
background = "#1a1b26"
surface = "#24283b"
border_inactive = "#414868"
border_active = "#7aa2f7"
text_primary = "#c0caf5"
text_muted = "#565f89"
text_accent = "#7aa2f7"

[method_colors]
get = "#9ece6a"
post = "#e0af68"
put = "#7aa2f7"
delete = "#f7768e"
patch = "#bb9af7"
head = "#2ac3de"
options = "#ff9e64"

[status_colors]
success = "#9ece6a"      # 2xx
redirect = "#7aa2f7"     # 3xx
client_error = "#ff9e64" # 4xx
server_error = "#f7768e" # 5xx
```

**Layout Presets**
- `vertical`: Request editor top, response bottom (default)
- `horizontal`: Request editor left, response right (wide monitors)
- `focused`: Hides sidebar, maximizes editor + response
- Custom splits: drag divider (mouse) or `: set split 40` (set request/response split percent)

**Keybinding Customization**
- All keybindings configurable in `config.toml`
- `: show keybindings` opens a searchable cheatsheet popup

**Mouse Support**
- Click to focus panels
- Click to select items in sidebar
- Scroll wheel in all panels
- Click tabs to switch
- Resize panels by dragging borders

### Implementation Tasks

- [ ] Implement config file loading with serde + toml
- [ ] Implement config hot-reload (file watcher on config.toml)
- [ ] Implement theme system with TOML theme files
- [ ] Implement all 8 built-in themes
- [ ] Implement custom theme loading from `~/.config/forge/themes/`
- [ ] Implement live theme switching
- [ ] Implement layout presets (vertical, horizontal, focused)
- [ ] Implement custom keybinding loading
- [ ] Implement keybinding cheatsheet popup
- [ ] Implement mouse support (click, scroll, resize)
- [ ] Implement `config.toml` with all documented options
- [ ] Implement settings UI (`: settings` command)

---

## Round 13 — Polish, Performance & Release

**Goal:** Ship a 1.0 that feels complete, fast, and trustworthy.

### Performance

**Startup Time Target: < 50ms**
- Profile with `cargo flamegraph`
- Lazy-load collections (don't parse all TOML files on startup — load on demand)
- Async file I/O for all storage operations
- Measure and document startup benchmark in README

**Large Response Handling**
- Responses > 1MB: render only visible portion, virtual scrolling
- JSON with > 1000 nodes: lazy tree rendering (expand on demand)
- Cap in-memory response history at 50MB total

**TUI Rendering**
- Target 60fps render loop (16ms frame budget)
- Only re-render changed areas (ratatui's diff-based rendering handles this)
- Debounce rapid keystrokes (e.g. holding `j` for fast scroll)

### Polish Features

**Onboarding**
- First launch: welcome screen with quick-start guide
- Example collection pre-loaded (httpbin.org requests demonstrating all features)
- Interactive tutorial: `: tutorial` command walks through basic workflow

**Accessibility**
- Full keyboard navigability (no mouse required)
- Screen reader compatible output (plain text mode: `--plain`)
- High contrast theme option

**Error Experience**
- All errors shown in a consistent, friendly format
- Network errors: suggest common fixes (check URL, try `--no-verify`, check proxy)
- TLS errors: show certificate details, expiry, issuer
- Script errors: show line number, stack trace

**Quality of Life**
- `Ctrl+Z` / Undo for request edits (within session)
- Auto-save: all changes written to disk within 500ms of modification
- Crash recovery: restore unsaved request state on next launch
- Update checker: `forge --version` checks for newer releases (opt-out in config)

### Distribution

**Binary Targets** (same as fyr):

| Target | Platform |
|--------|---------|
| `x86_64-unknown-linux-musl` | Linux x64 (static) |
| `aarch64-unknown-linux-musl` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x64 |

**Package Managers**:
- Homebrew: `brew install AlbertArakelyan/tap/forge`
- Cargo: `cargo install forge`
- Scoop (Windows), AUR (Arch), Nix flake
- Install script: `curl -fsSL https://forgeapp.dev/install.sh | bash`

**Release Workflow** (GitHub Actions):
1. Push tag `v1.0.0`
2. Build all targets with `cross`
3. Strip + compress binaries
4. Create GitHub Release with SHA256 checksums
5. Update Homebrew formula
6. Publish to crates.io
7. Post release notes to Discord/X/Reddit

### Launch Checklist

- [ ] Demo GIF: 60-second screencast showing core workflow (recorded with `vhs`)
- [ ] README: hero section, comparison table, install, quick-start, full docs
- [ ] Docs site: GitHub Pages or dedicated domain (`forgeapp.dev`)
- [ ] Comparison post: "Why I built forge instead of using Postman"
- [ ] YouTube demo: full walkthrough video
- [ ] r/rust + r/commandline + Hacker News Show HN
- [ ] Submit to `awesome-rust`, `awesome-tui`, `awesome-cli-tools`
- [ ] `tldr` page for forge
- [ ] Discord server for community

### Implementation Tasks

- [ ] Implement virtual scrolling for large responses
- [ ] Implement lazy collection loading
- [ ] Implement async storage I/O throughout
- [ ] Profile startup and hit < 50ms target
- [ ] Implement crash recovery (save state to temp file)
- [ ] Implement undo/redo for request edits
- [ ] Implement update checker
- [ ] Implement welcome screen + example collection
- [ ] Implement `forge --plain` accessible mode
- [ ] Write comprehensive integration test suite
- [ ] Write man page
- [ ] Create install script
- [ ] Set up GitHub Actions release pipeline
- [ ] Record demo GIF with `vhs`
- [ ] Write README

---

## Testing Strategy

### Unit Tests

Cover all pure logic without TUI:
- Environment variable resolver and interpolator
- Request builder (method, headers, auth injection)
- Item parser (all key-value separator types)
- Collection TOML serialization/deserialization
- Import parsers (Postman, OpenAPI, cURL)
- Script engine (Rhai context, built-in functions)

### Integration Tests

Use `wiremock` for a real local HTTP server:
- Full request-response cycle for all auth types
- Streaming SSE events
- Pre/post script execution and env mutation
- Collection CRUD on disk
- Environment resolution with all priority layers
- Import → send → history → export round-trip

### TUI Tests

Use `ratatui`'s test backend (renders to a buffer, not real terminal):
- Snapshot tests of rendered UI states (using `insta`)
- Keybinding dispatch tests (simulate keypresses, assert state changes)
- Focus transitions
- Error display rendering

### CI Matrix

```yaml
matrix:
  os: [ubuntu-latest, macos-latest, windows-latest]
  rust: [stable, beta, nightly]
```

---

## Milestones Summary

| Round | Focus | Deliverable |
|-------|-------|-------------|
| 1 | Core Request Engine | Send requests, view responses in TUI |
| 2 | Environment Variables | `{{variable}}` interpolation, env switcher |
| 3 | Collections & Workspaces | Sidebar tree, tabs, file persistence |
| 4 | Authentication | Basic, Bearer, API Key, OAuth 2.0 |
| 5 | Headers & Params | Key-value editors, autocomplete, bidirectional URL sync |
| 6 | Request Body Editor | JSON, Form, Multipart, GraphQL, Raw, Binary |
| 7 | Response Viewer | Collapsible JSON, search, timing waterfall |
| 8 | History | Persistent searchable request log |
| 9 | Scripting | Rhai pre/post hooks, script console |
| 10 | Streaming & SSE | Real-time event display, AI API streaming |
| 11 | Import & Export | Postman, OpenAPI, cURL, Insomnia, Bruno |
| 12 | Config & Theming | Themes, layouts, custom keybindings, mouse |
| 13 | Polish & Release | Performance, onboarding, distribution, launch |

**Recommended build order for delegating to Claude Code:**
Start with Round 1 (standalone, deliverable, impressive). Then Rounds 2 and 3 together (they share data structures). Then Rounds 4–6 in parallel (independent tab features). Rounds 7–13 sequentially.

---

*Documentation version: 0.1.0-draft*
*Project: forge — Terminal API Client*
*Author: Albert Arakelyan*
*License: MIT*
