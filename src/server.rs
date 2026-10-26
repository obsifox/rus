//! Tiny HTTP server: serves the UI and a small JSON API.
use crate::state::{now_ms, App, Job};
use crate::{horizon, install, mojang, mods, play, skin};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Request, Response, Server};

const UI_HTML: &str = include_str!("../ui/index.html");
const UI_JS: &str = include_str!("../ui/app.js");
const UI_CSS: &str = include_str!("../ui/style.css");
const FONTS_CSS: &str = include_str!("../assets/fonts.css");

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn respond(req: Request, status: u32, ctype: &str, body: Vec<u8>) {
    let len = body.len();
    let resp = Response::empty(status)
        .with_header(header("Content-Type", ctype))
        .with_header(header("Content-Length", &len.to_string()))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header("Access-Control-Allow-Origin", "*"))
        .with_header(header("Access-Control-Allow-Headers", "*"))
        .with_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
        .with_header(header("X-Frame-Options", "ALLOWALL"))
        .with_data(std::io::Cursor::new(body), Some(len));
    let _ = req.respond(resp);
}

fn json_resp(req: Request, status: u32, v: Value) {
    respond(req, status, "application/json; charset=utf-8", v.to_string().into_bytes());
}

fn body_json(req: &mut Request) -> Value {
    let mut buf = String::new();
    let _ = req.as_reader().read_to_string(&mut buf);
    serde_json::from_str(&buf).unwrap_or(Value::Null)
}

fn query(url: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Some(q) = url.split('?').nth(1) {
        for pair in q.split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or_default().to_string();
            let v = it.next().unwrap_or_default().to_string();
            map.insert(k, urldecode(&v));
        }
    }
    map
}

