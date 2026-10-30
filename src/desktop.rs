//! Native desktop window. No title bar, no browser.

use crate::preview3d::SkinPreview;
use crate::skinmodel::ArmModel;
use crate::state::{now_ms, App, Job, ServerEntry};
use crate::wallpapers::{self, Paper};
use eframe::egui::{self, CursorIcon, ResizeDirection, RichText, Sense, ViewportCommand};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[path = "tabs/mod.rs"]
mod tabs;

const WINDOW_W: f32 = 960.0;
const WINDOW_H: f32 = 540.0;
pub(crate) const HOME_RAIL: f32 = 86.0;

const HY_BG: egui::Color32 = egui::Color32::from_rgb(0x15, 0x24, 0x3A);
const HY_SURFACE: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x2F, 0x4B);
const HY_HIGH: egui::Color32 = egui::Color32::from_rgb(0x24, 0x3B, 0x5E);
const HY_BORDER: egui::Color32 = egui::Color32::from_rgb(0x32, 0x50, 0x7A);
const HY_TEXT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xEE, 0xF7);
const HY_MUTED: egui::Color32 = egui::Color32::from_rgb(0x9F, 0xB0, 0xC8);
const HY_ACCENT: egui::Color32 = egui::Color32::from_rgb(0xF2, 0xB8, 0x4B);
const HY_LINK: egui::Color32 = egui::Color32::from_rgb(0x5F, 0xA8, 0xFF);
const BG: egui::Color32 = HY_BG;
const SURFACE: egui::Color32 = HY_SURFACE;
const SURFACE2: egui::Color32 = HY_BG;
const SELECTED: egui::Color32 = HY_HIGH;
const BORDER: egui::Color32 = HY_BORDER;
const BORDER_BTN: egui::Color32 = HY_BORDER;
const BORDER_INPUT: egui::Color32 = HY_LINK;
const MARKER_OFF: egui::Color32 = HY_BORDER;
const TEXT: egui::Color32 = HY_TEXT;
const MUTED: egui::Color32 = HY_MUTED;
const ACCENT: egui::Color32 = HY_LINK;
const ON_ACCENT: egui::Color32 = HY_BG;
const GOLD: egui::Color32 = HY_ACCENT;
const SPEC_YELLOW: egui::Color32 = HY_ACCENT;
const SPEC_TEXT: egui::Color32 = HY_TEXT;
const SPEC_MUTED: egui::Color32 = HY_MUTED;
const SPEC_GREEN: egui::Color32 = egui::Color32::from_rgb(0x7F, 0xE0, 0x8B);
const SPEC_PANEL: egui::Color32 = HY_BG;
const SPEC_EDGE: egui::Color32 = HY_BORDER;
const SPEC_ITEM: egui::Color32 = HY_SURFACE;
const SPEC_SEL: egui::Color32 = HY_HIGH;
fn glass_fill() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(0x15, 0x24, 0x3A, 228) }
fn glass_hover() -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(0x24, 0x3B, 0x5E, 236) }
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
    fn nav_label(self) -> &'static str {
        match self {
            Tab::Home => "HOME",
            Tab::Versions => "INSTANCE",
            Tab::Mods => "MODS",
            Tab::Settings => "SETTINGS",
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
    FabricFail { mc: String, missing: bool },
    Explore(u64, Vec<crate::explore::Hit>),
    ExploreErr(u64, String),
    ExploreDetail(u64, crate::explore::Detail),
    ExploreDetailErr(u64, String),
    ExploreIcon { id: String, bytes: Vec<u8> },
    ExploreDone(crate::explore::Outcome),
    ExploreProgress(f32),
}

struct Desktop {
    app: Arc<App>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    tab: Tab,
    home_pane: u8,
    home_menu: bool,
    profile_open: bool,
    rail_view: u8,
    rail_nudge: f32,
    versions: Vec<Ver>,
    versions_note: String,
    version_latest: String,
    release_view: bool,
    loaders: crate::loaders::Catalog,
    loader_after: bool,
    fabric_wait: String,
    fabric_fail: u8,
    fabric_fail_mc: String,
    query: String,
    selected: String,
    papers: HashMap<String, egui::TextureHandle>,
    mark: Option<egui::TextureHandle>,
    logo_title: Option<egui::TextureHandle>,
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
    import_icons: HashMap<String, egui::TextureHandle>,
    mods: Vec<(String, bool, u64)>,
    username: String,
    ram: u32,
    jvm_args: String,
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
    tour: bool,
    tour_start: f64,
    seen_tab: Tab,
    tab_tick: f64,
    film: usize,
    play_after: bool,
    preview: SkinPreview,
    drop_hover: bool,
    instances: Vec<crate::instances::Instance>,
    instance_id: String,
    pack_tab: u8,
    mod_view: u8,
    explore_loader: String,
    explore_version: String,
    explore_query: String,
    explore_hits: Vec<crate::explore::Hit>,
    explore_note: String,
    explore_busy: bool,
    explore_gen: u64,
    explore_detail_gen: u64,
    explore_open: Option<crate::explore::Detail>,
    explore_optional: Vec<String>,
    explore_icons: HashMap<String, egui::TextureHandle>,
    explore_ver_open: bool,
    explore_ver_skip: bool,
    explore_installing: bool,
    explore_progress: f32,
    download_cancel: Arc<AtomicBool>,
    explore_dirty: bool,
    row_focus: usize,
    confirm_remove: Option<String>,
    confirm_instance: Option<String>,
    pending_pack: Option<std::path::PathBuf>,
    draft_open: bool,
    draft_name: String,
    draft_version: String,
    draft_loader: String,
    draft_name_touched: bool,
}

pub fn run(shot: Option<String>, tab: String) -> Result<(), String> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([WINDOW_W, WINDOW_H])
        .with_min_inner_size([WINDOW_W, WINDOW_H])
        .with_max_inner_size([WINDOW_W, WINDOW_H])
        .with_decorations(false)
        .with_resizable(false)
        .with_title("Rus")
        .with_icon(icon_data());
    if std::env::var("RUS_TOUR").ok().as_deref() == Some("1") {
        viewport = viewport.with_position([40.0, 30.0]);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Rus", options, Box::new(move |cc| Ok(Box::new(Desktop::new(cc, shot, tab)))))
        .map_err(|e| e.to_string())
}

