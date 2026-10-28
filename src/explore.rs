//! Modrinth search for the Mod page. No CurseForge.
//!
//! Explore asks for a project type, a Minecraft version, and — for mods and
//! modpacks — a loader. The popup then loads the matching file plus required
//! and optional dependencies.

use crate::net;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub downloads: u64,
    pub categories: Vec<String>,
    pub latest_version: String,
}

#[derive(Clone)]
pub struct Dep {
    pub project_id: String,
    pub title: String,
    pub kind: String,
    pub file_url: String,
    pub file_name: String,
}

#[derive(Clone)]
pub struct Detail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub icon_url: String,
    pub gallery_url: String,
    pub file_url: String,
    pub file_name: String,
    pub deps: Vec<Dep>,
    pub ready: bool,
    pub missing: String,
}

pub struct Outcome {
    pub note: String,
    pub pack: Option<PathBuf>,
}

pub fn project_type(tab: u8) -> &'static str {
    match tab {
        0 => "modpack",
        2 => "resourcepack",
        3 => "shader",
        _ => "mod",
    }
}

pub fn search(kind: &str, version: &str, loader: &str, query: &str) -> Result<Vec<Hit>, String> {
    let v = net::get_json(&search_url(kind, version, loader, query)).map_err(|e| short_err("Modrinth search", &e))?;
    let mut out = Vec::new();
    if let Some(hits) = v.get("hits").and_then(|h| h.as_array()) {
        for h in hits {
            let id = h.get("project_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            let categories = h
                .get("display_categories")
                .or_else(|| h.get("categories"))
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| c.as_str().map(|s| s.to_string()))
                        .filter(|s| !matches!(s.as_str(), "fabric" | "forge" | "neoforge" | "quilt"))
                        .take(3)
                        .collect()
                })
                .unwrap_or_default();
            out.push(Hit {
                id,
                title: h.get("title").and_then(|x| x.as_str()).unwrap_or("Untitled").into(),
                description: h.get("description").and_then(|x| x.as_str()).unwrap_or("").into(),
                icon_url: h.get("icon_url").and_then(|x| x.as_str()).unwrap_or("").into(),
                downloads: h.get("downloads").and_then(|x| x.as_u64()).unwrap_or(0),
                categories,
                latest_version: h.get("latest_version").and_then(|x| x.as_str()).unwrap_or("").into(),
            });
        }
    }
    Ok(out)
}

pub fn search_url(kind: &str, version: &str, loader: &str, query: &str) -> String {
    let mut facets = vec![vec![format!("project_type:{kind}")]];
    if !version.is_empty() {
        facets.push(vec![format!("versions:{version}")]);
    }
    if (kind == "mod" || kind == "modpack") && matches!(loader, "forge" | "fabric" | "neoforge") {
        facets.push(vec![format!("categories:{loader}")]);
    }
    let facets_json = serde_json::to_string(&facets).unwrap_or_else(|_| "[]".into());
    let index = if query.trim().is_empty() { "downloads" } else { "relevance" };
    let mut url = format!("https://api.modrinth.com/v2/search?limit=18&index={index}&facets={}", enc(&facets_json));
    if !query.trim().is_empty() {
        url.push_str("&query=");
        url.push_str(&enc(query.trim()));
    }
    url
}