fn urldecode(s: &str) -> String {
    let s = s.replace('+', " ");
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn start(app: Arc<App>, port: u16) -> Result<(), String> {
    let server = Server::http(("0.0.0.0", port)).map_err(|e| e.to_string())?;
    let actual = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    println!(
        "{{\"ready\":true,\"port\":{},\"url\":\"http://0.0.0.0:{}/\"}}",
        actual, actual
    );
    println!("launcher UI → http://127.0.0.1:{actual}/");
    use std::io::Write;
    let _ = std::io::stdout().flush();

    for req in server.incoming_requests() {
        let app = app.clone();
        std::thread::spawn(move || handle(app, req));
    }
    Ok(())
}

fn handle(app: Arc<App>, mut req: Request) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("/").to_string();
    let q = query(&url);

    if method == Method::Options {
        return respond(req, 204, "text/plain", Vec::new());
    }

    match (method.clone(), path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            return respond(req, 200, "text/html; charset=utf-8", UI_HTML.as_bytes().to_vec())
        }
        (Method::Get, "/app.js") => {
            return respond(req, 200, "application/javascript; charset=utf-8", UI_JS.as_bytes().to_vec())
        }
        (Method::Get, "/style.css") => {
            return respond(req, 200, "text/css; charset=utf-8", UI_CSS.as_bytes().to_vec())
        }
        (Method::Get, "/fonts.css") => {
            return respond(req, 200, "text/css; charset=utf-8", FONTS_CSS.as_bytes().to_vec())
        }
        (Method::Get, "/api/state") => {
            let s = app.settings();
            let installed = installed_versions(&app);
            let sys = json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "java": crate::java::resolve(&app, "java-runtime-delta", 21)
                    .filter(|p| crate::java::major_of(p).map(|m| m >= 21).unwrap_or(false))
                    .or_else(crate::java::system_java)
                    .map(|p| p.to_string_lossy().to_string()),
                "java_major": crate::java::system_java().and_then(|p| crate::java::major_of(&p)),
                "runtimes": java_runtimes(&app),
                "system_java": crate::java::system_java().map(|p| p.to_string_lossy().to_string()),
                "ram_mb": total_ram_mb(),
                "data_dir": app.dir.to_string_lossy(),
                "game_dir": app.p("game").to_string_lossy(),
                "version": env!("CARGO_PKG_VERSION"),
                "sandbox": crate::sandbox::describe(&app),
            });
            return json_resp(req, 200, json!({
                "settings": s,
                "uuid": crate::state::offline_uuid(&s.username),
                "system": sys,
                "installed": installed,
                "running": play::status(),
                "horizon": horizon::status(&app),
                "now": now_ms(),
            }));
        }
        (Method::Get, "/api/versions") => match mojang::version_list(&app) {
            Ok(mut v) => {
                v["installed_list"] = json!(installed_versions(&app));
                v["patches"] = json!(install::patch_defs());
                json_resp(req, 200, v)
            }
            Err(e) => json_resp(req, 502, json!({ "error": e })),
        },
        (Method::Post, "/api/install") => {
            let body = body_json(&mut req);
            let version = body.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if version.is_empty() {
                return json_resp(req, 400, json!({ "error": "version required" }));
            }
            let mut patches = install::patch_defs();
            if let Some(sel) = body.get("patches").and_then(|p| p.as_object()) {
                for p in patches.iter_mut() {
                    if let Some(v) = sel.get(&p.key).and_then(|v| v.as_bool()) {
                        p.enabled = v;
                    }
                }
            }
            let id = format!("job-{}", now_ms());
            let job = Arc::new(Mutex::new(Job::new(id.clone(), version.clone(), patches)));
            app.jobs.lock().unwrap().insert(id.clone(), job.clone());
            *app.last_job.lock().unwrap() = Some(id.clone());
            let app2 = app.clone();
            let job2 = job.clone();
            std::thread::spawn(move || install::run(app2, job2));
            json_resp(req, 200, json!({ "job": id }))
        }
        (Method::Get, "/api/job") => {
            let id = q.get("id").cloned().unwrap_or_default();
            match app.job(&id) {
                Some(j) => {
                    let v = j.lock().unwrap().to_json();
                    json_resp(req, 200, v)
                }
                None => json_resp(req, 404, json!({ "error": "unknown job" })),
            }
        }
        (Method::Post, "/api/cancel") => {
            let body = body_json(&mut req);
            if let Some(id) = body.get("job").and_then(|v| v.as_str()) {
                if let Some(j) = app.job(id) {
                    if let Ok(mut job) = j.lock() {
                        job.cancel.store(true, Ordering::Relaxed);
                        job.message = "Canceling…".into();
                    }
                }
            }
            json_resp(req, 200, json!({ "ok": true }))
        }
        (Method::Post, "/api/settings") => {
            let body = body_json(&mut req);
            let mut s = app.settings();
            if let Ok(new) = serde_json::from_value::<crate::state::Settings>(body) {
                if !new.username.trim().is_empty() {
                    s = new;
                }
            }
            app.update_settings(s.clone());
            json_resp(req, 200, json!({ "ok": true, "settings": s }))
        }
        (Method::Post, "/api/play") => {
            let body = body_json(&mut req);
            let version = body.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let engine = body.get("engine").and_then(|v| v.as_str()).unwrap_or("vanilla");
            if engine == "horizon" {
                match horizon::start(app) {
                    Ok(v) => json_resp(req, 200, v),
                    Err(e) => json_resp(req, 400, json!({ "error": e })),
                }
            } else {
                match play::launch(&app, &version) {
                    Ok(v) => json_resp(req, 200, v),
                    Err(e) => json_resp(req, 500, json!({ "error": e })),
                }
            }
        }
        (Method::Get, "/api/horizon") => json_resp(req, 200, horizon::status(&app)),
        (Method::Get, "/api/running") => json_resp(req, 200, play::status()),
        (Method::Get, "/api/sandbox") => json_resp(req, 200, crate::sandbox::describe(&app)),
        (Method::Post, "/api/kill") => {
            let _ = play::kill();
            json_resp(req, 200, json!({ "ok": true }))
        }
        (Method::Get, "/api/mods") => json_resp(req, 200, mods::list(&app)),
        (Method::Post, "/api/mods") => {
            let body = body_json(&mut req);
            let action = body.get("action").and_then(|a| a.as_str()).unwrap_or_default();
            let result = match action {
                "add" => {
                    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("mod.jar");
                    let data = body.get("data").and_then(|d| d.as_str()).unwrap_or_default();
                    mods::add(&app, name, data).map(|n| json!({ "added": n }))
                }
                "remove" => {
                    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                    mods::remove(&app, name).map(|_| json!({ "ok": true }))
                }
                "toggle" => {
                    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                    mods::toggle(&app, name).map(|en| json!({ "enabled": en }))
                }
                "stage" => {
                    let version = body.get("version").and_then(|v| v.as_str()).unwrap_or_default();
                    mods::stage(&app, version).map(|n| json!({ "staged": n }))
                }
                _ => Err(format!("unknown action {action}")),
            };
            match result {
                Ok(v) => json_resp(req, 200, v),
                Err(e) => json_resp(req, 400, json!({ "error": e })),
            }
        }
        (Method::Get, "/api/skins") => json_resp(req, 200, skin::list(&app)),
        (Method::Post, "/api/skins") => {
            let body = body_json(&mut req);
            let kind = body.get("kind").and_then(|k| k.as_str()).unwrap_or("skin");
            let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("custom");
            let data = body.get("data").and_then(|d| d.as_str()).unwrap_or_default();
            match skin::save_texture(&app, kind, name, data) {
                Ok(id) => json_resp(req, 200, json!({ "ok": true, "id": id })),
                Err(e) => json_resp(req, 400, json!({ "error": e })),
            }
        }
        (Method::Get, "/api/texture") => {
            // serve a stored skin/cape png for the in-UI 3D preview
            let id = q.get("id").cloned().unwrap_or_default();
            match skin::resolve_texture(&app, &id) {
                Some(p) => match std::fs::read(&p) {
                    Ok(bytes) => return respond(req, 200, "image/png", bytes),
                    Err(_) => return json_resp(req, 500, json!({ "error": "read failed" })),
                },
                None => return json_resp(req, 404, json!({ "error": "not found" })),
            }
        }
        (Method::Post, "/api/uninstall") => {
            let body = body_json(&mut req);
            let version = body.get("version").and_then(|v| v.as_str()).unwrap_or_default();
            match install::uninstall(&app, version) {
                Ok(_) => json_resp(req, 200, json!({ "ok": true })),
                Err(e) => json_resp(req, 500, json!({ "error": e })),
            }
        }
        (Method::Post, "/api/open") => {
            let body = body_json(&mut req);
            let which = body.get("path").and_then(|p| p.as_str()).unwrap_or("data");
            let target = match which {
                "mods" => mods::library_dir(&app),
                "game" => app.p("game"),
                "skins" => skin::skins_dir(&app),
                "logs" => app.p("logs"),
                _ => app.dir.clone(),
            };
            let tgt = target.clone();
            std::thread::spawn(move || {
                let target = tgt;
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&target).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("explorer").arg(&target).spawn();
            });
            json_resp(req, 200, json!({ "ok": true, "path": target.to_string_lossy() }))
        }
        _ => json_resp(req, 404, json!({ "error": "not found" })),
    }
}

