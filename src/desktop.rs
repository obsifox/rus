//! Native desktop window. No title bar, no browser.

use crate::preview3d::SkinPreview;
use crate::skinmodel::ArmModel;
use crate::state::{now_ms, App, Job, ServerEntry};
use crate::wallpapers::{self, Paper};
use eframe::egui::{self, CursorIcon, ResizeDirection, RichText, Sense, ViewportCommand};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const WINDOW_W: f32 = 960.0;
const WINDOW_H: f32 = 540.0;

const BG: egui::Color32 = egui::Color32::from_rgb(0x08, 0x10, 0x1C);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x10, 0x1A, 0x2C);
const SURFACE2: egui::Color32 = egui::Color32::from_rgb(0x16, 0x24, 0x3A);
const SELECTED: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x30, 0x4E);
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x52, 0x80);
const BORDER_BTN: egui::Color32 = egui::Color32::from_rgb(0x3E, 0x6C, 0xA4);
const BORDER_INPUT: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x74, 0xB0);
const MARKER_OFF: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x40, 0x5C);
const TEXT: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0xB4, 0xC2, 0xD4);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x3B, 0x82, 0xF6);
const ON_ACCENT: egui::Color32 = egui::Color32::WHITE;
const GOLD: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xBA, 0x4E);
fn glass_fill() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(12, 22, 40, 228) }
fn glass_hover() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(22, 38, 64, 236) }
const ERR: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xA0, 0x98);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Home,
    Versions,
    Mods,
    Settings,
}

enum Page {
    Mods,
    Settings,
}

impl Tab {
    fn from_name(s: &str) -> Self {
        match s {
            "versions" | "version" | "instance" | "instances" => Tab::Versions,
            "mods" => Tab::Mods,
            "settings" => Tab::Settings,
            _ => Tab::Home,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Versions => "Instance",
            Tab::Mods => "Mod",
            Tab::Settings => "Settings",
        }
    }
}

#[derive(Clone)]
struct Ver {
    id: String,
    channel: String,
    installed: bool,
    released: String,
}

enum Msg {
    Versions { rows: Vec<Ver>, latest: String },
    VersionsErr(String),
    Paper { title: String, bytes: Vec<u8> },
    Notice(String),
    Mods(Vec<(String, bool, u64)>),
    Loaders(crate::loaders::Catalog),
    Fabric(String, String),
}

struct Desktop {
    app: Arc<App>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    tab: Tab,
    versions: Vec<Ver>,
    versions_note: String,
    version_latest: String,
    release_view: bool,
    loaders: crate::loaders::Catalog,
    loader_after: bool,
    fabric_wait: String,
    query: String,
    selected: String,
    papers: HashMap<String, egui::TextureHandle>,
    mark: Option<egui::TextureHandle>,
    grass: Option<egui::TextureHandle>,
    icon_forge: Option<egui::TextureHandle>,
    icon_fabric: Option<egui::TextureHandle>,
    icon_neoforge: Option<egui::TextureHandle>,
    icon_optifine: Option<egui::TextureHandle>,
    loader: String,
    playtime: HashMap<String, u128>,
    servers: Vec<ServerEntry>,
    server_icons: HashMap<String, egui::TextureHandle>,
    server_name: String,
    server_addr: String,
    join: Option<String>,
    join_name: String,
    icons_dirty: bool,
    avatar: Option<egui::TextureHandle>,
    avatar_key: String,
    mods: Vec<(String, bool, u64)>,
    username: String,
    ram: u32,
    width: u32,
    height: u32,
    snapshots: bool,
    previews: bool,
    skin: String,
    cape: String,
    engine: String,
    notice: String,
    import_error: String,
    job_id: Option<String>,
    patches: Vec<crate::state::PatchState>,
    shot: Option<String>,
    frames: u32,
    shot_requested: bool,
    film: usize,
    play_after: bool,
    preview: SkinPreview,
    drop_hover: bool,
    instances: Vec<crate::instances::Instance>,
    instance_id: String,
    draft_open: bool,
    draft_name: String,
    draft_version: String,
    draft_loader: String,
}

pub fn run(shot: Option<String>, tab: String) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([WINDOW_W, WINDOW_H])
        .with_min_inner_size([WINDOW_W, WINDOW_H])
        .with_max_inner_size([WINDOW_W, WINDOW_H])
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Rus")
        .with_icon(icon_data());
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Rus", options, Box::new(move |cc| Ok(Box::new(Desktop::new(cc, shot, tab)))))
        .map_err(|e| e.to_string())
}

fn icon_data() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/mark-32.png")).expect("mark").to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData { rgba: img.into_raw(), width: w, height: h }
}

