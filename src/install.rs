//! Install pipeline: vanilla core → assets → java runtime → skin pack → mods.
//! Every step is a visible "patch" so the user can watch (and toggle) each one.
use crate::state::{App, Job, PatchState};
use crate::{java, mojang, net, skin};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn patch_defs() -> Vec<PatchState> {
    vec![
        PatchState {
            key: "vanilla".into(),
            title: "Vanilla Core".into(),
            detail: "version json + libraries + client jar".into(),
            enabled: true,
            status: "pending".into(),
            ..Default::default()
        },
        PatchState {
            key: "assets".into(),
            title: "Assets".into(),
            detail: "sounds, languages, textures & icons".into(),
            enabled: true,
            status: "pending".into(),
            ..Default::default()
        },
        PatchState {
            key: "java".into(),
            title: "Java Runtime".into(),
            detail: "Temurin JRE for this version".into(),
            enabled: true,
            status: "pending".into(),
            ..Default::default()
        },
        PatchState {
            key: "skin".into(),
            title: "Skin & Cape".into(),
            detail: "local skin pack for your profile".into(),
            enabled: true,
            status: "pending".into(),
            ..Default::default()
        },
        PatchState {
            key: "mods".into(),
            title: "Mods".into(),
            detail: "stage jars from the Mods tab".into(),
            enabled: true,
            status: "pending".into(),
            ..Default::default()
        },
    ]
}

pub fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "osx"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn features() -> HashMap<String, bool> {
    let mut m = HashMap::new();
    for f in [
        "is_demo_user",
        "has_custom_resolution",
        "has_quick_plays_support",
        "is_quick_play_singleplayer",
        "is_quick_play_multiplayer",
        "is_quick_play_realms",
    ] {
        m.insert(f.to_string(), false);
    }
    m
}

/// Evaluate library / argument `rules` for the current OS.
pub fn rules_ok(entry: &Value, feats: &HashMap<String, bool>) -> bool {
    let rules = match entry.get("rules").and_then(|r| r.as_array()) {
        Some(r) if !r.is_empty() => r,
        _ => return true,
    };
    let mut allowed = false;
    for rule in rules {
        let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("allow");
        let mut matches = true;
        if let Some(os) = rule.get("os") {
            if let Some(n) = os.get("name").and_then(|v| v.as_str()) {
                matches &= n == os_name();
            }
            if let Some(a) = os.get("arch").and_then(|v| v.as_str()) {
                let arch = std::env::consts::ARCH;
                matches &= a.contains(arch) || (a == "x86" && arch == "x86");
            }
        }
        if let Some(f) = rule.get("features").and_then(|v| v.as_object()) {
            for (k, want) in f {
                let have = feats.get(k).copied().unwrap_or(false);
                matches &= want.as_bool().unwrap_or(false) == have;
            }
        }
        if matches {
            allowed = action == "allow";
        }
    }
    allowed
}

struct Progress {
    job: Arc<Mutex<Job>>,
    value: f64,
}