fn icon_data() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/rus-icon.png")).expect("icon").to_rgba8();
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
        let mut preview = SkinPreview::default();
        preview.arms = if settings.skin_model == "slim" { ArmModel::Slim } else if settings.skin == "preset:alex" { ArmModel::Slim } else { ArmModel::Classic };
        let mut me = Self {
            app: app.clone(),
            tx: tx.clone(),
            rx,
            tab: Tab::from_name(&tab),
            home_pane: 0,
            home_menu: false,
            profile_open: std::env::var("RUS_PROFILE").ok().as_deref() == Some("1"),
            rail_view: 1,
            rail_nudge: 0.0,
            versions: Vec::new(),
            versions_note: "Loading versions…".into(),
            version_latest: String::new(),
            release_view: true,
            loaders: crate::loaders::Catalog::default(),
            loader_after: false,
            fabric_wait: String::new(),
            fabric_fail: 0,
            fabric_fail_mc: String::new(),
            query: String::new(),
            selected,
            papers: HashMap::new(),
            mark: load_texture(&cc.egui_ctx, "mark", include_bytes!("../assets/mark-32.png")),
            logo_title: load_texture(&cc.egui_ctx, "logo-title", include_bytes!("../assets/rus-logo-title.png")),
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
            import_icons: HashMap::new(),
            mods: Vec::new(),
            username: settings.username,
            ram: settings.ram_mb,
            jvm_args: settings.jvm_args,
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
            tour: std::env::var("RUS_TOUR").ok().as_deref() == Some("1"),
            tour_start: 0.0,
            seen_tab: Tab::from_name(&tab),
            tab_tick: 0.0,
            film: 0,
            play_after: false,
            preview,
            drop_hover: false,
            instances: Vec::new(),
            instance_id: String::new(),
            pack_tab: 1,
            mod_view: 0,
            explore_loader: String::new(),
            explore_version: String::new(),
            explore_query: String::new(),
            explore_hits: Vec::new(),
            explore_note: String::new(),
            explore_busy: false,
            explore_gen: 0,
            explore_detail_gen: 0,
            explore_open: None,
            explore_optional: Vec::new(),
            explore_icons: HashMap::new(),
            explore_ver_open: false,
            explore_ver_skip: false,
            explore_installing: false,
            explore_progress: 0.0,
            download_cancel: Arc::new(AtomicBool::new(false)),
            explore_dirty: false,
            row_focus: 0,
            confirm_remove: None,
            confirm_instance: None,
            pending_pack: None,
            draft_open: false,
            draft_name: String::new(),
            draft_version,
            draft_loader: "vanilla".into(),
            draft_name_touched: false,
        };
        if me.tour {
            me.tab = Tab::Home;
            me.seen_tab = Tab::Home;
        }
        me.bind_instances(&settings.last_instance);
        spawn_versions(app.clone(), tx.clone());
        spawn_mods(app.clone(), tx.clone());
        spawn_loaders(tx.clone());
        me.load_papers(&cc.egui_ctx);
        me.load_scenes(&cc.egui_ctx);
        if me.shot.is_some() && me.tab == Tab::Mods {
            me.mod_view = 1;
            me.explore_dirty = true;
            me.pack_tab = 1;
            me.loader = "fabric".into();
            me.selected = "26.3".into();
            me.explore_version = "26.3".into();
            me.explore_loader = "fabric".into();
            if let Ok(loader) = std::env::var("RUS_EXPLORE_LOADER") {
                if matches!(loader.as_str(), "forge" | "fabric" | "neoforge") {
                    me.explore_loader = loader;
                }
            }
            if std::env::var("RUS_EXPLORE_POPUP").ok().as_deref() == Some("1") {
                me.explore_query = "xaero".into();
            }
        }
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
                    let resolved = self.download_of(&self.selected);
                    if !resolved.is_empty() && resolved != self.selected {
                        self.selected = resolved.clone();
                        if let Some(inst) = self.instances.iter_mut().find(|i| i.id == self.instance_id) {
                            inst.version = resolved;
                        }
                        crate::instances::save(&self.app, &self.instances);
                        self.save_settings();
                    }
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
                    self.fabric_fail = 0;
                    self.loaders.fabric.retain(|(k, _)| k != &mc);
                    if !version.is_empty() {
                        self.loaders.fabric.push((mc, version));
                    }
                }
                Msg::FabricFail { mc, missing } => {
                    if self.fabric_wait == mc {
                        self.fabric_wait.clear();
                    }
                    if self.fabric_fail_mc != mc {
                        self.fabric_fail_mc = mc.clone();
                        self.fabric_fail = 0;
                    }
                    self.fabric_fail = self.fabric_fail.saturating_add(1);
                    if missing || self.fabric_fail >= 3 {
                        self.loaders.fabric.retain(|(k, _)| k != &mc);
                        self.loaders.fabric.push((mc, String::new()));
                    }
                }
                Msg::Explore(gen, hits) => self.on_explore(gen, hits),
                Msg::ExploreErr(gen, err) => {
                    if gen == self.explore_gen {
                        self.explore_busy = false;
                        self.explore_note = err;
                    }
                }
                Msg::ExploreDetail(gen, detail) => self.on_explore_detail(gen, detail),
                Msg::ExploreDetailErr(gen, err) => {
                    if gen == self.explore_detail_gen {
                        if let Some(open) = &mut self.explore_open {
                            open.ready = true;
                            open.missing = err;
                        } else {
                            self.explore_note = err;
                        }
                    }
                }
                Msg::ExploreIcon { id, bytes } => {
                    if let Some(tex) = load_texture(ctx, &format!("explore-{id}"), &bytes) {
                        self.explore_icons.insert(id, tex);
                    }
                }
                Msg::ExploreDone(outcome) => self.on_explore_done(outcome),
                Msg::ExploreProgress(p) => {
                    if self.explore_installing {
                        self.explore_progress = p.clamp(0.0, 1.0);
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
                } else if state == "error" || state == "canceled" {
                    self.play_after = false;
                }
            }
        }
        if self.loader_after {
            if let Some(state) = self.job_state() {
                if state == "done" && self.app.is_installed(&self.selected) {
                    self.loader_after = false;
                    self.spawn_loader_install();
                } else if state == "error" || state == "canceled" {
                    self.loader_after = false;
                }
            }
        }
    }

    fn ensure_fabric(&mut self) {
        let mc = self.selected.clone();
        if mc.is_empty() || self.fabric_wait == mc {
            return;
        }
        if self.loaders.fabric.iter().any(|(k, v)| k == &mc && !v.is_empty()) {
            return;
        }
        if self.loaders.fabric.iter().any(|(k, v)| k == &mc && v.is_empty()) {
            return;
        }
        if self.fabric_fail_mc != mc {
            self.fabric_fail = 0;
            self.fabric_fail_mc = mc.clone();
        }
        if self.fabric_fail >= 3 {
            return;
        }
        self.fabric_wait = mc.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || match crate::loaders::fetch_fabric(&mc) {
            Ok(version) => {
                let _ = tx.send(Msg::Fabric(mc, version));
            }
            Err(e) => {
                let missing = e.contains("HTTP 400") || e.contains("No Fabric");
                let _ = tx.send(Msg::FabricFail { mc, missing });
            }
        });
    }

    fn save_settings(&self) {
        let mut s = self.app.settings();
        s.username = self.username.clone();
        s.ram_mb = self.ram;
        s.jvm_args = self.jvm_args.clone();
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

    fn release_ids(&self) -> Vec<String> {
        self.versions.iter().filter(|v| v.channel == "release").map(|v| v.id.clone()).collect()
    }

    /// One entry per version line. The label is the line (`1.16`); the download is its newest release (`1.16.5`).
    pub(crate) fn release_lines(&self) -> Vec<wallpapers::ReleaseLine> {
        let ids = self.release_ids();
        if ids.is_empty() {
            return wallpapers::all()
                .iter()
                .filter_map(|paper| paper.prefixes.first().copied())
                .map(|label| wallpapers::ReleaseLine { label: label.to_string(), download: label.to_string() })
                .collect();
        }
        wallpapers::representatives(&ids)
    }

    pub(crate) fn download_of(&self, id: &str) -> String {
        let ids = self.release_ids();
        if ids.is_empty() {
            return id.to_string();
        }
        wallpapers::download_for(&ids, id)
    }

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
            "forge" | "forge+optifine" => self.icon_forge.as_ref(),
            "fabric" => self.icon_fabric.as_ref(),
            "neoforge" => self.icon_neoforge.as_ref(),
            "optifine" => self.icon_optifine.as_ref(),
            _ => self.grass.as_ref(),
        }
    }

    fn loader_active(&self, id: &str) -> bool {
        match self.loader.as_str() {
            "forge+optifine" => id == "forge" || id == "optifine",
            other => other == id,
        }
    }

    /// Forge is the installable half of the Forge + OptiFine pair.
    fn install_loader_id(&self) -> String {
        if self.loader == "forge+optifine" {
            "forge".into()
        } else {
            self.loader.clone()
        }
    }

    /// One loader at a time. OptiFine can also stay on with Forge.
    fn choose_loader(&mut self, id: &str) {
        let current = self.loader.as_str();
        let next = match id {
            "optifine" if current == "forge" => "forge+optifine",
            "optifine" if current == "forge+optifine" => "forge",
            "forge" if current == "forge+optifine" => "forge+optifine",
            "vanilla" | "forge" | "fabric" | "neoforge" | "optifine" => id,
            _ => return,
        };
        if self.loader == next {
            return;
        }
        self.loader = next.to_string();
        self.remember_instance();
        self.save_settings();
    }

    /// On/off. One loader stays on. Forge and OptiFine may both stay on.
    fn toggle_loader(&mut self, id: &str) {
        let on = self.loader_active(id);
        let next = if on {
            match (id, self.loader.as_str()) {
                ("optifine", "forge+optifine") => "forge",
                ("forge", "forge+optifine") => "optifine",
                _ => "",
            }
        } else if id == "optifine" && (self.loader == "forge" || self.loader == "forge+optifine") {
            "forge+optifine"
        } else if id == "forge" && self.loader == "optifine" {
            "forge+optifine"
        } else {
            id
        };
        if self.loader == next {
            return;
        }
        self.loader = next.to_string();
        self.remember_instance();
        self.save_settings();
    }

    fn delete_instance(&mut self, id: &str) {
        let dir = crate::instances::dir(&self.app, id);
        let _ = std::fs::remove_dir_all(&dir);
        self.instances.retain(|inst| inst.id != id);
        crate::instances::save(&self.app, &self.instances);
        if self.instance_id == id {
            if let Some(next) = self.instances.first().cloned() {
                self.select_instance(&next.id);
            } else {
                self.instance_id.clear();
            }
        }
        self.notice = "Instance deleted".into();
    }

    fn launch_selected(&mut self) {
        self.save_settings();
        if self.engine == "horizon" {
            let app = self.app.clone();
            let tx = self.tx.clone();
            self.notice = "Checking the Horizon release…".into();
            std::thread::spawn(move || {
                if let Err(e) = crate::horizon::start(app) {
                    let _ = tx.send(Msg::Notice(e));
                }
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
        let launch_loader = self.install_loader_id();
        let profile = if launch_loader != "vanilla" {
            match crate::loaders::installed_profile(&self.app, &id, &launch_loader) {
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

    fn play(&mut self) {
        let running = crate::play::status();
        if running.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
            if self.engine == "horizon" {
                crate::horizon::mark_stopped();
            }
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
        let version = self.download_of(&self.selected);
        if !version.is_empty() {
            self.selected = version.clone();
        }
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
        self.tab = Tab::Home;
        let app = self.app.clone();
        std::thread::spawn(move || crate::install::run(app, job));
        self.notice = format!("Installing {version}");
    }

    fn job_message(&self) -> Option<String> {
        let id = self.job_id.as_ref()?;
        let job = self.app.job(id)?;
        let j = job.lock().ok()?;
        Some(j.message.clone())
    }

    fn cancel_install(&mut self) {
        self.download_cancel.store(true, Ordering::Relaxed);
        if let Some(id) = &self.job_id {
            if let Some(job) = self.app.job(id) {
                if let Ok(mut j) = job.lock() {
                    j.cancel.store(true, Ordering::Relaxed);
                    if j.state == "running" {
                        j.message = "Stopping…".into();
                    }
                }
            }
        }
        self.notice = "Stopping…".into();
    }

    fn track_job(&mut self, label: &str) -> Arc<Mutex<Job>> {
        let id = format!("job-{}", now_ms());
        let job = Arc::new(Mutex::new(Job::new(id.clone(), label.to_string(), Vec::new())));
        if let Ok(mut j) = job.lock() {
            j.message = format!("Downloading {label}");
        }
        self.app.jobs.lock().unwrap().insert(id.clone(), job.clone());
        *self.app.last_job.lock().unwrap() = Some(id.clone());
        self.job_id = Some(id);
        self.tab = Tab::Home;
        job
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
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("import");
        match std::fs::read(path) {
            Ok(bytes) => match crate::skin::import_named(&self.app, kind, &bytes, stem) {
                Ok(id) => {
                    if kind == "cape" { self.cape = id; } else { self.skin = id; self.preview.arms = ArmModel::Auto; }
                    self.import_error.clear();
                    self.avatar_key.clear();
                    self.preview.invalidate();
                    self.save_settings();
                }
                Err(e) => self.import_error = e,
            },
            Err(e) => self.import_error = e.to_string(),
        }
    }

    fn choose_skin(&mut self, id: &str) {
        self.skin = id.to_string();
        self.preview.arms = if id == "preset:alex" {
            ArmModel::Slim
        } else if id == "preset:steve" {
            ArmModel::Classic
        } else {
            ArmModel::Auto
        };
        self.preview.invalidate();
        self.avatar_key.clear();
        self.save_settings();
    }

    fn choose_cape(&mut self, id: &str) {
        self.cape = id.to_string();
        self.preview.invalidate();
        self.save_settings();
    }

    fn ensure_import_icon(&mut self, ctx: &egui::Context, kind: &str, id: &str) {
        let key = format!("{kind}:{id}");
        if self.import_icons.contains_key(&key) {
            return;
        }
        let Some(path) = crate::skin::resolve_texture(&self.app, id) else { return };
        let Ok(bytes) = std::fs::read(path) else { return };
        let Ok(img) = image::load_from_memory(&bytes) else { return };
        let img = img.to_rgba8();
        let (thumb, name) = if kind == "cape" {
            (cape_face_icon(&img), format!("cape-icon-{id}"))
        } else {
            let Ok(prepared) = crate::skinmodel::prepare_skin(&img) else { return };
            (crate::skinmodel::face_avatar(&prepared, 32), format!("skin-icon-{id}"))
        };
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [thumb.width() as usize, thumb.height() as usize],
            thumb.as_raw(),
        );
        self.import_icons.insert(key, ctx.load_texture(name, color, egui::TextureOptions::NEAREST));
    }
}

fn cape_face_icon(src: &image::RgbaImage) -> image::RgbaImage {
    // Back face of a 64×32 cape is 10×16 at (12, 1). Scale it so the chip reads as cloth.
    let mut out = image::RgbaImage::new(20, 32);
    for y in 0..32 {
        for x in 0..20 {
            let sx = 12 + (x * 10) / 20;
            let sy = 1 + (y * 16) / 32;
            let c = if sx < src.width() && sy < src.height() {
                *src.get_pixel(sx, sy)
            } else {
                image::Rgba([0, 0, 0, 0])
            };
            out.put_pixel(x, y, c);
        }
    }
    out
}

impl eframe::App for Desktop {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0x15 as f32 / 255.0, 0x24 as f32 / 255.0, 0x3A as f32 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
        self.settle_playtime();
        self.refresh_avatar(ctx);
        if self.icons_dirty {
            self.reload_server_icons(ctx);
            self.icons_dirty = false;
        }
        self.drive_tour(ctx);
        let fast = self.tour || self.preview.auto_rotate || self.preview.walk;
        ctx.request_repaint_after(Duration::from_millis(if fast { 16 } else { 32 }));
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
        if self.tour {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(WINDOW_W, WINDOW_H)));
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(40.0, 30.0)));
        } else {
            place_fitted_window(ctx, egui::vec2(WINDOW_W, WINDOW_H), 0.60, 0.60);
        }
        if std::env::var("RUS_DRAFT").ok().as_deref() == Some("1") && self.frames == 2 {
            let version = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
            self.open_instance_draft(&version);
        }
        self.frames += 1;
        // winit 0.30 puts WS_CAPTION back on an undecorated window. On some
        // Windows machines the compositor still reserves that frame, so the
        // picture is drawn ~26px down and the top of the window stays black.
        #[cfg(windows)]
        fit_windows_frame();
        if self.frames <= 24 && self.frames % 4 == 0 {
            round_client_corners();
        }
        let paper_ready = self.papers.contains_key(self.paper_for(&self.selected).title);
        let versions_ready = !self.versions.is_empty() || self.frames > 40;
        let loaders_ready = self.loaders.ready && self.loaders.fabric.iter().any(|(k, v)| k == &self.selected && !v.is_empty());
        let explore_ready = self.tab != Tab::Mods || self.mod_view != 1 || self.explore_icons.len() >= 6 || self.frames > 360;
        let popup_ready = if std::env::var("RUS_EXPLORE_POPUP").ok().as_deref() != Some("1") {
            true
        } else if let Some(d) = &self.explore_open {
            let has_img = self.explore_icons.contains_key(&d.id) || self.explore_icons.contains_key(&format!("{}#shot", d.id));
            (d.ready && has_img) || self.frames > 500
        } else {
            self.frames > 500
        };
        if self.shot.is_some() && !self.shot_requested && self.frames > 8 && (paper_ready || self.frames > 90) && versions_ready && (loaders_ready || self.frames > 160) && explore_ready && popup_ready {
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
    fn drive_tour(&mut self, ctx: &egui::Context) {
        if !self.tour {
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.tour_start == 0.0 {
            self.tour_start = now;
        }
        let age = now - self.tour_start;
        self.tab = if age < 3.2 {
            Tab::Home
        } else if age < 7.0 {
            Tab::Versions
        } else if age < 10.6 {
            Tab::Mods
        } else if age < 14.2 {
            Tab::Settings
        } else {
            Tab::Home
        };
        if (4.2..5.4).contains(&age) {
            self.loader = "vanilla".into();
        } else if (5.4..6.8).contains(&age) {
            self.loader = "forge".into();
        }
    }

    fn shell(&mut self, ui: &mut egui::Ui) {
        use_page_fonts(ui);
        let full = ui.max_rect();
        let _ = ui.allocate_rect(full, Sense::hover());
        self.paint_wallpaper(ui, full);
        // Drag only the empty strip above the tabs. A full-window drag was
        // stealing clicks, so the pointer had to sit above the button.
        if !self.draft_open {
            let drag = egui::Rect::from_min_max(full.left_top(), egui::pos2(full.right(), uy(full, 12.0)));
            let drag_hit = ui.interact(drag, egui::Id::new("window-drag"), Sense::drag());
            if drag_hit.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
        }
        self.paint_home_rail(ui, full);
        self.paint_version_bar(ui, full);
        if self.tab != Tab::Home || !self.home_menu {
            self.paint_title_logo(ui, full);
        }
        self.paint_exit(ui, full);
        let now = ui.input(|i| i.time);
        if self.tab != self.seen_tab {
            self.profile_open = false;
            self.seen_tab = self.tab;
            self.tab_tick = now;
        }
        if !self.profile_open {
        match self.tab {
            Tab::Home => {
                self.paint_chrome(ui, full);
                if self.home_menu {
                    if ui.input(|i| i.pointer.any_click()) {
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            let pop = self.home_popup_rect(full);
                            let switch = egui::Rect::from_min_size(egui::pos2(HOME_RAIL + 10.0, 52.0), egui::vec2(220.0, 32.0));
                            if !pop.contains(pos) && !switch.contains(pos) && !self.loader_hit_rect().contains(pos) {
                                self.home_menu = false;
                            }
                        }
                    }
                }
                if self.home_menu {
                    if self.home_pane == 1 {
                        self.paint_mods_page(ui, full);
                    } else {
                        self.paint_version_instances(ui, full);
                    }
                    self.paint_home_switch(ui, full);
                }
            }
            Tab::Versions => self.paint_version_page(ui, full),
            Tab::Mods => self.paint_mods_page(ui, full),
            Tab::Settings => self.paint_settings_page(ui, full),
        }
        }
        let age = (now - self.tab_tick) as f32;
        let fade = ease_out((age / 0.18).clamp(0.0, 1.0));
        if self.shot.is_none() && fade < 0.995 {
            let body = egui::Rect::from_min_max(egui::pos2(full.left(), 64.0), full.right_bottom());
            ui.painter().rect_filled(body, 0.0, egui::Color32::from_rgba_unmultiplied(0x08, 0x12, 0x24, ((1.0 - fade) * 90.0) as u8));
        }
        if self.draft_open {
            self.paint_instance_popup(ui, full);
        }
        if self.confirm_remove.is_some() {
            self.paint_remove_confirm(ui, full);
        }
        if self.confirm_instance.is_some() {
            self.paint_instance_delete(ui, full);
        }
        if self.tab == Tab::Mods && self.explore_open.is_some() {
            self.paint_explore_popup(ui, full);
        }
        if self.pending_pack.is_some() {
            self.paint_pack_prompt(ui, full);
        }
        if self.profile_open {
            self.paint_profile_popup(ui, full);
        }
    }

    fn load_papers(&mut self, ctx: &egui::Context) {
        for paper in wallpapers::all() {
            if let Some(bytes) = wallpapers::embedded(paper.title) {
                if let Some(tex) = load_texture(ctx, paper.title, bytes) {
                    self.papers.insert(paper.title.to_string(), tex);
                }
            }
            if let Some(bytes) = wallpapers::square(paper.title) {
                if let Some(tex) = load_texture(ctx, &format!("{}#sq", paper.title), bytes) {
                    self.papers.insert(format!("{}#sq", paper.title), tex);
                }
            }
        }
    }

    fn load_scenes(&mut self, ctx: &egui::Context) {
        let scenes: [(&str, &[u8]); 5] = [
            ("scene:home", include_bytes!("../assets/scenes/home.jpg")),
            ("scene:version", include_bytes!("../assets/scenes/version.jpg")),
            ("scene:mods", include_bytes!("../assets/scenes/mods.jpg")),
            ("scene:settings", include_bytes!("../assets/scenes/settings.jpg")),
            ("scene:profile", include_bytes!("../assets/scenes/profile.jpg")),
        ];
        for (key, bytes) in scenes {
            if let Some(tex) = load_texture(ctx, key, bytes) {
                self.papers.insert(key.to_string(), tex);
            }
        }
    }

    fn scene_key(&self) -> &'static str {
        if self.profile_open {
            return "scene:profile";
        }
        match self.tab {
            Tab::Home => "scene:home",
            Tab::Versions => "scene:version",
            Tab::Mods => "scene:mods",
            Tab::Settings => "scene:settings",
        }
    }

    fn paint_wallpaper(&self, ui: &egui::Ui, full: egui::Rect) {
        let key = self.scene_key();
        let paper = self.paper_for(&self.selected);
        let tex = self.papers.get(key).or_else(|| self.papers.get(paper.title));
        if let Some(tex) = tex {
            ui.painter().rect_filled(full, 0.0, BG);
            // One official image, drawn at the window size. No wash, no crop.
            ui.painter().image(tex.id(), full, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
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
    }

    fn channel_label(&self) -> String {
        let raw = self.versions.iter().find(|v| v.id == self.selected).map(|v| v.channel.as_str()).unwrap_or("release");
        let mut chars = raw.chars();
        match chars.next() {
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            None => "Release".into(),
        }
    }

    /// Player card first: head, then the name. The tab bar follows it.
    fn status_tail(&self) -> String {
        if self.loader == "vanilla" || self.loader.is_empty() {
            format!("Worlds {}", self.world_count())
        } else {
            format!("Mods {}", self.mod_count())
        }
    }

    fn mod_count(&self) -> usize {
        self.mods.iter().filter(|m| m.1).count()
    }

    fn world_count(&self) -> usize {
        let root = if self.instance_id.is_empty() {
            self.app.game_dir(&self.selected)
        } else {
            crate::instances::dir(&self.app, &self.instance_id)
        };
        std::fs::read_dir(root.join("saves"))
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0)
    }

    fn loader_line(&self, version: &str) -> String {
        if self.loader == "forge+optifine" {
            let forge = crate::loaders::rows_for(version, &self.loaders)
                .into_iter()
                .find(|row| row.id == "forge")
                .map(|row| row.version)
                .unwrap_or_default();
            return if forge.is_empty() {
                "forge + optifine".into()
            } else {
                format!("forge {forge} + optifine")
            };
        }
        let name = match self.loader.as_str() {
            "forge" => "forge",
            "fabric" => "fabric",
            "neoforge" => "neoforge",
            "optifine" => "optifine",
            _ => "vanilla",
        };
        let found = crate::loaders::rows_for(version, &self.loaders)
            .into_iter()
            .find(|row| row.id == self.loader.as_str());
        match found {
            Some(row) if !row.version.is_empty() => format!("{name} {}", row.version),
            _ if name == "vanilla" => format!("vanilla {version}"),
            _ => name.to_string(),
        }
    }

    /// Text under Play. A click leaves it lit, with a glow, and restyles Play.
    fn select_paper(&mut self, paper: &Paper) {
        if let Some(prefix) = paper.prefixes.first() {
            if let Some(v) = self.versions.iter().find(|v| version_hit(&v.id, prefix)) {
                self.selected = v.id.clone();
                self.save_settings();
            }
        }
    }

    fn pack_dir(&self, folder: &str) -> std::path::PathBuf {
        if self.instance_id.is_empty() {
            self.app.game_dir(&self.selected).join(folder)
        } else {
            crate::instances::dir(&self.app, &self.instance_id).join(folder)
        }
    }

    fn scan_packs(&self, folder: &str) -> Vec<(String, u64)> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.pack_dir(folder)) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') { continue; }
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((name, size));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn pack_entries(&self) -> Vec<(String, bool, u64)> {
        match self.pack_tab {
            0 => self.scan_packs("modpacks").into_iter().map(|(n, s)| (n, true, s)).collect(),
            2 => self.scan_packs("resourcepacks").into_iter().map(|(n, s)| { let on = !n.ends_with(".disabled"); (n, on, s) }).collect(),
            3 => self.scan_packs("shaderpacks").into_iter().map(|(n, s)| { let on = !n.ends_with(".disabled"); (n, on, s) }).collect(),
            _ => self.scan_mods(),
        }
    }

    fn scan_mods(&self) -> Vec<(String, bool, u64)> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.pack_dir("mods")) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_ascii_lowercase();
                let enabled = lower.ends_with(".jar");
                if !enabled && !lower.ends_with(".jar.disabled") { continue; }
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((name, enabled, size));
            }
        }
        out.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
        out
    }

    fn pack_folder(&self) -> &'static str {
        match self.pack_tab {
            0 => "modpacks",
            2 => "resourcepacks",
            3 => "shaderpacks",
            _ => "mods",
        }
    }

    fn set_pack_enabled(&mut self, name: &str, enable: bool) {
        let dir = self.pack_dir(self.pack_folder());
        let src = dir.join(name);
        let dest_name = if enable { name.trim_end_matches(".disabled").to_string() } else if name.ends_with(".disabled") { name.to_string() } else { format!("{name}.disabled") };
        let dest = dir.join(safe_leaf(&dest_name).unwrap_or_else(|| "file.disabled".into()));
        if src != dest && src.exists() {
            if let Err(e) = std::fs::rename(&src, &dest) {
                self.notice = e.to_string();
            }
        }
    }

    fn trash_pack(&mut self, name: &str) {
        let leaf = match safe_leaf(name) {
            Some(n) => n,
            None => { self.notice = "That file name is not safe.".into(); return; }
        };
        let dir = self.pack_dir(self.pack_folder());
        let src = dir.join(&leaf);
        let trash = dir.join(".trash");
        let _ = std::fs::create_dir_all(&trash);
        let dest = trash.join(&leaf);
        match std::fs::rename(&src, &dest) {
            Ok(()) => self.notice = format!("Moved {leaf} to trash"),
            Err(e) => self.notice = e.to_string(),
        }
    }

    fn install_kind(&mut self, kind: u8) {
        self.pack_tab = kind;
        let (title, filter, import) = match kind {
            0 => ("Modpack", "*.zip", 3u8),
            2 => ("Resource pack", "*.zip", 1u8),
            3 => ("Shader pack", "*.zip", 2u8),
            _ => ("Add a mod", "*.jar", 0u8),
        };
        let Some(path) = crate::dialog::pick_file(title, filter) else {
            self.notice = "No file chosen.".into();
            return;
        };
        if kind == 0 {
            self.pending_pack = Some(path);
            return;
        }
        self.import_into(import, &path);
    }

    fn pick_pack_file(&mut self) {
        self.install_kind(self.pack_tab);
    }

    fn take_drops(&mut self, ui: &egui::Ui) {
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let Some(path) = file.path else { continue };
            match classify_drop(&path) {
                3 => self.pending_pack = Some(path),
                kind => self.import_into(kind, &path),
            }
        }
    }

    fn import_into(&mut self, kind: u8, path: &std::path::Path) {
        if kind == 0 && (self.loader == "vanilla" || self.loader.is_empty()) {
            self.notice = "This jar was added, but mods need Forge, Fabric, or NeoForge.".into();
        }
        let folder = match kind {
            1 => "resourcepacks",
            2 => "shaderpacks",
            _ => "mods",
        };
        let Some(leaf) = path.file_name().and_then(|n| safe_leaf(&n.to_string_lossy())) else {
            self.notice = "That file name is not safe.".into();
            return;
        };
        if kind == 0 && !leaf.to_ascii_lowercase().ends_with(".jar") {
            self.notice = "Only a .jar can be added as a mod.".into();
            return;
        }
        let dest_dir = self.pack_dir(folder);
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest = dest_dir.join(&leaf);
        if dest.exists() {
            self.notice = format!("{leaf} is already there. It was not overwritten.");
            return;
        }
        match std::fs::copy(path, &dest) {
            Ok(_) => {
                if self.notice.is_empty() || !self.notice.contains("need") {
                    self.notice = format!("Added {leaf}");
                }
            }
            Err(e) => self.notice = e.to_string(),
        }
    }

    fn store_pack_as_instance(&mut self, path: &std::path::Path) {
        let stem = path.file_stem().and_then(|s| safe_leaf(&s.to_string_lossy())).unwrap_or_else(|| "Pack".into());
        let inst = crate::instances::create(&stem, &self.selected, &self.loader);
        let dir = crate::instances::dir(&self.app, &inst.id).join("modpacks");
        let _ = std::fs::create_dir_all(&dir);
        let leaf = path.file_name().and_then(|n| safe_leaf(&n.to_string_lossy())).unwrap_or_else(|| "pack.zip".into());
        match std::fs::copy(path, dir.join(&leaf)) {
            Ok(_) => {
                self.instances.push(inst.clone());
                crate::instances::save(&self.app, &self.instances);
                self.select_instance(&inst.id);
                self.notice = format!("Stored {leaf} in a new instance. It was not applied.");
            }
            Err(e) => self.notice = e.to_string(),
        }
    }

    fn apply_pack(&mut self, path: &std::path::Path) {
        let dest = self.pack_dir("mods");
        let _ = std::fs::create_dir_all(&dest);
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => { self.notice = e.to_string(); return; }
        };
        let mut zip = match zip::ZipArchive::new(file) {
            Ok(z) => z,
            Err(e) => { self.notice = format!("Could not open the pack: {e}"); return; }
        };
        let mut added = 0usize;
        let mut skipped = 0usize;
        for i in 0..zip.len() {
            let mut entry = match zip.by_index(i) { Ok(e) => e, Err(_) => continue };
            if entry.is_dir() { continue; }
            let raw = entry.name().replace('\\', "/");
            if raw.split('/').any(|p| p == ".." || p.is_empty() && false) || raw.contains("..") {
                self.notice = "The pack had an unsafe path and was not applied.".into();
                return;
            }
            let Some(leaf) = std::path::Path::new(&raw).file_name().and_then(|n| safe_leaf(&n.to_string_lossy())) else { continue };
            if !leaf.to_ascii_lowercase().ends_with(".jar") { continue; }
            let target = dest.join(&leaf);
            if target.exists() { skipped += 1; continue; }
            let mut out = match std::fs::File::create(&target) { Ok(f) => f, Err(e) => { self.notice = e.to_string(); return; } };
            if std::io::copy(&mut entry, &mut out).is_err() {
                let _ = std::fs::remove_file(&target);
                self.notice = format!("{leaf} was incomplete, so it was not kept.");
                return;
            }
            added += 1;
        }
        self.notice = format!("Added {added} mods. Kept {skipped} existing files.");
    }

    fn loader_icon_for(&self, id: &str) -> Option<&egui::TextureHandle> {
        self.loader_icon_id(id)
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
        let loader = self.install_loader_id();
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
        let combo = self.loader == "forge+optifine";
        let loader = self.install_loader_id();
        let version = crate::loaders::rows_for(&mc, &self.loaders).into_iter().find(|r| r.id == loader).map(|r| r.version).unwrap_or_default();
        let app = self.app.clone();
        let tx = self.tx.clone();
        self.notice = if combo {
            format!("Installing forge {version} for {mc}. OptiFine has no build to add.")
        } else {
            format!("Installing {loader} {version} for {mc}…")
        };
        std::thread::spawn(move || {
            let msg = match crate::loaders::install(&app, &mc, &loader, &version) {
                Ok(id) => format!("{loader} is installed ({id})"),
                Err(e) => e,
            };
            let _ = tx.send(Msg::Notice(msg));
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

pub(crate) fn paint_download_bar(painter: &egui::Painter, rect: egui::Rect, frac: f32) {
    let frac = frac.clamp(0.0, 1.0);
    painter.rect_filled(rect, 1.5, HY_HIGH);
    painter.rect_stroke(rect, 1.5, egui::Stroke::new(1.0, HY_BORDER), egui::StrokeKind::Inside);
    let w = (rect.width() * frac).min(rect.width());
    if w > 0.5 {
        painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height())), 1.5, HY_ACCENT);
    }
    if rect.height() >= 16.0 {
        let pct = format!("{}%", (frac * 100.0).round() as u32);
        let ink = if frac > 0.45 { HY_BG } else { HY_TEXT };
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, pct, nav_font(rect.height() * 0.42), ink);
    }
}

