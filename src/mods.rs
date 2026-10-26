//! Local mod library: jars dropped in `<data>/mods` get staged into every
//! instance's `mods` folder (Horizon-style mods are plain jars).
use crate::net;
use crate::state::App;
use std::fs;
use std::path::{Path, PathBuf};

pub fn library_dir(app: &App) -> PathBuf {
    app.p("mods")
}

fn disabled_file(app: &App) -> PathBuf {
    app.p("mods/disabled.json")
}

fn disabled_set(app: &App) -> Vec<String> {
    fs::read_to_string(disabled_file(app))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
}

fn set_disabled(app: &App, list: Vec<String>) {
    let _ = fs::write(disabled_file(app), serde_json::to_string(&list).unwrap_or_default());
}

pub fn list(app: &App) -> serde_json::Value {
    let dir = library_dir(app);
    let disabled = disabled_set(app);
    let mut mods = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jar") {
                continue;
            }
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let size = p.metadata().map(|m| m.len()).unwrap_or(0);
            mods.push(serde_json::json!({
                "name": name,
                "size": size,
                "enabled": !disabled.contains(&name),
                "sha1": net::sha1_file(&p),
            }));
        }
    }
    mods.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    serde_json::json!({ "dir": dir.to_string_lossy(), "mods": mods })
}

pub fn add(app: &App, file_name: &str, data_url: &str) -> Result<String, String> {
    use base64::Engine;
    let clean: String = file_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+' { c } else { '_' })
        .collect();
    if !clean.ends_with(".jar") {
        return Err("only .jar mods are supported".into());
    }
    let b64 = data_url.split("base64,").nth(1).unwrap_or(data_url).trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("bad data: {e}"))?;
    if bytes.len() < 22 || &bytes[0..2] != b"PK" {
        return Err("not a valid jar (missing zip header)".into());
    }
    let dir = library_dir(app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(&clean), bytes).map_err(|e| e.to_string())?;
    Ok(clean)
}

pub fn remove(app: &App, name: &str) -> Result<(), String> {
    let path = library_dir(app).join(name);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let mut d = disabled_set(app);
    d.retain(|x| x != name);
    set_disabled(app, d);
    Ok(())
}

pub fn toggle(app: &App, name: &str) -> Result<bool, String> {
    let mut d = disabled_set(app);
    let now_enabled = if d.iter().any(|x| x == name) {
        d.retain(|x| x != name);
        true
    } else {
        d.push(name.to_string());
        false
    };
    set_disabled(app, d);
    Ok(now_enabled)
}

/// Copy enabled jars into an instance's mods folder. Returns how many.
pub fn stage(app: &App, version: &str) -> Result<usize, String> {
    let src = library_dir(app);
    let dst = app.game_dir(version).join("mods");
    fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
    let disabled = disabled_set(app);
    let mut n = 0;
    if let Ok(rd) = fs::read_dir(&src) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jar") {
                continue;
            }
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let target = dst.join(&name);
            if disabled.contains(&name) {
                let _ = fs::remove_file(&target);
                continue;
            }
            let same = match (p.metadata(), target.metadata()) {
                (Ok(a), Ok(b)) => a.len() == b.len() && a.modified().ok() == b.modified().ok(),
                _ => false,
            };
            if !same {
                fs::copy(&p, &target).map_err(|e| format!("{}: {e}", partial(&p)))?;
            }
            n += 1;
        }
    }
    // drop staged jars that no longer exist in the library
    if let Ok(rd) = fs::read_dir(&dst) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("jar") && !src.join(Path::new(p.file_name().unwrap_or_default())).exists() {
                let _ = fs::remove_file(&p);
            }
        }
    }
    Ok(n)
}

fn partial(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().to_string()
}
