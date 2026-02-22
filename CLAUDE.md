# CLAUDE.md

We're building the app described in @SPEC.md. Read that file for general architectural tasks or to double-check the exact project structure, tech stack or application architecture.

Keep your replies extremely concise and focus on conveying the key information. No unnecessary fluff, no long code snippets.

Whenever working with any third-party library or something similar, you MUST look up the official documentation to ensure that you're working with up-to-date information. Use the docs-explorer subagent for efficient documentation lookup.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**forge** is a terminal-native API client built in Rust — Postman in your terminal. See `SPEC.md` for the full design specification including all data structures, UI layouts, and feature roadmap.

## Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run

# Test
cargo test
cargo test <test_name>          # Run a single test by name
cargo test -- --nocapture       # Show println! output

# Check (faster than build, no binary output)
cargo check

# Lint
cargo clippy

# Format
cargo fmt
cargo fmt --check               # CI-style check without modifying

# Benchmarks
cargo bench
```

## Architecture

forge is a pure event-driven state machine. All state transitions are pure functions: `(AppState, Event) → AppState`. No mutation outside of the reducer.

### Module Layout (per SPEC.md)

```
src/
├── main.rs          # Entry: init terminal, run event loop
├── app.rs           # AppState struct + root reducer
├── event.rs         # Event enum: Key, Mouse, Tick, Response, Resize
├── terminal.rs      # Terminal setup/teardown (crossterm)
├── error.rs         # Unified error types (thiserror)
├── ui/              # Pure rendering functions — no state mutation
│   ├── layout.rs    # Root layout composition
│   ├── sidebar.rs   # Collections/workspaces/history tree
│   ├── request/     # URL bar, tab bar, headers/body/auth/params editors
│   ├── response/    # Body viewer, headers, cookies, timing tabs
│   └── ...          # status_bar, command_palette, popup, highlight
├── state/           # All state types: AppState, RequestState, ResponseState, etc.
├── actions/         # Dispatchable actions (navigation, request, response, etc.)
├── http/            # HTTP execution layer: client, executor, builder, auth, stream
├── storage/         # File-based persistence (TOML): workspace, collection, environment
├── env/             # {{variable}} resolution with layered priority
└── scripting/       # Rhai pre/post request hooks
```

### Key Design Patterns

**State machine**: `AppState` owns `mode` (Normal/Insert/Command/Visual), `focus` (Sidebar/UrlBar/TabBar/Editor/ResponseViewer), and all domain state. The root reducer in `app.rs` dispatches actions.

**Async requests**: HTTP requests run on a separate Tokio task so the TUI never blocks. Cancellation via `CancellationToken`. Result sent back via an async channel as a `Response` event.

**UI rendering**: All `ui/` functions take `&AppState` and a `Frame` ref. They are pure — they render state, never modify it.

**Storage**: All data stored as human-readable TOML files. Default path: `%APPDATA%\forge\` on Windows, `~/.local/share/forge/` on Linux, `~/Library/Application Support/forge/` on macOS. Uses the `dirs` crate.

**Variable interpolation**: `{{variable}}` syntax in URLs, headers, and body. Priority order: request-level > active environment > collection-level > workspace/global > OS env vars. Unresolved vars shown in red in the UI.

**Scripting**: Rhai engine with sandboxed `request`, `response`, `env`, and `console` objects. Pre-request hooks run before HTTP send; post-request hooks run after response is received.

### Tech Stack

- **TUI**: `ratatui` + `crossterm`
- **HTTP**: `reqwest` (with `rustls-tls`, `json`, `stream`, `multipart`, `gzip`, `brotli`)
- **Async**: `tokio`
- **Serialization**: `serde` + `serde_json` + `toml`
- **Syntax highlighting**: `syntect`
- **Fuzzy search**: `fuzzy-matcher`
- **Scripting**: `rhai`
- **File watching**: `notify`
- **Error handling**: `thiserror` + `anyhow`

### Testing

- **Unit tests**: Pure logic (env resolver, request builder, import parsers, scripting)
- **Integration tests** (in `tests/integration/`): Use `wiremock` for a real local HTTP server
- **TUI tests**: Use ratatui's test backend (renders to buffer); snapshot tests via `insta`

CI matrix covers `ubuntu-latest`, `macos-latest`, `windows-latest` on stable/beta/nightly Rust.

## Build Order (from SPEC.md)

The spec is organized into 13 rounds. Implement in this order:
1. **Round 1** — Core Request Engine (ratatui event loop, URL bar, reqwest executor, response viewer)
2. **Rounds 2 & 3** — Environment Variables + Collections & Workspaces (share data structures)
3. **Rounds 4–6** — Auth, Headers/Params, Body Editor (independent tab features, can parallelize)
4. **Rounds 7–13** — Response Viewer, History, Scripting, Streaming, Import/Export, Config/Theming, Polish
