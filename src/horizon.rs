//! Optional Horizon client for Minecraft 26.3.
//!
//! Horizon (https://github.com/ludovic111/horizon) is an unofficial Rust rewrite
//! of Java Edition 26.3. It contains no game files: Rus downloads the published
//! build, then `horizon-prepare` builds resources from the player's own 26.3
//! install. A button next to Play starts that path instead of the Java client.
use crate::state::App;
use crate::{java, mojang, net, play};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct Job {
    phase: String,
    progress: f64,
    message: String,
    error: Option<String>,
    logs: Vec<String>,
    started: Option<Instant>,
    opened: Option<Instant>,
    log_path: PathBuf,
}

fn job() -> &'static Mutex<Job> {
    static J: std::sync::OnceLock<Mutex<Job>> = std::sync::OnceLock::new();
    J.get_or_init(|| {
        Mutex::new(Job {
            phase: "idle".into(),
            progress: 0.0,
            message: "Horizon is not running".into(),
            error: None,
            logs: Vec::new(),
            started: None,
            opened: None,
            log_path: PathBuf::new(),
        })
    })
}

fn log_line(line: impl Into<String>) {
    let line = line.into();
    if let Ok(mut j) = job().lock() {
        if !line.trim().is_empty() {
            let msg: String = line.trim().chars().take(180).collect();
            j.message = msg.clone();
            j.logs.push(msg);
            if j.logs.len() > 80 {
                let cut = j.logs.len() - 80;
                j.logs.drain(0..cut);
            }
        }
    }
}

fn set_phase(phase: &str, progress: f64, message: &str) {
    if let Ok(mut j) = job().lock() {
        j.phase = phase.into();
        j.progress = progress;
        j.message = message.into();
        j.logs.push(message.into());
    }
}

pub fn status(app: &App) -> Value {
    let j = job().lock().unwrap();
    let client = find_bin(app, "horizon-client");
    let prepared = find_resources(app);
    json!({
        "phase": j.phase,
        "progress": j.progress,
        "message": j.message,
        "error": j.error,
        "logs": j.logs,
        "installed": client.is_some(),
        "prepared": prepared.is_some(),
        "client": client.map(|p| p.to_string_lossy().to_string()),
        "resources": prepared.map(|p| p.to_string_lossy().to_string()),
        "version": "26.3",
        "tag": installed_tag(app),
    })
}

/// Start preparing (if needed) and launch Horizon for 26.3.
/// Returns immediately; progress is on `GET /api/horizon`.
pub fn start(app: Arc<App>) -> Result<Value, String> {
    if !app.is_installed("26.3") {
        return Err("Install Minecraft 26.3 first. Horizon prepares its files from that copy.".into());
    }
    {
        let running = play::status();
        if running.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err("A game is already running.".into());
        }
    }
    {
        let j = job().lock().unwrap();
        if j.phase == "downloading"
            || j.phase == "extracting"
            || j.phase == "preparing"
            || j.phase == "launching"
        {
            return Ok(json!({
                "ok": true,
                "preparing": true,
                "engine": "horizon",
                "version": "26.3",
                "message": j.message,
            }));
        }
    }
    {
        let mut j = job().lock().unwrap();
        j.phase = "downloading".into();
        j.progress = 0.02;
        j.message = "Checking the Horizon release…".into();
        j.error = None;
        j.logs.clear();
        j.started = Some(Instant::now());
        j.opened = None;
    }
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&app)));
        let err = match result {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(_) => Some("Horizon stopped unexpectedly.".into()),
        };
        if let Some(e) = err {
            if let Ok(mut j) = job().lock() {
                j.phase = "error".into();
                j.error = Some(e.clone());
                j.message = e;
            }
        }
    });
    Ok(json!({
        "ok": true,
        "preparing": true,
        "engine": "horizon",
        "version": "26.3",
        "message": "Checking the Horizon release…",
    }))
}

pub struct View {
    pub progress: f32,
    pub message: String,
    pub failed: bool,
    pub busy: bool,
    pub open: bool,
}

/// What the Home line should say. Polled every frame so Play does not sit on "Starting".
pub fn view() -> View {
    let j = job().lock().unwrap();
    let busy = matches!(j.phase.as_str(), "downloading" | "extracting" | "preparing" | "launching");
    View {
        progress: j.progress as f32,
        message: j.message.clone(),
        failed: j.phase == "error",
        busy,
        open: j.phase == "ready",
    }
}

/// The player pressed Stop. Do not treat that as a failed start.
pub fn mark_stopped() {
    if let Ok(mut j) = job().lock() {
        if matches!(j.phase.as_str(), "ready" | "launching" | "error") {
            j.phase = "idle".into();
            j.message = "Stopped".into();
            j.error = None;
        }
    }
}