impl Desktop {
    fn new(cc: &eframe::CreationContext<'_>, shot: Option<String>, tab: String) -> Self {
        install_font(&cc.egui_ctx);
        style_ui(&cc.egui_ctx);
        let app = App::load();
        let _ = crate::skin::materialize_preset(&app, "skin", "steve");
        let _ = crate::skin::materialize_preset(&app, "skin", "alex");
        let _ = crate::skin::materialize_preset(&app, "cape", "rus");
        let settings = app.settings();
        let (tx, rx) = mpsc::channel();
        let selected = if settings.last_version.is_empty() { "26.3".to_string() } else { settings.last_version.clone() };
        let draft_version = selected.clone();
        let draft_loader = if settings.loader.is_empty() { "vanilla".to_string() } else { settings.loader.clone() };
        let mut preview = SkinPreview::default();
        preview.arms = if settings.skin_model == "slim" { ArmModel::Slim } else if settings.skin == "preset:alex" { ArmModel::Slim } else { ArmModel::Classic };
        let mut me = Self {
            app: app.clone(),
            tx: tx.clone(),
            rx,
            tab: Tab::from_name(&tab),
            versions: Vec::new(),
            versions_note: "Loading versions…".into(),
            version_latest: String::new(),
            release_view: true,
            loaders: crate::loaders::Catalog::default(),
            loader_after: false,
            fabric_wait: String::new(),
            query: String::new(),
            selected,
            papers: HashMap::new(),
            mark: load_texture(&cc.egui_ctx, "mark", include_bytes!("../assets/mark-32.png")),
            grass: load_texture(&cc.egui_ctx, "vanilla", include_bytes!("../assets/vanilla-grass.png")),
            icon_forge: load_texture(&cc.egui_ctx, "forge", include_bytes!("../assets/loader-forge.png")),
            icon_fabric: load_texture(&cc.egui_ctx, "fabric", include_bytes!("../assets/loader-fabric.png")),
            icon_neoforge: load_texture(&cc.egui_ctx, "neoforge", include_bytes!("../assets/loader-neoforge.png")),
            icon_optifine: load_texture(&cc.egui_ctx, "optifine", include_bytes!("../assets/loader-optifine.png")),
            loader: if settings.loader.is_empty() { "vanilla".into() } else { settings.loader },
            playtime: load_playtime(&app),
            servers: app.load_servers(),
            server_icons: HashMap::new(),
            server_name: String::new(),
            server_addr: String::new(),
            join: None,
            join_name: String::new(),
            icons_dirty: true,
            avatar: None,
            avatar_key: String::new(),
            mods: Vec::new(),
            username: settings.username,
            ram: settings.ram_mb,
            width: settings.width,
            height: settings.height,
            snapshots: settings.show_snapshots,
            previews: settings.show_previews,
            skin: settings.skin,
            cape: settings.cape,
            engine: settings.engine,
            notice: "Ready".into(),
            import_error: String::new(),
            job_id: None,
            patches: crate::install::patch_defs(),
            shot,
            frames: 0,
            shot_requested: false,
            film: 0,
            play_after: false,
            preview,
            drop_hover: false,
            instances: Vec::new(),
            instance_id: String::new(),
            draft_open: false,
            draft_name: String::new(),
            draft_version,
            draft_loader,
        };
        me.bind_instances(&settings.last_instance);
        spawn_versions(app.clone(), tx.clone());
        spawn_mods(app.clone(), tx.clone());
        spawn_loaders(tx.clone());
        me.load_papers(&cc.egui_ctx);
        if me.shot.is_some() && me.tab == Tab::Versions {
            me.loader = "forge".into();
            me.selected = "26.3".into();
            if let Some(inst) = me.instances.first_mut() {
                inst.version = "26.3".into();
                inst.loader = "forge".into();
                if inst.name.trim().is_empty() {
                    inst.name = "26.3".into();
                }
                me.instance_id = inst.id.clone();
            }
        }
        me
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Versions { rows, latest } => {
                    if self.selected.is_empty() {
                        if let Some(first) = rows.first() { self.selected = first.id.clone(); }
                    }
                    self.versions_note = format!("{} versions", rows.len());
                    self.version_latest = latest;
                    self.versions = rows;
                    self.ensure_fabric();
                }
                Msg::VersionsErr(e) => self.versions_note = e,
                Msg::Loaders(cat) => {
                    let fabric = self.loaders.fabric.clone();
                    self.loaders = cat;
                    if self.loaders.fabric.is_empty() {
                        self.loaders.fabric = fabric;
                    }
                    self.ensure_fabric();
                }
                Msg::Fabric(mc, version) => {
                    if self.fabric_wait == mc {
                        self.fabric_wait.clear();
                    }
                    self.loaders.fabric.retain(|(k, _)| k != &mc);
                    if !version.is_empty() {
                        self.loaders.fabric.push((mc, version));
                    } else {
                        self.loaders.fabric.push((mc, String::new()));
                    }
                }
                Msg::Paper { title, bytes } => {
                    if let Some(tex) = load_texture(ctx, &title, &bytes) {
                        self.papers.insert(title, tex);
                    }
                }
                Msg::Notice(s) => self.notice = s,
                Msg::Mods(m) => self.mods = m,
            }
        }
        if self.play_after {
            if let Some(state) = self.job_state() {
                if state == "done" && self.app.is_installed(&self.selected) {
                    self.play_after = false;
                    self.launch_selected();
                } else if state == "error" {
                    self.play_after = false;
                }
            }
        }
        if self.loader_after {
            if let Some(state) = self.job_state() {
                if state == "done" && self.app.is_installed(&self.selected) {
                    self.loader_after = false;
                    self.spawn_loader_install();
                } else if state == "error" {
                    self.loader_after = false;
                }
            }
        }
    }

    fn ensure_fabric(&mut self) {
        let mc = self.selected.clone();
        if mc.is_empty() || self.fabric_wait == mc || self.loaders.fabric.iter().any(|(k, _)| k == &mc) {
            return;
        }
        self.fabric_wait = mc.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if let Ok(version) = crate::loaders::fetch_fabric(&mc) {
                let _ = tx.send(Msg::Fabric(mc, version));
            } else {
                let _ = tx.send(Msg::Fabric(mc, String::new()));
            }
        });
    }

    fn save_settings(&self) {
        let mut s = self.app.settings();
        s.username = self.username.clone();
        s.ram_mb = self.ram;
        s.width = self.width;
        s.height = self.height;
        s.show_snapshots = self.snapshots;
        s.show_previews = self.previews;
        s.skin = self.skin.clone();
        s.cape = self.cape.clone();
        s.engine = self.engine.clone();
        s.loader = self.loader.clone();
        s.last_version = self.selected.clone();
        s.last_instance = self.instance_id.clone();
        s.skin_model = match self.preview.arms {
            ArmModel::Slim => "slim".into(),
            ArmModel::Classic => "classic".into(),
            ArmModel::Auto => {
                if self.detected_slim() { "slim".into() } else { "classic".into() }
            }
        };
        self.app.update_settings(s);
    }

    fn paper_for(&self, id: &str) -> &'static Paper { wallpapers::for_version(id) }

    fn settle_playtime(&mut self) {
        let finished = crate::play::drain_finished();
        if finished.is_empty() {
            return;
        }
        for (_version, ms) in finished {
            let key = if self.instance_id.is_empty() { self.selected.clone() } else { self.instance_id.clone() };
            *self.playtime.entry(key).or_insert(0) += ms;
        }
        save_playtime(&self.app, &self.playtime);
    }

    fn shown_play_ms(&self) -> u128 {
        let key = if self.instance_id.is_empty() { self.selected.as_str() } else { self.instance_id.as_str() };
        let stored = self.playtime.get(key).copied().unwrap_or(0);
        stored + crate::play::live_ms_for(&self.selected).unwrap_or(0)
    }

    fn bind_instances(&mut self, preferred: &str) {
        self.instances = crate::instances::ensure(&self.app, &self.selected, &self.loader);
        let pick = self.instances.iter().find(|i| i.id == preferred).or_else(|| self.instances.first());
        if let Some(inst) = pick.cloned() {
            self.instance_id = inst.id;
            if !inst.version.is_empty() {
                self.selected = inst.version;
            }
            if !inst.loader.is_empty() {
                self.loader = inst.loader;
            }
        }
    }

    fn current_instance(&self) -> Option<&crate::instances::Instance> {
        self.instances.iter().find(|i| i.id == self.instance_id)
    }

    fn select_instance(&mut self, id: &str) {
        let Some(inst) = self.instances.iter().find(|i| i.id == id).cloned() else { return };
        self.instance_id = inst.id;
        self.selected = inst.version;
        self.loader = inst.loader;
        self.save_settings();
        self.ensure_fabric();
    }

    fn remember_instance(&mut self) {
        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == self.instance_id) {
            inst.version = self.selected.clone();
            inst.loader = self.loader.clone();
            crate::instances::save(&self.app, &self.instances);
        }
    }

    fn loader_icon(&self) -> Option<&egui::TextureHandle> {
        match self.loader.as_str() {
            "forge" => self.icon_forge.as_ref(),
            "fabric" => self.icon_fabric.as_ref(),
            "neoforge" => self.icon_neoforge.as_ref(),
            "optifine" => self.icon_optifine.as_ref(),
            _ => self.grass.as_ref(),
        }
    }

    fn launch_selected(&mut self) {
        self.save_settings();
        if self.engine == "horizon" {
            let app = self.app.clone();
            let tx = self.tx.clone();
            self.notice = "Preparing Horizon…".into();
            std::thread::spawn(move || {
                let msg = match crate::horizon::start(app) {
                    Ok(v) => v.get("message").and_then(|m| m.as_str()).unwrap_or("Horizon is starting").to_string(),
                    Err(e) => e,
                };
                let _ = tx.send(Msg::Notice(msg));
            });
            return;
        }
        let id = self.selected.clone();
        let app = self.app.clone();
        let tx = self.tx.clone();
        let join = self.join.clone();
        let game_dir = if self.instance_id.is_empty() {
            self.app.game_dir(&id)
        } else {
            crate::instances::dir(&self.app, &self.instance_id)
        };
        let _ = std::fs::create_dir_all(&game_dir);
        if let Some(addr) = join.clone() {
            crate::state::mark_played(&mut self.servers, &self.join_name, &addr);
            self.app.save_servers(&self.servers);
            self.icons_dirty = true;
            self.notice = format!("Starting {id} on {addr}…");
        } else {
            self.notice = format!("Starting {id}…");
        }
        let profile = if self.loader != "vanilla" {
            match crate::loaders::installed_profile(&self.app, &id, &self.loader) {
                Some(profile) => profile,
                None => {
                    self.notice = format!("Install {} on the Instance tab first", crate::state::loader_label(&self.loader));
                    self.tab = Tab::Versions;
                    return;
                }
            }
        } else {
            id.clone()
        };
        std::thread::spawn(move || {
            let msg = match crate::play::launch_at(&app, &profile, &game_dir, join.as_deref()) {
                Ok(_) => {
                    if let Some(addr) = join.as_deref() {
                        format!("{id} is joining {addr}")
                    } else {
                        format!("{id} is running in the sandbox")
                    }
                }
                Err(e) => e,
            };
            let _ = tx.send(Msg::Notice(msg));
        });
        self.join = None;
    }

    fn join_server(&mut self, name: String, address: String) {
        let address = address.trim().to_string();
        if address.is_empty() {
            self.notice = "Server address is empty".into();
            return;
        }
        self.join_name = if name.trim().is_empty() { address.clone() } else { name.trim().to_string() };
        self.join = Some(address);
        self.engine = "vanilla".into();
        self.save_settings();
        self.play();
    }

    fn add_server(&mut self) {
        let address = self.server_addr.trim().to_string();
        if address.is_empty() {
            return;
        }
        let name = self.server_name.trim().to_string();
        if let Some(existing) = self.servers.iter_mut().find(|s| s.address.eq_ignore_ascii_case(&address)) {
            if !name.is_empty() {
                existing.name = name;
            }
        } else {
            self.servers.push(ServerEntry {
                name: if name.is_empty() { address.clone() } else { name },
                address,
                icon: String::new(),
                played_at: 0,
            });
        }
        self.app.save_servers(&self.servers);
        self.server_name.clear();
        self.server_addr.clear();
    }

    fn set_server_icon(&mut self, address: &str, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.import_error = e.to_string();
                return;
            }
        };
        if image::load_from_memory(&bytes).is_err() {
            self.import_error = "Server icon must be a PNG. The previous icon was kept.".into();
            return;
        }
        let file = icon_file(address);
        let dest = self.app.p(&format!("servers/{file}"));
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&dest, &bytes) {
            self.import_error = e.to_string();
            return;
        }
        if let Some(entry) = self.servers.iter_mut().find(|s| s.address.eq_ignore_ascii_case(address)) {
            entry.icon = file;
        }
        self.app.save_servers(&self.servers);
        self.import_error.clear();
        self.icons_dirty = true;
    }

    fn reload_server_icons(&mut self, ctx: &egui::Context) {
        self.server_icons.clear();
        for server in &self.servers {
            if server.icon.is_empty() {
                continue;
            }
            let path = self.app.p(&format!("servers/{}", server.icon));
            let Ok(bytes) = std::fs::read(&path) else { continue };
            if let Some(tex) = load_texture(ctx, &format!("srv-{}", server.address), &bytes) {
                self.server_icons.insert(server.address.clone(), tex);
            }
        }
    }

    fn paint_recent_servers(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let recent = crate::state::recent_played(&self.servers);
        if recent.is_empty() {
            return;
        }
        let icons = self.server_icons.clone();
        for (i, server) in recent.iter().enumerate() {
            let y = 256.0 + i as f32 * 74.0;
            let rect = egui::Rect::from_min_size(egui::pos2(ux(full, 1204.0), uy(full, y)), egui::vec2(us(full, 62.0), us(full, 62.0)));
            let tex = icons.get(&server.address);
            let label = if server.name.is_empty() { server.address.as_str() } else { server.name.as_str() };
            if server_badge(ui, rect, tex, &format!("recent-{i}"), label) {
                self.join_server(server.name.clone(), server.address.clone());
            }
        }
    }

    fn play(&mut self) {
        let running = crate::play::status();
        if running.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
            let _ = crate::play::kill();
            self.notice = "Stopped".into();
            return;
        }
        if self.engine != "horizon" && !self.app.is_installed(&self.selected) {
            self.play_after = true;
            self.start_install();
            return;
        }
        self.launch_selected();
    }

    fn start_install(&mut self) {
        let version = self.selected.clone();
        if version.is_empty() {
            self.notice = "Select a version".into();
            return;
        }
        let id = format!("job-{}", now_ms());
        let mut patches = self.patches.clone();
        for p in patches.iter_mut() { p.status = "pending".into(); }
        let job = Arc::new(Mutex::new(Job::new(id.clone(), version.clone(), patches)));
        self.app.jobs.lock().unwrap().insert(id.clone(), job.clone());
        *self.app.last_job.lock().unwrap() = Some(id.clone());
        self.job_id = Some(id);
        let app = self.app.clone();
        std::thread::spawn(move || crate::install::run(app, job));
        self.notice = format!("Installing {version}");
    }

    fn job_line(&self) -> Option<(f32, String)> {
        let id = self.job_id.as_ref()?;
        let job = self.app.job(id)?;
        let j = job.lock().ok()?;
        Some((j.progress as f32, format!("{} · {}", j.state, j.message)))
    }

    fn job_state(&self) -> Option<String> {
        let id = self.job_id.as_ref()?;
        let job = self.app.job(id)?;
        let guard = job.lock().ok()?;
        Some(guard.state.clone())
    }

    fn quit(&self, ctx: &egui::Context) {
        let _ = crate::play::kill();
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    fn skin_image(&self) -> Option<RgbaImage> {
        let path = crate::skin::resolve_texture(&self.app, &self.skin)?;
        let bytes = std::fs::read(path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        crate::skinmodel::prepare_skin(&img).ok()
    }

    fn cape_image(&self) -> Option<RgbaImage> {
        let path = crate::skin::resolve_texture(&self.app, &self.cape)?;
        let bytes = std::fs::read(path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        crate::skinmodel::prepare_cape(&img).ok()
    }

    fn detected_slim(&self) -> bool {
        if self.skin == "preset:alex" { return true; }
        if self.skin == "preset:steve" { return false; }
        self.skin_image().map(|img| crate::skinmodel::detect_slim(&img)).unwrap_or(false)
    }

    fn refresh_avatar(&mut self, ctx: &egui::Context) {
        if self.avatar_key == self.skin && self.avatar.is_some() { return; }
        let Some(img) = self.skin_image() else { return; };
        let face = crate::skinmodel::face_avatar(&img, 64);
        let color = egui::ColorImage::from_rgba_unmultiplied([64, 64], face.as_raw());
        self.avatar = Some(ctx.load_texture("avatar", color, egui::TextureOptions::NEAREST));
        self.avatar_key = self.skin.clone();
        self.preview.invalidate();
    }

    fn import_bytes(&mut self, kind: &str, bytes: &[u8]) {
        match crate::skin::import_texture(&self.app, kind, bytes) {
            Ok(id) => {
                if kind == "cape" { self.cape = id; } else { self.skin = id; self.preview.arms = ArmModel::Auto; }
                self.import_error.clear();
                self.avatar_key.clear();
                self.preview.invalidate();
                self.save_settings();
            }
            Err(e) => self.import_error = e,
        }
    }

    fn import_path(&mut self, kind: &str, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(bytes) => self.import_bytes(kind, &bytes),
            Err(e) => self.import_error = e.to_string(),
        }
    }
}

impl eframe::App for Desktop {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0x0B as f32 / 255.0, 0x0D as f32 / 255.0, 0x0A as f32 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
        self.settle_playtime();
        self.refresh_avatar(ctx);
        if self.icons_dirty {
            self.reload_server_icons(ctx);
            self.icons_dirty = false;
        }
        ctx.request_repaint_after(Duration::from_millis(if self.preview.auto_rotate || self.preview.walk { 16 } else { 250 }));
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Num1)) { self.tab = Tab::Home; }
            if ctx.input(|i| i.key_pressed(egui::Key::Num2)) { self.tab = Tab::Versions; }
            if ctx.input(|i| i.key_pressed(egui::Key::Num3)) { self.tab = Tab::Mods; }
            if ctx.input(|i| i.key_pressed(egui::Key::Num4)) { self.tab = Tab::Settings; }
        }
        let drops: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for path in drops {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
            if name.ends_with(".jar") {
                self.tab = Tab::Mods;
                match crate::mods::add_path(&self.app, &path) {
                    Ok(n) => self.notice = format!("Added {n}"),
                    Err(e) => self.notice = e,
                }
                spawn_mods(self.app.clone(), self.tx.clone());
            } else if name.ends_with(".png") && self.tab == Tab::Settings {
                self.import_path("skin", &path);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show(ctx, |ui| self.shell(ui));
        paint_frame(ctx);
        let size = ctx.input(|i| i.viewport().inner_rect.map(|r| r.size()));
        if let Some(size) = size {
            if (size.x - WINDOW_W).abs() > 1.0 || (size.y - WINDOW_H).abs() > 1.0 {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(WINDOW_W, WINDOW_H)));
            }
        }
        self.frames += 1;
        let paper_ready = self.papers.contains_key(self.paper_for(&self.selected).title);
        let versions_ready = !self.versions.is_empty() || self.frames > 40;
        let loaders_ready = self.loaders.ready && self.loaders.fabric.iter().any(|(k, v)| k == &self.selected && !v.is_empty());
        if self.shot.is_some() && !self.shot_requested && self.frames > 8 && (paper_ready || self.frames > 90) && versions_ready && (loaders_ready || self.frames > 80) {
            self.shot_requested = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
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

impl Desktop {
    fn shell(&mut self, ui: &mut egui::Ui) {
        use_page_fonts(ui);
        let full = ui.max_rect();
        let _ = ui.allocate_rect(full, Sense::hover());
        self.paint_wallpaper(ui, full);
        if self.tab != Tab::Home {
            ui.painter().rect_filled(full, 0.0, egui::Color32::from_rgba_unmultiplied(0x16, 0x20, 0x39, 166));
        }
        // Drag only the empty strip above the tabs. A full-window drag was
        // stealing clicks, so the pointer had to sit above the button.
        if !self.draft_open {
            let drag = egui::Rect::from_min_max(full.left_top(), egui::pos2(full.right(), uy(full, 12.0)));
            let drag_hit = ui.interact(drag, egui::Id::new("window-drag"), Sense::drag());
            if drag_hit.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
        }
        self.paint_shared_tabs(ui, full);
        self.paint_exit(ui, full);
        match self.tab {
            Tab::Home => self.paint_chrome(ui, full),
            Tab::Versions => self.paint_version_page(ui, full),
            Tab::Mods => self.page_in_panel(ui, full, Page::Mods),
            Tab::Settings => self.page_in_panel(ui, full, Page::Settings),
        }
        if self.draft_open {
            self.paint_instance_popup(ui, full);
        }
    }

    fn load_papers(&mut self, ctx: &egui::Context) {
        for paper in wallpapers::all() {
            if let Some(bytes) = wallpapers::embedded(paper.title) {
                if let Some(tex) = load_texture(ctx, paper.title, bytes) {
                    self.papers.insert(paper.title.to_string(), tex);
                }
            }
        }
    }

    fn page_in_panel(&mut self, ui: &mut egui::Ui, full: egui::Rect, page: Page) {
        let panel = drect(full, 26.0, 96.0, 1228.0, 608.0);
        panel_card(ui.painter(), panel);
        let inner = panel.shrink(us(full, 14.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.set_clip_rect(inner);
        match page {
            Page::Mods => self.page_mods(&mut child),
            Page::Settings => self.page_settings(&mut child),
        }
    }

    fn paint_shared_tabs(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let tabs = [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings];
        let h = us(full, 40.0);
        let font = nav_font(us(full, 14.5));
        let track = us(full, 0.7);
        let pad = us(full, 16.0);
        let mut widths = Vec::with_capacity(tabs.len());
        for tab in tabs {
            if tab == Tab::Home {
                widths.push(h * 1.35);
            } else {
                let label = tab.label().to_uppercase();
                widths.push(tracked_width(ui.painter(), &label, font.clone(), track) + pad * 2.0);
            }
        }
        let total: f32 = widths.iter().sum();
        let bar = egui::Rect::from_min_size(egui::pos2(ux(full, 22.0), uy(full, 16.0)), egui::vec2(total, h));
        paint_hytale_bar(ui.painter(), bar);

        let mut x = bar.left();
        for (i, tab) in tabs.into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(widths[i], h));
            if i > 0 {
                let top = rect.top() + h * 0.26;
                let bot = rect.bottom() - h * 0.26;
                ui.painter().vline(rect.left(), top..=bot, egui::Stroke::new(1.0, egui::Color32::from_rgb(132, 152, 176)));
            }
            let hit = ui.interact(rect, egui::Id::new("hynav").with(tab.label()), Sense::click());
            let on = self.tab == tab;
            if on {
                paint_active_fade(ui.painter(), rect);
            }
            let color = if hit.hovered() && !on {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(220, 232, 235)
            };
            if tab == Tab::Home {
                paint_nav_house(ui.painter(), rect.center(), h * 0.42, color);
            } else {
                let label = tab.label().to_uppercase();
                paint_tracked(
                    ui.painter(),
                    rect.center() + egui::vec2(0.0, 1.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    font.clone(),
                    egui::Color32::from_black_alpha(80),
                    track,
                );
                paint_tracked(ui.painter(), rect.center(), egui::Align2::CENTER_CENTER, &label, font.clone(), color, track);
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.tab = tab;
            }
            hit.on_hover_text(tab.label());
            x += widths[i];
        }
    }

    fn paint_wallpaper(&self, ui: &egui::Ui, full: egui::Rect) {
        let paper = self.paper_for(&self.selected);
        if let Some(tex) = self.papers.get(paper.title) {
            ui.painter().rect_filled(full, 0.0, BG);
            let fitted = contain_rect(tex.size_vec2(), full);
            ui.painter().image(tex.id(), fitted, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(full, 0.0, BG);
            ui.painter().text(
                full.center(),
                egui::Align2::CENTER_CENTER,
                "Wallpaper is missing from this build",
                body_font(16.0),
                MUTED,
            );
        }
        // Soft edge so the glass cards stay readable without hiding the art.
        for i in 0..8 {
            let t = (i + 1) as f32 / 8.0;
            let a = (t * 28.0) as u8;
            let band = 18.0;
            let y0 = full.top() + i as f32 * band;
            ui.painter().rect_filled(
                egui::Rect::from_min_max(egui::pos2(full.left(), y0), egui::pos2(full.right(), y0 + band)),
                0.0,
                egui::Color32::from_black_alpha(a / 3),
            );
            let y1 = full.bottom() - (i as f32 + 1.0) * band;
            ui.painter().rect_filled(
                egui::Rect::from_min_max(egui::pos2(full.left(), y1), egui::pos2(full.right(), y1 + band)),
                0.0,
                egui::Color32::from_black_alpha(a),
            );
        }
    }

    fn paint_chrome(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let horizon_on = self.engine == "horizon";
        self.paint_recent_servers(ui, full);

        self.paint_player_card(ui, full);
        self.paint_instance_card(ui, full);

        let play = egui::Rect::from_min_size(egui::pos2(ux(full, 1004.0), uy(full, 640.0)), egui::vec2(us(full, 248.0), us(full, 52.0)));
        let live = crate::play::status().get("running").and_then(|v| v.as_bool()).unwrap_or(false);
        let label = if live { "STOP" } else { "PLAY" };
        let note = if let Some((_, line)) = self.job_line() {
            line
        } else if self.notice != "Ready" && !self.notice.is_empty() {
            self.notice.clone()
        } else {
            format!("Patch {}", env!("CARGO_PKG_VERSION"))
        };
        let note_font = lexend(us(full, 14.0));
        let note_w = ui.painter().layout_no_wrap(note.clone(), note_font.clone(), GOLD).size().x;
        paint_diamond(ui.painter(), egui::pos2(play.right() - note_w - us(full, 18.0), play.top() - us(full, 16.0)), GOLD);
        ui.painter().text(egui::pos2(play.right() - us(full, 4.0), play.top() - us(full, 16.0)), egui::Align2::RIGHT_CENTER, &note, note_font, GOLD);
        if self.selected.starts_with("26.3") {
            let href = egui::Rect::from_min_size(
                egui::pos2(play.left() - us(full, 124.0), play.top()),
                egui::vec2(us(full, 112.0), play.height()),
            );
            if icon_button(ui, href, "horizon-toggle", horizon_on, "Start 26.3 with the Horizon client") {
                self.engine = if horizon_on { "vanilla".into() } else { "horizon".into() };
                self.save_settings();
            }
            ui.painter().text(href.center(), egui::Align2::CENTER_CENTER, "HORIZON", nav_font(us(full, 12.0)), egui::Color32::from_rgb(220, 232, 235));
        }
        if play_button(ui, play, label) {
            self.join = None;
            self.play();
        }
        if let Some((progress, _)) = self.job_line() {
            let bar = egui::Rect::from_min_max(
                egui::pos2(play.left(), play.bottom() - 4.0),
                egui::pos2(play.left() + play.width() * progress.clamp(0.0, 1.0), play.bottom()),
            );
            ui.painter().rect_filled(bar, 2.0, egui::Color32::from_white_alpha(180));
        }
    }

    fn channel_label(&self) -> String {
        let raw = self.versions.iter().find(|v| v.id == self.selected).map(|v| v.channel.as_str()).unwrap_or("release");
        let mut chars = raw.chars();
        match chars.next() {
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            None => "Release".into(),
        }
    }

    fn paint_player_card(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        // Top-right of the window, under the exit mark. Only the head.
        let side = us(full, 40.0).round().max(28.0);
        let head = egui::Rect::from_min_size(
            egui::pos2(ux(full, 1218.0).round(), uy(full, 65.0).round()),
            egui::vec2(side, side),
        );
        let name = if self.username.is_empty() { "Player" } else { self.username.as_str() };
        let head_hit = ui.interact(head, egui::Id::new("player-card"), Sense::click());
        if let Some(face) = &self.avatar {
            paint_head_glow(ui.painter(), head, face.id());
            ui.painter().image(face.id(), head, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(head, 1.0, egui::Color32::from_rgb(0x1A, 0x14, 0x10));
        }

        let shown = head_hit.hovered();
        let t = ui.ctx().animate_bool(egui::Id::new("player-name-rise"), shown);
        if t > 0.02 {
            let font = title_font(us(full, 14.0));
            let pos = egui::pos2(head.center().x, head.bottom() + us(full, 4.0) + (1.0 - t) * us(full, 6.0));
            let a = (t * 235.0) as u8;
            ui.painter().text(pos + egui::vec2(0.0, 1.0), egui::Align2::CENTER_TOP, name, font.clone(), egui::Color32::from_black_alpha(a / 2));
            ui.painter().text(pos, egui::Align2::CENTER_TOP, name, font, egui::Color32::from_rgba_unmultiplied(236, 228, 206, a));
        }

        if head_hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if head_hit.clicked() {
            self.tab = Tab::Settings;
        }
    }

    /// Version and play time. Display only — not a control.
    fn paint_instance_card(&self, ui: &mut egui::Ui, full: egui::Rect) {
        let card = egui::Rect::from_min_size(egui::pos2(ux(full, 26.0), uy(full, 640.0)), egui::vec2(us(full, 692.0), us(full, 52.0)));
        paint_hytale_bar(ui.painter(), card);
        let icon_rect = egui::Rect::from_center_size(egui::pos2(card.left() + us(full, 34.0), card.center().y), egui::vec2(us(full, 36.0), us(full, 36.0)));
        if let Some(icon) = self.loader_icon() {
            ui.painter().image(icon.id(), icon_rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        let name = self.current_instance().map(|i| i.name.clone()).unwrap_or_else(|| self.selected.clone());
        let detail = format!("{} · {}", crate::state::loader_label(&self.loader), self.selected);
        ui.painter().text(egui::pos2(card.left() + us(full, 62.0), card.center().y - us(full, 9.0)), egui::Align2::LEFT_CENTER, &name, title_font(us(full, 15.0)), TEXT);
        ui.painter().text(egui::pos2(card.left() + us(full, 62.0), card.center().y + us(full, 10.0)), egui::Align2::LEFT_CENTER, &detail, body_font(us(full, 13.0)), MUTED);
        paint_clock(ui.painter(), egui::pos2(card.right() - us(full, 118.0), card.center().y), GOLD);
        paint_play_stat(ui, egui::pos2(card.right() - us(full, 100.0), card.center().y), self.shown_play_ms(), us(full, 15.0));
    }

    fn account_bar(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, h: f32) {
        let name = if self.username.is_empty() { "Player" } else { &self.username };
        let rect = egui::Rect::from_min_size(origin, egui::vec2(h * 2.9, h));
        let hit = ui.interact(rect, egui::Id::new("account-chip"), Sense::click());
        glass(ui.painter(), rect, 10.0, if hit.hovered() { glass_hover() } else { glass_fill() });
        let head = egui::Rect::from_center_size(egui::pos2(rect.left() + h * 0.36, rect.center().y), egui::vec2(h * 0.42, h * 0.42));
        ui.painter().rect_filled(head, 3.0, egui::Color32::from_rgb(0x1A, 0x14, 0x10));
        if let Some(face) = &self.avatar {
            ui.painter().image(face.id(), head.shrink(1.0), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        ui.painter().text(egui::pos2(rect.left() + h * 0.66, rect.center().y), egui::Align2::LEFT_CENTER, name, lexend(h * 0.36), TEXT);
        if hit.clicked() {
            self.tab = Tab::Settings;
        }
        hit.on_hover_text("Local account");
    }

    fn nav_bar(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, h: f32) {
        let tabs = [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings];
        let mut widths = Vec::new();
        let mut total = 8.0;
        for tab in tabs {
            let galley = ui.painter().layout_no_wrap(tab.label().to_string(), tab_font(15.0), TEXT);
            let w = 18.0 + 8.0 + galley.size().x + 16.0;
            widths.push(w);
            total += w + 4.0;
        }
        total += 4.0;
        let rect = egui::Rect::from_min_size(origin, egui::vec2(total, h));
        glass(ui.painter(), rect, 12.0, glass_fill());
        let mut x = rect.left() + 6.0;
        for (i, tab) in tabs.into_iter().enumerate() {
            let item = egui::Rect::from_min_size(egui::pos2(x, rect.top() + 6.0), egui::vec2(widths[i], h - 12.0));
            let hit = ui.interact(item, egui::Id::new("nav").with(i), Sense::click());
            let on = self.tab == tab;
            if on || hit.hovered() {
                ui.painter().rect_filled(item, 8.0, if on { SELECTED } else { egui::Color32::from_white_alpha(16) });
            }
            let color = if on { TEXT } else { MUTED };
            let icon_at = egui::pos2(item.left() + 16.0, item.center().y);
            paint_material(ui.painter(), icon_at, 16.0, tab_icon(tab), color);
            ui.painter().text(egui::pos2(item.left() + 30.0, item.center().y), egui::Align2::LEFT_CENTER, tab.label(), tab_font(15.0), color);
            if hit.clicked() {
                self.tab = tab;
            }
            x += widths[i] + 4.0;
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        drag_block(ui, 52.0, |ui, rect| {
            if let Some(mark) = &self.mark {
                let r = egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 4.0), egui::vec2(40.0, 40.0));
                ui.painter().image(mark.id(), r, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            ui.painter().text(rect.min + egui::vec2(50.0, 12.0), egui::Align2::LEFT_TOP, "RUS", egui::FontId::proportional(26.0), TEXT);
        });
        ui.add_space(14.0);
        for tab in [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings] {
            if nav_button(ui, tab.label(), self.tab == tab) { self.tab = tab; }
            ui.add_space(4.0);
        }
        let gap = (ui.available_height() - 148.0).max(8.0);
        drag_block(ui, gap, |_, _| {});
        account_chip(ui, self.avatar.as_ref(), &self.username);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 8.0) / 2.0;
            if button(ui, "Minimize", w, 44.0, false) {
                ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
            }
            if button(ui, "Exit", w, 44.0, false) {
                self.quit(ui.ctx());
            }
        });
    }

    fn page_home(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_rect_before_wrap();
        let _ = ui.allocate_rect(full, Sense::hover());
        let hero_h = (full.height() - 168.0).clamp(360.0, 520.0);
        let hero = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), hero_h));
        let drag = ui.interact(hero, egui::Id::new("hero-drag"), Sense::click_and_drag());
        if drag.drag_started() { ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag); }
        if drag.double_clicked() { toggle_max(ui.ctx()); }
        let paper = self.paper_for(&self.selected);
        if let Some(tex) = self.papers.get(paper.title) {
            ui.painter().image(tex.id(), hero, cover_uv(tex.size_vec2(), hero), egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(hero, 0.0, SURFACE);
            ui.painter().text(hero.center(), egui::Align2::CENTER_CENTER, "Wallpaper is missing from this build", body_font(16.0), MUTED);
        }
        paint_fade(ui, hero);
        ui.painter().text(
            hero.min + egui::vec2(28.0, 18.0),
            egui::Align2::LEFT_TOP,
            "Collection wallpaper, loaded from minecraft.net",
            egui::FontId::proportional(13.0),
            TEXT,
        );
        let card = egui::Rect::from_min_max(
            egui::pos2(hero.left() + 24.0, hero.bottom() - 156.0),
            egui::pos2(hero.right() - 24.0, hero.bottom() - 18.0),
        );
        ui.painter().rect_filled(card, 14.0, egui::Color32::from_rgba_unmultiplied(11, 13, 10, 220));
        ui.painter().rect_stroke(card, 14.0, egui::Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(card.shrink(16.0)));
        let ui = &mut child;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 250.0);
                chip(ui, &self.selected, true);
                ui.add_space(6.0);
                ui.label(RichText::new(paper.title).size(40.0).color(TEXT));
                let installed = self.app.is_installed(&self.selected);
                ui.label(RichText::new(if installed { "Installed · runs in a sandbox" } else { "Not installed · runs in a sandbox" }).size(15.0).color(MUTED));
            });
            ui.vertical(|ui| {
                ui.set_min_width(230.0);
                ui.horizontal(|ui| {
                    let w = 104.0;
                    if button(ui, "Vanilla", w, 36.0, self.engine != "horizon") {
                        self.engine = "vanilla".into();
                        self.save_settings();
                    }
                    if button(ui, "Horizon", w, 36.0, self.engine == "horizon") {
                        self.engine = "horizon".into();
                        self.save_settings();
                    }
                });
                ui.add_space(8.0);
                let live = crate::play::status().get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                let label = if live { "Stop" } else if self.engine == "horizon" { "Play Horizon" } else if self.app.is_installed(&self.selected) { "Play" } else { "Install and play" };
                if button(ui, label, 220.0, 56.0, true) { self.play(); }
                ui.add_space(4.0);
                ui.label(RichText::new(format!("{} MB memory · {}×{} window", self.ram, self.width, self.height)).size(12.0).color(MUTED));
            });
        });
        if let Some((progress, line)) = self.job_line() {
            let bar = egui::Rect::from_min_max(egui::pos2(hero.left(), hero.bottom() - 4.0), egui::pos2(hero.left() + hero.width() * progress.clamp(0.0, 1.0), hero.bottom()));
            ui.painter().rect_filled(bar, 0.0, ACCENT);
            let _ = line;
        }

        let strip = egui::Rect::from_min_max(egui::pos2(full.left() + 28.0, hero.bottom() + 12.0), egui::pos2(full.right() - 28.0, full.bottom() - 8.0));
        let mut strip_ui = ui.new_child(egui::UiBuilder::new().max_rect(strip));
        let ui = &mut strip_ui;
        ui.horizontal(|ui| {
            ui.label(RichText::new("COLLECTIONS").size(12.0).color(ACCENT));
            ui.add_space(12.0);
            if button(ui, "Previous", 96.0, 32.0, false) && self.film > 0 { self.film -= 1; }
            if button(ui, "Next", 72.0, 32.0, false) { self.film += 1; }
        });
        ui.add_space(8.0);
        let papers = wallpapers::all();
        let visible = 5;
        if self.film + visible > papers.len() { self.film = papers.len().saturating_sub(visible); }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for paper in papers.iter().skip(self.film).take(visible) {
                let on = paper.prefixes.iter().any(|p| version_hit(&self.selected, p));
                ui.vertical(|ui| {
                    let size = egui::vec2(150.0, 96.0);
                    if let Some(tex) = self.papers.get(paper.title) {
                        let img = ui.add(egui::Image::from_texture((tex.id(), tex.size_vec2())).fit_to_exact_size(size).sense(Sense::click()));
                        let stroke = if on { egui::Stroke::new(2.0, ACCENT) } else { egui::Stroke::new(1.0, BORDER) };
                        ui.painter().rect_stroke(img.rect, 8.0, stroke, egui::StrokeKind::Inside);
                        if img.clicked() { self.select_paper(paper); }
                        img.on_hover_text(paper.title);
                    } else {
                        let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                        ui.painter().rect_filled(r, 8.0, SURFACE);
                    }
                    ui.label(RichText::new(paper.title).size(12.0).color(MUTED));
                });
            }
        });
    }

    fn select_paper(&mut self, paper: &Paper) {
        if let Some(prefix) = paper.prefixes.first() {
            if let Some(v) = self.versions.iter().find(|v| version_hit(&v.id, prefix)) {
                self.selected = v.id.clone();
                self.save_settings();
            }
        }
    }

    fn paint_exit(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        // Same corner as the reference: above and to the right of the profile head.
        let center = egui::pos2(ux(full, 1263.0), uy(full, 41.0));
        let size = us(full, 12.0).max(8.0);
        let exit = egui::Rect::from_center_size(center, egui::vec2(us(full, 18.0), us(full, 18.0)));
        let hit = ui.interact(exit, egui::Id::new("exit-version"), Sense::click());
        paint_empty_cross(ui.painter(), center, size, empty_gold(hit.hovered()));
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if hit.clicked() {
            self.quit(ui.ctx());
        }
        hit.on_hover_text("Exit");
    }

    fn paint_version_page(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        self.paint_version_list(ui, full);
        self.paint_loader_panel(ui, full);
    }

    fn paint_version_list(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let panel = drect(full, 26.0, 106.0, 719.0, 587.0);
        panel_card(ui.painter(), panel);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 48.0), uy(full, 142.0)), egui::Align2::LEFT_CENTER, "INSTANCES", sub_font(us(full, 12.0)), V_MUTED, us(full, 1.2));
        let count = format!("{}", self.instances.len());
        paint_tracked(ui.painter(), egui::pos2(ux(full, 560.0), uy(full, 142.0)), egui::Align2::RIGHT_CENTER, &count, sub_font(us(full, 12.0)), V_MUTED, us(full, 1.0));
        let new_btn = drect(full, 590.0, 118.0, 128.0, 44.0);
        if pill_action(ui, new_btn, "New", true) {
            self.draft_name.clear();
            self.draft_version = if self.selected.is_empty() { "26.3".into() } else { self.selected.clone() };
            self.draft_loader = self.loader.clone();
            self.draft_open = true;
        }

        let list = drect(full, 28.0, 174.0, 715.0, 460.0);
        let row_h = uy(full, 234.0) - uy(full, 174.0);
        let painter = ui.painter().clone();
        let rows: Vec<crate::instances::Instance> = self.instances.clone();
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
        child.set_clip_rect(list);
        let ui = &mut child;
        ui.set_min_size(list.size());
        if rows.is_empty() {
            ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, "No instances yet", body_font(us(full, 15.0)), V_MUTED);
            let _ = ui.allocate_rect(list, Sense::hover());
        } else {
            egui::ScrollArea::vertical().id_salt("instance-list").auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.set_width(list.width());
                for inst in &rows {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(list.width(), row_h), Sense::click());
                    let on = self.instance_id == inst.id;
                    if on {
                        paint_active_fade(ui.painter(), rect);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_white_alpha(10));
                    }
                    let line = egui::Rect::from_min_max(egui::pos2(rect.left() + 12.0, rect.bottom() - 1.0), egui::pos2(rect.right() - 8.0, rect.bottom()));
                    ui.painter().rect_filled(line, 0.0, V_LINE);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 24.0), rect.center().y), egui::Align2::LEFT_CENTER, &inst.name, title_font(us(full, 18.0)), V_TEXT);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 250.0), rect.center().y), egui::Align2::LEFT_CENTER, &inst.version, body_font(us(full, 15.0)), V_TEXT);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 420.0), rect.center().y), egui::Align2::LEFT_CENTER, crate::state::loader_label(&inst.loader), sub_font(us(full, 13.0)), V_GOLD);
                    let isolated = crate::instances::dir(&self.app, &inst.id).exists();
                    if isolated {
                        ui.painter().circle_filled(egui::pos2(rect.right() - us(full, 28.0), rect.center().y), us(full, 4.5), V_GREEN);
                    }
                    if resp.clicked() {
                        self.select_instance(&inst.id);
                    }
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                }
            });
        }

        let foot = drect(full, 28.0, 636.0, 715.0, 55.0);
        painter.rect_filled(egui::Rect::from_min_max(egui::pos2(panel.left() + 1.0, foot.top()), egui::pos2(panel.right() - 1.0, foot.top() + 1.0)), 0.0, V_BORDER);
        paint_tracked(&painter, egui::pos2(foot.left() + us(full, 16.0), foot.center().y), egui::Align2::LEFT_CENTER, "SELECTED", sub_font(us(full, 12.0)), V_MUTED, us(full, 1.2));
        let selected_name = self.current_instance().map(|i| i.name.clone()).unwrap_or_else(|| self.selected.clone());
        painter.text(egui::pos2(foot.left() + us(full, 108.0), foot.center().y), egui::Align2::LEFT_CENTER, &selected_name, title_font(us(full, 16.0)), V_GOLD);
        let drop = self.paper_for(&self.selected).title.to_uppercase();
        paint_tracked(&painter, egui::pos2(foot.right() - us(full, 16.0), foot.center().y), egui::Align2::RIGHT_CENTER, &drop, sub_font(us(full, 12.0)), V_MUTED, us(full, 1.0));
    }

    fn paint_instance_popup(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let dim = ui.interact(full, egui::Id::new("instance-overlay"), Sense::click());
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(150));
        let card = egui::Rect::from_center_size(full.center(), egui::vec2(520.0, 420.0));
        paint_hytale_bar(ui.painter(), card);
        if dim.clicked() {
            if let Some(at) = dim.interact_pointer_pos() {
                if !card.contains(at) {
                    self.draft_open = false;
                    return;
                }
            }
        }
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(card.shrink(18.0)));
        child.set_clip_rect(card.shrink(18.0));
        let ui = &mut child;
        ui.horizontal(|ui| {
            icon_label(ui, I_LAYERS, 20.0, V_GOLD);
            ui.label(egui::RichText::new("New instance").font(title_font(22.0)).color(TEXT));
        });
        ui.label(egui::RichText::new("A separate world, options, and mods folder. The version files stay shared.").font(sub_font(13.0)).color(MUTED));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Name").font(body_font(14.0)).color(TEXT));
        field(ui, &mut self.draft_name, ui.available_width(), "Survival, creative, test…");
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Version").font(body_font(14.0)).color(TEXT));
        let versions: Vec<String> = self.versions.iter().filter(|v| {
            if v.channel == "release" { true } else { (self.snapshots && v.channel == "snapshot") || (self.previews && v.channel == "preview") }
        }).map(|v| v.id.clone()).take(80).collect();
        let picked = self.draft_version.clone();
        egui::ScrollArea::vertical().id_salt("instance-versions").max_height(96.0).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            if versions.is_empty() {
                ui.label(egui::RichText::new("Loading versions…").font(sub_font(13.0)).color(MUTED));
            }
            for id in versions {
                let on = id == picked;
                if button(ui, &id, 120.0, 28.0, on) {
                    self.draft_version = id;
                }
            }
        });
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Loader").font(body_font(14.0)).color(TEXT));
        ui.horizontal(|ui| {
            for (id, label) in [("vanilla", "Vanilla"), ("forge", "Forge"), ("fabric", "Fabric"), ("neoforge", "NeoForge"), ("optifine", "OptiFine")] {
                if button(ui, label, 78.0, 30.0, self.draft_loader == id) {
                    self.draft_loader = id.into();
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if button(ui, "Cancel", 110.0, 36.0, false) {
                self.draft_open = false;
            }
            if button(ui, "Create", 120.0, 36.0, true) {
                let inst = crate::instances::create(&self.draft_name, &self.draft_version, &self.draft_loader);
                let _ = std::fs::create_dir_all(crate::instances::dir(&self.app, &inst.id));
                self.instance_id = inst.id.clone();
                self.selected = inst.version.clone();
                self.loader = inst.loader.clone();
                self.instances.push(inst);
                crate::instances::save(&self.app, &self.instances);
                self.save_settings();
                self.draft_open = false;
            }
        });
    }

    fn paint_loader_panel(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let panel = drect(full, 766.0, 106.0, 487.0, 587.0);
        panel_card(ui.painter(), panel);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 790.0), uy(full, 136.0)), egui::Align2::LEFT_CENTER, "LOADER", sub_font(us(full, 12.0)), V_MUTED, us(full, 1.3));
        let for_mc = format!("FOR MC {}", self.selected);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 1230.0), uy(full, 136.0)), egui::Align2::RIGHT_CENTER, &for_mc, sub_font(us(full, 13.0)), V_GOLD, us(full, 0.8));

        let mc = self.selected.clone();
        let rows = crate::loaders::rows_for(&mc, &self.loaders);
        let mut clicked: Option<String> = None;
        for (i, row) in rows.iter().enumerate() {
            let y = 168.0 + i as f32 * 78.0;
            let rect = drect(full, 768.0, y, 483.0, 78.0);
            let on = self.loader == row.id;
            let hit = ui.interact(rect, egui::Id::new("loader-row").with(row.id), Sense::click());
            if on {
                paint_active_fade(ui.painter(), rect);
            } else if hit.hovered() && self.shot.is_none() {
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_white_alpha(8));
            }
            if i + 1 < rows.len() {
                let line = egui::Rect::from_min_max(egui::pos2(rect.left() + us(full, 16.0), rect.bottom() - 1.0), egui::pos2(rect.right() - us(full, 12.0), rect.bottom()));
                ui.painter().rect_filled(line, 0.0, V_LINE);
            }
            let well = egui::Rect::from_center_size(egui::pos2(ux(full, 814.0), rect.center().y), egui::vec2(us(full, 46.0), us(full, 46.0)));
            ui.painter().rect_filled(well, us(full, 8.0), loader_well(row.id));
            if let Some(tex) = self.loader_icon_id(row.id) {
                let inner = well.shrink(us(full, 5.0));
                ui.painter().image(tex.id(), inner, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            ui.painter().text(egui::pos2(ux(full, 852.0), rect.center().y), egui::Align2::LEFT_CENTER, row.name, title_font(us(full, 18.0)), V_TEXT);
            let detail = loader_detail(row, &mc, self.loaders.ready);
            let detail_color = if row.kind == crate::loaders::Badge::Preview { V_GOLD } else { V_MUTED };
            ui.painter().text(egui::pos2(ux(full, 1004.0), rect.center().y), egui::Align2::LEFT_CENTER, &detail, body_font(us(full, 15.0)), detail_color);
            let badge = loader_badge_rect(full, rect);
            paint_badge(ui.painter(), badge, row.badge, row.kind);
            if hit.clicked() {
                clicked = Some(row.id.to_string());
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
        if let Some(id) = clicked {
            self.loader = id;
            self.remember_instance();
            self.save_settings();
        }

        let selected = rows.iter().find(|r| r.id == self.loader).cloned();
        let foot_y = uy(full, 578.0);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 790.0), foot_y), egui::Align2::LEFT_CENTER, "SELECTED LOADER", sub_font(us(full, 12.0)), V_MUTED, us(full, 1.1));
        if let Some(row) = &selected {
            let right = if row.version.is_empty() {
                row.name.to_uppercase()
            } else {
                format!("{} {}", row.name.to_uppercase(), row.version)
            };
            paint_tracked(ui.painter(), egui::pos2(ux(full, 1230.0), foot_y), egui::Align2::RIGHT_CENTER, &right, sub_font(us(full, 13.0)), V_GOLD, us(full, 0.7));
        }
        let install = drect(full, 786.0, 612.0, 448.0, 58.0);
        let label = self.install_label(selected.as_ref());
        let enabled = label == "INSTALL";
        if install_button(ui, install, &label, enabled) && enabled {
            self.on_install();
        }
    }

    fn loader_icon_id(&self, id: &str) -> Option<&egui::TextureHandle> {
        match id {
            "forge" => self.icon_forge.as_ref(),
            "fabric" => self.icon_fabric.as_ref(),
            "neoforge" => self.icon_neoforge.as_ref(),
            "optifine" => self.icon_optifine.as_ref(),
            _ => self.grass.as_ref(),
        }
    }

    fn install_label(&self, row: Option<&crate::loaders::LoaderRow>) -> String {
        let Some(row) = row else { return "INSTALL".into() };
        if row.id == "optifine" || (row.kind == crate::loaders::Badge::Preview && row.version.is_empty() && self.loaders.ready) {
            return "UNAVAILABLE".into();
        }
        if row.id == "vanilla" {
            return if self.app.is_installed(&self.selected) { "INSTALLED".into() } else { "INSTALL".into() };
        }
        if crate::loaders::is_profile_installed(&self.app, &self.selected, row.id) {
            "INSTALLED".into()
        } else {
            "INSTALL".into()
        }
    }

    fn on_install(&mut self) {
        let mc = self.selected.clone();
        let loader = self.loader.clone();
        if mc.is_empty() {
            self.notice = "Select a version".into();
            return;
        }
        if loader == "vanilla" {
            if self.app.is_installed(&mc) {
                self.notice = format!("{mc} is already installed");
            } else {
                self.start_install();
            }
            return;
        }
        if loader == "optifine" {
            self.notice = format!("OptiFine has no build for {mc}");
            return;
        }
        let version = crate::loaders::rows_for(&mc, &self.loaders).into_iter().find(|r| r.id == loader).map(|r| r.version).unwrap_or_default();
        if version.is_empty() {
            self.notice = format!("No {loader} build for {mc}");
            return;
        }
        if !self.app.is_installed(&mc) {
            self.loader_after = true;
            self.notice = format!("Installing {mc}, then {loader}");
            self.start_install();
            return;
        }
        self.spawn_loader_install();
    }

    fn spawn_loader_install(&mut self) {
        let mc = self.selected.clone();
        let loader = self.loader.clone();
        let version = crate::loaders::rows_for(&mc, &self.loaders).into_iter().find(|r| r.id == loader).map(|r| r.version).unwrap_or_default();
        let app = self.app.clone();
        let tx = self.tx.clone();
        self.notice = format!("Installing {loader} {version} for {mc}…");
        std::thread::spawn(move || {
            let msg = match crate::loaders::install(&app, &mc, &loader, &version) {
                Ok(id) => format!("{loader} is installed ({id})"),
                Err(e) => e,
            };
            let _ = tx.send(Msg::Notice(msg));
        });
    }


    fn page_mods(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icon_label(ui, I_CUBE, 22.0, TEXT);
            ui.label(RichText::new("Mod").font(title_font(22.0)).color(TEXT));
        });
        ui.label(RichText::new("The home card icon is the loader. Jars are staged when you play.").font(sub_font(14.0)).color(MUTED));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (id, label) in [("vanilla", "Vanilla"), ("forge", "Forge"), ("fabric", "Fabric"), ("neoforge", "NeoForge"), ("optifine", "OptiFine")] {
                let on = self.loader == id;
                if button(ui, label, 108.0, 40.0, on) {
                    self.loader = id.into();
                    self.save_settings();
                }
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if button(ui, "Open folder", 140.0, 44.0, false) {
                if let Err(e) = crate::dialog::open_dir(&self.app.p("mods")) { self.notice = e; }
            }
            ui.add_space(8.0);
            if button(ui, "Add .jar", 120.0, 44.0, true) {
                if let Some(path) = crate::dialog::pick_file("Add a mod", "*.jar") {
                    match crate::mods::add_path(&self.app, &path) {
                        Ok(n) => self.notice = format!("Added {n}"),
                        Err(e) => self.notice = e,
                    }
                    spawn_mods(self.app.clone(), self.tx.clone());
                } else {
                    self.notice = "Drop a .jar here, or install zenity to browse.".into();
                }
            }
        });
        ui.add_space(12.0);
        let (zone, zresp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), Sense::click());
        let hover = zresp.hovered() || ui.input(|i| !i.raw.hovered_files.is_empty());
        ui.painter().rect_filled(zone, 12.0, if hover { SURFACE2 } else { SURFACE });
        dash_rect(ui, zone);
        ui.painter().text(zone.center() + egui::vec2(0.0, -10.0), egui::Align2::CENTER_CENTER, "Drop .jar files here", body_font(16.0), TEXT);
        ui.painter().text(zone.center() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, "Choose files", body_font(13.0), egui::Color32::from_rgb(220, 186, 96));
        if zresp.clicked() {
            if let Some(path) = crate::dialog::pick_file("Add a mod", "*.jar") {
                let _ = crate::mods::add_path(&self.app, &path);
                spawn_mods(self.app.clone(), self.tx.clone());
            }
        }
        ui.add_space(12.0);
        let enabled = self.mods.iter().filter(|m| m.1).count();
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.app.p("mods").display().to_string()).font(sub_font(13.0)).color(MUTED));
            ui.add_space(8.0);
            if button(ui, "Copy path", 110.0, 36.0, false) {
                ui.output_mut(|o| o.copied_text = self.app.p("mods").display().to_string());
                self.notice = "Path copied".into();
            }
            ui.add_space(12.0);
            ui.label(RichText::new(format!("{} mods · {enabled} enabled", self.mods.len())).font(sub_font(13.0)).color(MUTED));
        });
        ui.add_space(8.0);
        let list_h = ui.available_height().max(48.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .auto_shrink([false, false])
            .scroll_bar_visibility(if self.mods.is_empty() {
                egui::scroll_area::ScrollBarVisibility::AlwaysHidden
            } else {
                egui::scroll_area::ScrollBarVisibility::AlwaysVisible
            })
            .show(ui, |ui| {
            for (name, on, size) in self.mods.clone() {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut enabled = on;
                        if check(ui, &mut enabled, &name) {
                            let _ = crate::mods::toggle(&self.app, &name);
                            spawn_mods(self.app.clone(), self.tx.clone());
                        }
                        ui.label(RichText::new(format_bytes(size)).font(sub_font(12.0)).color(MUTED));
                        if button(ui, "Remove", 90.0, 36.0, false) {
                            let _ = crate::mods::remove(&self.app, &name);
                            spawn_mods(self.app.clone(), self.tx.clone());
                        }
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icon_label(ui, I_FLARE, 22.0, TEXT);
            ui.label(RichText::new("Settings").font(title_font(22.0)).color(TEXT));
        });
        ui.add_space(2.0);
        let width = ui.available_width();
        let body_h = ui.available_height();
        egui::ScrollArea::vertical()
            .max_height(body_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
        ui.set_min_width(width);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((width - 12.0) * 0.56);
                card(ui, |ui| {
                    ui.label(RichText::new("PROFILE").font(sub_font(12.0)).color(ACCENT));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 180.0).max(140.0));
                            ui.label(RichText::new("Username").font(body_font(14.0)).color(TEXT));
                            if field(ui, &mut self.username, ui.available_width(), "Player").changed() { self.save_settings(); }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Skin").font(body_font(14.0)).color(TEXT));
                            ui.horizontal(|ui| {
                                if button(ui, "Steve", 84.0, 40.0, self.skin == "preset:steve") {
                                    self.skin = "preset:steve".into();
                                    self.preview.arms = ArmModel::Classic;
                                    self.preview.invalidate();
                                    self.avatar_key.clear();
                                    self.save_settings();
                                }
                                if button(ui, "Alex", 84.0, 40.0, self.skin == "preset:alex") {
                                    self.skin = "preset:alex".into();
                                    self.preview.arms = ArmModel::Slim;
                                    self.preview.invalidate();
                                    self.avatar_key.clear();
                                    self.save_settings();
                                }
                                if button(ui, "Import skin…", 130.0, 40.0, false) {
                                    if let Some(path) = crate::dialog::pick_file("Import skin", "*.png") {
                                        self.import_path("skin", &path);
                                    } else {
                                        self.import_error = "No file chosen. You can also drop a PNG here.".into();
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new("Cape").font(body_font(14.0)).color(TEXT));
                            ui.horizontal(|ui| {
                                if button(ui, "Rus", 84.0, 40.0, self.cape == "preset:rus") {
                                    self.cape = "preset:rus".into();
                                    self.preview.invalidate();
                                    self.save_settings();
                                }
                                if button(ui, "None", 84.0, 40.0, self.cape == "preset:none" || self.cape.is_empty()) {
                                    self.cape = "preset:none".into();
                                    self.preview.invalidate();
                                    self.save_settings();
                                }
                                if button(ui, "Import cape…", 130.0, 40.0, false) {
                                    if let Some(path) = crate::dialog::pick_file("Import cape", "*.png") {
                                        self.import_path("cape", &path);
                                    } else {
                                        self.import_error = "Cape must be a 64×32 PNG.".into();
                                    }
                                }
                            });
                            if !self.import_error.is_empty() {
                                ui.add_space(6.0);
                                ui.label(RichText::new(&self.import_error).font(sub_font(13.0)).color(ERR));
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Local account only. No Microsoft login.").font(sub_font(12.0)).color(MUTED));
                            ui.label(RichText::new(crate::state::offline_uuid(&self.username)).font(sub_font(12.0)).color(MUTED));
                        });
                        ui.vertical(|ui| {
                            let skin = self.skin_image();
                            let cape = self.cape_image();
                            if let Some(skin) = skin.as_ref() {
                                let slim = self.detected_slim();
                                self.preview.show(ui, skin, cape.as_ref(), slim);
                                if matches!(self.preview.arms, ArmModel::Classic | ArmModel::Slim) {
                                    self.save_settings();
                                }
                            } else {
                                ui.label(RichText::new("Skin file missing").color(ERR));
                            }
                        });
                    });
                });
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.set_width((width - 12.0) * 0.42);
                card(ui, |ui| {
                    ui.label(RichText::new("GAME").font(sub_font(12.0)).color(ACCENT));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Memory").font(body_font(14.0)).color(TEXT));
                        let value = format!("{} MB", self.ram);
                        let used = ui.available_width();
                        ui.add_space((used - 72.0).max(8.0));
                        ui.label(RichText::new(value).font(body_font(14.0)).color(TEXT));
                    });
                    if slider(ui, &mut self.ram, 512, 16384) { self.save_settings(); }
                    ui.add_space(2.0);
                    ui.label(RichText::new("Game window").font(body_font(14.0)).color(TEXT));
                    ui.horizontal(|ui| {
                        let mut w = self.width.to_string();
                        let mut h = self.height.to_string();
                        if field(ui, &mut w, 90.0, "1280").changed() {
                            if let Ok(n) = w.parse::<u32>() { self.width = n.clamp(640, 3840); self.save_settings(); }
                        }
                        ui.label(RichText::new("×").color(MUTED));
                        if field(ui, &mut h, 90.0, "720").changed() {
                            if let Ok(n) = h.parse::<u32>() { self.height = n.clamp(480, 2160); self.save_settings(); }
                        }
                    });
                    ui.add_space(4.0);
                    if check(ui, &mut self.snapshots, "Show snapshots (series 26 and up)") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    if check(ui, &mut self.previews, "Show previews (series 26 and up)") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    ui.add_space(2.0);
                    ui.label(RichText::new("Servers").font(body_font(14.0)).color(TEXT));
                    ui.label(RichText::new("Played servers show on Home.").font(sub_font(12.0)).color(MUTED));
                    if field(ui, &mut self.server_name, 250.0, "Name").changed() {}
                    if field(ui, &mut self.server_addr, 250.0, "host or host:port").changed() {}
                    ui.horizontal(|ui| {
                        if button(ui, "Add", 72.0, 36.0, false) {
                            self.add_server();
                        }
                        if button(ui, "Join", 72.0, 36.0, true) {
                            let name = self.server_name.clone();
                            let addr = self.server_addr.clone();
                            self.add_server();
                            if !addr.trim().is_empty() {
                                self.join_server(name, addr);
                            }
                        }
                    });
                    let rows: Vec<(String, String, String)> = self.servers.iter().map(|s| (s.name.clone(), s.address.clone(), s.icon.clone())).collect();
                    for (name, addr, icon) in rows {
                        ui.horizontal(|ui| {
                            let label = if name.is_empty() { addr.clone() } else { name.clone() };
                            ui.label(RichText::new(label).font(body_font(13.0)).color(if icon.is_empty() { MUTED } else { TEXT }));
                            if button(ui, "Play", 64.0, 32.0, false) {
                                self.join_server(name.clone(), addr.clone());
                            }
                            if button(ui, "Icon", 56.0, 32.0, false) {
                                if let Some(path) = crate::dialog::pick_file("Server icon", "*.png") {
                                    self.set_server_icon(&addr, &path);
                                }
                            }
                        });
                    }
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Data").font(body_font(13.0)).color(TEXT));
                        ui.label(RichText::new(self.app.dir.display().to_string()).font(sub_font(12.0)).color(MUTED));
                        if button(ui, "Open", 64.0, 28.0, false) {
                            if let Err(e) = crate::dialog::open_dir(&self.app.dir) { self.notice = e; }
                        }
                    });
                });
            });
        });
        });
    }
}

