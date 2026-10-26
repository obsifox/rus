//! Official loader builds for the selected Minecraft version.
//!
//! Forge, Fabric and NeoForge versions come from their own feeds. OptiFine has
//! no public feed and no 26.x build, so it stays a preview.

use crate::net;
use crate::state::App;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOADERS: [&str; 5] = ["vanilla", "forge", "fabric", "neoforge", "optifine"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    BuiltIn,
    Ready,
    Preview,
}

#[derive(Clone)]
pub struct LoaderRow {
    pub id: &'static str,
    pub name: &'static str,
    pub version: String,
    pub detail: String,
    pub badge: &'static str,
    pub kind: Badge,
}

#[derive(Clone, Default)]
pub struct Catalog {
    /// `26.3` → `66.1.0`, preferring the recommended promotion.
    pub forge: Vec<(String, String)>,
    pub neoforge: Vec<String>,
    /// Minecraft version → newest stable Fabric Loader.
    pub fabric: Vec<(String, String)>,
    pub ready: bool,
    pub note: String,
}

pub fn dot_date(raw: &str) -> String {
    let date = raw.split('T').next().unwrap_or(raw);
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    } else {
        date.replace('-', ".")
    }
}

pub fn missing_line(mc: &str) -> String {
    format!("NO {mc} BUILD")
}

pub fn profile_id(mc: &str, loader: &str) -> String {
    format!("{mc}-{loader}")
}

pub fn rows_for(mc: &str, cat: &Catalog) -> Vec<LoaderRow> {
    let forge = forge_for(mc, &cat.forge);
    let fabric = fabric_for(mc, &cat.fabric);
    let neo = neoforge_for(mc, &cat.neoforge);
    vec![
        LoaderRow {
            id: "vanilla",
            name: "Vanilla",
            version: mc.to_string(),
            detail: mc.to_string(),
            badge: "BUILT-IN",
            kind: Badge::BuiltIn,
        },
        row("forge", "Forge", forge),
        row("fabric", "Fabric", fabric),
        row("neoforge", "NeoForge", neo),
        LoaderRow {
            id: "optifine",
            name: "OptiFine",
            version: String::new(),
            detail: missing_line(mc),
            badge: "PREVIEW",
            kind: Badge::Preview,
        },
    ]
}

fn row(id: &'static str, name: &'static str, version: Option<String>) -> LoaderRow {
    match version {
        Some(version) => LoaderRow {
            detail: version.clone(),
            version,
            id,
            name,
            badge: "READY",
            kind: Badge::Ready,
        },
        None => LoaderRow {
            id,
            name,
            version: String::new(),
            detail: String::new(),
            badge: "PREVIEW",
            kind: Badge::Preview,
        },
    }
}

pub fn forge_for(mc: &str, promos: &[(String, String)]) -> Option<String> {
    let rec = format!("{mc}-recommended");
    let latest = format!("{mc}-latest");
    promos
        .iter()
        .find(|(k, _)| k == &rec)
        .or_else(|| promos.iter().find(|(k, _)| k == &latest))
        .map(|(_, v)| v.clone())
}

pub fn fabric_for(mc: &str, known: &[(String, String)]) -> Option<String> {
    known.iter().find(|(k, v)| k == mc && !v.is_empty()).map(|(_, v)| v.clone())
}

/// NeoForge lines: `26.3` → `26.3.0.*`, `26.1.2` → `26.1.2.*`, `1.21.1` → `21.1.*`.
pub fn neo_prefix(mc: &str) -> String {
    let parts: Vec<&str> = mc.split('.').filter(|p| !p.is_empty()).collect();
    if parts.first() == Some(&"1") {
        if parts.len() == 2 {
            format!("{}.0", parts[1])
        } else if parts.len() >= 3 {
            format!("{}.{}", parts[1], parts[2])
        } else {
            mc.to_string()
        }
    } else if parts.len() == 2 {
        format!("{mc}.0")
    } else {
        mc.to_string()
    }
}

pub fn neoforge_for(mc: &str, versions: &[String]) -> Option<String> {
    let prefix = format!("{}.", neo_prefix(mc));
    let mut best: Option<&str> = None;
    for v in versions {
        let lower = v.to_ascii_lowercase();
        if lower.contains("beta") || lower.contains("alpha") || lower.contains("snapshot") {
            continue;
        }
        if !v.starts_with(&prefix) {
            continue;
        }
        if best.map(|b| ver_less(b, v)).unwrap_or(true) {
            best = Some(v);
        }
    }
    best.map(|s| s.to_string())
}

fn ver_less(a: &str, b: &str) -> bool {
    ver_key(a) < ver_key(b)
}

fn ver_key(s: &str) -> Vec<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect()
}