fn play_button(ui: &mut egui::Ui, id: &str, rect: egui::Rect, label: &str, stop: bool, horizon: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(hit.id.with("hot"), hit.hovered(), 0.12);
    let (fill, edge, ink) = if stop {
        (
            lerp_color(egui::Color32::from_rgb(176, 52, 64), egui::Color32::from_rgb(206, 72, 82), hot),
            egui::Color32::from_rgb(255, 166, 170),
            egui::Color32::from_rgb(255, 244, 242),
        )
    } else if horizon {
        (
            lerp_color(egui::Color32::from_rgb(138, 96, 28), egui::Color32::from_rgb(176, 126, 42), hot),
            egui::Color32::from_rgb(255, 214, 120),
            egui::Color32::from_rgb(255, 236, 196),
        )
    } else {
        (
            lerp_color(egui::Color32::from_rgb(18, 42, 108), egui::Color32::from_rgb(28, 58, 138), hot),
            egui::Color32::from_rgb(110, 186, 255),
            egui::Color32::WHITE,
        )
    };
    paint_neon_plate(ui.painter(), rect, 10.0, fill, edge);
    let font = nav_font(rect.height() * 0.34);
    let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), ink).size().x;
    let group = text_w + 22.0;
    let tri = egui::pos2(rect.center().x - group * 0.5 + 6.0, rect.center().y);
    if stop {
        paint_material(ui.painter(), tri, 16.0, I_STOP, ink);
    } else {
        paint_play_tri(ui.painter(), tri, 16.0, ink);
    }
    ui.painter().text(egui::pos2(tri.x + 16.0, rect.center().y), egui::Align2::LEFT_CENTER, label, font, ink);
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
        egui::Color32::from_rgb(255, 236, 176)
    } else {
        GOLD
    }
}

