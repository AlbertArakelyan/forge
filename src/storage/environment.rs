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