pub fn fetch_shared() -> Result<Catalog, String> {
    let mut cat = Catalog { ready: true, ..Catalog::default() };
    match net::get_json("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json") {
        Ok(v) => {
            if let Some(obj) = v.get("promos").and_then(|p| p.as_object()) {
                for (k, val) in obj {
                    if let Some(s) = val.as_str() {
                        cat.forge.push((k.clone(), s.to_string()));
                    }
                }
            }
        }
        Err(e) => cat.note = e,
    }
    match net::get_bytes("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml") {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                let line = line.trim();
                if let Some(inner) = line.strip_prefix("<version>").and_then(|s| s.strip_suffix("</version>")) {
                    cat.neoforge.push(inner.to_string());
                }
            }
        }
        Err(e) => {
            if cat.note.is_empty() {
                cat.note = e;
            }
        }
    }
    Ok(cat)
}

pub fn fetch_fabric(mc: &str) -> Result<String, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc}");
    let v = net::get_json(&url)?;
    let arr = v.as_array().ok_or_else(|| "Fabric meta was not a list".to_string())?;
    let stable = arr.iter().find(|e| e.get("loader").and_then(|l| l.get("stable")).and_then(|s| s.as_bool()).unwrap_or(false));
    let pick = stable.or_else(|| arr.first()).ok_or_else(|| format!("No Fabric loader for {mc}"))?;
    pick.get("loader")
        .and_then(|l| l.get("version"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Fabric loader for {mc} had no version"))
}

pub fn is_profile_installed(app: &App, mc: &str, loader: &str) -> bool {
    installed_profile(app, mc, loader).is_some()
}

pub fn installed_profile(app: &App, mc: &str, loader: &str) -> Option<String> {
    if loader == "vanilla" {
        return app.is_installed(mc).then(|| mc.to_string());
    }
    let marker = read_marker(app, mc, loader);
    if let Some(id) = marker {
        if app.is_installed(&id) {
            return Some(id);
        }
    }
    let direct = profile_id(mc, loader);
    if app.is_installed(&direct) {
        return Some(direct);
    }
    let prefix = format!("{mc}-{loader}");
    let dir = app.p("versions");
    let rd = fs::read_dir(&dir).ok()?;
    let mut found = None;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name == prefix || name.starts_with(&format!("{prefix}-")) {
            if app.is_installed(&name) {
                found = Some(name);
            }
        }
    }
    found
}

fn marker_path(app: &App) -> PathBuf {
    app.p("loaders.json")
}

fn read_marker(app: &App, mc: &str, loader: &str) -> Option<String> {
    let txt = fs::read_to_string(marker_path(app)).ok()?;
    let v: Value = serde_json::from_str(&txt).ok()?;
    v.get(mc)?.get(loader)?.as_str().map(|s| s.to_string())
}

fn write_marker(app: &App, mc: &str, loader: &str, profile: &str) {
    let path = marker_path(app);
    let mut v: Value = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    if !v.is_object() {
        v = json!({});
    }
    v.as_object_mut().unwrap().entry(mc.to_string()).or_insert_with(|| json!({}));
    if let Some(obj) = v.get_mut(mc).and_then(|x| x.as_object_mut()) {
        obj.insert(loader.to_string(), json!(profile));
    }
    let _ = fs::create_dir_all(app.dir.clone());
    let _ = fs::write(path, serde_json::to_string_pretty(&v).unwrap_or_default());
}

/// Install the selected loader. Vanilla must already be installed.
pub fn install(app: &App, mc: &str, loader: &str, loader_version: &str) -> Result<String, String> {
    match loader {
        "vanilla" => Ok(mc.to_string()),
        "optifine" => Err(format!("OptiFine has {}", missing_line(mc).to_lowercase())),
        "fabric" => install_fabric(app, mc, loader_version),
        "forge" => install_forge_like(app, mc, "forge", loader_version),
        "neoforge" => install_forge_like(app, mc, "neoforge", loader_version),
        other => Err(format!("Unknown loader {other}")),
    }
}

