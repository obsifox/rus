//! Mojang piston-meta access: version manifest + per-version metadata, with caches.
use crate::net;
use crate::state::App;
use std::time::{Duration, Instant};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

/// `release` for stable builds, `preview` for pre-releases / release candidates,
/// `snapshot` for weekly snapshots.
pub fn classify(id: &str) -> &'static str {
    if id.contains("-pre-") || id.contains("-rc-") {
        "preview"
    } else if id.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        && !id.contains('.')
        && !id.contains('-')
    {
        "snapshot"
    } else if id.contains("-snapshot-") || id.contains('w') {
        "snapshot"
    } else {
        "release"
    }
}

/// Leading numeric series of a version id (`26.3` -> 26, `1.21.4` -> 1).
pub fn series(id: &str) -> u32 {
    let digits: String = id.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

/// The user only wants vanilla builds: every release, plus snapshots/previews of 26+.
pub fn supported(id: &str, vtype: &str) -> bool {
    match vtype {
        "release" => true,
        "snapshot" => series(id) >= 26,
        _ => false,
    }
}

pub fn manifest(app: &App) -> Result<serde_json::Value, String> {
    {
        let cache = app.manifest.lock().unwrap();
        if let Some((at, v)) = cache.as_ref() {
            if at.elapsed() < Duration::from_secs(300) {
                return Ok(v.clone());
            }
        }
    }
    let disk = app.p("cache/manifest.json");
    match net::get_json(MANIFEST_URL) {
        Ok(v) => {
            let _ = std::fs::write(&disk, serde_json::to_string(&v).unwrap_or_default());
            *app.manifest.lock().unwrap() = Some((Instant::now(), v.clone()));
            Ok(v)
        }
        Err(e) => {
            if let Ok(txt) = std::fs::read_to_string(&disk) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                    return Ok(v);
                }
            }
            Err(e)
        }
    }
}

/// Filtered, UI-ready version list (newest first).
pub fn version_list(app: &App) -> Result<serde_json::Value, String> {
    let m = manifest(app)?;
    let latest = m.get("latest").cloned().unwrap_or(serde_json::json!({}));
    let mut out = Vec::new();
    for v in m
        .get("versions")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let id = v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let vtype = v.get("type").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        if !supported(&id, &vtype) {
            continue;
        }
        let channel = classify(&id);
        let settings = app.settings();
        if channel == "snapshot" && !settings.show_snapshots {
            continue;
        }
        if channel == "preview" && !settings.show_previews {
            continue;
        }
        out.push(serde_json::json!({
            "id": id,
            "type": vtype,
            "channel": channel,
            "released": v.get("releaseTime").and_then(|x| x.as_str()).unwrap_or_default(),
            "series": series(&id),
            "installed": app.is_installed(&id),
            "meta_url": v.get("url").and_then(|x| x.as_str()).unwrap_or_default(),
        }));
    }
    Ok(serde_json::json!({ "latest": latest, "versions": out }))
}

pub fn meta_url(app: &App, id: &str) -> Result<String, String> {
    let m = manifest(app)?;
    m.get("versions")
        .and_then(|x| x.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|v| v.get("id").and_then(|i| i.as_str()) == Some(id))
        })
        .and_then(|v| v.get("url").and_then(|u| u.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| format!("version '{id}' not found in manifest"))
}

/// Per-version metadata (cached on disk so installs survive offline restarts).
pub fn version_meta(app: &App, id: &str) -> Result<serde_json::Value, String> {
    let cache = app.p("cache/versions").join(format!("{id}.json"));
    if let Ok(txt) = std::fs::read_to_string(&cache) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            return Ok(v);
        }
    }
    let url = meta_url(app, id)?;
    let v = net::get_json(&url)?;
    let _ = std::fs::write(&cache, serde_json::to_string(&v).unwrap_or_default());
    Ok(v)
}
