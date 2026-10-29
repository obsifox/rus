//! Rus Gate. One solid-color vertical window. No title bar, no browser.
//!
//! First run asks for a folder, writes it to the registry, and copies this
//! program there as `rus-gate.exe` and `Rus Launcher.exe`. Later launches skip
//! that question. They read tags — not the release list — and download
//! `rus_{version}.exe` from that tag's release. No connection: start the copy
//! already on disk.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../product.rs"]
mod product;

use eframe::egui::{self, CursorIcon, Sense, ViewportCommand};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

const REPO: &str = "obsifox/rus";
const WINDOW_W: f32 = 420.0;
const WINDOW_H: f32 = 746.0;

const BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x26, 0x5A);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x10, 0x19, 0x40);
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x3C, 0x86);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xF7, 0xFF);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0xB7, 0xC3, 0xDE);
const GOLD: egui::Color32 = egui::Color32::from_rgb(0xF2, 0xB8, 0x4B);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xDB, 0x03);
const INK: egui::Color32 = egui::Color32::from_rgb(0x14, 0x14, 0x18);
const WARN: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x7A, 0x6A);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Rus Gate {version}\n\n  --shot <file>   save the install window, then exit\n\nRus Gate does not open a browser. It checks tags and downloads the matching release.\n",
            version = product::VERSION
        );
        return;
    }
    let shot = args
        .iter()
        .position(|a| a == "--shot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    if let Err(e) = run(shot) {
        eprintln!("Rus Gate could not open its window: {e}");
        std::process::exit(1);
    }
}

fn run(shot: Option<String>) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([WINDOW_W, WINDOW_H])
        .with_min_inner_size([WINDOW_W, WINDOW_H])
        .with_max_inner_size([WINDOW_W, WINDOW_H])
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true)
        .with_title("Rus Gate")
        .with_icon(icon_data());
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "Rus Gate",
        options,
        Box::new(move |cc| Ok(Box::new(Gate::new(cc, shot)))),
    )
    .map_err(|e| e.to_string())
}

fn icon_data() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../../assets/rus-icon.png"))
        .expect("icon")
        .to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData { rgba: img.into_raw(), width: w, height: h }
}

enum Mode {
    Install,
    Launch,
}

enum Msg {
    Status(String),
    Progress(u64, u64),
    Error(String),
    Exit,
}

struct Gate {
    shot: Option<String>,
    shot_done: bool,
    frames: u32,
    mode: Mode,
    path: String,
    status: String,
    error: String,
    got: u64,
    total: u64,
    working: bool,
    rx: Option<Receiver<Msg>>,
    install_art: Option<egui::TextureHandle>,
    update_art: Option<egui::TextureHandle>,
    logo: Option<egui::TextureHandle>,
}

impl Gate {
    fn new(cc: &eframe::CreationContext<'_>, shot: Option<String>) -> Self {
        install_font(&cc.egui_ctx);
        style_ui(&cc.egui_ctx);
        let install_art = load_art(&cc.egui_ctx, "gate-install", include_bytes!("../../assets/gate-install.jpg"));
        let update_art = load_art(&cc.egui_ctx, "gate-update", include_bytes!("../../assets/gate-update.jpg"));
        let logo = load_art(&cc.egui_ctx, "gate-logo", include_bytes!("../../assets/rus-logo-title.png"));
        let launch = shot.is_none() && (daily_launcher() || install_dir().is_some());
        let preview = shot.is_some();
        let preview_update = preview && std::env::var("RUS_GATE_PREVIEW").ok().as_deref() == Some("update");
        let mut me = Self {
            shot,
            shot_done: false,
            frames: 0,
            mode: if launch || preview_update { Mode::Launch } else { Mode::Install },
            path: if preview {
                r"C:\Users\Player\AppData\Local\Programs\Rus".into()
            } else {
                default_install_dir()
            },
            status: if preview_update { "Checking update".into() } else { String::new() },
            error: String::new(),
            got: if preview_update { 42 } else { 0 },
            total: if preview_update { 100 } else { 0 },
            working: false,
            rx: None,
            install_art,
            update_art,
            logo,
        };
        if launch {
            if let Some(dir) = install_dir() {
                me.start_update(dir);
            } else {
                me.error = "Rus is not installed. Run rus-gate.exe once.".into();
            }
        }
        me
    }