fn install_fabric(app: &App, mc: &str, loader_version: &str) -> Result<String, String> {
    if !app.is_installed(mc) {
        return Err(format!("Install {mc} first"));
    }
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{loader_version}/profile/json");
    let profile = net::get_json(&url)?;
    let vanilla_path = app.p(&format!("versions/{mc}/{mc}.json"));
    let vanilla_txt = fs::read_to_string(&vanilla_path).map_err(|e| e.to_string())?;
    let mut meta: Value = serde_json::from_str(&vanilla_txt).map_err(|e| e.to_string())?;
    let id = profile_id(mc, "fabric");
    meta["id"] = json!(id);
    if let Some(main) = profile.get("mainClass") {
        meta["mainClass"] = main.clone();
    }
    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        let dest = meta.as_object_mut().ok_or("version json")?.entry("libraries").or_insert_with(|| json!([]));
        if let Some(arr) = dest.as_array_mut() {
            for lib in libs {
                arr.push(lib.clone());
            }
        }
    }
    if let Some(jvm) = profile.pointer("/arguments/jvm").and_then(|v| v.as_array()) {
        let args = meta.as_object_mut().unwrap().entry("arguments").or_insert_with(|| json!({}));
        let jvm_dest = args.as_object_mut().unwrap().entry("jvm").or_insert_with(|| json!([]));
        if let Some(arr) = jvm_dest.as_array_mut() {
            for a in jvm {
                arr.insert(0, a.clone());
            }
        }
    }
    let dir = app.p(&format!("versions/{id}"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{id}.json")), serde_json::to_string_pretty(&meta).unwrap_or_default()).map_err(|e| e.to_string())?;
    let src_jar = app.p(&format!("versions/{mc}/{mc}.jar"));
    let dst_jar = dir.join(format!("{id}.jar"));
    if !dst_jar.exists() {
        fs::copy(&src_jar, &dst_jar).map_err(|e| format!("client jar: {e}"))?;
    }
    let plan = crate::install::plan_libraries(app, &id, &meta);
    for (i, dest) in plan.dests.iter().enumerate() {
        if let Some((url, sha)) = plan.downloads.get(i) {
            net::download(url, dest, sha.as_deref(), |_| {})?;
        }
    }
    write_marker(app, mc, "fabric", &id);
    Ok(id)
}

fn install_forge_like(app: &App, mc: &str, loader: &str, version: &str) -> Result<String, String> {
    if !app.is_installed(mc) {
        return Err(format!("Install {mc} first"));
    }
    if version.is_empty() {
        return Err(missing_line(mc));
    }
    let (url, jar_name) = if loader == "neoforge" {
        (
            format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"),
            format!("neoforge-{version}-installer.jar"),
        )
    } else {
        (
            format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-installer.jar"),
            format!("forge-{mc}-{version}-installer.jar"),
        )
    };
    let dest = app.p(&format!("cache/installers/{jar_name}"));
    net::download(&url, &dest, None, |_| {})?;
    let java = find_java(app).ok_or_else(|| "Java is not installed yet. Install the game first, then try the loader again.".to_string())?;
    let mut cmd = Command::new(&java);
    cmd.arg("-jar").arg(&dest).arg("--installClient").arg(&app.dir);
    cmd.current_dir(&app.dir);
    crate::play::hide_console(&mut cmd);
    cmd.stdin(std::process::Stdio::null());
    let out = cmd.output().map_err(|e| format!("Could not start the {loader} installer: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: String = err.chars().rev().take(280).collect::<String>().chars().rev().collect();
        return Err(format!("{loader} installer failed. {tail}"));
    }
    if let Some(id) = installed_profile(app, mc, loader) {
        write_marker(app, mc, loader, &id);
        return Ok(id);
    }
    // The installer may have used its own version id. Pick the newest match.
    let prefix = if loader == "neoforge" {
        format!("neoforge-{version}")
    } else {
        format!("{mc}-forge-{version}")
    };
    if let Some(id) = find_version_dir(&app.p("versions"), &prefix).or_else(|| find_version_dir(&app.p("versions"), &format!("{mc}-{loader}"))) {
        write_marker(app, mc, loader, &id);
        return Ok(id);
    }
    Err(format!("{loader} installer finished, but no version profile was found"))
}

fn find_version_dir(dir: &Path, prefix: &str) -> Option<String> {
    let rd = fs::read_dir(dir).ok()?;
    let mut names: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n == prefix || n.starts_with(prefix))
        .collect();
    names.sort();
    names.pop()
}

fn find_java(app: &App) -> Option<PathBuf> {
    if let Some(p) = crate::java::system_java() {
        return Some(p);
    }
    for component in ["java-runtime-delta", "java-runtime-epsilon", "java-runtime-gamma", "jre-legacy"] {
        if let Some(p) = crate::java::bundled(app, component) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_date_uses_dots() {
        assert_eq!(dot_date("2026-09-15T10:00:00+00:00"), "2026.09.15");
        assert_eq!(dot_date("2025-12-09"), "2025.12.09");
    }

    #[test]
    fn missing_build_line() {
        assert_eq!(missing_line("26.3"), "NO 26.3 BUILD");
    }

    #[test]
    fn forge_prefers_recommended() {
        let promos = vec![
            ("26.3-latest".into(), "66.0.9".into()),
            ("26.3-recommended".into(), "66.1.0".into()),
            ("26.2-latest".into(), "65.1.5".into()),
        ];
        assert_eq!(forge_for("26.3", &promos).as_deref(), Some("66.1.0"));
        assert_eq!(forge_for("26.2", &promos).as_deref(), Some("65.1.5"));
        assert_eq!(forge_for("1.0", &promos), None);
    }

    #[test]
    fn neoforge_picks_newest_stable_line() {
        let versions = vec![
            "26.3.0.98-beta".into(),
            "26.3.0.97".into(),
            "26.3.0.99".into(),
            "26.1.2.124".into(),
            "26.1.0.40".into(),
            "21.11.48".into(),
            "21.1.265".into(),
        ];
        assert_eq!(neoforge_for("26.3", &versions).as_deref(), Some("26.3.0.99"));
        assert_eq!(neoforge_for("26.1.2", &versions).as_deref(), Some("26.1.2.124"));
        assert_eq!(neoforge_for("26.1", &versions).as_deref(), Some("26.1.0.40"));
        assert_eq!(neoforge_for("1.21.11", &versions).as_deref(), Some("21.11.48"));
        assert_eq!(neoforge_for("1.21.1", &versions).as_deref(), Some("21.1.265"));
    }
}