/// If the client exited right after launch, surface the log instead of leaving Play on Starting.
pub fn settle() {
    let phase = job().lock().ok().map(|j| j.phase.clone()).unwrap_or_default();
    if phase != "ready" {
        return;
    }
    let running = crate::play::status()
        .get("running")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if running {
        return;
    }
    let opened = job().lock().ok().and_then(|j| j.opened);
    let young = opened.map(|t| t.elapsed().as_secs() < 8).unwrap_or(true);
    if !young {
        if let Ok(mut j) = job().lock() {
            if j.phase == "ready" {
                j.phase = "idle".into();
                j.message = "Horizon closed".into();
            }
        }
        return;
    }
    let tail = log_tail();
    let msg = if tail.is_empty() {
        "Horizon closed before the window opened.".into()
    } else {
        format!("Horizon closed. {tail}")
    };
    if let Ok(mut j) = job().lock() {
        if j.phase == "ready" {
            j.phase = "error".into();
            j.error = Some(msg.clone());
            j.message = msg;
        }
    }
}

fn run(app: &App) -> Result<(), String> {
    let (tag, archive_name, url, sha) = latest_asset()?;
    let root = app.p(&format!("horizon/{tag}"));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    if find_bin(app, "horizon-client").is_none() {
        set_phase("downloading", 0.08, &format!("Downloading Horizon {tag}…"));
        let archive = app.p(&format!("cache/{archive_name}"));
        let mut last = 0u64;
        net::download_with(&url, &archive, sha.as_deref(), |n| {
            last += n;
            if let Ok(mut j) = job().lock() {
                j.progress = (0.08 + (last as f64 / (32.0 * 1024.0 * 1024.0)).min(0.35)).min(0.42);
                j.message = format!("Downloading Horizon · {} MB", last / (1024 * 1024));
            }
        })?;
        set_phase("extracting", 0.46, "Extracting Horizon…");
        if root.exists() {
            let _ = fs::remove_dir_all(&root);
        }
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        extract_archive(&archive, &root)?;
        chmod_bins(&root);
    }

    let client = find_bin(app, "horizon-client").ok_or("horizon-client was not in the archive")?;
    let prepare = find_bin(app, "horizon-prepare").ok_or("horizon-prepare was not in the archive")?;
    let release = release_root(&client)?;
    let minecraft = minecraft_dir(app)?;

    let resources_dir = release.join(".local/horizon-client/26.3");
    if !resources_dir.join("horizon-resources.json").is_file() {
        set_phase("preparing", 0.55, "Downloading the 26.3 server jar…");
        let server_jar = ensure_server_jar(app)?;
        let java_exe = java::resolve(app, "java-runtime-epsilon", 25)
            .or_else(|| java::resolve(app, "java-runtime-delta", 21))
            .ok_or("Java 25 was not found. Install 26.3 first, or set a Java path in Settings.")?;
        let java_home = java_exe
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| java_exe.clone());
        set_phase("preparing", 0.62, "Preparing Horizon from your 26.3 install…");
        let server_jar_s = server_jar.to_string_lossy().to_string();
        let minecraft_s = minecraft.to_string_lossy().to_string();
        let release_s = release.to_string_lossy().to_string();
        run_tool(
            &prepare,
            &release,
            &java_home,
            &[
                "--prepare",
                "--server-jar",
                &server_jar_s,
                "--minecraft-dir",
                &minecraft_s,
                "--root",
                &release_s,
            ],
        )?;
        set_phase("preparing", 0.82, "Copying sounds…");
        if let Err(e) = run_tool(
            &prepare,
            &release,
            &java_home,
            &["sounds", "--minecraft-dir", &minecraft_s, "--root", &release_s],
        ) {
            log_line(format!("sounds skipped: {e}"));
        }
    }

    let resources = if resources_dir.join("horizon-resources.json").is_file() {
        resources_dir
    } else {
        find_resources(app).ok_or("Horizon prepared no resources. Check the log and try again.")?
    };
    set_phase("launching", 0.94, "Opening Horizon…");
    let settings = app.settings();
    let home = app.p("horizon/home");
    fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    prefer_dx12(&home);
    let mut cmd = Command::new(&client);
    cmd.current_dir(&home);
    cmd.env("HORIZON_HOME", &home);
    cmd.env("HOME", &home);
    cmd.env("USERPROFILE", &home);
    cmd.env("APPDATA", home.join("AppData/Roaming"));
    cmd.env("LOCALAPPDATA", home.join("AppData/Local"));
    cmd.env("HORIZON_PLAYER_NAME", &settings.username);
    // Vulkan leaves the first screen frozen on some Windows GPUs. DirectX 12 gets past it.
    #[cfg(windows)]
    cmd.env("WGPU_BACKEND", "dx12");
    cmd.arg("--resources")
        .arg(&resources)
        .arg("--name")
        .arg(&settings.username)
        .arg("--width")
        .arg(settings.width.to_string())
        .arg("--height")
        .arg(settings.height.to_string());
    let log_path = app.p("logs/horizon-26.3.log");
    if let Ok(mut j) = job().lock() {
        j.log_path = log_path.clone();
    }
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &log_path,
        format!(
            "engine: horizon\nclient: {}\nresources: {}\nhome: {}\n\n",
            client.display(),
            resources.display(),
            home.display()
        ),
    );
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Could not open the Horizon log: {e}"))?;
    let err = log.try_clone().map_err(|e| format!("Could not open the Horizon log: {e}"))?;
    cmd.stdout(Stdio::from(log)).stderr(Stdio::from(err)).stdin(Stdio::null());
    crate::play::hide_console(&mut cmd);
    let child = cmd.spawn().map_err(|e| format!("Failed to start Horizon: {e}"))?;
    let pid = child.id();
    play::adopt("26.3", "horizon", child);
    if let Ok(mut j) = job().lock() {
        j.phase = "ready".into();
        j.progress = 1.0;
        j.opened = Some(Instant::now());
        j.message = format!("Horizon is open · pid {pid}");
        j.error = None;
    }
    Ok(())
}