fn paint_empty_cross(painter: &egui::Painter, center: egui::Pos2, size: f32, color: egui::Color32) {
    paint_material(painter, center, size * 1.35, I_CLOSE, color);
}

fn paint_play_tri(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    paint_material(painter, c, s.max(14.0), I_PLAY, color);
}

fn paint_chevron(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32, down: bool) {
    let icon = if down { I_DOWN } else { '\u{E5CC}' };
    paint_material(painter, c, (s * 2.6).max(14.0), icon, color);
}

fn paint_search_icon(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    paint_material(painter, c, s.max(14.0), I_SEARCH, color);
}

fn paint_tick(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    paint_material(painter, c, s.max(14.0), I_CHECK, color);
}

fn paint_trash(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    paint_material(painter, c, s.max(14.0), I_DELETE, color);
}

fn paint_tab_glyph(painter: &egui::Painter, tab: Tab, c: egui::Pos2, s: f32, color: egui::Color32) {
    paint_material(painter, c, s.max(16.0), tab_icon(tab), color);
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
    paint_material(painter, c, 16.0, I_CLOCK, color);
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
    let ink = if primary { GOLD } else { egui::Color32::from_rgb(255, 246, 226) };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(13.0), ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn bar_button(ui: &mut egui::Ui, id: &str, rect: egui::Rect, label: &str, primary: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new(id), Sense::click());
    paint_hytale_bar(ui.painter(), rect);
    let ink = if primary { GOLD } else { egui::Color32::from_rgb(255, 246, 226) };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(13.0), ink);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn chip(ui: &mut egui::Ui, label: &str, on: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 26.0), Sense::hover());
    ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(1.0, if on { ACCENT } else { BORDER_BTN }), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), if on { ACCENT } else { MUTED });
}

