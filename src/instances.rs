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
    /// `26.3/vanilla 26.3`. Empty means the older `instances/{id}` folder.
    #[serde(default)]
    pub folder: String,
}

/// Shared vanilla client for one version. Downloaded once.
pub fn library_dir(app: &App, version: &str) -> PathBuf {
    app.p(&format!("instances/library_{}", sanitize(version)))
}

/// Isolated copy: `instances/26.3/vanilla 26.3`.
pub fn path_of(app: &App, inst: &Instance) -> PathBuf {
    if inst.folder.is_empty() {
        app.p(&format!("instances/{}", inst.id))
    } else {
        app.p(&format!("instances/{}", inst.folder))
    }
}

pub fn dir(app: &App, id: &str) -> PathBuf {
    load(app)
        .into_iter()
        .find(|i| i.id == id)
        .map(|inst| path_of(app, &inst))
        .unwrap_or_else(|| app.p(&format!("instances/{id}")))
}

fn sanitize(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { ' ' } else { c })
        .collect();
    let flat = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() { "client".into() } else { flat }
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
        if n.is_empty() || n == version {
            format!("vanilla {version}")
        } else {
            n.to_string()
        }
    };
    let loader = if loader.trim().is_empty() { "vanilla".to_string() } else { loader.trim().to_string() };
    Instance {
        id: format!("{:x}{:02x}", now_ms(), name.len() as u8),
        folder: format!("{}/{}", sanitize(&version), sanitize(&name)),
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
        assert_eq!(inst.name, "vanilla 1.21.11");
        assert_eq!(inst.folder, "1.21.11/vanilla 1.21.11");
        assert_eq!(inst.version, "1.21.11");
        assert_eq!(inst.loader, "fabric");
        assert!(!inst.id.is_empty());
    }
}