fn latest_asset() -> Result<(String, String, String, Option<String>), String> {
    let name = asset_name()?;
    let v = net::get_json("https://api.github.com/repos/ludovic111/horizon/releases/latest")?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("latest")
        .to_string();
    let assets = v.get("assets").and_then(|a| a.as_array()).cloned().unwrap_or_default();
    let url = assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name))
        .and_then(|a| a.get("browser_download_url").and_then(|u| u.as_str()))
        .ok_or_else(|| format!("release {tag} has no {name}"))?
        .to_string();
    let sums_url = assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some("SHA256SUMS"))
        .and_then(|a| a.get("browser_download_url").and_then(|u| u.as_str()));
    let sha = if let Some(u) = sums_url {
        net::get_bytes(u).ok().and_then(|b| {
            let text = String::from_utf8_lossy(&b);
            sha_for(&text, name)
        })
    } else {
        None
    };
    Ok((tag, name.to_string(), url, sha))
}

fn asset_name() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("horizon-linux-x86_64.tar.gz"),
        ("windows", "x86_64") => Ok("horizon-windows-x64.zip"),
        ("macos", "aarch64") => Ok("horizon-macos-arm64.zip"),
        (os, arch) => Err(format!("Horizon has no published build for {os}/{arch}")),
    }
}

fn sha_for(sums: &str, name: &str) -> Option<String> {
    for line in sums.lines() {
        let mut it = line.split_whitespace();
        let hash = it.next()?.to_string();
        let file = it.next()?.trim_start_matches('*');
        if file == name || file.ends_with(&format!("/{name}")) {
            return Some(hash);
        }
    }
    None
}

fn ensure_server_jar(app: &App) -> Result<PathBuf, String> {
    let dest = app.p("cache/26.3-server.jar");
    let meta = mojang::version_meta(app, "26.3")?;
    let dl = meta
        .get("downloads")
        .and_then(|d| d.get("server"))
        .ok_or("26.3 has no server jar in its version json")?;
    let url = dl.get("url").and_then(|u| u.as_str()).ok_or("server jar url missing")?;
    let sha = dl.get("sha1").and_then(|s| s.as_str());
    if net::verify(&dest, sha) {
        return Ok(dest);
    }
    net::download(url, &dest, sha, |_| {})?;
    Ok(dest)
}

fn run_tool(bin: &Path, cwd: &Path, java_home: &Path, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new(bin);
    cmd.current_dir(cwd).args(args);
    cmd.env("JAVA_HOME", java_home);
    let bin_dir = java_home.join("bin");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir];
    paths.extend(std::env::split_paths(&path));
    cmd.env("PATH", std::env::join_paths(paths).unwrap_or(path));
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", bin.display()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let t1 = stdout.map(|p| {
        std::thread::spawn(move || {
            for line in BufReader::new(p).lines().map_while(Result::ok) {
                log_line(line);
            }
        })
    });
    let t2 = stderr.map(|p| {
        std::thread::spawn(move || {
            for line in BufReader::new(p).lines().map_while(Result::ok) {
                log_line(line);
            }
        })
    });
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Some(t) = t1 {
        let _ = t.join();
    }
    if let Some(t) = t2 {
        let _ = t.join();
    }
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with {status}", bin.file_name().unwrap_or_default().to_string_lossy()))
    }
}

