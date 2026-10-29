//! Building the java command line and launching the game (offline account).
use crate::install::{os_name, rules_ok, LibPlan};
use crate::state::{offline_uuid, App};
use crate::{java, mojang};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};

pub struct Running {
    pub version: String,
    pub engine: String,
    pub child: Child,
    pub started: u128,
}

/// Register a process started outside `launch` (Horizon).
pub fn adopt(version: &str, engine: &str, child: Child) {
    *running().lock().unwrap() = Some(Running {
        version: version.to_string(),
        engine: engine.to_string(),
        child,
        started: crate::state::now_ms(),
    });
}

pub fn running() -> &'static Mutex<Option<Running>> {
    static R: OnceLock<Mutex<Option<Running>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(None))
}

fn subst(arg: &str, vars: &HashMap<String, String>) -> String {
    let mut out = arg.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

/// JSON jvm args that would clash with the classpath we inject ourselves.
fn is_classpath_arg(raw: &str) -> bool {
    let t = raw.trim();
    t == "-cp"
        || t == "-classpath"
        || t == "--class-path"
        || t.contains("${classpath}")
        || t.contains("${game_assets}")
}

fn parse_legacy_args(s: &str, vars: &HashMap<String, String>) -> Vec<String> {
    s.split_whitespace().map(|a| subst(a, vars)).collect()
}

pub fn build_command(app: &App, id: &str) -> Result<(Command, PathBuf), String> {
    build_command_join(app, id, None)
}

pub fn build_command_join(app: &App, id: &str, server: Option<&str>) -> Result<(Command, PathBuf), String> {
    let dir = app.game_dir(id);
    build_command_at(app, id, &dir, server)
}

pub fn build_command_at(app: &App, id: &str, game_dir: &std::path::Path, server: Option<&str>) -> Result<(Command, PathBuf), String> {
    let settings = app.settings();
    let library = crate::instances::library_dir(app, id);
    let version_dir = if library.join(format!("{id}.json")).exists() && library.join(format!("{id}.jar")).exists() {
        library
    } else {
        app.p(&format!("versions/{id}"))
    };
    if !version_dir.join(format!("{id}.json")).exists() {
        return Err(format!("{id} is not installed"));
    }
    let meta_txt = fs::read_to_string(version_dir.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
    let meta: Value = serde_json::from_str(&meta_txt).map_err(|e| e.to_string())?;

    let plan: LibPlan = crate::install::plan_libraries(app, id, &meta);
    let client_jar = version_dir.join(format!("{id}.jar"));
    let mut classpath: Vec<String> = plan.classpath.iter().map(|p| p.to_string_lossy().to_string()).collect();
    classpath.push(client_jar.to_string_lossy().to_string());
    let classpath_str = classpath.join(if cfg!(target_os = "windows") { ";" } else { ":" });

    let natives_dir = version_dir.join("natives");
    let _ = fs::create_dir_all(&natives_dir);
    crate::install::extract_natives(&std::sync::Arc::new(Mutex::new(crate::state::Job::new(
        "natives".into(),
        id.into(),
        vec![],
    ))), &plan.natives);

    let assets_root = app.p("assets");
    let asset_index = meta
        .get("assetIndex")
        .and_then(|a| a.get("id"))
        .and_then(|i| i.as_str())
        .unwrap_or("legacy")
        .to_string();
    fs::create_dir_all(game_dir).map_err(|e| e.to_string())?;

    let component = meta
        .get("javaVersion")
        .and_then(|j| j.get("component"))
        .and_then(|c| c.as_str())
        .unwrap_or("java-runtime-delta");
    let major = meta
        .get("javaVersion")
        .and_then(|j| j.get("majorVersion"))
        .and_then(|m| m.as_u64())
        .unwrap_or(0) as u32;
    let javaw = java::gui_exe(&java::resolve(app, component, major).ok_or_else(|| {
        format!("Java {major} was not found. Reinstall the version or set a Java path in Settings")
    })?);

    let uuid = offline_uuid(&settings.username);
    let mut vars: HashMap<String, String> = HashMap::new();
    let add = |m: &mut HashMap<String, String>, k: &str, v: &str| {
        m.insert(k.to_string(), v.to_string());
    };
    add(&mut vars, "auth_player_name", &settings.username);
    add(&mut vars, "version_name", id);
    add(&mut vars, "game_directory", &game_dir.to_string_lossy());
    add(&mut vars, "assets_root", &assets_root.to_string_lossy());
    add(&mut vars, "game_assets", &assets_root.join("virtual/legacy").to_string_lossy());
    add(&mut vars, "assets_index_name", &asset_index);
    add(&mut vars, "auth_uuid", &uuid);
    add(&mut vars, "auth_access_token", "0");
    add(&mut vars, "auth_session", "0");
    add(
        &mut vars,
        "clientid",
        "launcher-local",
    );
    add(&mut vars, "auth_xuid", "0");
    add(&mut vars, "user_type", "legacy");
    add(&mut vars, "user_properties", "{}");
    add(&mut vars, "version_type", "release");
    add(&mut vars, "launcher_name", "launcher");
    add(&mut vars, "launcher_version", crate::product::VERSION);
    add(&mut vars, "classpath", &classpath_str);
    add(&mut vars, "natives_directory", &natives_dir.to_string_lossy());
    add(&mut vars, "resolution_width", &settings.width.to_string());
    add(&mut vars, "resolution_height", &settings.height.to_string());

    let mut feats: HashMap<String, bool> = HashMap::new();
    feats.insert("is_demo_user".into(), settings.demo);
    feats.insert("has_custom_resolution".into(), settings.width > 0 && settings.height > 0);

    let (bin, prefix, seal) = crate::sandbox::prepare(&javaw, app);
    let mut cmd = Command::new(&bin);
    for arg in prefix {
        cmd.arg(arg);
    }
    crate::sandbox::apply_env(&mut cmd, app);
    // memory + user jvm args
    cmd.arg(format!("-Xmx{}M", settings.ram_mb.max(512)));
    cmd.arg(format!("-Xms{}M", (settings.ram_mb / 2).max(256)));
    let skin_path = crate::skin::resolve_texture(app, &settings.skin);
    let cape_path = crate::skin::resolve_texture(app, &settings.cape);
    if skin_path.is_some() || cape_path.is_some() {
        let agent = crate::skin::stage_agent(app);
        let log = app.p("logs/rus-skin.log");
        if let Some(parent) = log.parent() {
            let _ = fs::create_dir_all(parent);
        }
        cmd.arg(format!("-javaagent:{}", agent.display()));
        cmd.arg(format!("-Drus.log={}", log.display()));
        cmd.arg(format!("-Drus.uuid={uuid}"));
        cmd.arg(format!("-Drus.name={}", settings.username));
        cmd.arg(format!("-Drus.model={}", crate::skin::model_flag(app, &settings.skin, &settings.skin_model)));
        if let Some(path) = &skin_path {
            cmd.arg(format!("-Drus.skin={}", path.display()));
        }
        if let Some(path) = &cape_path {
            cmd.arg(format!("-Drus.cape={}", path.display()));
        }
    }
    for a in settings.jvm_args.split_whitespace() {
        cmd.arg(a);
    }
    cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
    for a in [
        "-Dminecraft.launcher.brand=launcher",
        "-Dminecraft.launcher.version=1.0.0",
        "-Dfile.encoding=UTF-8",
        "-Dorg.lwjgl.system.SharedLibraryExtractPath=",
    ] {
        if a.ends_with('=') {
            continue;
        }
        cmd.arg(a);
    }

    // jvm args from the version json (new format)
    if let Some(args) = meta.get("arguments").and_then(|a| a.get("jvm")).and_then(|j| j.as_array()) {
        for a in args {
            match a {
                Value::String(raw) => {
                    if !is_classpath_arg(raw) {
                        cmd.arg(subst(raw, &vars));
                    }
                }
                Value::Object(_) => {
                    if !rules_ok(a, &feats) {
                        continue;
                    }
                    let Some(vals) = a.get("value") else { continue };
                    let list: Vec<String> = match vals {
                        Value::String(s) => vec![s.clone()],
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect(),
                        _ => vec![],
                    };
                    for raw in list {
                        if !is_classpath_arg(&raw) {
                            cmd.arg(subst(&raw, &vars));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    cmd.arg("-cp").arg(&classpath_str);
    if os_name() == "osx" {
        cmd.arg("-XstartOnFirstThread");
    }

    let main_class = meta
        .get("mainClass")
        .and_then(|m| m.as_str())
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();
    cmd.arg(main_class);

    // game args
    if let Some(args) = meta.get("arguments").and_then(|a| a.get("game")).and_then(|g| g.as_array()) {
        for a in args {
            match a {
                Value::String(raw) => {
                    if !raw.contains("${classpath}") {
                        cmd.arg(subst(raw, &vars));
                    }
                }
                Value::Object(_) => {
                    if rules_ok(a, &feats) {
                        if let Some(vals) = a.get("value") {
                            match vals {
                                Value::String(s) => {
                                    cmd.arg(subst(s, &vars));
                                }
                                Value::Array(arr) => {
                                    for v in arr.iter().filter_map(|v| v.as_str()) {
                                        cmd.arg(subst(v, &vars));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    } else if let Some(legacy) = meta.get("minecraftArguments").and_then(|m| m.as_str()) {
        for a in parse_legacy_args(legacy, &vars) {
            cmd.arg(a);
        }
    }
    if settings.fullscreen {
        cmd.arg("--fullscreen");
    }
    if let Some(addr) = server.map(str::trim).filter(|s| !s.is_empty()) {
        let (host, port) = split_server(addr);
        cmd.arg("--server").arg(host);
        if let Some(port) = port {
            cmd.arg("--port").arg(port);
        }
    }

    let log_path = app.p(&format!("logs/{id}.log"));
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let argv: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    let _ = fs::write(
        &log_path,
        format!(
            "launcher: {id}\nsandbox: {}\njava: {}\ncmd: {} {}\n\n",
            seal.label,
            javaw.display(),
            javaw.display(),
            argv.join(" ")
        ),
    );
    match fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => {
            let f2 = f.try_clone().ok();
            cmd.stdout(Stdio::from(f));
            match f2 {
                Some(err) => {
                    cmd.stderr(Stdio::from(err));
                }
                None => {
                    cmd.stderr(Stdio::null());
                }
            }
        }
        Err(_) => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    hide_console(&mut cmd);
    cmd.stdin(Stdio::null());
    Ok((cmd, javaw))
}

pub fn hide_console(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW — do not open a console for this process.
        cmd.creation_flags(0x08000000);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = cmd;
    }
}

fn split_server(addr: &str) -> (String, Option<String>) {
    if let Some((host, port)) = addr.rsplit_once(':') {
        if !host.is_empty() && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return (host.to_string(), Some(port.to_string()));
        }
    }
    (addr.to_string(), None)
}

pub fn launch(app: &App, id: &str) -> Result<serde_json::Value, String> {
    launch_join(app, id, None)
}

pub fn launch_join(app: &App, id: &str, server: Option<&str>) -> Result<serde_json::Value, String> {
    let dir = app.game_dir(id);
    launch_at(app, id, &dir, server)
}

pub fn launch_at(app: &App, id: &str, game_dir: &std::path::Path, server: Option<&str>) -> Result<serde_json::Value, String> {
    {
        let mut r = running().lock().unwrap();
        if let Some(proc) = r.as_mut() {
            match proc.child.try_wait() {
                Ok(None) => return Err(format!("A game is already running ({})", proc.version)),
                _ => *r = None,
            }
        }
    }
    // re-stage mods. Skin and cape go on through rus-skin.jar, not a resource pack.
    let gdir = game_dir.to_path_buf();
    let _ = crate::mods::stage_into(app, &gdir);
    crate::skin::retire_pack(&gdir);

    let (mut cmd, javaw) = build_command_at(app, id, &gdir, server)?;
    let log_path = app.p(&format!("logs/{id}.log"));
    let child = cmd.spawn().map_err(|e| format!("Failed to start Java: {e}"))?;
    crate::sandbox::attach(&child);
    let pid = child.id();
    *running().lock().unwrap() = Some(Running {
        version: id.to_string(),
        engine: "vanilla".into(),
        child,
        started: crate::state::now_ms(),
    });
    let _ = mojang::classify(id);
    Ok(serde_json::json!({
        "ok": true,
        "version": id,
        "engine": "vanilla",
        "pid": pid,
        "java": javaw.to_string_lossy(),
        "log": log_path.to_string_lossy(),
    }))
}

fn note_exit(version: String, started: u128) {
    let elapsed = crate::state::now_ms().saturating_sub(started);
    if elapsed > 0 {
        finished().lock().unwrap().push((version, elapsed));
    }
}

fn finished() -> &'static Mutex<Vec<(String, u128)>> {
    static FINISHED: OnceLock<Mutex<Vec<(String, u128)>>> = OnceLock::new();
    FINISHED.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn drain_finished() -> Vec<(String, u128)> {
    std::mem::take(&mut *finished().lock().unwrap())
}

pub fn live_ms_for(version: &str) -> Option<u128> {
    let r = running().lock().unwrap();
    let p = r.as_ref()?;
    if p.version != version {
        return None;
    }
    Some(crate::state::now_ms().saturating_sub(p.started))
}

pub fn status() -> serde_json::Value {
    let mut r = running().lock().unwrap();
    if let Some(p) = r.as_mut() {
        match p.child.try_wait() {
            Ok(None) => {
                return serde_json::json!({
                    "running": true,
                    "version": p.version,
                    "engine": p.engine,
                    "pid": p.child.id(),
                    "elapsed": (crate::state::now_ms().saturating_sub(p.started)) as f64 / 1000.0,
                })
            }
            _ => {
                let version = p.version.clone();
                let started = p.started;
                *r = None;
                note_exit(version, started);
            }
        }
    }
    serde_json::json!({ "running": false })
}

pub fn kill() -> Result<(), String> {
    crate::sandbox::terminate();
    let mut r = running().lock().unwrap();
    if let Some(p) = r.as_mut() {
        let _ = p.child.kill();
        note_exit(p.version.clone(), p.started);
    }
    *r = None;
    Ok(())
}