fn java_runtimes(app: &App) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(app.p("runtime")) {
        for e in rd.flatten() {
            let comp = e.path();
            if !comp.is_dir() {
                continue;
            }
            let name = comp.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Ok(rd2) = std::fs::read_dir(&comp) {
                for p in rd2.flatten() {
                    let exe = crate::java::java_exe(&p.path());
                    if exe.is_file() {
                        out.push(json!({
                            "component": name,
                            "path": exe.to_string_lossy(),
                            "major": crate::java::major_of(&exe),
                        }));
                    }
                }
            }
        }
    }
    out
}

fn installed_versions(app: &App) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(app.p("versions")) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let id = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !p.join(format!("{id}.json")).exists() {
                continue;
            }
            let size: u64 = dir_size(&p);
            out.push(json!({
                "id": id,
                "size": size,
                "channel": mojang::classify(&id),
                "jar": p.join(format!("{id}.jar")).exists(),
            }));
        }
    }
    out.sort_by(|a, b| b["id"].as_str().cmp(&a["id"].as_str()));
    out
}

fn dir_size(p: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                total += if m.is_dir() { dir_size(&e.path()) } else { m.len() };
            }
        }
    }
    total
}

fn total_ram_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(txt) = std::fs::read_to_string("/proc/meminfo") {
            for line in txt.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0);
                    return kb / 1024;
                }
            }
        }
    }
    16384
}