fn extract_archive(archive: &Path, dest: &Path) -> Result<(), String> {
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    if name.ends_with(".zip") {
        let f = fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        z.extract(dest).map_err(|e| e.to_string())?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let f = fs::File::open(archive).map_err(|e| e.to_string())?;
        let gz = flate2::read::GzDecoder::new(f);
        let mut ar = tar::Archive::new(gz);
        ar.unpack(dest).map_err(|e| e.to_string())?;
    } else {
        return Err(format!("unsupported archive {name}"));
    }
    Ok(())
}

fn chmod_bins(root: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["horizon-client", "horizon-prepare", "horizon-ctl", "horizon-server"] {
            if let Some(p) = walk_bin(root, name) {
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
            }
        }
    }
}

fn find_bin(app: &App, name: &str) -> Option<PathBuf> {
    let root = app.p("horizon");
    if !root.is_dir() {
        return None;
    }
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    walk_bin(&root, &exe)
}

fn walk_bin(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0;
    while let Some(d) = stack.pop() {
        seen += 1;
        if seen > 4000 {
            break;
        }
        let Ok(rd) = fs::read_dir(&d) else { continue; };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(p);
            }
        }
    }
    None
}

fn find_resources(app: &App) -> Option<PathBuf> {
    let root = app.p("horizon");
    if !root.is_dir() {
        return None;
    }
    let mut stack = vec![root];
    let mut seen = 0;
    while let Some(d) = stack.pop() {
        seen += 1;
        if seen > 4000 {
            break;
        }
        let marker = d.join("horizon-resources.json");
        if marker.is_file() {
            return Some(d);
        }
        if let Ok(rd) = fs::read_dir(&d) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    stack.push(e.path());
                }
            }
        }
    }
    None
}

fn release_root(client: &Path) -> Result<PathBuf, String> {
    let mut cur = client.parent().map(|p| p.to_path_buf());
    while let Some(dir) = cur {
        if dir.join("research-lock.json").is_file() {
            return Ok(dir);
        }
        cur = dir.parent().map(|p| p.to_path_buf());
    }
    Err("The Horizon download has no research-lock.json. Delete the horizon folder and try Play again.".into())
}

fn minecraft_dir(app: &App) -> Result<PathBuf, String> {
    let json = app.p("versions/26.3/26.3.json");
    let jar = app.p("versions/26.3/26.3.jar");
    if json.is_file() && jar.is_file() {
        return Ok(app.dir.clone());
    }
    let lib = crate::instances::library_dir(app, "26.3");
    if lib.join("26.3.json").is_file() && lib.join("26.3.jar").is_file() {
        let dest = app.p("versions/26.3");
        fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        if !json.is_file() {
            fs::copy(lib.join("26.3.json"), &json).map_err(|e| e.to_string())?;
        }
        if !jar.is_file() && fs::hard_link(lib.join("26.3.jar"), &jar).is_err() {
            fs::copy(lib.join("26.3.jar"), &jar).map_err(|e| e.to_string())?;
        }
        return Ok(app.dir.clone());
    }
    Err("Install Minecraft 26.3 first. Horizon prepares its files from that copy.".into())
}

fn prefer_dx12(home: &Path) {
    #[cfg(windows)]
    {
        let path = home.join("horizon-graphics.txt");
        let text = fs::read_to_string(&path).unwrap_or_default();
        if text.lines().any(|l| l.trim().starts_with("graphicsApi:") && !l.contains("auto")) {
            return;
        }
        let mut lines: Vec<String> = text
            .lines()
            .filter(|l| !l.trim().starts_with("graphicsApi:"))
            .map(|l| l.to_string())
            .collect();
        lines.push("graphicsApi:dx12".into());
        let _ = fs::write(&path, lines.join("\n") + "\n");
    }
    #[cfg(not(windows))]
    {
        let _ = home;
    }
}

fn log_tail() -> String {
    let path = job().lock().ok().map(|j| j.log_path.clone()).unwrap_or_default();
    let bytes = fs::read(&path).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let line = text
        .lines()
        .rev()
        .find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("engine:") && !t.starts_with("client:") && !t.starts_with("resources:") && !t.starts_with("home:")
        })
        .unwrap_or("")
        .trim();
    line.chars().take(160).collect()
}

fn installed_tag(app: &App) -> String {
    let root = app.p("horizon");
    let mut tags = Vec::new();
    if let Ok(rd) = fs::read_dir(&root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                tags.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    tags.sort();
    tags.pop().unwrap_or_default()
}