    fn start_install(&mut self) {
        let path = default_install_dir();
        self.path = path.clone();
        if path.trim().is_empty() {
            self.error = "No install folder is available.".into();
            return;
        }
        self.error.clear();
        self.working = true;
        self.status = "Preparing the folder".into();
        self.got = 0;
        self.total = 0;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || install_and_launch(PathBuf::from(path), tx));
    }

    fn start_update(&mut self, dir: PathBuf) {
        self.working = true;
        self.error.clear();
        self.status = "Checking update".into();
        self.got = 0;
        self.total = 0;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || update_and_launch(dir, tx));
    }

    fn pump(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let mut exit = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Status(s) => {
                    self.status = s;
                    self.error.clear();
                }
                Msg::Progress(got, total) => {
                    self.got = got;
                    self.total = total;
                }
                Msg::Error(e) => {
                    self.error = e;
                    self.working = false;
                }
                Msg::Exit => exit = true,
            }
            ctx.request_repaint();
        }
        if self.working {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
        if exit {
            std::process::exit(0);
        }
    }
}

impl eframe::App for Gate {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        self.pump(ctx);
        #[cfg(windows)]
        fit_windows_frame();
        place_fitted_window(ctx, egui::vec2(WINDOW_W, WINDOW_H), 0.42, 0.72);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                self.paint(ui, full);
            });
        if self.shot.is_some() {
            ctx.request_repaint();
            if !self.shot_done && self.frames > 2 {
                self.shot_done = true;
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
            }
        }
        let shot = self.shot.clone();
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(path) = &shot {
                        let _ = save_png(path, image);
                        std::process::exit(0);
                    }
                }
            }
        });
    }
}

impl Gate {
    fn paint(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let art = if matches!(self.mode, Mode::Install) && !self.working {
            self.install_art.as_ref()
        } else {
            self.update_art.as_ref()
        };
        if let Some(art) = art {
            paint_cover(ui, art, full);
        }
        paint_bottom_shade(ui.painter(), full);
        if let Some(logo) = &self.logo {
            paint_title(ui, logo, full);
        }
        paint_exit(ui, full);
        match self.mode {
            Mode::Install if !self.working => self.paint_form(ui, full),
            _ => self.paint_status(ui, full),
        }
    }

    fn paint_form(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let path = fit_middle(ui, &self.path, body_font(14.0), full.width() - 56.0);
        ui.painter().text(
            egui::pos2(full.center().x, full.bottom() - 132.0),
            egui::Align2::CENTER_CENTER,
            "Install directory",
            body_font(15.0),
            TEXT,
        );
        ui.painter().text(
            egui::pos2(full.center().x, full.bottom() - 108.0),
            egui::Align2::CENTER_CENTER,
            &path,
            body_font(14.0),
            TEXT,
        );
        if !self.error.is_empty() {
            ui.painter().text(
                egui::pos2(full.center().x, full.bottom() - 86.0),
                egui::Align2::CENTER_CENTER,
                &ellipsize(&self.error, 42),
                body_font(12.0),
                WARN,
            );
        }
        let install = egui::Rect::from_center_size(egui::pos2(full.center().x, full.bottom() - 52.0), egui::vec2(148.0, 42.0));
        if gold_button(ui, install, "Install") {
            self.start_install();
        }
    }

    fn paint_status(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let bar = egui::Rect::from_min_size(egui::pos2(full.left() + 36.0, full.bottom() - 56.0), egui::vec2(full.width() - 72.0, 8.0));
        let raw = if self.error.is_empty() {
            if self.status.is_empty() { "Checking update" } else { self.status.as_str() }
        } else {
            self.error.as_str()
        };
        let line = ellipsize(raw, 36);
        let color = if self.error.is_empty() { YELLOW } else { WARN };
        ui.painter().text(egui::pos2(bar.left(), bar.top() - 18.0), egui::Align2::LEFT_CENTER, &line, body_font(16.0), color);
        ui.painter().rect_filled(bar, 4.0, SURFACE);
        let frac = if self.total > 0 {
            (self.got as f32 / self.total as f32).clamp(0.0, 1.0)
        } else if self.working {
            0.08
        } else {
            0.0
        };
        if frac > 0.0 {
            let fill = egui::Rect::from_min_max(bar.left_top(), egui::pos2(bar.left() + bar.width() * frac, bar.bottom()));
            ui.painter().rect_filled(fill, 4.0, YELLOW);
        }
        if !self.working && !self.error.is_empty() && matches!(self.mode, Mode::Launch) {
            let retry = egui::Rect::from_min_size(egui::pos2(bar.right() - 148.0, bar.top() - 78.0), egui::vec2(148.0, 46.0));
            if gold_button(ui, retry, "Retry") {
                if let Some(dir) = install_dir() {
                    self.start_update(dir);
                }
            }
        }
    }
}