fn glass(painter: &egui::Painter, rect: egui::Rect, radius: f32, fill: egui::Color32) {
    let shadow = rect.translate(egui::vec2(0.0, 4.0));
    painter.rect_filled(shadow, radius, egui::Color32::from_black_alpha(70));
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.5, BORDER_BTN), egui::StrokeKind::Inside);
}

fn icon_button(ui: &mut egui::Ui, rect: egui::Rect, id: &str, on: bool, tip: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    if on || hit.hovered() {
        paint_active_fade(ui.painter(), rect);
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let clicked = hit.clicked();
    hit.on_hover_text(tip);
    clicked
}

fn play_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new("play"), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    paint_active_fade(ui.painter(), rect);
    let tri = egui::pos2(rect.left() + rect.width() * 0.22, rect.center().y);
    let s = rect.height() * 0.16;
    let ink = egui::Color32::from_rgb(220, 232, 235);
    let stroke = egui::Stroke::new(1.4, ink);
    ui.painter().line_segment([tri + egui::vec2(-s * 0.45, -s), tri + egui::vec2(s * 0.7, 0.0)], stroke);
    ui.painter().line_segment([tri + egui::vec2(s * 0.7, 0.0), tri + egui::vec2(-s * 0.45, s)], stroke);
    ui.painter().line_segment([tri + egui::vec2(-s * 0.45, s), tri + egui::vec2(-s * 0.45, -s)], stroke);
    ui.painter().text(egui::pos2(rect.left() + rect.width() * 0.62, rect.center().y), egui::Align2::CENTER_CENTER, label, nav_font(rect.height() * 0.30), ink);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn note_width(ui: &egui::Ui, text: &str) -> f32 {
    ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(13.0), MUTED).size().x
}