pub fn detail(id: &str, version: &str, loader: &str, kind: &str) -> Result<Detail, String> {
    let project = net::get_json(&format!("https://api.modrinth.com/v2/project/{}", enc(id))).map_err(|e| short_err("Modrinth", &e))?;
    let title = project.get("title").and_then(|v| v.as_str()).unwrap_or(id).to_string();
    let summary = project.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let body_raw = plain(project.get("body").and_then(|v| v.as_str()).unwrap_or(""));
    let mut body = body_raw.chars().take(700).collect::<String>();
    if body_raw.chars().count() > 700 {
        body.push('…');
    }
    if body == summary {
        body.clear();
    }
    let icon_url = project.get("icon_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let gallery_url = project
        .get("gallery")
        .and_then(|g| g.as_array())
        .and_then(|a| a.iter().find_map(|g| g.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())))
        .unwrap_or_default();
    let use_loader = kind == "mod" || kind == "modpack";
    let versions = fetch_versions(id, version, if use_loader { loader } else { "" })?;
    let Some(ver) = versions.as_array().and_then(|a| a.first()) else {
        return Ok(Detail {
            id: id.into(),
            title,
            description: if summary.is_empty() { "No description.".into() } else { summary },
            body,
            icon_url,
            gallery_url,
            file_url: String::new(),
            file_name: String::new(),
            deps: Vec::new(),
            ready: true,
            missing: format!("No file for {version}."),
        });
    };
    let (file_url, file_name) = primary_file(ver);
    let mut deps = Vec::new();
    if kind == "mod" {
        if let Some(arr) = ver.get("dependencies").and_then(|d| d.as_array()) {
            let mut raw = Vec::new();
            for d in arr {
                let dep_kind = d.get("dependency_type").and_then(|v| v.as_str()).unwrap_or("");
                if dep_kind != "required" && dep_kind != "optional" {
                    continue;
                }
                let project_id = d.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if project_id.is_empty() {
                    continue;
                }
                let version_id = d.get("version_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                raw.push((project_id, version_id, dep_kind.to_string()));
                if raw.len() >= 12 {
                    break;
                }
            }
            let titles = project_titles(&raw.iter().map(|(id, _, _)| id.clone()).collect::<Vec<_>>());
            for (project_id, version_id, dep_kind) in raw {
                let dep_title = titles.get(&project_id).cloned().unwrap_or_else(|| project_id.clone());
                let (dep_url, dep_name) = resolve_dep_file(&project_id, &version_id, version, loader);
                deps.push(Dep {
                    project_id,
                    title: dep_title,
                    kind: dep_kind,
                    file_url: dep_url,
                    file_name: dep_name,
                });
            }
        }
    }
    let missing = if file_url.is_empty() { format!("No file for {version}.") } else { String::new() };
    Ok(Detail {
        id: id.into(),
        title,
        description: if summary.is_empty() { "No description.".into() } else { summary },
        body,
        icon_url,
        gallery_url,
        file_url,
        file_name,
        deps,
        ready: true,
        missing,
    })
}

pub fn install(
    detail: &Detail,
    optional: &[String],
    dest: &Path,
    with_deps: bool,
    as_pack: bool,
    cancel: &std::sync::atomic::AtomicBool,
    mut on: impl FnMut(f32),
) -> Outcome {
    if detail.file_url.is_empty() {
        return Outcome { note: "No file for this version.".into(), pack: None };
    }
    if let Err(e) = std::fs::create_dir_all(dest) {
        return Outcome { note: e.to_string(), pack: None };
    }
    let mut files: Vec<(&str, &str, Option<&str>)> = vec![(&detail.file_url, &detail.file_name, None)];
    if with_deps {
        for dep in &detail.deps {
            let take = dep.kind == "required" || optional.iter().any(|id| id == &dep.project_id);
            if take {
                files.push((&dep.file_url, &dep.file_name, Some(dep.title.as_str())));
            }
        }
    }
    let n = files.len().max(1);
    let mut saved = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    for (i, (url, name, label)) in files.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Outcome { note: "Download stopped".into(), pack: None };
        }
        if url.is_empty() {
            let who = label.unwrap_or("this project");
            failed.push(format!("No file for {who}"));
            on(blend(i + 1, n, 0.0));
            continue;
        }
        let report = |frac: f32| on(blend(i, n, frac));
        match write_url(url, name, dest, cancel, report) {
            Ok(Wrote::Saved(file)) => {
                saved.push(match label {
                    Some(title) => format!("{title} ({file})"),
                    None => file,
                });
            }
            Ok(Wrote::Exists(file)) => skipped.push(file),
            Err(e) if e == "canceled" => {
                return Outcome { note: "Download stopped".into(), pack: None };
            }
            Err(e) => failed.push(match label {
                Some(title) => format!("{title}: {e}"),
                None => e,
            }),
        }
    }
    on(1.0);
    let mut parts = Vec::new();
    if !saved.is_empty() {
        parts.push(format!("Installed {}", saved.join(", ")));
    }
    if !skipped.is_empty() {
        parts.push(format!("Already there: {}", skipped.join(", ")));
    }
    if !failed.is_empty() {
        parts.push(failed.join(" · "));
    }
    if parts.is_empty() {
        parts.push("Nothing was installed.".into());
    }
    let pack = if as_pack {
        safe_name(&detail.file_name).map(|name| dest.join(name)).filter(|p| p.exists())
    } else {
        None
    };
    Outcome { note: parts.join(". "), pack }
}

enum Wrote {
    Saved(String),
    Exists(String),
}

fn blend(index: usize, count: usize, file: f32) -> f32 {
    let count = count.max(1) as f32;
    ((index as f32 + file.clamp(0.0, 1.0)) / count).clamp(0.0, 1.0)
}

fn write_url(url: &str, file_name: &str, dest: &Path, cancel: &std::sync::atomic::AtomicBool, mut on: impl FnMut(f32)) -> Result<Wrote, String> {
    if !url.starts_with("https://") {
        return Err("Refusing a non-https download".into());
    }
    let name = safe_name(file_name).unwrap_or_else(|| "download.jar".into());
    let path = dest.join(&name);
    if path.exists() {
        on(1.0);
        return Ok(Wrote::Exists(name));
    }
    match net::copy_url(url, &path, cancel, &mut on) {
        Ok(_) => Ok(Wrote::Saved(name)),
        Err(e) if e == "canceled" => Err(e),
        Err(e) => Err(short_err("Download", &e)),
    }
}

fn fetch_versions(id: &str, game: &str, loader: &str) -> Result<Value, String> {
    let mut url = format!("https://api.modrinth.com/v2/project/{}/version?", enc(id));
    if !game.is_empty() {
        let games = serde_json::to_string(&vec![game]).unwrap_or_else(|_| "[]".into());
        url.push_str(&format!("game_versions={}&", enc(&games)));
    }
    if !loader.is_empty() {
        let loaders = serde_json::to_string(&vec![loader]).unwrap_or_else(|_| "[]".into());
        url.push_str(&format!("loaders={}", enc(&loaders)));
    }
    let url = url.trim_end_matches('&').trim_end_matches('?');
    net::get_json(url).map_err(|e| short_err("Modrinth", &e))
}