impl Progress {
    fn new(job: Arc<Mutex<Job>>) -> Self {
        Self { job, value: 0.0 }
    }
    fn set(&mut self, p: f64, msg: &str) {
        let p = p.clamp(0.0, 1.0);
        if p > self.value {
            self.value = p;
        }
        let mut j = self.job.lock().unwrap();
        j.progress = self.value;
        j.message = msg.to_string();
    }
    fn tick(&mut self) {
        if let Ok(j) = self.job.lock() {
            if j.canceled() {
                drop(j);
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

fn canceled(job: &Arc<Mutex<Job>>) -> bool {
    job.lock().map(|j| j.canceled()).unwrap_or(false)
}

fn log(job: &Arc<Mutex<Job>>, line: impl Into<String>) {
    if let Ok(mut j) = job.lock() {
        j.log(line);
    }
}

/// Downloads a batch of files with a small worker pool and aggregated progress.
fn download_batch(
    job: Arc<Mutex<Job>>,
    patch: &str,
    items: Vec<(String, Option<String>)>, // (url, sha1)
    dests: Vec<PathBuf>,
    mut on_progress: impl FnMut(f64, u64, u64) + Send,
) -> Result<(usize, u64), String> {
    let total = items.len();
    let cursor = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let bytes = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(Mutex::new(Vec::<String>::new()));
    let dests = Arc::new(dests);
    let items = Arc::new(items);
    let threads = 8usize.min(total.max(1));
    let mut handles = Vec::new();
    for _ in 0..threads {
        let job = job.clone();
        let cursor = cursor.clone();
        let done = done.clone();
        let bytes = bytes.clone();
        let failed = failed.clone();
        let items = items.clone();
        let dests = dests.clone();
        let _patch = patch.to_string();
        handles.push(std::thread::spawn(move || loop {
            if canceled(&job) {
                return;
            }
            let i = cursor.fetch_add(1, Ordering::SeqCst);
            if i >= items.len() {
                return;
            }
            let (url, sha) = &items[i];
            let dest = &dests[i];
            if let Err(e) = net::download(url, dest, sha.as_deref(), |n| {
                bytes.fetch_add(n, Ordering::Relaxed);
            }) {
                failed.lock().unwrap().push(e);
            }
            done.fetch_add(1, Ordering::SeqCst);
        }));
    }
    while handles.iter().any(|h| !h.is_finished()) {
        let d = done.load(Ordering::SeqCst);
        let b = bytes.load(Ordering::Relaxed);
        on_progress(d as f64 / total.max(1) as f64, b, b);
        if let Ok(mut j) = job.lock() {
            if let Some(p) = j.patch(patch) {
                p.bytes = b;
            }
        }
        if canceled(&job) {
            break;
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    for h in handles {
        let _ = h.join();
    }
    let errs = failed.lock().unwrap().clone();
    if !errs.is_empty() {
        for e in errs.iter().take(6) {
            log(&job, format!("download failed: {e}"));
        }
        return Err(format!("{} downloads failed — {}", errs.len(), errs[0]));
    }
    Ok((done.load(Ordering::SeqCst), bytes.load(Ordering::Relaxed)))
}

fn asset_dest(app: &App, hash: &str) -> PathBuf {
    let prefix = &hash[0..2];
    app.p(&format!("assets/objects/{prefix}/{hash}"))
}

/// Resolve libraries + natives for a version, applying OS rules.
pub struct LibPlan {
    pub classpath: Vec<PathBuf>,
    pub downloads: Vec<(String, Option<String>)>,
    pub dests: Vec<PathBuf>,
    pub natives: Vec<(PathBuf, PathBuf)>, // (jar, extract dir)
}

pub fn plan_libraries(app: &App, id: &str, meta: &Value) -> LibPlan {
    let lib_root = app.p("libraries");
    let natives_root = app.p(&format!("versions/{id}/natives"));
    let feats = features();
    let mut classpath = Vec::new();
    let mut downloads = Vec::new();
    let mut dests = Vec::new();
    let mut natives = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for lib in meta
        .get("libraries")
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default()
    {
        if !rules_ok(&lib, &feats) {
            continue;
        }
        let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        if let Some(dl) = lib.get("downloads") {
            if let Some(art) = dl.get("artifact") {
                let path = art.get("path").and_then(|p| p.as_str()).unwrap_or_default().to_string();
                if !path.is_empty() && seen.insert(path.clone()) {
                    let dest = lib_root.join(&path);
                    let url = art
                        .get("url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("https://libraries.minecraft.net/{path}"));
                    let sha = art.get("sha1").and_then(|s| s.as_str()).map(|s| s.to_string());
                    if !net::verify(&dest, sha.as_deref()) {
                        downloads.push((url, sha));
                        dests.push(dest.clone());
                    }
                    classpath.push(dest);
                }
            }
            // legacy natives (1.8 … 1.13 style)
            if let Some(natives_map) = lib.get("natives").and_then(|n| n.as_object()) {
                if let Some(classifier_key) = natives_map
                    .get(os_name())
                    .and_then(|c| c.as_str())
                    .map(|c| c.replace("${arch}", if cfg!(target_arch = "x86") { "32" } else { "64" }))
                {
                    if let Some(cl) = dl.get("classifiers").and_then(|c| c.get(&classifier_key)) {
                        let path = cl.get("path").and_then(|p| p.as_str()).unwrap_or_default();
                        if !path.is_empty() && seen.insert(path.to_string()) {
                            let dest = lib_root.join(path);
                            let url = cl
                                .get("url")
                                .and_then(|u| u.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| format!("https://libraries.minecraft.net/{path}"));
                            let sha = cl.get("sha1").and_then(|s| s.as_str()).map(|s| s.to_string());
                            if !net::verify(&dest, sha.as_deref()) {
                                downloads.push((url, sha));
                                dests.push(dest.clone());
                            }
                            let short = name.rsplit(':').next().unwrap_or("native").to_string();
                            natives.push((dest, natives_root.join(short)));
                        }
                    }
                }
            }
        } else if let Some(base) = lib.get("url").and_then(|u| u.as_str()) {
            // maven-only library entry (very old versions)
            let parts: Vec<&str> = name.split(':').collect();
            if parts.len() >= 3 {
                let (g, a, v) = (parts[0], parts[1], parts[2]);
                let path = format!(
                    "{}/{}/{}/{}-{}.jar",
                    g.replace('.', "/"),
                    a,
                    v,
                    a,
                    v
                );
                if seen.insert(path.clone()) {
                    let dest = lib_root.join(&path);
                    let base = base.trim_end_matches('/');
                    if !net::verify(&dest, None) {
                        downloads.push((format!("{base}/{path}"), None));
                        dests.push(dest.clone());
                    }
                    classpath.push(dest);
                }
            }
        }
    }
    LibPlan {
        classpath,
        downloads,
        dests,
        natives,
    }
}

pub fn extract_natives(job: &Arc<Mutex<Job>>, natives: &[(PathBuf, PathBuf)]) {
    for (jar, dir) in natives {
        let _ = fs::create_dir_all(dir);
        let f = match fs::File::open(jar) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if let Ok(mut zip) = zip::ZipArchive::new(f) {
            for i in 0..zip.len() {
                if let Ok(mut e) = zip.by_index(i) {
                    let name = e.name().to_string();
                    if e.is_dir() || name.contains("META-INF") || !name.ends_with(".so") && !name.ends_with(".dll") && !name.ends_with(".dylib") && !name.ends_with(".jnilib") {
                        continue;
                    }
                    let target = match Path::new(&name).file_name() {
                        Some(n) => dir.join(n),
                        None => continue,
                    };
                    if let Ok(mut out) = fs::File::create(&target) {
                        let _ = std::io::copy(&mut e, &mut out);
                    }
                }
            }
        }
        log(job, format!("natives extracted → {}", dir.display()));
    }
}

/// The whole install. Returns Ok(()) when everything requested finished.
pub fn run(app: Arc<App>, job: Arc<Mutex<Job>>) {
    if let Err(e) = run_inner(app.clone(), job.clone()) {
        if let Ok(mut j) = job.lock() {
            if j.canceled() {
                j.state = "canceled".into();
                j.message = "Install canceled".into();
            } else {
                j.state = "error".into();
                j.error = Some(e.clone());
                j.message = format!("Error: {e}");
                j.log(format!("ERROR: {e}"));
            }
        }
        return;
    }
    if let Ok(mut j) = job.lock() {
        if j.canceled() {
            j.state = "canceled".into();
            j.message = "Install canceled".into();
            return;
        }
        j.state = "done".into();
        j.progress = 1.0;
        j.message = "Install complete".into();
        j.log("done");
    }
}

fn run_inner(app: Arc<App>, job: Arc<Mutex<Job>>) -> Result<(), String> {
    let id = job.lock().unwrap().version.clone();
    let enabled: HashMap<String, bool> = job
        .lock()
        .unwrap()
        .patches
        .iter()
        .map(|p| (p.key.clone(), p.enabled))
        .collect();
    let want = |k: &str| enabled.get(k).copied().unwrap_or(true);
    let mut prog = Progress::new(job.clone());
    let version_dir = app.p(&format!("versions/{id}"));
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    prog.set(0.01, "Fetching version metadata…");
    log(&job, format!("installing {id}"));
    let meta = mojang::version_meta(&app, &id)?;
    let meta_txt = serde_json::to_string_pretty(&meta).unwrap_or_default();
    fs::write(version_dir.join(format!("{id}.json")), &meta_txt).map_err(|e| e.to_string())?;
    prog.set(0.03, "Version identified");

    // ---------- 1. vanilla core: libraries + client jar ----------
    if want("vanilla") {
        if let Ok(mut j) = job.lock() {
            j.set_patch("vanilla", "running", "libraries…");
        }
        let plan = plan_libraries(&app, &id, &meta);
        let lib_count = plan.downloads.len();
        log(&job, format!("libraries: {} files, {} to fetch", plan.classpath.len(), lib_count));
        if !plan.downloads.is_empty() {
            download_batch(
                job.clone(),
                "vanilla",
                plan.downloads.clone(),
                plan.dests.clone(),
                |f, b, _| {
                    prog.set(0.04 + f * 0.12, &format!("Libraries {:.0}%", f * 100.0));
                    let _ = b;
                },
            )?;
        }
        prog.set(0.17, "Libraries ready");
        extract_natives(&job, &plan.natives);

        // client jar
        let client = meta.get("downloads").and_then(|d| d.get("client")).cloned();
        if let Some(c) = client {
            let url = c.get("url").and_then(|u| u.as_str()).unwrap_or_default();
            let sha = c.get("sha1").and_then(|s| s.as_str()).unwrap_or_default();
            let dest = version_dir.join(format!("{id}.jar"));
            if !net::verify(&dest, Some(sha)) {
                prog.set(0.18, "Downloading client…");
                net::download(url, &dest, Some(sha), |n| {
                    if let Ok(mut j) = job.lock() {
                        if let Some(p) = j.patch("vanilla") {
                            p.bytes += n;
                        }
                    }
                })?;
            }
            log(&job, format!("client jar ok ({})", dest.display()));
        }
        if let Ok(mut j) = job.lock() {
            if let Some(p) = j.patch("vanilla") {
                p.total = plan.classpath.len() as u64;
                p.bytes = plan.classpath.len() as u64;
            }
            j.set_patch("vanilla", "ok", &format!("{} libraries", plan.classpath.len()));
        }
        prog.set(0.25, "Core ready");
    } else if let Ok(mut j) = job.lock() {
        j.set_patch("vanilla", "skipped", "disabled");
    }

    // ---------- 2. assets ----------
    if want("assets") {
        if let Ok(mut j) = job.lock() {
            j.set_patch("assets", "running", "asset index…");
        }
        if let Some(ai) = meta.get("assetIndex") {
            let index_id = ai.get("id").and_then(|s| s.as_str()).unwrap_or("legacy").to_string();
            let url = ai.get("url").and_then(|s| s.as_str()).unwrap_or_default().to_string();
            let sha = ai.get("sha1").and_then(|s| s.as_str()).map(|s| s.to_string());
            let index_path = app.p(&format!("assets/indexes/{index_id}.json"));
            if !net::verify(&index_path, sha.as_deref()) {
                net::download(&url, &index_path, sha.as_deref(), |_| {})?;
            }
            let txt = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
            let index: Value = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
            let objects = index
                .get("objects")
                .and_then(|o| o.as_object())
                .cloned()
                .unwrap_or_default();
            let mut urls = Vec::new();
            let mut shas = Vec::new();
            let mut dests = Vec::new();
            for (_name, obj) in objects.iter() {
                let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or_default().to_string();
                if hash.len() < 4 {
                    continue;
                }
                let dest = asset_dest(&app, &hash);
                if dest.exists() {
                    continue;
                }
                urls.push((
                    format!("{}/{}/{}", mojang::RESOURCES_URL, &hash[0..2], hash),
                    Some(hash.clone()),
                ));
                shas.push(hash.clone());
                dests.push(dest);
            }
            log(&job, format!("assets: {} objects, {} missing", objects.len(), urls.len()));
            if let Ok(mut j) = job.lock() {
                if let Some(p) = j.patch("assets") {
                    p.total = urls.len() as u64;
                }
            }
            if !urls.is_empty() {
                let files = urls.len();
                download_batch(job.clone(), "assets", urls, dests, |f, b, _| {
                    prog.set(0.26 + f * 0.46, &format!("Assets {:.0}%", f * 100.0));
                    if let Ok(mut j) = job.lock() {
                        if let Some(p) = j.patch("assets") {
                            p.bytes = b;
                            p.note = format!("{:.0}% · {} files", f * 100.0, files);
                        }
                    }
                })?;
            }
            // legacy (pre-1.7) asset layout
            if index.get("map_to_resources").and_then(|m| m.as_bool()).unwrap_or(false)
                || index_id == "legacy"
                || index_id == "pre-1.6"
            {
                let virt = app.p("assets/virtual/legacy");
                let _ = fs::create_dir_all(&virt);
                for (name, obj) in objects.iter() {
                    let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or_default();
                    if hash.len() < 4 {
                        continue;
                    }
                    let src = asset_dest(&app, hash);
                    let dst = virt.join(name);
                    if let Some(par) = dst.parent() {
                        let _ = fs::create_dir_all(par);
                    }
                    if !dst.exists() && src.exists() {
                        let _ = fs::copy(&src, &dst);
                    }
                }
            }
            if let Ok(mut j) = job.lock() {
                j.set_patch("assets", "ok", &format!("{} objects", objects.len()));
            }
        } else if let Ok(mut j) = job.lock() {
            j.set_patch("assets", "skipped", "no asset index");
        }
        prog.set(0.72, "Assets ready");
    } else if let Ok(mut j) = job.lock() {
        j.set_patch("assets", "skipped", "disabled");
    }

    // ---------- 3. java runtime ----------
    if want("java") {
        let component = meta
            .get("javaVersion")
            .and_then(|j| j.get("component"))
            .and_then(|c| c.as_str())
            .unwrap_or("java-runtime-delta")
            .to_string();
        let major = meta
            .get("javaVersion")
            .and_then(|j| j.get("majorVersion"))
            .and_then(|m| m.as_u64())
            .unwrap_or(0) as u32;
        let label = java::component_label(&component, major);
        if let Ok(mut j) = job.lock() {
            j.set_patch("java", "running", &format!("{label} ({component})"));
        }
        // 1) already bundled?  2) a system JVM new enough?  3) fetch Temurin
        let bundled = java::bundled(&app, &component);
        let system_ok = java::resolve(&app, &component, major);
        if let Some(exe) = &bundled {
            if let Ok(mut j) = job.lock() {
                j.set_patch("java", "ok", format!("bundled · {}", exe.display()));
            }
        } else if let Some(exe) = &system_ok {
            let have = java::major_of(exe).unwrap_or(0);
            if let Ok(mut j) = job.lock() {
                j.set_patch(
                    "java",
                    "ok",
                    format!("system Java {have} · {}", exe.display()),
                );
            }
            log(&job, format!("using system java {have}: {}", exe.display()));
        } else {
            prog.set(0.73, &format!("Downloading {label}…"));
            match java::install(&app, &component, major, |done, total| {
                let f = if total > 0 {
                    done as f64 / total as f64
                } else {
                    0.0
                };
                let mb = done as f64 / (1024.0 * 1024.0);
                prog.set(0.73 + f * 0.17, &format!("{label} · {mb:.0} MB"));
                if let Ok(mut j) = job.lock() {
                    if let Some(p) = j.patch("java") {
                        p.bytes = done;
                        p.total = total.max(done);
                        p.note = format!("{label} · {mb:.0} MB");
                    }
                }
            }) {
                Ok(exe) => {
                    if let Ok(mut j) = job.lock() {
                        j.set_patch("java", "ok", format!("{label} installed · {}", exe.display()));
                    }
                    log(&job, format!("java runtime ready: {}", exe.display()));
                }
                Err(e) => {
                    if let Ok(mut j) = job.lock() {
                        j.set_patch("java", "error", format!("{label}: {e}"));
                    }
                    log(&job, format!("java install failed: {e}"));
                }
            }
        }
        prog.set(0.90, "Java ready");
    } else if let Ok(mut j) = job.lock() {
        j.set_patch("java", "skipped", "disabled");
    }

    // ---------- 4. skin & cape ----------
    if want("skin") {
        if let Ok(mut j) = job.lock() {
            j.set_patch("skin", "running", "building local skin pack…");
        }
        let s = app.settings();
        let gdir = app.game_dir(&id);
        match skin::build_pack(&app, &gdir, &s.username, &s.skin, &s.cape) {
            Ok(note) => {
                if let Ok(mut j) = job.lock() {
                    j.set_patch("skin", "ok", note);
                }
            }
            Err(e) => {
                if let Ok(mut j) = job.lock() {
                    j.set_patch("skin", "error", e);
                }
            }
        }
    } else if let Ok(mut j) = job.lock() {
        j.set_patch("skin", "skipped", "disabled");
    }

    // ---------- 5. mods staging ----------
    if want("mods") {
        let staged = crate::mods::stage(&app, &id);
        if let Ok(mut j) = job.lock() {
            j.set_patch(
                "mods",
                if staged.is_ok() { "ok" } else { "error" },
                match &staged {
                    Ok(n) => format!("{n} mod(s) staged"),
                    Err(e) => e.clone(),
                },
            );
        }
    } else if let Ok(mut j) = job.lock() {
        j.set_patch("mods", "skipped", "disabled");
    }

    prog.set(1.0, "Done");
    Ok(())
}

/// Delete an installed version (keeps shared libraries/assets).
pub fn uninstall(app: &App, id: &str) -> Result<(), String> {
    let vdir = app.p(&format!("versions/{id}"));
    if vdir.exists() {
        fs::remove_dir_all(&vdir).map_err(|e| e.to_string())?;
    }
    let gdir = app.game_dir(id);
    if gdir.exists() {
        let _ = fs::remove_dir_all(&gdir);
    }
    Ok(())
}