fn empty_gold(hot: bool) -> egui::Color32 {
    if hot {
        egui::Color32::from_rgb(255, 232, 168)
    } else {
        egui::Color32::from_rgb(236, 198, 112)
    }
}

fn paint_empty_cross(painter: &egui::Painter, center: egui::Pos2, size: f32, color: egui::Color32) {
    let h = size * 0.46;
    let stroke = egui::Stroke::new(1.15, color);
    painter.line_segment([center + egui::vec2(-h, -h), center + egui::vec2(h, h)], stroke);
    painter.line_segment([center + egui::vec2(h, -h), center + egui::vec2(-h, h)], stroke);
}

fn paint_head_glow(painter: &egui::Painter, head: egui::Rect, tex: egui::TextureId) {
    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    // Faint larger copies of the head, not a stroked frame.
    for i in (1..6).rev() {
        let grow = i as f32 * 1.35;
        let alpha = (12 + (6 - i) * 4) as u8;
        painter.image(tex, head.expand(grow), uv, egui::Color32::from_white_alpha(alpha));
    }
}

fn paint_exit_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let s = egui::Stroke::new(1.6, color);
    let box_r = egui::Rect::from_center_size(center + egui::vec2(-2.0, 1.0), egui::vec2(12.0, 14.0));
    painter.rect_stroke(box_r, 1.0, s, egui::StrokeKind::Inside);
    painter.line_segment([center + egui::vec2(-1.0, 1.0), center + egui::vec2(8.0, 1.0)], s);
    painter.line_segment([center + egui::vec2(4.0, -3.0), center + egui::vec2(8.0, 1.0)], s);
    painter.line_segment([center + egui::vec2(4.0, 5.0), center + egui::vec2(8.0, 1.0)], s);
}