fn paint_popup_field(ui: &mut egui::Ui, rect: egui::Rect, text: &mut String, hint: &str) {
    paint_hytale_bar(ui.painter(), rect);
    let inner = rect.shrink2(egui::vec2(10.0, 6.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.set_clip_rect(rect.shrink(2.0));
    let vis = child.visuals_mut();
    let clear = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    vis.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.inactive.bg_stroke = clear;
    vis.widgets.hovered.bg_stroke = clear;
    vis.widgets.active.bg_stroke = clear;
    vis.override_text_color = Some(TEXT);
    child.add(
        egui::TextEdit::singleline(text)
            .hint_text(RichText::new(hint).color(MUTED))
            .desired_width(inner.width())
            .font(egui::TextStyle::Body)
            .frame(false)
            .margin(egui::vec2(0.0, 1.0)),
    );
}

fn style_popup_scroll(ui: &mut egui::Ui) {
    let scroll = &mut ui.style_mut().spacing.scroll;
    *scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 4.0;
    scroll.bar_inner_margin = 6.0;
    scroll.bar_outer_margin = 1.0;
    scroll.handle_min_length = 28.0;
    scroll.foreground_color = true;
    let pale = egui::Color32::from_rgb(176, 196, 214);
    let vis = ui.visuals_mut();
    vis.extreme_bg_color = egui::Color32::from_rgb(36, 62, 112);
    vis.widgets.inactive.fg_stroke.color = pale;
    vis.widgets.hovered.fg_stroke.color = GOLD;
    vis.widgets.active.fg_stroke.color = GOLD;
    vis.widgets.inactive.bg_fill = pale;
    vis.widgets.hovered.bg_fill = GOLD;
    vis.widgets.active.bg_fill = GOLD;
    vis.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
}

fn style_popup_fields(ui: &mut egui::Ui) {
    let vis = ui.visuals_mut();
    let fill = egui::Color32::from_rgb(18, 32, 58);
    let edge = egui::Stroke::new(1.0, egui::Color32::from_rgb(176, 196, 214));
    vis.extreme_bg_color = fill;
    vis.widgets.inactive.bg_fill = fill;
    vis.widgets.inactive.bg_stroke = edge;
    vis.widgets.hovered.bg_fill = fill;
    vis.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, GOLD);
    vis.widgets.active.bg_fill = fill;
    vis.widgets.active.bg_stroke = egui::Stroke::new(1.0, GOLD);
    vis.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
    vis.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
    vis.widgets.active.corner_radius = egui::CornerRadius::same(2);
}

fn field(ui: &mut egui::Ui, text: &mut String, width: f32, hint: &str) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::hover());
    paint_element_template(ui.painter(), rect);
    let inner = rect.shrink2(egui::vec2(8.0, 4.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.set_clip_rect(rect.shrink(1.5));
    let clear = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    let vis = child.visuals_mut();
    vis.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
    vis.widgets.inactive.bg_stroke = clear;
    vis.widgets.hovered.bg_stroke = clear;
    vis.widgets.active.bg_stroke = clear;
    vis.override_text_color = Some(TEXT);
    child.add(
        egui::TextEdit::singleline(text)
            .hint_text(RichText::new(hint).color(MUTED))
            .desired_width(inner.width())
            .font(egui::TextStyle::Body)
            .frame(false)
            .margin(egui::vec2(0.0, 1.0)),
    )
}

fn check_inline(ui: &mut egui::Ui, on: &mut bool, label: &str, width: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::click());
    let box_r = egui::Rect::from_center_size(rect.left_center() + egui::vec2(8.0, 0.0), egui::vec2(14.0, 14.0));
    ui.painter().rect_stroke(box_r, 2.0, egui::Stroke::new(1.0, if *on { GOLD } else { egui::Color32::from_rgb(180, 160, 110) }), egui::StrokeKind::Inside);
    if *on {
        ui.painter().rect_filled(box_r.shrink(3.5), 1.0, GOLD);
    }
    ui.painter().text(rect.left_center() + egui::vec2(22.0, 0.0), egui::Align2::LEFT_CENTER, label, body_font(14.0), if *on { GOLD } else { TEXT });
    if resp.clicked() {
        *on = !*on;
        true
    } else {
        false
    }
}

fn check(ui: &mut egui::Ui, on: &mut bool, label: &str) -> bool {
    let font = body_font(14.0);
    let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), TEXT).size().x;
    let w = (36.0 + text_w + 12.0).min(ui.available_width().max(80.0));
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 24.0), Sense::click());
    paint_element_template(ui.painter(), rect);
    let box_r = egui::Rect::from_center_size(rect.left_center() + egui::vec2(14.0, 0.0), egui::vec2(12.0, 12.0));
    ui.painter().rect_stroke(box_r, 2.0, egui::Stroke::new(1.0, if *on { GOLD } else { egui::Color32::from_rgb(196, 214, 232) }), egui::StrokeKind::Inside);
    if *on { ui.painter().rect_filled(box_r.shrink(3.5), 1.0, GOLD); }
    ui.painter().text(rect.left_center() + egui::vec2(26.0, 0.0), egui::Align2::LEFT_CENTER, label, font, TEXT);
    if resp.clicked() { *on = !*on; true } else { false }
}

fn slider(ui: &mut egui::Ui, value: &mut u32, min: u32, max: u32) -> bool {
    let width = ui.available_width().max(160.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::click_and_drag());
    paint_element_template(ui.painter(), rect);
    let track = egui::Rect::from_min_max(egui::pos2(rect.left() + 12.0, rect.center().y - 3.0), egui::pos2(rect.right() - 12.0, rect.center().y + 3.0));
    ui.painter().rect_filled(track, 3.0, egui::Color32::from_rgba_unmultiplied(0xF4, 0xCA, 0x54, 70));
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

fn style_page_widgets(ui: &mut egui::Ui) {
    let vis = ui.visuals_mut();
    let clear = egui::Color32::TRANSPARENT;
    vis.extreme_bg_color = clear;
    vis.panel_fill = clear;
    vis.widgets.inactive.bg_fill = clear;
    vis.widgets.hovered.bg_fill = clear;
    vis.widgets.active.bg_fill = clear;
    vis.widgets.inactive.weak_bg_fill = clear;
    vis.widgets.hovered.weak_bg_fill = egui::Color32::from_white_alpha(8);
    vis.widgets.active.weak_bg_fill = clear;
    vis.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    vis.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    vis.widgets.active.bg_stroke = egui::Stroke::NONE;
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 2)).show(ui, add);
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
        .rect_stroke(rect.shrink(0.5), 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(8, 14, 28, 40)), egui::StrokeKind::Inside);
}

fn paint_tab_sep(painter: &egui::Painter, x: f32, y: f32, h: f32) {
    let x = x.round();
    let n = h.round().max(8.0) as i32;
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32;
        let k = (1.0 - (t - 0.5).abs() * 2.0).powf(1.35);
        let alpha = (k * 230.0) as u8;
        if alpha < 10 {
            continue;
        }
        let yy = y + i as f32;
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x, yy), egui::pos2(x + 1.0, yy + 1.0)),
            0.0,
            egui::Color32::from_rgba_unmultiplied(214, 224, 234, alpha),
        );
    }
}

fn paint_faded_rule(painter: &egui::Painter, x: f32, y: f32, w: f32) {
    let steps = w.ceil().clamp(16.0, 72.0) as i32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let k = (1.0 - (t - 0.5).abs() * 2.0).powf(1.35);
        let alpha = (k * 170.0) as u8;
        if alpha < 8 {
            continue;
        }
        let x0 = x + w * (i as f32 / steps as f32);
        let x1 = x + w * ((i + 1) as f32 / steps as f32);
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 1.0)),
            0.0,
            egui::Color32::from_rgba_unmultiplied(214, 224, 234, alpha),
        );
    }
}

/// Content wash for Instance, Mods, and Settings. No template color:
/// black at 15% opacity, so the wallpaper stays visible and only a little darker.
fn page_veil(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(0x15, 0x24, 0x3A, 153));
}

fn paint_gold_bar(painter: &egui::Painter, x: f32, y: f32, h: f32) {
    let x = x.round() + 0.5;
    let steps = h.ceil().clamp(12.0, 64.0) as i32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let k = (1.0 - (t - 0.5).abs() * 2.0).powf(1.35);
        let alpha = (k * 235.0) as u8;
        if alpha < 12 {
            continue;
        }
        let y0 = y + h * (i as f32 / steps as f32);
        let y1 = y + h * ((i + 1) as f32 / steps as f32);
        let color = egui::Color32::from_rgba_unmultiplied(0xF2, 0xB8, 0x4B, alpha);
        painter.vline(x, y0..=y1, egui::Stroke::new(1.0, color));
    }
}

fn gold_slot(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, height), Sense::hover());
    if height > 48.0 {
        let h = (rect.height() * 0.86).max(24.0);
        paint_gold_bar(ui.painter(), rect.center().x, rect.center().y - h * 0.5, h);
    } else {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "|", nav_font(16.0), GOLD);
    }
}

fn element_label(ui: &mut egui::Ui, text: &str, font: egui::FontId, color: egui::Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), font.clone(), color);
    let pad = egui::vec2(8.0, 3.0);
    let max_w = ui.available_width().max(48.0);
    let size = egui::vec2((galley.size().x + pad.x * 2.0).min(max_w), galley.size().y + pad.y * 2.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    paint_element_template(ui.painter(), rect);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, font, color);
}

