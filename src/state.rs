//! Global launcher state: settings, paths, install jobs.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("LAUNCHER_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("Rus");
            }
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".rus")
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub username: String,
    /// preset id (`preset:steve`) or uploaded file (`file:<name>.png`)
    pub skin: String,
    pub cape: String,
    /// classic | slim
    pub skin_model: String,
    pub ram_mb: u32,
    pub java_path: String,
    pub jvm_args: String,
    pub game_dir: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub show_snapshots: bool,
    pub show_previews: bool,
    pub last_version: String,
    pub demo: bool,
    /// vanilla | horizon
    #[serde(default = "default_engine")]
    pub engine: String,
}

fn default_engine() -> String {
    "vanilla".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            username: "Player".into(),
            skin: "preset:steve".into(),
            cape: "preset:rus".into(),
            skin_model: "classic".into(),
            ram_mb: 4096,
            java_path: String::new(),
            jvm_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions".into(),
            game_dir: String::new(),
            width: 1280,
            height: 720,
            fullscreen: false,
            show_snapshots: false,
            show_previews: false,
            last_version: String::new(),
            demo: false,
            engine: "vanilla".into(),
        }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct PatchState {
    pub key: String,
    pub title: String,
    pub detail: String,
    pub enabled: bool,
    /// pending | running | ok | skipped | error
    pub status: String,
    pub bytes: u64,
    pub total: u64,
    pub note: String,
}

pub struct Job {
    pub id: String,
    pub version: String,
    pub state: String, // running | done | error | canceled
    pub progress: f64,
    pub message: String,
    pub logs: Vec<String>,
    pub patches: Vec<PatchState>,
    pub error: Option<String>,
    pub cancel: AtomicBool,
    pub started: u128,
}

impl Job {
    pub fn new(id: String, version: String, patches: Vec<PatchState>) -> Self {
        Self {
            id,
            version,
            state: "running".into(),
            progress: 0.0,
            message: "Preparing…".into(),
            logs: Vec::new(),
            patches,
            error: None,
            cancel: AtomicBool::new(false),
            started: now_ms(),
        }
    }

    pub fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.logs.push(line);
        if self.logs.len() > 400 {
            let cut = self.logs.len() - 400;
            self.logs.drain(0..cut);
        }
    }

    pub fn patch(&mut self, key: &str) -> Option<&mut PatchState> {
        self.patches.iter_mut().find(|p| p.key == key)
    }

    pub fn set_patch(&mut self, key: &str, status: &str, note: impl Into<String>) {
        if let Some(p) = self.patch(key) {
            p.status = status.to_string();
            let note = note.into();
            if !note.is_empty() {
                p.note = note;
            }
        }
    }

    pub fn canceled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "version": self.version,
            "state": self.state,
            "progress": self.progress,
            "message": self.message,
            "logs": self.logs,
            "patches": self.patches,
            "error": self.error,
            "elapsed": (now_ms().saturating_sub(self.started)) as f64 / 1000.0,
        })
    }
}

pub struct App {
    pub dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub manifest: Mutex<Option<(std::time::Instant, serde_json::Value)>>,
    pub jobs: Mutex<HashMap<String, Arc<Mutex<Job>>>>,
    pub last_job: Mutex<Option<String>>,
}

impl App {
    pub fn load() -> Arc<App> {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let cfg = dir.join("config.json");
        let settings = std::fs::read_to_string(&cfg)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default();
        let app = Arc::new(App {
            dir,
            settings: Mutex::new(settings),
            manifest: Mutex::new(None),
            jobs: Mutex::new(HashMap::new()),
            last_job: Mutex::new(None),
        });
        for sub in [
            "versions",
            "libraries",
            "assets/indexes",
            "assets/objects",
            "mods",
            "skins",
            "capes",
            "cache/versions",
            "logs",
            "runtime",
        ] {
            let _ = std::fs::create_dir_all(app.dir.join(sub));
        }
        app
    }

    pub fn p(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(&self) {
        let s = self.settings.lock().unwrap().clone();
        let _ = std::fs::write(
            self.dir.join("config.json"),
            serde_json::to_string_pretty(&s).unwrap_or_default(),
        );
    }

    pub fn update_settings(&self, new: Settings) {
        *self.settings.lock().unwrap() = new;
        self.save_settings();
    }

    /// Effective game directory (per-version instances live inside it).
    pub fn game_dir(&self, version: &str) -> PathBuf {
        let base = {
            let s = self.settings.lock().unwrap();
            if s.game_dir.trim().is_empty() {
                self.dir.join("game")
            } else {
                PathBuf::from(s.game_dir.trim())
            }
        };
        base.join(version)
    }

    pub fn job(&self, id: &str) -> Option<Arc<Mutex<Job>>> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.p("versions").join(id).join(format!("{id}.json")).exists()
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
    pub icon: String,
    pub played_at: u128,
}

impl App {
    pub fn load_servers(&self) -> Vec<ServerEntry> {
        std::fs::read_to_string(self.p("servers.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_servers(&self, servers: &[ServerEntry]) {
        let _ = std::fs::create_dir_all(self.p("servers"));
        let _ = std::fs::write(
            self.p("servers.json"),
            serde_json::to_string_pretty(servers).unwrap_or_default(),
        );
    }
}

/// Last servers the player actually launched. Saved-but-unplayed entries stay off the rail.
pub fn recent_played(servers: &[ServerEntry]) -> Vec<ServerEntry> {
    let mut list: Vec<ServerEntry> = servers
        .iter()
        .filter(|s| s.played_at > 0 && !s.address.trim().is_empty())
        .cloned()
        .collect();
    list.sort_by(|a, b| b.played_at.cmp(&a.played_at));
    list.truncate(3);
    list
}

pub fn mark_played(servers: &mut Vec<ServerEntry>, name: &str, address: &str) {
    let address = address.trim();
    if address.is_empty() {
        return;
    }
    let now = now_ms();
    if let Some(entry) = servers.iter_mut().find(|s| s.address.eq_ignore_ascii_case(address)) {
        entry.played_at = now;
        if !name.trim().is_empty() {
            entry.name = name.trim().to_string();
        }
    } else {
        servers.insert(
            0,
            ServerEntry {
                name: if name.trim().is_empty() { address.to_string() } else { name.trim().to_string() },
                address: address.to_string(),
                icon: String::new(),
                played_at: now,
            },
        );
    }
}

/// Offline (local) account UUID: UUID v3 of "OfflinePlayer:<name>", like vanilla.
pub fn offline_uuid(name: &str) -> String {
    use md5::{Digest, Md5};
    let mut h = Md5::new();
    h.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut d = h.finalize();
    d[6] = (d[6] & 0x0f) | 0x30;
    d[8] = (d[8] & 0x3f) | 0x80;
    let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_hides_until_a_server_is_played() {
        let saved = vec![
            ServerEntry { name: "Lobby".into(), address: "play.example".into(), icon: String::new(), played_at: 0 },
            ServerEntry { name: "Old".into(), address: "old.example".into(), icon: String::new(), played_at: 10 },
            ServerEntry { name: "New".into(), address: "new.example".into(), icon: String::new(), played_at: 30 },
        ];
        let recent = recent_played(&saved);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].address, "new.example");
        assert!(recent.iter().all(|s| s.played_at > 0));
    }
}