fn paint_house(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    let s = egui::Stroke::new(1.6, color);
    painter.add(egui::Shape::convex_polygon(
        vec![c + egui::vec2(0.0, -7.0), c + egui::vec2(-8.0, 0.0), c + egui::vec2(8.0, 0.0)],
        egui::Color32::TRANSPARENT,
        s,
    ));
    painter.line_segment([c + egui::vec2(-6.0, 0.0), c + egui::vec2(-6.0, 7.0)], s);
    painter.line_segment([c + egui::vec2(6.0, 0.0), c + egui::vec2(6.0, 7.0)], s);
    painter.line_segment([c + egui::vec2(-6.0, 7.0), c + egui::vec2(6.0, 7.0)], s);
}

fn paint_layers(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    let s = egui::Stroke::new(1.5, color);
    for dy in [-5.0, 0.0, 5.0] {
        painter.line_segment([c + egui::vec2(-7.0, dy), c + egui::vec2(7.0, dy)], s);
    }
}

fn paint_cube(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    let s = egui::Stroke::new(1.5, color);
    let pts = [
        c + egui::vec2(0.0, -7.0),
        c + egui::vec2(7.0, -3.0),
        c + egui::vec2(7.0, 4.0),
        c + egui::vec2(0.0, 8.0),
        c + egui::vec2(-7.0, 4.0),
        c + egui::vec2(-7.0, -3.0),
    ];
    for i in 0..6 {
        painter.line_segment([pts[i], pts[(i + 1) % 6]], s);
    }
    painter.line_segment([pts[0], pts[3]], s);
    painter.line_segment([pts[1], c], s);
    painter.line_segment([pts[5], c], s);
}