fn plain_action(ui: &mut egui::Ui, label: &str, width: f32, height: f32, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    paint_element_template(ui.painter(), rect);
    if on {
        ui.painter().rect_stroke(rect.shrink(1.5), 1.5, egui::Stroke::new(1.0, GOLD), egui::StrokeKind::Inside);
    }
    let ink = if on { GOLD } else { TEXT };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(14.0), ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn item_action(ui: &mut egui::Ui, label: &str, width: f32, height: f32, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let ink = if on { GOLD } else { TEXT };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(14.0), ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn paint_faded_bar(painter: &egui::Painter, x: f32, y: f32, h: f32) {
    let x = x.round() + 0.5;
    let steps = h.ceil().clamp(12.0, 48.0) as i32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let k = (1.0 - (t - 0.5).abs() * 2.0).powf(1.35);
        let alpha = (k * 235.0) as u8;
        if alpha < 12 {
            continue;
        }
        let y0 = y + h * (i as f32 / steps as f32);
        let y1 = y + h * ((i + 1) as f32 / steps as f32);
        let color = egui::Color32::from_rgba_unmultiplied(214, 224, 234, alpha);
        painter.vline(x, y0..=y1, egui::Stroke::new(1.0, color));
    }
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

fn cover_focus(src: egui::Vec2, dst: egui::Rect, fx: f32, fy: f32) -> egui::Rect {
    if src.x <= 0.0 || src.y <= 0.0 || dst.width() <= 0.0 || dst.height() <= 0.0 {
        return egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    }
    let src_aspect = src.x / src.y;
    let dst_aspect = dst.width() / dst.height();
    if src_aspect > dst_aspect {
        let visible = dst_aspect / src_aspect;
        let x = (fx - visible * 0.5).clamp(0.0, (1.0 - visible).max(0.0));
        egui::Rect::from_min_max(egui::pos2(x, 0.0), egui::pos2(x + visible, 1.0))
    } else {
        let visible = src_aspect / dst_aspect;
        let y = (fy - visible * 0.5).clamp(0.0, (1.0 - visible).max(0.0));
        egui::Rect::from_min_max(egui::pos2(0.0, y), egui::pos2(1.0, y + visible))
    }
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
    // Program lettering for title, body, and subtext. Icons stay Material.
    fonts.font_data.insert("Lettering".into(), egui::FontData::from_static(include_bytes!("../assets/fonts/RUS-Launcher-Lettering-Regular.ttf")).into());
    fonts.font_data.insert("Roboto400".into(), egui::FontData::from_static(include_bytes!("../assets/fonts/Roboto-400.ttf")).into());
    fonts.font_data.insert("Material".into(), egui::FontData::from_static(include_bytes!("../assets/MaterialSymbols.ttf")).into());
    let text = vec!["Lettering".into(), "Roboto400".into()];
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "Lettering".into());
    fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "Lettering".into());
    fonts.families.insert(egui::FontFamily::Name("Title".into()), text.clone());
    fonts.families.insert(egui::FontFamily::Name("Tab".into()), text.clone());
    fonts.families.insert(egui::FontFamily::Name("Nav".into()), text.clone());
    fonts.families.insert(egui::FontFamily::Name("Body".into()), text.clone());
    fonts.families.insert(egui::FontFamily::Name("Sub".into()), text);
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
    style.visuals.selection.bg_fill = HY_HIGH;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, HY_ACCENT);
    let radius = egui::CornerRadius::same(2);
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.corner_radius = radius;
        w.bg_stroke = egui::Stroke::new(1.0, HY_BORDER);
        w.fg_stroke.color = HY_TEXT;
    }
    style.visuals.widgets.inactive.weak_bg_fill = HY_SURFACE;
    style.visuals.widgets.hovered.weak_bg_fill = HY_HIGH;
    style.visuals.widgets.active.bg_fill = HY_ACCENT;
    style.visuals.widgets.active.weak_bg_fill = HY_ACCENT;
    style.visuals.widgets.active.fg_stroke.color = HY_BG;
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
    let mut img = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, image.as_raw().to_vec()).ok_or("bad image")?;
    if let Some(parent) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(parent); }
    punch_round_corners(&mut img, 4);
    img.save(path).map_err(|e| e.to_string())
}

fn round_client_corners() {
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if DONE.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if apply_round_corners() {
        DONE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(target_os = "linux")]
fn apply_round_corners() -> bool {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};

    #[repr(C)]
    struct XRectangle {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    }

    #[link(name = "X11")]
    extern "C" {
        fn XOpenDisplay(name: *const c_char) -> *mut c_void;
        fn XDefaultRootWindow(dpy: *mut c_void) -> c_ulong;
        fn XQueryTree(
            dpy: *mut c_void,
            w: c_ulong,
            root: *mut c_ulong,
            parent: *mut c_ulong,
            children: *mut *mut c_ulong,
            nchildren: *mut c_uint,
        ) -> c_int;
        fn XFetchName(dpy: *mut c_void, w: c_ulong, name: *mut *mut c_char) -> c_int;
        fn XFree(data: *mut c_void) -> c_int;
        fn XCloseDisplay(dpy: *mut c_void) -> c_int;
        fn XGetGeometry(
            dpy: *mut c_void,
            w: c_ulong,
            root: *mut c_ulong,
            x: *mut c_int,
            y: *mut c_int,
            width: *mut c_uint,
            height: *mut c_uint,
            border: *mut c_uint,
            depth: *mut c_uint,
        ) -> c_int;
    }
    #[link(name = "Xext")]
    extern "C" {
        fn XShapeCombineRectangles(
            dpy: *mut c_void,
            dest: c_ulong,
            dest_kind: c_int,
            x_off: c_int,
            y_off: c_int,
            rects: *const XRect,
            n: c_int,
            op: c_int,
            ordering: c_int,
        );
    }

    unsafe fn named(dpy: *mut c_void, win: c_ulong, want: &str) -> bool {
        let mut raw = std::ptr::null_mut();
        if XFetchName(dpy, win, &mut raw) == 0 || raw.is_null() {
            return false;
        }
        let ok = CStr::from_ptr(raw).to_str().ok() == Some(want);
        XFree(raw as *mut c_void);
        ok
    }

    unsafe fn find(dpy: *mut c_void, win: c_ulong) -> Option<c_ulong> {
        if named(dpy, win, "Rus") {
            return Some(win);
        }
        let mut root = 0;
        let mut parent = 0;
        let mut children = std::ptr::null_mut();
        let mut n = 0;
        if XQueryTree(dpy, win, &mut root, &mut parent, &mut children, &mut n) == 0 {
            return None;
        }
        let mut found = None;
        for i in 0..n as isize {
            if let Some(hit) = find(dpy, *children.offset(i)) {
                found = Some(hit);
                break;
            }
        }
        if !children.is_null() {
            XFree(children as *mut c_void);
        }
        found
    }

    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return false;
        }
        let root = XDefaultRootWindow(dpy);
        let Some(win) = find(dpy, root) else {
            XCloseDisplay(dpy);
            return false;
        };
        let mut root_ret = 0;
        let mut x = 0;
        let mut y = 0;
        let mut width = 0u32;
        let mut height = 0u32;
        let mut border = 0u32;
        let mut depth = 0u32;
        if XGetGeometry(dpy, win, &mut root_ret, &mut x, &mut y, &mut width, &mut height, &mut border, &mut depth) == 0 {
            XCloseDisplay(dpy);
            return false;
        }
        let rects = corner_cut_rects(width as i32, height as i32, 4);
        if !rects.is_empty() {
            XShapeCombineRectangles(dpy, win, 0, 0, 0, rects.as_ptr(), rects.len() as c_int, 3, 0);
        }
        XCloseDisplay(dpy);
        true
    }
}

#[cfg(windows)]
fn rus_hwnd() -> *mut std::os::raw::c_void {
    use std::os::raw::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, window: *const u16) -> *mut c_void;
    }
    let name: Vec<u16> = "Rus\0".encode_utf16().collect();
    unsafe { FindWindowW(std::ptr::null(), name.as_ptr()) }
}

/// Logical layout stays at the design size. Zoom makes that layout fill about
/// `frac_w` / `frac_h` of the monitor, then the window is centered. Monitor size
/// from egui is already divided by the current zoom, so multiply it back before
/// choosing the next zoom or the window shrinks every frame.
fn place_fitted_window(ctx: &egui::Context, design: egui::Vec2, frac_w: f32, frac_h: f32) {
    static ZOOM_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    static STAGE: AtomicU8 = AtomicU8::new(0);
    let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
        return;
    };
    if monitor.x < 240.0 || monitor.y < 240.0 {
        return;
    }
    // Measure once, while zoom is still 1. A later monitor size is already
    // divided by the zoom we just set, and using it again makes the window grow.
    let stored = ZOOM_BITS.load(Ordering::Relaxed);
    let zoom = if stored == 0 {
        if (ctx.zoom_factor() - 1.0).abs() > 0.02 {
            return;
        }
        let zoom = fit_zoom(monitor, design, frac_w, frac_h);
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

fn fit_zoom(monitor: egui::Vec2, design: egui::Vec2, frac_w: f32, frac_h: f32) -> f32 {
    let want = (monitor.x * frac_w / design.x).min(monitor.y * frac_h / design.y);
    let cap = (monitor.x * 0.92 / design.x).min(monitor.y * 0.92 / design.y);
    if cap < 1.0 {
        return cap.clamp(0.62, 1.0);
    }
    want.clamp(1.0, cap.min(2.6))
}

/// Drop the caption winit leaves on a borderless window, then ask Windows to
/// rebuild the frame so the picture starts at the top-left instead of under a
/// black bar.
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
        fn GetWindowLongPtrW(hwnd: *mut c_void, index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: *mut c_void, index: i32, value: isize) -> isize;
        fn SetWindowPos(hwnd: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: *mut c_void, margins: *const Margins) -> i32;
    }
    let hwnd = rus_hwnd();
    if hwnd.is_null() {
        return;
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next = style & !(WS_CAPTION | WS_THICKFRAME | WS_BORDER);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next_ex = ex & !(WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE);
        let changed = next != style || next_ex != ex;
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
        }
        if next_ex != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_ex);
        }
        if changed {
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
    }
}

