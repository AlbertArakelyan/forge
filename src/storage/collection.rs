use std::path::PathBuf;

use crate::state::collection::Collection;

fn collections_dir(ws_name: &str) -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("forge").join("workspaces").join(ws_name).join("collections")
}

/// Load all collections from a workspace's collections directory.
pub fn load_all_collections(ws_name: &str) -> Vec<Collection> {
    let dir = collections_dir(ws_name);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut collections = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path().join("collection.toml");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(col) = toml::from_str::<Collection>(&content) {
                collections.push(col);
            }
        }
    }
    collections.sort_by(|a, b| a.name.cmp(&b.name));
    collections
}

/// Save a collection's metadata to `<ws>/collections/<slug>/collection.toml`.
pub fn save_collection_meta(ws_name: &str, col: &Collection) -> anyhow::Result<()> {
    let slug = col.name.to_lowercase().replace(' ', "_");
    let dir = collections_dir(ws_name).join(&slug);
    std::fs::create_dir_all(&dir)?;
    let content = toml::to_string_pretty(col)?;
    std::fs::write(dir.join("collection.toml"), content)?;
    Ok(())
}

/// Delete a collection directory identified by its name slug.
pub fn delete_collection(ws_name: &str, col_name: &str) -> anyhow::Result<()> {
    let slug = col_name.to_lowercase().replace(' ', "_");
    let dir = collections_dir(ws_name).join(&slug);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}