fn ellipsize(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(1);
    format!("{}…", text.chars().take(keep).collect::<String>())
}

fn install_and_launch(dir: PathBuf, tx: Sender<Msg>) {
    if let Err(e) = install_files(&dir, &tx) {
        let _ = tx.send(Msg::Error(e));
        return;
    }
    update_and_launch(dir, tx);
}

fn install_files(dir: &Path, tx: &Sender<Msg>) -> Result<(), String> {
    let _ = tx.send(Msg::Status("Creating the folder".into()));
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create the folder: {e}"))?;
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let gate = dir.join(gate_file_name());
    let launcher = dir.join(launcher_file_name());
    copy_exe(&current, &gate)?;
    copy_exe(&current, &launcher)?;
    let _ = tx.send(Msg::Status("Saving the install folder".into()));
    write_install_dir(dir)?;
    let _ = desktop_shortcut(&launcher, dir);
    Ok(())
}

fn update_and_launch(dir: PathBuf, tx: Sender<Msg>) {
    let _ = tx.send(Msg::Status("Checking update".into()));
    match fetch_latest() {
        Fetch::Offline => match newest_local(&dir) {
            Some(path) => {
                let _ = tx.send(Msg::Status("No connection. Starting the installed Rus.".into()));
                if let Err(e) = spawn_launcher(&path) {
                    let _ = tx.send(Msg::Error(e));
                    return;
                }
                let _ = tx.send(Msg::Exit);
            }
            None => {
                let _ = tx.send(Msg::Error("No connection, and no Rus version is installed.".into()));
            }
        },
        Fetch::Failed(e) => {
            let _ = tx.send(Msg::Error(e));
        }
        Fetch::Tag(tag) => {
            if newer_than_self(&tag) {
                let _ = tx.send(Msg::Status(format!("Updating Rus Gate to {tag}")));
                if let Err(e) = self_update(&dir, &tag, &tx) {
                    let _ = tx.send(Msg::Error(e));
                    return;
                }
            }
            let dest = dir.join(payload_name(&tag));
            if !dest.is_file() {
                let url = format!("https://github.com/{REPO}/releases/download/{tag}/{}", payload_name(&tag));
                let _ = tx.send(Msg::Status(format!("Downloading {tag}")));
                if let Err(e) = download(&url, &dest, &tx) {
                    let _ = tx.send(Msg::Error(e));
                    return;
                }
                let _ = cleanup_old(&dir, &dest);
            }
            let _ = tx.send(Msg::Status("Starting Rus".into()));
            if let Err(e) = spawn_launcher(&dest) {
                let _ = tx.send(Msg::Error(e));
                return;
            }
            let _ = tx.send(Msg::Exit);
        }
    }
}

enum Fetch {
    Offline,
    Failed(String),
    Tag(String),
}

fn fetch_latest() -> Fetch {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(20))
        .user_agent(&format!("RusGate/{}", product::VERSION))
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/tags?per_page=100");
    let resp = match agent.get(&url).set("Accept", "application/vnd.github+json").call() {
        Ok(resp) => resp,
        Err(ureq::Error::Status(code, _)) if code == 404 => return Fetch::Failed("The tag list was not found.".into()),
        Err(_) => return Fetch::Offline,
    };
    let body = match resp.into_string() {
        Ok(body) => body,
        Err(e) => return Fetch::Failed(e.to_string()),
    };
    let tags: Vec<serde_json::Value> = match serde_json::from_str(&body) {
        Ok(tags) => tags,
        Err(e) => return Fetch::Failed(format!("Tag list: {e}")),
    };
    let mut best: Option<([u32; 4], String)> = None;
    for tag in tags {
        let Some(name) = tag.get("name").and_then(|v| v.as_str()) else { continue };
        let Some(parts) = product::parse(name) else { continue };
        if best.as_ref().map(|(have, _)| parts > *have).unwrap_or(true) {
            best = Some((parts, name.to_string()));
        }
    }
    match best {
        Some((_, name)) => Fetch::Tag(name),
        None => Fetch::Failed("No version tag in {version}.{year}.{month}.{patch} form.".into()),
    }
}

