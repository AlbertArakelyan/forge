use std::path::PathBuf;

use crate::state::environment::Environment;

fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("forge").join("environments")
}

/// Save an environment as `<id>.toml` in the forge data directory.
pub fn save(env: &Environment) -> anyhow::Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", env.id));
    let content = toml::to_string_pretty(env)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Delete the environment `.toml` file for the given id.
pub fn delete(id: &str) -> anyhow::Result<()> {
    let path = data_dir().join(format!("{}.toml", id));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Load all `*.toml` files from the environments directory.
pub fn load_all() -> Vec<Environment> {
    let dir = data_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut envs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(env) = toml::from_str::<Environment>(&content) {
                envs.push(env);
            }
        }
    }
    envs
}

// ─── Workspace-scoped environment storage ────────────────────────────────────

fn ws_data_dir(ws_name: &str) -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("forge").join("workspaces").join(ws_name).join("environments")
}

/// Save an environment into the given workspace's environments directory.
pub fn save_ws(ws_name: &str, env: &Environment) -> anyhow::Result<()> {
    let dir = ws_data_dir(ws_name);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", env.id));
    let content = toml::to_string_pretty(env)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Delete an environment from the given workspace's environments directory.
pub fn delete_ws(ws_name: &str, id: &str) -> anyhow::Result<()> {
    let path = ws_data_dir(ws_name).join(format!("{}.toml", id));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Load all environments from the given workspace's environments directory.
pub fn load_all_ws(ws_name: &str) -> Vec<Environment> {
    let dir = ws_data_dir(ws_name);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut envs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(env) = toml::from_str::<Environment>(&content) {
                envs.push(env);
            }
        }
    }
    envs
}