#[cfg(windows)]
fn apply_round_corners() -> bool {
    use std::os::raw::c_void;
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
        fn SetWindowRgn(hwnd: *mut c_void, rgn: *mut c_void, redraw: i32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> *mut c_void;
    }
    fit_windows_frame();
    let hwnd = rus_hwnd();
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        let mut rect = Rect { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let rgn = CreateRoundRectRgn(0, 0, w, h, 8, 8);
        if rgn.is_null() {
            return false;
        }
        SetWindowRgn(hwnd, rgn, 1);
        true
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn apply_round_corners() -> bool {
    true
}

#[cfg(target_os = "linux")]
fn corner_cut_rects(width: i32, height: i32, radius: i32) -> Vec<XRect> {
    let mut out = Vec::new();
    let r = radius as f32;
    for y in 0..radius {
        for x in 0..radius {
            let cx = x as f32 + 0.5;
            let cy = y as f32 + 0.5;
            if (cx - r).powi(2) + (cy - r).powi(2) <= r * r {
                continue;
            }
            for (px, py) in [
                (x, y),
                (width - 1 - x, y),
                (x, height - 1 - y),
                (width - 1 - x, height - 1 - y),
            ] {
                out.push(XRect { x: px as i16, y: py as i16, width: 1, height: 1 });
            }
        }
    }
    out
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct XRect {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

fn punch_round_corners(img: &mut image::RgbaImage, radius: i32) {
    let (w, h) = img.dimensions();
    let r = radius as f32;
    for y in 0..radius {
        for x in 0..radius {
            let cx = x as f32 + 0.5;
            let cy = y as f32 + 0.5;
            if (cx - r).powi(2) + (cy - r).powi(2) <= r * r {
                continue;
            }
            let corners = [
                (x as u32, y as u32),
                (w - 1 - x as u32, y as u32),
                (x as u32, h - 1 - y as u32),
                (w - 1 - x as u32, h - 1 - y as u32),
            ];
            for (px, py) in corners {
                img.put_pixel(px, py, image::Rgba([0, 0, 0, 0]));
            }
        }
    }
}

fn version_hit(id: &str, prefix: &str) -> bool {
    id == prefix || id.starts_with(&format!("{prefix}.")) || id.starts_with(&format!("{prefix}-"))
}

fn format_bytes(n: u64) -> String {
    if n < 1024 { format!("{n} B") } else if n < 1024 * 1024 { format!("{} KB", n / 1024) } else { format!("{:.1} MB", n as f64 / 1_048_576.0) }
}

const V_PANEL: egui::Color32 = HY_BG;
const V_BORDER: egui::Color32 = HY_BORDER;
const V_SEL: egui::Color32 = HY_HIGH;
const V_LINE: egui::Color32 = HY_BORDER;
const V_GOLD: egui::Color32 = HY_ACCENT;
const V_GOLD_EDGE: egui::Color32 = HY_ACCENT;
const V_MUTED: egui::Color32 = HY_MUTED;
const V_TEXT: egui::Color32 = HY_TEXT;
const V_GREEN: egui::Color32 = egui::Color32::from_rgb(93, 192, 74);
const V_GREEN_INK: egui::Color32 = egui::Color32::from_rgb(16, 36, 18);
const V_TAB: egui::Color32 = HY_BG;
const V_TAB_ON: egui::Color32 = HY_HIGH;
const V_TAB_EDGE: egui::Color32 = HY_BORDER;
const V_TAB_ON_EDGE: egui::Color32 = HY_ACCENT;
const V_RELEASE: egui::Color32 = HY_SURFACE;
const V_BLUE: egui::Color32 = HY_LINK;
const V_PREVIEW: egui::Color32 = egui::Color32::from_rgb(110, 78, 40);
const V_PREVIEW_INK: egui::Color32 = HY_ACCENT;

const I_HOME: char = '\u{E88A}';
const I_SETTINGS: char = '\u{E8B8}';
const I_LAYERS: char = '\u{E53B}';
const I_CUBE: char = '\u{E87B}';
const I_FLARE: char = '\u{E3E4}';
const I_CLOCK: char = '\u{E8B5}';
const I_DOWN: char = '\u{E5DB}';
const I_FOLDER: char = '\u{E2C7}';
const I_ADD: char = '\u{E145}';
const I_CLOSE: char = '\u{E5CD}';
const I_GLOBE: char = '\u{E80B}';
const I_SEARCH: char = '\u{E8B6}';
const I_PLAY: char = '\u{E037}';
const I_STOP: char = '\u{E047}';
const I_DELETE: char = '\u{E872}';
const I_CHECK: char = '\u{E5CA}';
const I_DOWNLOAD: char = '\u{E2C4}';

fn drect(full: egui::Rect, x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(ux(full, x), uy(full, y)), egui::pos2(ux(full, x + w), uy(full, y + h)))
}

fn tab_icon(tab: Tab) -> char {
    match tab {
        Tab::Home => I_HOME,
        Tab::Versions => I_LAYERS,
        Tab::Mods => I_CUBE,
        Tab::Settings => I_SETTINGS,
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

fn panel_fill(alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(8, 10, 14, alpha)
}

fn glow_edge() -> egui::Color32 {
    egui::Color32::from_white_alpha(42)
}

fn paint_neon_plate(painter: &egui::Painter, rect: egui::Rect, radius: f32, fill: egui::Color32, edge: egui::Color32) {
    for i in 1..=3 {
        let a = 16 + i * 8;
        painter.rect_stroke(
            rect.expand(i as f32 * 1.15),
            radius + 1.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(edge.r(), edge.g(), edge.b(), a)),
            egui::StrokeKind::Inside,
        );
    }
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.2, edge), egui::StrokeKind::Inside);
}

fn paint_glass_chip(painter: &egui::Painter, rect: egui::Rect, radius: f32, alpha: u8) {
    painter.rect_filled(rect.expand(1.2), radius + 1.0, egui::Color32::from_rgba_unmultiplied(50, 120, 220, 28));
    painter.rect_filled(rect, radius, panel_fill(alpha));
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.05, glow_edge()), egui::StrokeKind::Inside);
}

fn paint_glass_panel(painter: &egui::Painter, rect: egui::Rect, radius: f32) {
    paint_neon_plate(painter, rect, radius, panel_fill(148), glow_edge());
}

fn paint_hytale_bar(painter: &egui::Painter, rect: egui::Rect) {
    paint_glass_chip(painter, rect, 8.0, 214);
}

/// Page outline only. The wallpaper stays open behind the stroke.
fn paint_open_frame(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, 12.0, egui::Stroke::new(1.15, glow_edge()), egui::StrokeKind::Inside);
}

/// Version-page element: stroke the length and the width, keep the corner cap,
/// and keep the smaller cap inside each corner.
fn paint_version_corners(painter: &egui::Painter, rect: egui::Rect, on: bool) {
    let color = if on { GOLD } else { HY_BORDER };
    let side = egui::Stroke::new(1.15, color);
    let corner = egui::Stroke::new(1.35, color);
    let left = rect.left() + 1.0;
    let right = rect.right() - 1.0;
    let top = rect.top() + 1.0;
    let bottom = rect.bottom() - 1.0;
    painter.line_segment([egui::pos2(left, top), egui::pos2(right, top)], side);
    painter.line_segment([egui::pos2(left, bottom), egui::pos2(right, bottom)], side);
    painter.line_segment([egui::pos2(left, top), egui::pos2(left, bottom)], side);
    painter.line_segment([egui::pos2(right, top), egui::pos2(right, bottom)], side);
    let small = rect.height().min(rect.width()) < 30.0;
    let leg = if small { 5.0 } else { 6.0 };
    let inner = if small { 3.0 } else { 4.0 };
    let gap = 3.0;
    let corners = [
        (left, top, 1.0, 1.0),
        (right, top, -1.0, 1.0),
        (left, bottom, 1.0, -1.0),
        (right, bottom, -1.0, -1.0),
    ];
    for (x, y, sx, sy) in corners {
        painter.line_segment([egui::pos2(x, y), egui::pos2(x + sx * leg, y)], corner);
        painter.line_segment([egui::pos2(x, y), egui::pos2(x, y + sy * leg)], corner);
        let ix = x + sx * (leg + gap);
        let iy = y + sy * (leg + gap);
        if ix > left + 2.0 && ix < right - 2.0 && iy > top + 2.0 && iy < bottom - 2.0 {
            painter.line_segment([egui::pos2(ix, iy), egui::pos2(ix + sx * inner, iy)], corner);
            painter.line_segment([egui::pos2(ix, iy), egui::pos2(ix, iy + sy * inner)], corner);
        }
    }
}

/// One element, one template. Slightly open so the wallpaper still shows through.
fn paint_element_template(painter: &egui::Painter, rect: egui::Rect) {
    paint_glass_chip(painter, rect, 7.0, 220);
}

fn paint_corner_caps(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.15, color);
    let leg = 5.0;
    let inset = 1.5;
    let corners = [
        (rect.left() + inset, rect.top() + inset, 1.0, 1.0),
        (rect.right() - inset, rect.top() + inset, -1.0, 1.0),
        (rect.left() + inset, rect.bottom() - inset, 1.0, -1.0),
        (rect.right() - inset, rect.bottom() - inset, -1.0, -1.0),
    ];
    for (x, y, sx, sy) in corners {
        painter.line_segment([egui::pos2(x, y), egui::pos2(x + sx * leg, y)], stroke);
        painter.line_segment([egui::pos2(x, y), egui::pos2(x, y + sy * leg)], stroke);
    }
}

fn paint_active_fade(painter: &egui::Painter, rect: egui::Rect) {
    paint_active_fade_strength(painter, rect, 1.0);
}

fn paint_active_fade_strength(painter: &egui::Painter, rect: egui::Rect, strength: f32) {
    let fade = painter.with_clip_rect(rect.shrink(0.5));
    if rect.width() > rect.height() * 1.8 {
        let steps = (rect.width() / 1.6).ceil().clamp(28.0, 72.0) as i32;
        for i in 0..steps {
            let t0 = i as f32 / steps as f32;
            let t1 = (i + 1) as f32 / steps as f32;
            let dist = ((t0 + t1) * 0.5 - 0.5).abs() * 2.0;
            let k = (1.0 - dist).powf(1.45) * strength;
            let r = 36.0 + 150.0 * k;
            let g = 62.0 + 96.0 * k;
            let b = 112.0 - 36.0 * k;
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
        let k = (1.0 - t).powf(1.6) * strength;
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

fn install_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, enabled: bool) -> bool {
    let hit = ui.interact(rect, egui::Id::new("loader-install"), Sense::click());
    paint_version_corners(ui.painter(), rect, enabled);
    let ink = if enabled { GOLD } else { MUTED };
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
    paint_version_corners(ui.painter(), rect, primary);
    let ink = if primary { GOLD } else { egui::Color32::from_rgb(255, 246, 226) };
    paint_material(ui.painter(), egui::pos2(rect.left() + rect.height() * 0.42, rect.center().y), rect.height() * 0.34, I_ADD, ink);
    ui.painter().text(egui::pos2(rect.center().x + 8.0, rect.center().y), egui::Align2::CENTER_CENTER, label, nav_font(rect.height() * 0.32), ink);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

/// Same two install colors, blended top to bottom instead of a hard split.
fn spec_content(full: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(HOME_RAIL + 10.0, 102.0), egui::pos2(full.right() - 16.0, full.bottom() - 14.0))
}

fn paint_main_box(painter: &egui::Painter, rect: egui::Rect) {
    paint_glass_panel(painter, rect, 12.0);
}

fn paint_opaque_card(painter: &egui::Painter, rect: egui::Rect, selected: f32) {
    paint_mixed_card(painter, rect, selected, 0.0);
}

fn paint_mixed_card(painter: &egui::Painter, rect: egui::Rect, selected: f32, fade: f32) {
    let selected = selected.clamp(0.0, 1.0);
    let fade = fade.clamp(0.0, 1.0);
    let fill = lerp_color(
        lerp_color(panel_fill(214), egui::Color32::from_rgba_unmultiplied(0x1A, 0x2C, 0x52, 230), selected),
        egui::Color32::from_rgba_unmultiplied(0x0A, 0x12, 0x22, 150),
        fade,
    );
    let edge = lerp_color(lerp_color(glow_edge(), HY_ACCENT, selected), egui::Color32::from_rgb(0x3A, 0x4C, 0x68), fade);
    painter.rect_filled(rect, 8.0, fill);
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0 + selected * 0.6, edge), egui::StrokeKind::Inside);
}