fn newer_than_self(tag: &str) -> bool {
    match (product::parse(tag), product::parse(product::VERSION)) {
        (Some(remote), Some(local)) => remote > local,
        _ => false,
    }
}

fn self_update(dir: &Path, tag: &str, tx: &Sender<Msg>) -> Result<(), String> {
    let staged = dir.join(format!("{}.new", gate_file_name()));
    let url = format!("https://github.com/{REPO}/releases/download/{tag}/{}", gate_file_name());
    download(&url, &staged, tx)?;
    replace_and_relaunch(dir, &staged)
}

fn replace_and_relaunch(dir: &Path, staged: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let script = dir.join("rus-gate-update.cmd");
        let gate = dir.join(gate_file_name());
        let launcher = dir.join(launcher_file_name());
        let body = format!(
            "@echo off\r\ntimeout /t 1 /nobreak >nul\r\ncopy /Y \"{}\" \"{}\" >nul\r\ncopy /Y \"{}\" \"{}\" >nul\r\ndel \"{}\"\r\nstart \"\" \"{}\"\r\ndel \"%~f0\"\r\n",
            staged.display(),
            gate.display(),
            staged.display(),
            launcher.display(),
            staged.display(),
            launcher.display(),
        );
        std::fs::write(&script, body).map_err(|e| e.to_string())?;
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &script.display().to_string()]);
        hide_console(&mut cmd);
        cmd.spawn().map_err(|e| e.to_string())?;
        std::process::exit(0);
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let old = current.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&current, &old).map_err(|e| e.to_string())?;
        std::fs::rename(staged, &current).map_err(|e| e.to_string())?;
        Command::new(&current).spawn().map_err(|e| e.to_string())?;
        std::process::exit(0);
    }
}

fn download(url: &str, dest: &Path, tx: &Sender<Msg>) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(60))
        .user_agent(&format!("RusGate/{}", product::VERSION))
        .build();
    let mut last = String::new();
    for attempt in 0..3u32 {
        match download_once(&agent, url, dest, tx) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = e;
                std::thread::sleep(Duration::from_millis(400 * (attempt as u64 + 1)));
            }
        }
    }
    Err(last)
}

fn download_once(agent: &ureq::Agent, url: &str, dest: &Path, tx: &Sender<Msg>) -> Result<(), String> {
    let resp = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let total = resp.header("content-length").and_then(|s| s.parse().ok()).unwrap_or(0);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("partial");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 64 * 1024];
    let mut got = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        let _ = tx.send(Msg::Progress(got, total));
    }
    file.sync_all().ok();
    drop(file);
    if got < 1024 || (total > 0 && got != total) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Download stopped at {got} of {total}."));
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn newest_local(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<([u32; 4], PathBuf)> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy();
        let stem = name.strip_prefix("rus_")?;
        let stem = stem.strip_suffix(".exe").unwrap_or(stem);
        let parts = product::parse(stem)?;
        if best.as_ref().map(|(have, _)| parts > *have).unwrap_or(true) {
            best = Some((parts, path));
        }
    }
    best.map(|(_, path)| path)
}

fn cleanup_old(dir: &Path, keep: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with("rus_") && (name.ends_with(".exe") || !name.contains('.')) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

fn copy_exe(from: &Path, to: &Path) -> Result<(), String> {
    if same_file(from, to) {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("Could not copy into the folder: {e}"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn spawn_launcher(path: &Path) -> Result<(), String> {
    let mut cmd = Command::new(path);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("Could not start Rus: {e}"))
}

fn gate_file_name() -> &'static str {
    if cfg!(windows) { "rus-gate.exe" } else { "rus-gate" }
}

fn launcher_file_name() -> &'static str {
    if cfg!(windows) { "Rus Launcher.exe" } else { "Rus Launcher" }
}

fn payload_name(tag: &str) -> String {
    if cfg!(windows) {
        format!("rus_{tag}.exe")
    } else {
        format!("rus_{tag}")
    }
}

fn daily_launcher() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()))
        .map(|s| s.contains("launcher"))
        .unwrap_or(false)
}