fn paint_gear(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    painter.circle_stroke(c, 4.0, egui::Stroke::new(1.6, color));
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let inner = egui::vec2(a.cos(), a.sin()) * 6.0;
        let outer = egui::vec2(a.cos(), a.sin()) * 8.5;
        painter.line_segment([c + inner, c + outer], egui::Stroke::new(1.6, color));
    }
}

fn paint_clock(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    painter.circle_stroke(c, 7.0, egui::Stroke::new(1.5, color));
    painter.line_segment([c, c + egui::vec2(0.0, -4.0)], egui::Stroke::new(1.5, color));
    painter.line_segment([c, c + egui::vec2(3.0, 1.0)], egui::Stroke::new(1.5, color));
}

fn paint_diamond(painter: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    painter.add(egui::Shape::convex_polygon(
        vec![c + egui::vec2(0.0, -5.0), c + egui::vec2(4.0, 0.0), c + egui::vec2(0.0, 5.0), c + egui::vec2(-4.0, 0.0)],
        color,
        egui::Stroke::NONE,
    ));
}

fn paint_creature(painter: &egui::Painter, c: egui::Pos2) {
    let green = egui::Color32::from_rgb(0x3D, 0xD6, 0x6A);
    painter.circle_filled(c + egui::vec2(0.0, 1.0), 8.0, green);
    painter.circle_filled(c + egui::vec2(-3.0, -6.0), 2.2, green);
    painter.circle_filled(c + egui::vec2(2.0, -7.0), 1.6, green);
    painter.circle_filled(c + egui::vec2(-2.0, 0.0), 1.3, egui::Color32::from_rgb(0x0C, 0x2A, 0x14));
    painter.circle_filled(c + egui::vec2(3.0, 1.0), 1.1, egui::Color32::from_rgb(0x0C, 0x2A, 0x14));
}

fn paint_crest(painter: &egui::Painter, c: egui::Pos2) {
    let gold = GOLD;
    painter.circle_stroke(c, 9.0, egui::Stroke::new(1.6, gold));
    painter.circle_filled(c, 3.0, egui::Color32::from_rgb(0x7D, 0xD3, 0x6A));
    painter.line_segment([c + egui::vec2(-5.0, 2.0), c + egui::vec2(0.0, -4.0)], egui::Stroke::new(1.4, gold));
    painter.line_segment([c + egui::vec2(5.0, 2.0), c + egui::vec2(0.0, -4.0)], egui::Stroke::new(1.4, gold));
}

fn grass_texture(ctx: &egui::Context) -> egui::TextureHandle {
    // Front-facing grass block. Original pixels, not a Mojang texture.
    let mut img = image::RgbaImage::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let grass = y < 6;
            let edge = x == 0 || x == 15 || y == 0 || y == 15 || y == 5;
            let c = if grass {
                if edge { [0x2E, 0x62, 0x1C] } else if (x + y) % 4 == 0 { [0x8E, 0xD2, 0x52] } else { [0x5C, 0xA8, 0x36] }
            } else if edge {
                [0x3E, 0x2A, 0x1A]
            } else if (x * 3 + y) % 5 == 0 {
                [0xA6, 0x74, 0x48]
            } else {
                [0x7A, 0x54, 0x34]
            };
            img.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], 255]));
        }
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([16, 16], img.as_raw());
    ctx.load_texture("grass-block", color, egui::TextureOptions::NEAREST)
}

fn account_chip(ui: &mut egui::Ui, avatar: Option<&egui::TextureHandle>, name: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), Sense::hover());
    ui.painter().rect_filled(rect, 12.0, SURFACE2);
    ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(1.0, BORDER_BTN), egui::StrokeKind::Inside);
    if let Some(tex) = avatar {
        let face = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(40.0, 40.0));
        ui.painter().image(tex.id(), face, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
    ui.painter().text(rect.min + egui::vec2(56.0, 12.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(15.0), TEXT);
    ui.painter().text(rect.min + egui::vec2(56.0, 32.0), egui::Align2::LEFT_TOP, "Local account", egui::FontId::proportional(12.0), MUTED);
}

fn nav_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
    if on || resp.hovered() { ui.painter().rect_filled(rect, 8.0, SELECTED); }
    let mark = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 18.0), egui::vec2(8.0, 8.0));
    ui.painter().rect_filled(mark, 1.0, if on { ACCENT } else { MARKER_OFF });
    ui.painter().text(rect.left_center() + egui::vec2(28.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(15.0), if on { TEXT } else { MUTED });
    if resp.has_focus() { ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Inside); }
    resp.clicked()
}

fn button(ui: &mut egui::Ui, label: &str, width: f32, height: f32, primary: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    if primary || resp.hovered() {
        paint_active_fade(ui.painter(), rect);
    }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(13.0), egui::Color32::from_rgb(220, 232, 235));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn chip(ui: &mut egui::Ui, label: &str, on: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 26.0), Sense::hover());
    ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(1.0, if on { ACCENT } else { BORDER_BTN }), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), if on { ACCENT } else { MUTED });
}

fn field(ui: &mut egui::Ui, text: &mut String, width: f32, hint: &str) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(width)
            .min_size(egui::vec2(width, 32.0))
            .font(egui::TextStyle::Body)
            .margin(egui::vec2(8.0, 6.0)),
    )
}

fn check(ui: &mut egui::Ui, on: &mut bool, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(180.0), 22.0), Sense::click());
    let box_r = egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 6.0), egui::vec2(18.0, 18.0));
    ui.painter().rect_filled(box_r, 3.0, BG);
    ui.painter().rect_stroke(box_r, 3.0, egui::Stroke::new(1.5, if *on { ACCENT } else { BORDER_INPUT }), egui::StrokeKind::Inside);
    if *on { ui.painter().rect_filled(box_r.shrink(4.0), 2.0, ACCENT); }
    ui.painter().text(rect.min + egui::vec2(28.0, 6.0), egui::Align2::LEFT_TOP, label, body_font(14.0), TEXT);
    if resp.has_focus() { ui.painter().rect_stroke(box_r.expand(2.0), 3.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Outside); }
    if resp.clicked() { *on = !*on; true } else { false }
}

fn slider(ui: &mut egui::Ui, value: &mut u32, min: u32, max: u32) -> bool {
    let width = ui.available_width().max(160.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 20.0), Sense::click_and_drag());
    let track = egui::Rect::from_min_max(egui::pos2(rect.left(), rect.center().y - 3.0), egui::pos2(rect.right(), rect.center().y + 3.0));
    ui.painter().rect_filled(track, 3.0, BORDER_INPUT);
    let t = (*value as f32 - min as f32) / (max as f32 - min as f32);
    let fill = egui::Rect::from_min_max(track.min, egui::pos2(track.left() + track.width() * t, track.bottom()));
    ui.painter().rect_filled(fill, 3.0, egui::Color32::from_rgb(176, 138, 62));
    let hx = track.left() + track.width() * t;
    ui.painter().circle_filled(egui::pos2(hx, track.center().y), 7.0, egui::Color32::from_rgb(220, 186, 96));
    ui.painter().circle_stroke(egui::pos2(hx, track.center().y), 7.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 176, 198)));
    if (resp.dragged() || resp.clicked()) && resp.interact_pointer_pos().is_some() {
        let p = resp.interact_pointer_pos().unwrap();
        let nt = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let raw = min as f32 + nt * (max - min) as f32;
        *value = ((raw / 256.0).round() * 256.0) as u32;
        *value = (*value).clamp(min, max);
        return true;
    }
    false
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(37, 53, 86))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 176, 198)))
        .corner_radius(2.0)
        .inner_margin(14.0)
        .show(ui, add);
}

fn dash_rect(ui: &egui::Ui, rect: egui::Rect) {
    let color = BORDER_INPUT;
    let step = 8.0;
    let mut x = rect.left();
    while x < rect.right() {
        let x2 = (x + 5.0).min(rect.right());
        ui.painter().line_segment([egui::pos2(x, rect.top()), egui::pos2(x2, rect.top())], egui::Stroke::new(1.5, color));
        ui.painter().line_segment([egui::pos2(x, rect.bottom()), egui::pos2(x2, rect.bottom())], egui::Stroke::new(1.5, color));
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        let y2 = (y + 5.0).min(rect.bottom());
        ui.painter().line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.left(), y2)], egui::Stroke::new(1.5, color));
        ui.painter().line_segment([egui::pos2(rect.right(), y), egui::pos2(rect.right(), y2)], egui::Stroke::new(1.5, color));
        y += step;
    }
}

fn drag_block(ui: &mut egui::Ui, height: f32, paint: impl FnOnce(&mut egui::Ui, egui::Rect)) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click_and_drag());
    paint(ui, rect);
    if resp.drag_started() { ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag); }
    if resp.double_clicked() { toggle_max(ui.ctx()); }
}