fn resolve_dep_file(project_id: &str, version_id: &str, game: &str, loader: &str) -> (String, String) {
    if !version_id.is_empty() {
        let url = format!("https://api.modrinth.com/v2/version/{}", enc(version_id));
        if let Ok(v) = net::get_json(&url) {
            return primary_file(&v);
        }
    }
    if let Ok(v) = fetch_versions(project_id, game, loader) {
        if let Some(first) = v.as_array().and_then(|a| a.first()) {
            return primary_file(first);
        }
    }
    (String::new(), String::new())
}

fn project_titles(ids: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return out;
    }
    let json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    let url = format!("https://api.modrinth.com/v2/projects?ids={}", enc(&json));
    let Ok(v) = net::get_json(&url) else { return out };
    if let Some(arr) = v.as_array() {
        for p in arr {
            let id = p.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let title = p.get("title").and_then(|x| x.as_str()).unwrap_or(id);
            if !id.is_empty() {
                out.insert(id.to_string(), title.to_string());
            }
        }
    }
    out
}

fn primary_file(ver: &Value) -> (String, String) {
    let Some(files) = ver.get("files").and_then(|f| f.as_array()) else {
        return (String::new(), String::new());
    };
    let file = files.iter().find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false)).or_else(|| files.first());
    let Some(file) = file else { return (String::new(), String::new()) };
    (
        file.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        file.get("filename").and_then(|v| v.as_str()).unwrap_or("download.jar").to_string(),
    )
}

fn safe_name(name: &str) -> Option<String> {
    let name = name.trim().replace('\\', "/");
    let leaf = name.rsplit('/').next().unwrap_or("").trim();
    if leaf.is_empty() || leaf == "." || leaf == ".." || leaf.contains('\0') || leaf.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(leaf.to_string())
}

fn short_err(what: &str, err: &str) -> String {
    if let Some(rest) = err.split("HTTP ").nth(1) {
        let code = rest.split_whitespace().next().unwrap_or("error");
        return format!("{what} failed (HTTP {code})");
    }
    format!("{what} failed")
}

pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn plain(md: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = md.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' && chars.get(i + 1) == Some(&'[') {
            if let Some(end) = skip_link(&chars, i + 1) {
                i = end;
                continue;
            }
        }
        if chars[i] == '[' {
            if let Some((text, end)) = take_link(&chars, i) {
                out.push_str(&text);
                i = end;
                continue;
            }
        }
        let c = chars[i];
        if c == '`' || c == '*' || c == '_' {
            i += 1;
            continue;
        }
        if c == '#' && (i == 0 || chars[i - 1] == '\n') {
            while i < chars.len() && chars[i] == '#' {
                i += 1;
            }
            if chars.get(i) == Some(&' ') {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    let mut cleaned = String::new();
    let mut blank = 0;
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            blank += 1;
            if blank <= 1 && !cleaned.is_empty() {
                cleaned.push('\n');
            }
            continue;
        }
        blank = 0;
        if !cleaned.is_empty() && !cleaned.ends_with('\n') {
            cleaned.push('\n');
        }
        cleaned.push_str(line);
    }
    let mut stripped = String::new();
    let mut in_tag = false;
    for c in cleaned.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' && in_tag {
            in_tag = false;
            if !stripped.ends_with(' ') && !stripped.ends_with('\n') {
                stripped.push(' ');
            }
            continue;
        }
        if !in_tag {
            stripped.push(c);
        }
    }
    stripped.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn skip_link(chars: &[char], start: usize) -> Option<usize> {
    take_link(chars, start).map(|(_, end)| end)
}

fn take_link(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let close = chars[start + 1..].iter().position(|c| *c == ']')? + start + 1;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars[close + 2..].iter().position(|c| *c == ')')? + close + 3;
    let text: String = chars[start + 1..close].iter().collect();
    Some((text, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_url_encodes_facets_and_skips_loader_for_shaders() {
        let url = search_url("mod", "26.3", "fabric", "");
        assert!(url.contains("%5B"), "{url}");
        assert!(!url.contains("[["), "{url}");
        assert!(url.contains("index=downloads"), "{url}");
        let shader = search_url("shader", "26.3", "fabric", "complementary");
        assert!(!shader.contains("categories"), "{shader}");
        assert!(shader.contains("query="), "{shader}");
    }

    #[test]
    fn blend_walks_files_then_fills() {
        assert!((blend(0, 2, 0.0) - 0.0).abs() < 0.001);
        assert!((blend(0, 2, 0.5) - 0.25).abs() < 0.001);
        assert!((blend(1, 2, 1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn plain_text_drops_markdown_links() {
        let text = plain("# Hello\n\nSee [Sodium](https://modrinth.com) and **keep** this.");
        assert!(text.contains("Hello"));
        assert!(text.contains("Sodium"));
        assert!(!text.contains("https://"));
        assert!(text.contains("keep"));
    }
}