fn default_install_dir() -> String {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local).join("Programs").join("Rus").display().to_string();
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile)
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("Rus")
            .display()
            .to_string();
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local").join("share").join("rus").display().to_string();
    }
    "Rus".into()
}

fn install_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        return read_registry();
    }
    #[cfg(not(windows))]
    {
        let path = config_file();
        let text = std::fs::read_to_string(path).ok()?;
        let dir = PathBuf::from(text.trim());
        if dir.as_os_str().is_empty() { None } else { Some(dir) }
    }
}

fn write_install_dir(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        return write_registry(dir);
    }
    #[cfg(not(windows))]
    {
        let path = config_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, dir.display().to_string()).map_err(|e| e.to_string())
    }
}

fn config_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("rus").join("install-dir")
}

#[cfg(windows)]
fn read_registry() -> Option<PathBuf> {
    let key = open_key(false).ok()?;
    let value = query_sz(key, "InstallDir").ok();
    close_key(key);
    let value = value?;
    if value.is_empty() { None } else { Some(PathBuf::from(value)) }
}

#[cfg(windows)]
fn write_registry(dir: &Path) -> Result<(), String> {
    let key = open_key(true)?;
    let result = set_sz(key, "InstallDir", &dir.display().to_string());
    close_key(key);
    result
}

#[cfg(windows)]
fn open_key(create: bool) -> Result<usize, String> {
    type Hkey = *mut std::ffi::c_void;
    const HKCU: Hkey = 0x8000_0001usize as Hkey;
    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExW(
            h: Hkey,
            sub: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            sam: u32,
            security: *mut std::ffi::c_void,
            result: *mut Hkey,
            disposition: *mut u32,
        ) -> i32;
        fn RegOpenKeyExW(h: Hkey, sub: *const u16, options: u32, sam: u32, result: *mut Hkey) -> i32;
    }
    let sub = wide("Software\\Rus");
    let mut out: Hkey = std::ptr::null_mut();
    let code = if create {
        unsafe {
            RegCreateKeyExW(
                HKCU,
                sub.as_ptr(),
                0,
                std::ptr::null(),
            0,
            0x20006 | 0x20019,
                std::ptr::null_mut(),
                &mut out,
                std::ptr::null_mut(),
            )
        }
    } else {
        unsafe { RegOpenKeyExW(HKCU, sub.as_ptr(), 0, 0x20019, &mut out) }
    };
    if code != 0 || out.is_null() {
        return Err(format!("Could not open the registry ({code})."));
    }
    Ok(out as usize)
}

#[cfg(windows)]
fn query_sz(key: usize, name: &str) -> Result<String, String> {
    type Hkey = *mut std::ffi::c_void;
    #[link(name = "advapi32")]
    extern "system" {
        fn RegQueryValueExW(
            h: Hkey,
            name: *const u16,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            len: *mut u32,
        ) -> i32;
    }
    let name = wide(name);
    let mut kind = 0u32;
    let mut len = 0u32;
    let h = key as Hkey;
    let code = unsafe { RegQueryValueExW(h, name.as_ptr(), std::ptr::null_mut(), &mut kind, std::ptr::null_mut(), &mut len) };
    if code != 0 || len < 2 {
        return Err(format!("Install path is not registered ({code})."));
    }
    let mut buf = vec![0u16; (len as usize / 2) + 1];
    let code = unsafe {
        RegQueryValueExW(
            h,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut len,
        )
    };
    if code != 0 {
        return Err(format!("Could not read the install path ({code})."));
    }
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..end]))
}

#[cfg(windows)]
fn set_sz(key: usize, name: &str, value: &str) -> Result<(), String> {
    type Hkey = *mut std::ffi::c_void;
    #[link(name = "advapi32")]
    extern "system" {
        fn RegSetValueExW(h: Hkey, name: *const u16, reserved: u32, kind: u32, data: *const u8, len: u32) -> i32;
    }
    let name = wide(name);
    let data = wide(value);
    let code = unsafe {
        RegSetValueExW(
            key as Hkey,
            name.as_ptr(),
            0,
            1,
            data.as_ptr() as *const u8,
            (data.len() * 2) as u32,
        )
    };
    if code != 0 {
        return Err(format!("Could not save the install path ({code})."));
    }
    Ok(())
}

