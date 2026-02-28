use std::path::PathBuf;

use crate::state::workspace::{WorkspaceFile, WorkspaceState};
use crate::storage::collection as col_storage;
use crate::storage::environment as env_storage;

fn workspaces_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("forge").join("workspaces")
}

/// Return a sorted list of all workspace names (directory names under `workspaces/`).
/// Falls back to `["default"]` if the directory does not exist or is empty.
pub fn list_workspaces() -> Vec<String> {
    let dir = workspaces_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec!["default".to_string()];
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    if names.is_empty() {
        names.push("default".to_string());
    }
    names.sort();
    names
}

/// Load the `workspace.toml` for `name`. Returns a default `WorkspaceFile` on any error.
pub fn load_workspace(name: &str) -> WorkspaceFile {
    let path = workspaces_dir().join(name).join("workspace.toml");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(ws) = toml::from_str::<WorkspaceFile>(&content) {
            return ws;
        }
    }
    WorkspaceFile {
        name: name.to_string(),
        active_environment_idx: None,
    }
}

/// Persist the workspace file to disk, creating the directory if needed.
pub fn save_workspace(ws: &WorkspaceFile) -> anyhow::Result<()> {
    let dir = workspaces_dir().join(&ws.name);
    std::fs::create_dir_all(&dir)?;
    let content = toml::to_string_pretty(ws)?;
    std::fs::write(dir.join("workspace.toml"), content)?;
    Ok(())
}

/// Load a `WorkspaceState` by name, including its collections and environments.
/// Open tabs start empty — they are not persisted.
pub fn load_workspace_full(name: &str) -> WorkspaceState {
    let ws_file = load_workspace(name);
    let collections = col_storage::load_all_collections(name);
    let environments = env_storage::load_all_ws(name);
    let active_environment_idx = ws_file.active_environment_idx
        .filter(|&i| i < environments.len())
        .or_else(|| if environments.is_empty() { None } else { Some(0) });

    WorkspaceState {
        name: name.to_string(),
        collections,
        environments,
        active_environment_idx,
        open_tabs: Vec::new(),
        active_tab_idx: 0,
    }
}