fn toggle_max(ctx: &egui::Context) {
    let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maxed));
}

fn paint_fade(ui: &egui::Ui, rect: egui::Rect) {
    let top = rect.top() + rect.height() * 0.45;
    for i in 0..14 {
        let t0 = i as f32 / 14.0;
        let t1 = (i + 1) as f32 / 14.0;
        let y0 = top + (rect.bottom() - top) * t0;
        let y1 = top + (rect.bottom() - top) * t1;
        let band = egui::Rect::from_min_max(egui::pos2(rect.left(), y0), egui::pos2(rect.right(), y1));
        ui.painter().rect_filled(band, 0.0, egui::Color32::from_black_alpha((t1 * 190.0) as u8));
    }
}

fn paint_frame(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("frame")))
        .rect_stroke(rect, 0.0, egui::Stroke::new(2.0, BORDER_BTN), egui::StrokeKind::Inside);
}

fn window_edges(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let t = 6.0;
    let c = 14.0;
    let zones = [
        (ResizeDirection::NorthWest, egui::Rect::from_min_size(rect.min, egui::vec2(c, c)), CursorIcon::ResizeNorthWest),
        (ResizeDirection::NorthEast, egui::Rect::from_min_size(egui::pos2(rect.right() - c, rect.top()), egui::vec2(c, c)), CursorIcon::ResizeNorthEast),
        (ResizeDirection::SouthWest, egui::Rect::from_min_size(egui::pos2(rect.left(), rect.bottom() - c), egui::vec2(c, c)), CursorIcon::ResizeSouthWest),
        (ResizeDirection::SouthEast, egui::Rect::from_min_size(rect.max - egui::vec2(c, c), egui::vec2(c, c)), CursorIcon::ResizeSouthEast),
        (ResizeDirection::North, egui::Rect::from_min_max(egui::pos2(rect.left() + c, rect.top()), egui::pos2(rect.right() - c, rect.top() + t)), CursorIcon::ResizeNorth),
        (ResizeDirection::South, egui::Rect::from_min_max(egui::pos2(rect.left() + c, rect.bottom() - t), egui::pos2(rect.right() - c, rect.bottom())), CursorIcon::ResizeSouth),
        (ResizeDirection::West, egui::Rect::from_min_max(egui::pos2(rect.left(), rect.top() + c), egui::pos2(rect.left() + t, rect.bottom() - c)), CursorIcon::ResizeWest),
        (ResizeDirection::East, egui::Rect::from_min_max(egui::pos2(rect.right() - t, rect.top() + c), egui::pos2(rect.right(), rect.bottom() - c)), CursorIcon::ResizeEast),
    ];
    egui::Area::new(egui::Id::new("window-edges")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        for (i, (dir, zone, cursor)) in zones.iter().enumerate() {
            let resp = ui.interact(*zone, egui::Id::new("edge").with(i), Sense::click_and_drag());
            if resp.hovered() || resp.dragged() { ui.ctx().set_cursor_icon(*cursor); }
            if resp.drag_started() { ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(*dir)); }
        }
    });
}

fn paint_play_stat(ui: &egui::Ui, origin: egui::Pos2, ms: u128, num_size: f32) {
    let (number, unit, hours) = crate::state::play_label(ms);
    let num_font = lexend(num_size);
    ui.painter().text(origin, egui::Align2::LEFT_CENTER, &number, num_font.clone(), TEXT);
    let width = ui.painter().layout_no_wrap(number, num_font, TEXT).size().x;
    let unit_size = if hours { num_size * 1.35 } else { num_size * 0.68 };
    let unit_at = origin + egui::vec2(width + 4.0, if hours { -1.0 } else { 2.0 });
    ui.painter().text(unit_at, egui::Align2::LEFT_CENTER, unit, lexend(unit_size), if hours { TEXT } else { MUTED });
}

fn load_playtime(app: &App) -> HashMap<String, u128> {
    let Ok(text) = std::fs::read_to_string(app.p("playtime.json")) else { return HashMap::new(); };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { return HashMap::new(); };
    let Some(obj) = value.as_object() else { return HashMap::new(); };
    obj.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as u128))).collect()
}

fn save_playtime(app: &App, times: &HashMap<String, u128>) {
    let value: serde_json::Map<String, serde_json::Value> = times.iter().map(|(k, v)| (k.clone(), serde_json::json!(v))).collect();
    let _ = std::fs::write(app.p("playtime.json"), serde_json::to_string(&value).unwrap_or_default());
}

fn contain_rect(src: egui::Vec2, dst: egui::Rect) -> egui::Rect {
    if src.x <= 0.0 || src.y <= 0.0 || dst.width() <= 0.0 || dst.height() <= 0.0 {
        return dst;
    }
    let scale = (dst.width() / src.x).min(dst.height() / src.y);
    egui::Rect::from_center_size(dst.center(), src * scale)
}

fn cover_uv(src: egui::Vec2, dst: egui::Rect) -> egui::Rect {
    if src.x <= 0.0 || src.y <= 0.0 || dst.width() <= 0.0 || dst.height() <= 0.0 {
        return egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    }
    let src_aspect = src.x / src.y;
    let dst_aspect = dst.width() / dst.height();
    if src_aspect > dst_aspect {
        let visible = dst_aspect / src_aspect;
        let x = (1.0 - visible) * 0.5;
        egui::Rect::from_min_max(egui::pos2(x, 0.0), egui::pos2(x + visible, 1.0))
    } else {
        let visible = src_aspect / dst_aspect;
        let y = (1.0 - visible) * 0.5;
        egui::Rect::from_min_max(egui::pos2(0.0, y), egui::pos2(1.0, y + visible))
    }
}

fn title_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Title".into()))
}

fn tab_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Tab".into()))
}

fn body_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Body".into()))
}

fn sub_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Sub".into()))
}

fn icon_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Icons".into()))
}

fn lexend(size: f32) -> egui::FontId {
    title_font(size)
}

fn nav_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Nav".into()))
}

fn install_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Faces downloaded from hytale.com: Lexend and Nunito Sans, every weight the site declares.
    fonts.font_data.insert("Lexend300".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-300.ttf")).into());
    fonts.font_data.insert("Lexend400".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-400.ttf")).into());
    fonts.font_data.insert("Lexend500".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-500.ttf")).into());
    fonts.font_data.insert("Lexend600".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-600.ttf")).into());
    fonts.font_data.insert("Lexend700".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-700.ttf")).into());
    fonts.font_data.insert("Lexend800".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/Lexend-800.ttf")).into());
    fonts.font_data.insert("Nunito300".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-300.ttf")).into());
    fonts.font_data.insert("Nunito400".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-400.ttf")).into());
    fonts.font_data.insert("Nunito500".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-500.ttf")).into());
    fonts.font_data.insert("Nunito600".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-600.ttf")).into());
    fonts.font_data.insert("Nunito700".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-700.ttf")).into());
    fonts.font_data.insert("Nunito800".into(), egui::FontData::from_static(include_bytes!("../assets/hytale/NunitoSans-800.ttf")).into());
    fonts.font_data.insert("Material".into(), egui::FontData::from_static(include_bytes!("../assets/MaterialSymbols.ttf")).into());
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "Nunito400".into());
    fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "Nunito400".into());
    fonts.families.insert(egui::FontFamily::Name("Title".into()), vec!["Lexend700".into(), "Nunito700".into()]);
    fonts.families.insert(egui::FontFamily::Name("Tab".into()), vec!["Lexend600".into(), "Nunito600".into()]);
    // hytale.com #navigation-links is Lexend, and the header links are extra-bold.
    fonts.families.insert(egui::FontFamily::Name("Nav".into()), vec!["Lexend800".into(), "Nunito800".into()]);
    fonts.families.insert(egui::FontFamily::Name("Body".into()), vec!["Nunito500".into(), "Lexend500".into()]);
    fonts.families.insert(egui::FontFamily::Name("Sub".into()), vec!["Nunito300".into(), "Lexend300".into()]);
    fonts.families.insert(egui::FontFamily::Name("Icons".into()), vec!["Material".into()]);
    ctx.set_fonts(fonts);
}

fn ux(full: egui::Rect, x: f32) -> f32 { full.left() + x * full.width() / 1280.0 }
fn uy(full: egui::Rect, y: f32) -> f32 { full.top() + y * full.height() / 720.0 }
fn us(full: egui::Rect, v: f32) -> f32 { v * (full.width() / 1280.0).min(full.height() / 720.0) }

fn icon_file(address: &str) -> String {
    let slug: String = address.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("{slug}.png")
}

fn server_badge(ui: &mut egui::Ui, rect: egui::Rect, tex: Option<&egui::TextureHandle>, id: &str, tip: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    glass(ui.painter(), rect, 12.0, if hit.hovered() { glass_hover() } else { glass_fill() });
    let inner = rect.shrink(6.0);
    if let Some(tex) = tex {
        ui.painter().image(tex.id(), inner, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    } else {
        let letter = tip.chars().next().unwrap_or('?').to_uppercase().to_string();
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, letter, lexend(rect.height() * 0.42), GOLD);
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let clicked = hit.clicked();
    hit.on_hover_text(tip);
    clicked
}

fn image_button(ui: &mut egui::Ui, rect: egui::Rect, tex: Option<&egui::TextureHandle>, id: &str, tip: &str, on: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    if let Some(tex) = tex {
        let tint = if hit.hovered() { egui::Color32::from_rgb(235, 235, 235) } else { egui::Color32::WHITE };
        ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint);
    } else {
        glass(ui.painter(), rect, 8.0, glass_fill());
    }
    if on {
        ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Outside);
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let clicked = hit.clicked();
    hit.on_hover_text(tip);
    clicked
}

fn style_ui(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = BG;
    style.visuals.extreme_bg_color = BG;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(92, 74, 36);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 170, 70));
    let radius = egui::CornerRadius::same(2);
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.corner_radius = radius;
        w.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 176, 198));
        w.fg_stroke.color = egui::Color32::from_rgb(220, 232, 235);
    }
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(28, 40, 64);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(70, 58, 32);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(120, 96, 42);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(120, 96, 42);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(220, 232, 235);
    style.visuals.window_corner_radius = radius;
    style.visuals.menu_corner_radius = radius;
    style.visuals.widgets.inactive.bg_fill = MARKER_OFF;
    style.spacing.scroll.floating = false;
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.foreground_color = true;
    ctx.set_style(style);
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let opts = if name == "mark" || name == "avatar" || name == "grass" { egui::TextureOptions::NEAREST } else { egui::TextureOptions::LINEAR };
    Some(ctx.load_texture(name, color, opts))
}

fn save_png(path: &str, image: &egui::ColorImage) -> Result<(), String> {
    let img = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, image.as_raw().to_vec()).ok_or("bad image")?;
    if let Some(parent) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(parent); }
    img.save(path).map_err(|e| e.to_string())
}

fn version_hit(id: &str, prefix: &str) -> bool {
    id == prefix || id.starts_with(&format!("{prefix}.")) || id.starts_with(&format!("{prefix}-"))
}

fn format_bytes(n: u64) -> String {
    if n < 1024 { format!("{n} B") } else if n < 1024 * 1024 { format!("{} KB", n / 1024) } else { format!("{:.1} MB", n as f64 / 1_048_576.0) }
}

const V_PANEL: egui::Color32 = egui::Color32::from_rgb(13, 22, 39);
const V_BORDER: egui::Color32 = egui::Color32::from_rgb(42, 58, 88);
const V_SEL: egui::Color32 = egui::Color32::from_rgb(26, 43, 73);
const V_LINE: egui::Color32 = egui::Color32::from_rgb(30, 42, 64);
const V_GOLD: egui::Color32 = egui::Color32::from_rgb(232, 195, 104);
const V_GOLD_EDGE: egui::Color32 = egui::Color32::from_rgb(168, 140, 72);
const V_MUTED: egui::Color32 = egui::Color32::from_rgb(126, 146, 174);
const V_TEXT: egui::Color32 = egui::Color32::from_rgb(242, 246, 252);
const V_GREEN: egui::Color32 = egui::Color32::from_rgb(93, 192, 74);
const V_GREEN_INK: egui::Color32 = egui::Color32::from_rgb(16, 36, 18);
const V_TAB: egui::Color32 = egui::Color32::from_rgb(15, 25, 44);
const V_TAB_ON: egui::Color32 = egui::Color32::from_rgb(27, 43, 72);
const V_TAB_EDGE: egui::Color32 = egui::Color32::from_rgb(38, 52, 79);
const V_TAB_ON_EDGE: egui::Color32 = egui::Color32::from_rgb(58, 77, 112);
const V_RELEASE: egui::Color32 = egui::Color32::from_rgb(44, 46, 48);
const V_BLUE: egui::Color32 = egui::Color32::from_rgb(74, 131, 234);
const V_PREVIEW: egui::Color32 = egui::Color32::from_rgb(110, 78, 40);
const V_PREVIEW_INK: egui::Color32 = egui::Color32::from_rgb(242, 220, 180);

const I_HOME: char = '\u{E88A}';
const I_LAYERS: char = '\u{E53B}';
const I_CUBE: char = '\u{F720}';
const I_FLARE: char = '\u{E3E4}';
const I_CLOCK: char = '\u{E8B5}';
const I_DOWN: char = '\u{E5DB}';
const I_FOLDER: char = '\u{E2C7}';
const I_ADD: char = '\u{E145}';
const I_CLOSE: char = '\u{E5CD}';

fn drect(full: egui::Rect, x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(ux(full, x), uy(full, y)), egui::pos2(ux(full, x + w), uy(full, y + h)))
}

fn tab_icon(tab: Tab) -> char {
    match tab {
        Tab::Home => I_HOME,
        Tab::Versions => I_LAYERS,
        Tab::Mods => I_CUBE,
        Tab::Settings => I_FLARE,
    }
}