#[cfg(windows)]
fn close_key(key: usize) {
    type Hkey = *mut std::ffi::c_void;
    #[link(name = "advapi32")]
    extern "system" {
        fn RegCloseKey(h: Hkey) -> i32;
    }
    unsafe { RegCloseKey(key as Hkey); }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn desktop_shortcut(target: &Path, workdir: &Path) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (target, workdir);
        return Ok(());
    }
    #[cfg(windows)]
    {
        let desktop = std::env::var("USERPROFILE")
            .map(|p| PathBuf::from(p).join("Desktop"))
            .unwrap_or_else(|_| PathBuf::from("."));
        let link = desktop.join("Rus Launcher.lnk");
        let vbs = workdir.join("rus-shortcut.vbs");
        let script = format!(
            "Set w = CreateObject(\"WScript.Shell\")\r\nSet s = w.CreateShortcut(\"{}\")\r\ns.TargetPath = \"{}\"\r\ns.WorkingDirectory = \"{}\"\r\ns.Description = \"Rus Launcher\"\r\ns.Save\r\n",
            link.display().to_string().replace('"', "\"\""),
            target.display().to_string().replace('"', "\"\""),
            workdir.display().to_string().replace('"', "\"\""),
        );
        std::fs::write(&vbs, script).map_err(|e| e.to_string())?;
        let mut cmd = Command::new("wscript");
        cmd.args(["//nologo", &vbs.display().to_string()]);
        hide_console(&mut cmd);
        let out = cmd.output().map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&vbs);
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

fn pick_folder() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let script = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = 'Install Rus'; $f.ShowNewFolderButton = $true; if ($f.ShowDialog() -eq 'OK') { $f.SelectedPath }";
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", script]);
        hide_console(&mut cmd);
        let out = cmd.output().ok()?;
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(PathBuf::from(path)) }
    }
    #[cfg(not(windows))]
    {
        if Command::new("zenity").arg("--version").output().is_ok() {
            let out = Command::new("zenity")
                .args(["--file-selection", "--directory", "--title", "Install Rus"])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(PathBuf::from(path)) }
        } else {
            None
        }
    }
}

fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Drop the caption winit leaves on a borderless window, so the picture is not
/// drawn under a black bar.
#[cfg(windows)]
fn fit_windows_frame() {
    use std::os::raw::c_void;
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_CAPTION: isize = 0x00C0_0000;
    const WS_THICKFRAME: isize = 0x0004_0000;
    const WS_BORDER: isize = 0x0080_0000;
    const WS_EX_WINDOWEDGE: isize = 0x0000_0100;
    const WS_EX_CLIENTEDGE: isize = 0x0000_0200;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    #[repr(C)]
    struct Margins {
        cx_left: i32,
        cx_right: i32,
        cy_top: i32,
        cy_bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, window: *const u16) -> *mut c_void;
        fn GetWindowLongPtrW(hwnd: *mut c_void, index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: *mut c_void, index: i32, value: isize) -> isize;
        fn SetWindowPos(hwnd: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn GetClientRect(hwnd: *mut c_void, rect: *mut WinRect) -> i32;
        fn SetWindowRgn(hwnd: *mut c_void, rgn: *mut c_void, redraw: i32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, width: i32, height: i32) -> *mut c_void;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: *mut c_void, margins: *const Margins) -> i32;
    }
    #[repr(C)]
    struct WinRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    let name: Vec<u16> = "Rus Gate\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), name.as_ptr()) };
    if hwnd.is_null() {
        return;
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next = style & !(WS_CAPTION | WS_THICKFRAME | WS_BORDER);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next_ex = ex & !(WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE);
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
        }
        if next_ex != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_ex);
        }
        if next != style || next_ex != ex {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        let margins = Margins { cx_left: -1, cx_right: -1, cy_top: -1, cy_bottom: -1 };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        let mut rect = WinRect { left: 0, top: 0, right: 0, bottom: 0 };
        if GetClientRect(hwnd, &mut rect) != 0 {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            static LAST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
            let key = w.wrapping_mul(10000).wrapping_add(h);
            if LAST.swap(key, Ordering::Relaxed) != key && w > 0 && h > 0 {
                // Same 3% as the painted poster. Ellipse size is the corner diameter.
                let diameter = ((w.min(h) as f32) * 0.06).round().max(2.0) as i32;
                let rgn = CreateRoundRectRgn(0, 0, w, h, diameter, diameter);
                if !rgn.is_null() {
                    SetWindowRgn(hwnd, rgn, 1);
                }
            }
        }
    }
}