fn red_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str) -> bool {
    action_button(ui, rect, label, BtnKind::Danger)
}

fn opaque_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, primary: bool) -> bool {
    action_button(ui, rect, label, if primary { BtnKind::Gold } else { BtnKind::Quiet })
}

#[derive(Clone, Copy)]
enum BtnKind {
    Gold,
    Outline,
    Danger,
    Quiet,
}

fn action_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, kind: BtnKind) -> bool {
    let hit = ui.interact(rect, egui::Id::new("act-btn").with(rect.top().to_bits()).with(rect.left().to_bits()).with(label), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(hit.id.with("hot"), hit.hovered(), 0.1);
    let ink = match kind {
        BtnKind::Gold => egui::Color32::from_rgb(0x1A, 0x14, 0x08),
        BtnKind::Danger => egui::Color32::from_rgb(255, 244, 242),
        BtnKind::Outline => HY_ACCENT,
        BtnKind::Quiet => HY_TEXT,
    };
    match kind {
        BtnKind::Gold => {
            let fill = lerp_color(egui::Color32::from_rgb(0xF0, 0xB4, 0x2E), egui::Color32::from_rgb(0xFF, 0xC8, 0x62), hot);
            painter_gold(ui.painter(), rect, fill);
        }
        BtnKind::Danger => {
            let fill = lerp_color(egui::Color32::from_rgb(176, 48, 62), egui::Color32::from_rgb(206, 70, 82), hot);
            ui.painter().rect_filled(rect, 8.0, fill);
            ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 150, 156)), egui::StrokeKind::Inside);
        }
        BtnKind::Outline => {
            ui.painter().rect_filled(rect, 8.0, panel_fill(if hot > 0.2 { 200 } else { 160 }));
            ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.15, HY_ACCENT), egui::StrokeKind::Inside);
        }
        BtnKind::Quiet => {
            paint_glass_chip(ui.painter(), rect, 8.0, if hot > 0.2 { 230 } else { 200 });
        }
    }
    let icon = match (kind, label) {
        (BtnKind::Danger, _) => Some("trash"),
        (BtnKind::Outline, _) => Some("tick"),
        (BtnKind::Gold, "NEW" | "New instance") => Some("add"),
        (BtnKind::Gold, "INSTALL") => Some("down"),
        (BtnKind::Gold, "Search") => Some("search"),
        _ => None,
    };
    let font = nav_font(11.5);
    let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), ink).size().x;
    let icon_w = if icon.is_some() { 16.0 } else { 0.0 };
    let group = icon_w + text_w;
    let start = rect.center().x - group * 0.5;
    if icon.is_some() {
        let at = egui::pos2(start + 6.0, rect.center().y);
        match icon {
            Some("trash") => paint_trash(ui.painter(), at, 11.0, ink),
            Some("tick") => paint_tick(ui.painter(), at, 9.0, ink),
            Some("add") => paint_material(ui.painter(), at, 13.0, I_ADD, ink),
            Some("down") => paint_material(ui.painter(), at, 13.0, I_DOWN, ink),
            Some("search") => paint_search_icon(ui.painter(), at, 8.0, ink),
            _ => {}
        }
    }
    let text_x = if icon.is_some() { start + icon_w + 4.0 } else { rect.center().x };
    let align = if icon.is_some() { egui::Align2::LEFT_CENTER } else { egui::Align2::CENTER_CENTER };
    ui.painter().text(egui::pos2(text_x, rect.center().y), align, label, font, ink);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn painter_gold(painter: &egui::Painter, rect: egui::Rect, fill: egui::Color32) {
    painter.rect_filled(rect.expand(1.4), 9.0, egui::Color32::from_rgba_unmultiplied(0xF2, 0xB8, 0x4B, 36));
    painter.rect_filled(rect, 8.0, fill);
}

fn paint_faded_card(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 1.5, egui::Color32::from_rgb(0x12, 0x1C, 0x30));
    painter.rect_stroke(rect, 1.5, egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2A, 0x3A, 0x52)), egui::StrokeKind::Inside);
}

fn paint_purple_card(painter: &egui::Painter, rect: egui::Rect) {
    let edge = egui::Color32::from_rgb(0xC4, 0x5A, 0x68);
    painter.rect_filled(rect, 8.0, egui::Color32::from_rgba_unmultiplied(0x2A, 0x12, 0x1C, 210));
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.3, edge), egui::StrokeKind::Inside);
}

fn paint_slide_switch(ui: &mut egui::Ui, rect: egui::Rect, on: bool, id: egui::Id, enabled: bool) -> bool {
    let t = ui.ctx().animate_bool_with_time(id.with("knob"), on, 0.16);
    let track = if !enabled {
        egui::Color32::from_rgb(0x2A, 0x36, 0x4C)
    } else if t > 0.5 {
        lerp_color(egui::Color32::from_rgb(0x3A, 0x48, 0x62), HY_ACCENT, (t - 0.35).clamp(0.0, 1.0))
    } else {
        egui::Color32::from_rgb(0x3A, 0x4C, 0x68)
    };
    ui.painter().rect_filled(rect, rect.height() * 0.5, track);
    let knob = (rect.height() - 4.0).max(8.0);
    let x = rect.left() + 2.0 + t * (rect.width() - knob - 4.0);
    let center = egui::pos2(x + knob * 0.5, rect.center().y);
    ui.painter().circle_filled(center, knob * 0.5, if on { egui::Color32::from_rgb(255, 248, 236) } else { egui::Color32::from_rgb(0xC8, 0xD2, 0xE0) });
    if !enabled {
        return false;
    }
    let hit = ui.interact(rect, id, Sense::click());
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn paint_warn(painter: &egui::Painter, center: egui::Pos2) {
    let a = center + egui::vec2(0.0, -7.0);
    let b = center + egui::vec2(-6.5, 6.0);
    let c = center + egui::vec2(6.5, 6.0);
    painter.add(egui::Shape::convex_polygon(vec![a, c, b], HY_ACCENT, egui::Stroke::new(1.4, HY_ACCENT)));
    painter.text(center + egui::vec2(0.0, 1.5), egui::Align2::CENTER_CENTER, "!", nav_font(10.0), HY_BG);
}

fn count_files(path: &std::path::Path) -> usize {
    let mut n = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); } else { n += 1; }
            if n > 4000 { return n; }
        }
    }
    n
}

fn paint_blend_stage(painter: &egui::Painter, rect: egui::Rect) {
    let _ = (painter, rect);
}

fn spec_panel(painter: &egui::Painter, rect: egui::Rect) {
    paint_glass_panel(painter, rect, 12.0);
}

fn spec_card(painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    let edge = if selected { HY_ACCENT } else { glow_edge() };
    painter.rect_filled(rect, 8.0, panel_fill(if selected { 220 } else { 196 }));
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(if selected { 1.4 } else { 1.0 }, edge), egui::StrokeKind::Inside);
}

fn paint_check(painter: &egui::Painter, rect: egui::Rect, on: bool) {
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.5, if on { SPEC_YELLOW } else { SPEC_MUTED }), egui::StrokeKind::Inside);
    if on {
        let a = egui::pos2(rect.left() + 3.0, rect.center().y);
        let b = egui::pos2(rect.left() + 6.5, rect.bottom() - 3.5);
        let c = egui::pos2(rect.right() - 3.0, rect.top() + 4.0);
        let stroke = egui::Stroke::new(2.0, SPEC_YELLOW);
        painter.line_segment([a, b], stroke);
        painter.line_segment([b, c], stroke);
    }
}

fn display_pack_name(name: &str) -> String {
    name.trim_end_matches(".disabled").to_string()
}

fn safe_leaf(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }
    let clean: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+' || c == ' ' { c } else { '_' }).collect();
    let clean = clean.trim_matches('.').to_string();
    if clean.is_empty() { None } else { Some(clean) }
}

fn classify_drop(path: &std::path::Path) -> u8 {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "jar" { return 0; }
    if path.is_dir() {
        if path.join("pack.mcmeta").exists() { return 1; }
        if path.join("shaders").is_dir() { return 2; }
        return 1;
    }
    if ext != "zip" { return 3; }
    let Ok(file) = std::fs::File::open(path) else { return 3 };
    let Ok(mut zip) = zip::ZipArchive::new(file) else { return 3 };
    let mut kind = 3u8;
    for i in 0..zip.len().min(80) {
        let Ok(entry) = zip.by_index(i) else { continue };
        let name = entry.name().replace('\\', "/");
        if name.ends_with("pack.mcmeta") { return 1; }
        if name.contains("shaders/") { kind = 2; }
        if name.ends_with("modrinth.index.json") || name.ends_with("manifest.json") { kind = 3; }
    }
    kind
}

fn spec_button(ui: &mut egui::Ui, rect: egui::Rect, label: &str, primary: bool) -> bool {
    action_button(ui, rect, label, if primary { BtnKind::Gold } else { BtnKind::Quiet })
}

fn paint_dashed_rect(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    let dash = 6.0;
    let gap = 4.0;
    let step = dash + gap;
    let edges = [
        (rect.left_top(), rect.right_top(), egui::vec2(1.0, 0.0)),
        (rect.right_top(), rect.right_bottom(), egui::vec2(0.0, 1.0)),
        (rect.right_bottom(), rect.left_bottom(), egui::vec2(-1.0, 0.0)),
        (rect.left_bottom(), rect.left_top(), egui::vec2(0.0, -1.0)),
    ];
    for (start, end, dir) in edges {
        let len = start.distance(end);
        let mut t = 0.0;
        while t < len {
            let a = start + dir * t;
            let b = start + dir * (t + dash).min(len);
            painter.line_segment([a, b], stroke);
            t += step;
        }
    }
}

fn ellipsize(painter: &egui::Painter, text: &str, font: egui::FontId, max_w: f32) -> String {
    if painter.layout_no_wrap(text.to_string(), font.clone(), SPEC_TEXT).size().x <= max_w {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let next = format!("{out}{ch}…");
        if painter.layout_no_wrap(next, font.clone(), SPEC_TEXT).size().x > max_w {
            break;
        }
        out.push(ch);
    }
    if out.is_empty() { "…".into() } else { format!("{out}…") }
}

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

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
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
