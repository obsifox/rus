//! Isolated copies of a Minecraft version.
//!
//! Several instances may share one version id. Each has its own game
//! directory, so worlds, options, and staged mods do not mix.

use crate::state::{now_ms, App};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub created: u128,
    /// Shown in the three slots beside the version bar. At most three stay on.
    #[serde(default)]
    pub pinned: bool,
}

pub fn dir(app: &App, id: &str) -> PathBuf {
    app.p(&format!("instances/{id}"))
}

pub fn load(app: &App) -> Vec<Instance> {
    std::fs::read_to_string(app.p("instances.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &App, items: &[Instance]) {
    let _ = std::fs::create_dir_all(&app.dir);
    let _ = std::fs::write(
        app.p("instances.json"),
        serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".into()),
    );
}

pub fn create(name: &str, version: &str, loader: &str) -> Instance {
    let version = if version.trim().is_empty() { "26.3".to_string() } else { version.trim().to_string() };
    let name = {
        let n = name.trim();
        if n.is_empty() { version.clone() } else { n.to_string() }
    };
    let loader = if loader.trim().is_empty() { "vanilla".to_string() } else { loader.trim().to_string() };
    Instance {
        id: format!("{:x}{:02x}", now_ms(), name.len() as u8),
        name,
        version,
        loader,
        created: now_ms(),
        pinned: false,
    }
}

/// Keep at least one instance so Play has an isolated directory.
pub fn ensure(app: &App, version: &str, loader: &str) -> Vec<Instance> {
    let mut items = load(app);
    if items.is_empty() {
        let inst = create(version, version, loader);
        let _ = std::fs::create_dir_all(dir(app, &inst.id));
        items.push(inst);
        save(app, &items);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_name_uses_the_version() {
        let inst = create("  ", "1.21.11", "fabric");
        assert_eq!(inst.name, "1.21.11");
        assert_eq!(inst.version, "1.21.11");
        assert_eq!(inst.loader, "fabric");
        assert!(!inst.id.is_empty());
    }
}