fn install_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("Roboto400".into(), egui::FontData::from_static(include_bytes!("../../assets/fonts/Roboto-400.ttf")).into());
    fonts.font_data.insert("Roboto500".into(), egui::FontData::from_static(include_bytes!("../../assets/fonts/Roboto-500.ttf")).into());
    fonts.font_data.insert("Roboto700".into(), egui::FontData::from_static(include_bytes!("../../assets/fonts/Roboto-700.ttf")).into());
    fonts.font_data.insert("Material".into(), egui::FontData::from_static(include_bytes!("../../assets/MaterialSymbols.ttf")).into());
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "Roboto400".into());
    fonts.families.insert(egui::FontFamily::Name("Title".into()), vec!["Roboto700".into()]);
    fonts.families.insert(egui::FontFamily::Name("Nav".into()), vec!["Roboto500".into()]);
    fonts.families.insert(egui::FontFamily::Name("Body".into()), vec!["Roboto400".into()]);
    fonts.families.insert(egui::FontFamily::Name("Icons".into()), vec!["Material".into()]);
    ctx.set_fonts(fonts);
}

fn place_fitted_window(ctx: &egui::Context, design: egui::Vec2, frac_w: f32, frac_h: f32) {
    static ZOOM_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    static STAGE: AtomicU8 = AtomicU8::new(0);
    let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
        return;
    };
    if monitor.x < 240.0 || monitor.y < 240.0 {
        return;
    }
    let stored = ZOOM_BITS.load(Ordering::Relaxed);
    let zoom = if stored == 0 {
        if (ctx.zoom_factor() - 1.0).abs() > 0.02 {
            return;
        }
        let want = (monitor.x * frac_w / design.x).min(monitor.y * frac_h / design.y);
        let cap = (monitor.x * 0.92 / design.x).min(monitor.y * 0.92 / design.y);
        let zoom = if cap < 1.0 { cap.clamp(0.62, 1.0) } else { want.clamp(1.0, cap.min(2.6)) };
        ZOOM_BITS.store(zoom.to_bits(), Ordering::Relaxed);
        zoom
    } else {
        f32::from_bits(stored)
    };
    if (ctx.zoom_factor() - zoom).abs() > 0.02 {
        ctx.set_zoom_factor(zoom);
    }
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(design));
    let stage = STAGE.load(Ordering::Relaxed);
    if stage < 8 {
        if stage >= 2 {
            if let Some(cmd) = ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
        }
        STAGE.store(stage + 1, Ordering::Relaxed);
    }
}

fn style_ui(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.extreme_bg_color = SURFACE;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = BORDER;
    style.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
    let clear = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    style.visuals.widgets.inactive.bg_stroke = clear;
    style.visuals.widgets.hovered.bg_stroke = clear;
    style.visuals.widgets.active.bg_stroke = clear;
    ctx.set_style(style);
}

fn clear_field_chrome(ui: &mut egui::Ui) {
    let vis = ui.visuals_mut();
    let clear = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    vis.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.inactive.bg_stroke = clear;
    vis.widgets.hovered.bg_stroke = clear;
    vis.widgets.active.bg_stroke = clear;
    vis.override_text_color = Some(TEXT);
}

fn title_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Title".into()))
}

fn body_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Body".into()))
}

fn gold_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new(label), Sense::click());
    let fill = if hit.hovered() { egui::Color32::from_rgb(0xFF, 0xE6, 0x4A) } else { YELLOW };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, title_font(16.0), INK);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn outline_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new(label), Sense::click());
    let stroke = if hit.hovered() { GOLD } else { BORDER };
    ui.painter().rect_filled(rect, 8.0, SURFACE);
    ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, body_font(14.0), TEXT);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn paint_exit(ui: &mut egui::Ui, full: egui::Rect) {
    let h = 4.0;
    let corner = egui::pos2(full.right() - 7.0, full.top() + 7.0);
    let center = corner + egui::vec2(-h, h);
    let exit = egui::Rect::from_center_size(center, egui::vec2(16.0, 16.0));
    let hit = ui.interact(exit, egui::Id::new("gate-exit"), Sense::click());
    let color = if hit.hovered() { egui::Color32::from_rgb(255, 236, 176) } else { GOLD };
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "\u{E5CD}",
        egui::FontId::new(16.0, egui::FontFamily::Name("Icons".into())),
        color,
    );
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if hit.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