fn paint_material(painter: &egui::Painter, center: egui::Pos2, size: f32, icon: char, color: egui::Color32) {
    painter.text(center, egui::Align2::CENTER_CENTER, icon.to_string(), icon_font(size), color);
}

fn use_page_fonts(ui: &mut egui::Ui) {
    let styles = &mut ui.style_mut().text_styles;
    styles.insert(egui::TextStyle::Heading, title_font(22.0));
    styles.insert(egui::TextStyle::Body, body_font(14.0));
    styles.insert(egui::TextStyle::Button, body_font(14.0));
    styles.insert(egui::TextStyle::Small, sub_font(12.0));
    styles.insert(egui::TextStyle::Monospace, sub_font(12.0));
}

fn icon_label(ui: &mut egui::Ui, icon: char, size: f32, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size + 6.0, size + 4.0), Sense::hover());
    paint_material(ui.painter(), rect.center(), size, icon, color);
}

fn panel_card(painter: &egui::Painter, rect: egui::Rect) {
    paint_hytale_bar(painter, rect);
}

fn paint_hytale_bar(painter: &egui::Painter, rect: egui::Rect) {
    let fill = egui::Color32::from_rgb(37, 53, 86);
    let edge = egui::Color32::from_rgb(158, 176, 198);
    painter.rect_filled(rect, 1.5, fill);
    painter.rect_stroke(rect, 1.5, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    let plate = egui::Color32::from_rgb(186, 200, 216);
    let s = 3.4;
    for (cx, cy) in [
        (rect.left(), rect.top()),
        (rect.right(), rect.top()),
        (rect.left(), rect.bottom()),
        (rect.right(), rect.bottom()),
    ] {
        painter.rect_stroke(
            egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(s, s)),
            0.0,
            egui::Stroke::new(1.15, plate),
            egui::StrokeKind::Inside,
        );
    }
}

fn paint_active_fade(painter: &egui::Painter, rect: egui::Rect) {
    let fade = painter.with_clip_rect(rect.shrink(0.5));
    if rect.width() > rect.height() * 1.8 {
        let steps = (rect.width() / 1.6).ceil().clamp(28.0, 72.0) as i32;
        for i in 0..steps {
            let t0 = i as f32 / steps as f32;
            let t1 = (i + 1) as f32 / steps as f32;
            let dist = ((t0 + t1) * 0.5 - 0.5).abs() * 2.0;
            let k = (1.0 - dist).powf(1.45);
            let r = 37.0 + 78.0 * k;
            let g = 53.0 + 46.0 * k;
            let b = 86.0 - 22.0 * k;
            let band = egui::Rect::from_min_max(
                egui::pos2(rect.left() + rect.width() * t0, rect.top()),
                egui::pos2(rect.left() + rect.width() * t1, rect.bottom()),
            );
            fade.rect_filled(band, 0.0, egui::Color32::from_rgb(r as u8, g as u8, b as u8));
        }
        return;
    }
    let center = rect.center();
    let reach = rect.height() * 0.95;
    let steps = 16;
    for i in (0..steps).rev() {
        let t = i as f32 / (steps as f32 - 1.0);
        let k = (1.0 - t).powf(1.6);
        let r = 37.0 + 96.0 * k;
        let g = 53.0 + 54.0 * k;
        let b = 86.0 - 30.0 * k;
        fade.circle_filled(center, reach * (0.2 + 0.8 * t), egui::Color32::from_rgb(r as u8, g as u8, b as u8));
    }
}

fn paint_nav_house(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    let stroke = egui::Stroke::new((s * 0.09).max(1.15), color);
    let peak = c + egui::vec2(0.0, -s * 0.56);
    let eave_l = c + egui::vec2(-s * 0.58, -s * 0.05);
    let eave_r = c + egui::vec2(s * 0.58, -s * 0.05);
    painter.line_segment([eave_l, peak], stroke);
    painter.line_segment([peak, eave_r], stroke);
    painter.line_segment([eave_l, eave_r], stroke);
    let left = c + egui::vec2(-s * 0.38, -s * 0.02);
    let right = c + egui::vec2(s * 0.38, -s * 0.02);
    let foot_l = c + egui::vec2(-s * 0.38, s * 0.50);
    let foot_r = c + egui::vec2(s * 0.38, s * 0.50);
    painter.line_segment([left, foot_l], stroke);
    painter.line_segment([right, foot_r], stroke);
    let door = s * 0.15;
    painter.line_segment([foot_l, c + egui::vec2(-door, s * 0.50)], stroke);
    painter.line_segment([c + egui::vec2(door, s * 0.50), foot_r], stroke);
}

fn tracked_width(painter: &egui::Painter, text: &str, font: egui::FontId, tracking: f32) -> f32 {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    for ch in &chars {
        total += painter.layout_no_wrap(ch.to_string(), font.clone(), egui::Color32::WHITE).size().x;
    }
    if chars.len() > 1 {
        total += tracking * (chars.len() - 1) as f32;
    }
    total
}

fn pill_tab(ui: &mut egui::Ui, rect: egui::Rect, tab: Tab, on: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new("vtab").with(tab.label()), Sense::click());
    let fill = if on {
        V_TAB_ON
    } else if hit.hovered() {
        egui::Color32::from_rgb(22, 34, 56)
    } else {
        V_TAB
    };
    let edge = if on { V_TAB_ON_EDGE } else { V_TAB_EDGE };
    ui.painter().rect_filled(rect, 10.0, fill);
    ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    let color = if on { V_TEXT } else { V_MUTED };
    let icon_at = egui::pos2(rect.left() + rect.height() * 0.42, rect.center().y);
    paint_material(ui.painter(), icon_at, rect.height() * 0.36, tab_icon(tab), color);
    ui.painter().text(egui::pos2(icon_at.x + rect.height() * 0.34, rect.center().y), egui::Align2::LEFT_CENTER, tab.label(), tab_font(rect.height() * 0.30), color);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn segment_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, on: bool, id: &str) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    if on {
        ui.painter().rect_filled(rect, 8.0, V_RELEASE);
        ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.5, V_GOLD_EDGE), egui::StrokeKind::Inside);
        paint_tracked(ui.painter(), rect.center(), egui::Align2::CENTER_CENTER, label, sub_font(rect.height() * 0.30), V_GOLD, 1.1);
    } else {
        ui.painter().rect_filled(rect, 8.0, V_PANEL);
        ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, V_BORDER), egui::StrokeKind::Inside);
        paint_tracked(ui.painter(), rect.center(), egui::Align2::CENTER_CENTER, label, sub_font(rect.height() * 0.30), V_MUTED, 1.1);
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn paint_tracked(painter: &egui::Painter, pos: egui::Pos2, align: egui::Align2, text: &str, font: egui::FontId, color: egui::Color32, tracking: f32) {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return;
    }
    let mut widths = Vec::new();
    let mut total = 0.0;
    for ch in &chars {
        let w = painter.layout_no_wrap(ch.to_string(), font.clone(), color).size().x;
        widths.push(w);
        total += w;
    }
    if chars.len() > 1 {
        total += tracking * (chars.len() - 1) as f32;
    }
    let mut x = if align.x() == egui::Align::RIGHT {
        pos.x - total
    } else if align.x() == egui::Align::Center {
        pos.x - total * 0.5
    } else {
        pos.x
    };
    for (ch, w) in chars.iter().zip(widths) {
        painter.text(egui::pos2(x, pos.y), egui::Align2::LEFT_CENTER, ch.to_string(), font.clone(), color);
        x += w + tracking;
    }
}

fn loader_well(id: &str) -> egui::Color32 {
    match id {
        "forge" => egui::Color32::from_rgb(8, 8, 8),
        "neoforge" => egui::Color32::from_rgb(186, 196, 204),
        "optifine" => egui::Color32::from_rgb(92, 28, 32),
        "fabric" => egui::Color32::from_rgb(28, 32, 42),
        _ => egui::Color32::from_rgb(18, 26, 40),
    }
}

fn loader_detail(row: &crate::loaders::LoaderRow, mc: &str, ready: bool) -> String {
    if row.id == "optifine" {
        return crate::loaders::missing_line(mc);
    }
    if row.version.is_empty() {
        if ready {
            crate::loaders::missing_line(mc)
        } else {
            "…".into()
        }
    } else {
        row.detail.clone()
    }
}

fn loader_badge_rect(full: egui::Rect, row: egui::Rect) -> egui::Rect {
    let w = us(full, 78.0);
    let h = us(full, 22.0);
    egui::Rect::from_min_size(egui::pos2(row.right() - w - us(full, 16.0), row.center().y - h * 0.5), egui::vec2(w, h))
}

fn paint_badge(painter: &egui::Painter, rect: egui::Rect, label: &str, kind: crate::loaders::Badge) {
    let (fill, ink) = match kind {
        crate::loaders::Badge::BuiltIn | crate::loaders::Badge::Ready => (V_GREEN, V_GREEN_INK),
        crate::loaders::Badge::Preview => (V_PREVIEW, V_PREVIEW_INK),
    };
    painter.rect_filled(rect, rect.height() * 0.35, fill);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, sub_font(rect.height() * 0.52), ink);
}

fn install_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, enabled: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new("loader-install"), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    if enabled {
        paint_active_fade(ui.painter(), rect);
    }
    let ink = if enabled { egui::Color32::from_rgb(220, 232, 235) } else { V_MUTED };
    if enabled {
        paint_material(ui.painter(), egui::pos2(rect.center().x - us_rect(rect, 54.0), rect.center().y), rect.height() * 0.36, I_DOWN, ink);
    }
    ui.painter().text(egui::pos2(rect.center().x + if enabled { 16.0 } else { 0.0 }, rect.center().y), egui::Align2::CENTER_CENTER, label, nav_font(rect.height() * 0.32), ink);
    if hit.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked() && enabled
}

fn pill_action(ui: &mut egui::Ui, rect: egui::Rect, label: &str, primary: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new("pill-action").with(label), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    if primary || hit.hovered() {
        paint_active_fade(ui.painter(), rect);
    }
    let ink = egui::Color32::from_rgb(220, 232, 235);
    paint_material(ui.painter(), egui::pos2(rect.left() + rect.height() * 0.42, rect.center().y), rect.height() * 0.34, I_ADD, ink);
    ui.painter().text(egui::pos2(rect.center().x + 8.0, rect.center().y), egui::Align2::CENTER_CENTER, label, nav_font(rect.height() * 0.32), ink);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

/// Same two install colors, blended top to bottom instead of a hard split.
fn paint_vgradient(painter: &egui::Painter, rect: egui::Rect, radius: f32, top: egui::Color32, bot: egui::Color32) {
    let steps = 24;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1;
        let mid = (t0 + t1) * 0.5;
        let color = lerp_color(top, bot, mid);
        let inset = rounded_inset(rect, radius, y0, y1);
        let band = egui::Rect::from_min_max(egui::pos2(rect.left() + inset, y0), egui::pos2(rect.right() - inset, y1));
        if band.width() > 0.5 {
            painter.rect_filled(band, 0.0, color);
        }
    }
}

fn rounded_inset(rect: egui::Rect, radius: f32, y0: f32, y1: f32) -> f32 {
    let y = (y0 + y1) * 0.5;
    let from_top = y - rect.top();
    let from_bot = rect.bottom() - y;
    let d = from_top.min(from_bot);
    if d >= radius || radius <= 0.0 {
        return 0.0;
    }
    let dy = radius - d;
    (radius - (radius * radius - dy * dy).max(0.0).sqrt()).max(0.0)
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

fn us_rect(rect: egui::Rect, design: f32) -> f32 {
    design * rect.height() / 58.0
}

fn spawn_versions(app: Arc<App>, tx: Sender<Msg>) {
    std::thread::spawn(move || match crate::mojang::version_list(&app) {
        Ok(v) => {
            let latest = v.get("latest").and_then(|l| l.get("release")).and_then(|s| s.as_str()).unwrap_or("").to_string();
            let mut out = Vec::new();
            if let Some(arr) = v.get("versions").and_then(|x| x.as_array()) {
                for item in arr {
                    out.push(Ver {
                        id: item.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
                        channel: item.get("channel").and_then(|x| x.as_str()).unwrap_or("").into(),
                        installed: item.get("installed").and_then(|x| x.as_bool()).unwrap_or(false),
                        released: item.get("released").and_then(|x| x.as_str()).unwrap_or("").into(),
                    });
                }
            }
            let _ = tx.send(Msg::Versions { rows: out, latest });
        }
        Err(e) => { let _ = tx.send(Msg::VersionsErr(e)); }
    });
}

fn spawn_loaders(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let cat = crate::loaders::fetch_shared().unwrap_or_else(|e| crate::loaders::Catalog { note: e, ready: true, ..Default::default() });
        let _ = tx.send(Msg::Loaders(cat));
    });
}

fn spawn_mods(app: Arc<App>, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let v = crate::mods::list(&app);
        let mut out = Vec::new();
        if let Some(arr) = v.get("mods").and_then(|x| x.as_array()) {
            for m in arr {
                out.push((
                    m.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
                    m.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
                    m.get("size").and_then(|x| x.as_u64()).unwrap_or(0),
                ));
            }
        }
        let _ = tx.send(Msg::Mods(out));
    });
}

fn spawn_papers(dir: std::path::PathBuf, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let mut papers: Vec<_> = wallpapers::all().iter().collect();
        papers.sort_by_key(|p| if p.title == "Wilderness Bound" { 0 } else if !p.prefixes.is_empty() { 1 } else { 2 });
        for paper in papers {
            if let Ok(bytes) = wallpapers::load_cached(&dir, paper) {
                let _ = tx.send(Msg::Paper { title: paper.title.into(), bytes });
            }
        }
    });
}