fn load_art(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

/// Corner radius is 3% of the shorter side.
fn corner_radius(full: egui::Rect) -> f32 {
    full.width().min(full.height()) * 0.03
}

fn paint_cover(ui: &mut egui::Ui, tex: &egui::TextureHandle, full: egui::Rect) {
    let size = tex.size();
    let aspect = if size[1] == 0 { 9.0 / 16.0 } else { size[0] as f32 / size[1] as f32 };
    let window = full.width() / full.height().max(1.0);
    let uv = if window > aspect {
        let shown = aspect / window;
        let y0 = (1.0 - shown) * 0.5;
        egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(1.0, y0 + shown))
    } else {
        let shown = window / aspect.max(0.001);
        let x0 = (1.0 - shown) * 0.5;
        egui::Rect::from_min_max(egui::pos2(x0, 0.0), egui::pos2(x0 + shown, 1.0))
    };
    let radius = corner_radius(full).round().clamp(1.0, 255.0) as u8;
    egui::Image::from_texture((tex.id(), egui::vec2(size[0] as f32, size[1] as f32)))
        .uv(uv)
        .corner_radius(egui::CornerRadius::same(radius))
        .paint_at(ui, full);
}

fn paint_title(ui: &mut egui::Ui, tex: &egui::TextureHandle, full: egui::Rect) {
    let size = tex.size();
    let aspect = if size[1] == 0 { 1465.0 / 353.0 } else { size[0] as f32 / size[1] as f32 };
    let w = (full.width() * 0.74).min(full.width() - 52.0);
    let h = w / aspect;
    let rect = egui::Rect::from_min_size(
        egui::pos2(full.center().x - w * 0.5, full.top() + 22.0),
        egui::vec2(w, h),
    );
    ui.painter().image(
        tex.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

fn paint_bottom_shade(painter: &egui::Painter, full: egui::Rect) {
    let h = 168.0;
    let radius = corner_radius(full);
    let top = full.bottom() - h;
    let mut y = top;
    while y < full.bottom() {
        let t = ((y - top) / h).clamp(0.0, 1.0);
        let alpha = (t * t * 110.0) as u8;
        let (left, right) = rounded_span(full, y, radius);
        painter.hline(left..=right, y, egui::Stroke::new(1.15, egui::Color32::from_black_alpha(alpha)));
        y += 1.0;
    }
}

fn rounded_span(full: egui::Rect, y: f32, radius: f32) -> (f32, f32) {
    let edge = if y - full.top() < radius {
        radius - (y - full.top())
    } else if full.bottom() - y < radius {
        radius - (full.bottom() - y)
    } else {
        0.0
    };
    let inset = if edge <= 0.0 {
        0.0
    } else {
        radius - (radius * radius - edge * edge).max(0.0).sqrt()
    };
    (full.left() + inset, full.right() - inset)
}

fn fit_middle(ui: &egui::Ui, text: &str, font: egui::FontId, max_w: f32) -> String {
    let width = |s: &str| ui.painter().layout_no_wrap(s.to_string(), font.clone(), TEXT).size().x;
    if width(text) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 8 {
        return ellipsize(text, 8);
    }
    let mut keep = chars.len() / 2;
    while keep > 4 {
        let head: String = chars.iter().take(keep).collect();
        let tail: String = chars.iter().skip(chars.len() - keep).collect();
        let shown = format!("{head}…{tail}");
        if width(&shown) <= max_w {
            return shown;
        }
        keep -= 1;
    }
    ellipsize(text, 24)
}

fn save_png(path: &str, image: &egui::ColorImage) -> Result<(), String> {
    let img = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, image.as_raw().to_vec())
        .ok_or("bad image")?;
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    img.save(path).map_err(|e| e.to_string())
}
