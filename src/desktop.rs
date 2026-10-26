//! Native desktop window. No title bar, no browser.

use crate::preview3d::SkinPreview;
use crate::skinmodel::ArmModel;
use crate::state::{now_ms, App, Job};
use crate::wallpapers::{self, Paper};
use eframe::egui::{self, CursorIcon, ResizeDirection, RichText, Sense, ViewportCommand};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

impl Tab {
    fn from_name(s: &str) -> Self {
        match s {
            "versions" | "version" => Tab::Versions,
            "mods" => Tab::Mods,
            "settings" => Tab::Settings,
            _ => Tab::Home,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Versions => "Version",
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
}

enum Msg {
    Versions(Vec<Ver>),
    VersionsErr(String),
    Paper { title: String, bytes: Vec<u8> },
    Notice(String),
    Mods(Vec<(String, bool, u64)>),
}

struct Desktop {
    app: Arc<App>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    tab: Tab,
    versions: Vec<Ver>,
    versions_note: String,
    query: String,
    selected: String,
    papers: HashMap<String, egui::TextureHandle>,
    mark: Option<egui::TextureHandle>,
    grass: Option<egui::TextureHandle>,
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
}

pub fn run(shot: Option<String>, tab: String) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 720.0])
        .with_min_inner_size([1100.0, 640.0])
        .with_decorations(false)
        .with_resizable(true)
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
        let selected = if settings.last_version.is_empty() { "26.3".into() } else { settings.last_version.clone() };
        let mut preview = SkinPreview::default();
        preview.arms = if settings.skin_model == "slim" { ArmModel::Slim } else if settings.skin == "preset:alex" { ArmModel::Slim } else { ArmModel::Classic };
        let me = Self {
            app: app.clone(),
            tx: tx.clone(),
            rx,
            tab: Tab::from_name(&tab),
            versions: Vec::new(),
            versions_note: "Loading versions…".into(),
            query: String::new(),
            selected,
            papers: HashMap::new(),
            mark: load_texture(&cc.egui_ctx, "mark", include_bytes!("../assets/mark-32.png")),
            grass: Some(grass_texture(&cc.egui_ctx)),
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
        };
        spawn_versions(app.clone(), tx.clone());
        spawn_mods(app.clone(), tx.clone());
        spawn_papers(app.dir.clone(), tx);
        me
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Versions(v) => {
                    if self.selected.is_empty() {
                        if let Some(first) = v.first() { self.selected = first.id.clone(); }
                    }
                    self.versions_note = format!("{} versions", v.len());
                    self.versions = v;
                }
                Msg::VersionsErr(e) => self.versions_note = e,
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
        s.last_version = self.selected.clone();
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
        self.notice = format!("Starting {id}…");
        std::thread::spawn(move || {
            let msg = match crate::play::launch(&app, &id) {
                Ok(_) => format!("{id} is running in the sandbox"),
                Err(e) => e,
            };
            let _ = tx.send(Msg::Notice(msg));
        });
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
        let face = crate::skinmodel::face_avatar(&img, 32);
        let color = egui::ColorImage::from_rgba_unmultiplied([32, 32], face.as_raw());
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
        self.refresh_avatar(ctx);
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
        window_edges(ctx);
        paint_frame(ctx);
        self.frames += 1;
        let paper_ready = self.papers.contains_key(self.paper_for(&self.selected).title);
        let versions_ready = !self.versions.is_empty() || self.frames > 40;
        if self.shot.is_some() && !self.shot_requested && self.frames > 6 && (paper_ready || self.frames > 90) && versions_ready {
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
        let full = ui.max_rect();
        let _ = ui.allocate_rect(full, Sense::hover());
        self.paint_wallpaper(ui, full);
        let drag = ui.interact(full, egui::Id::new("window-drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            toggle_max(ui.ctx());
        }

        let margin = 16.0;
        let top = 16.0;
        let bar_h = 52.0;
        let rail = 46.0;
        let bottom_h = 58.0;
        let content = egui::Rect::from_min_max(
            egui::pos2(margin, top + bar_h + 14.0),
            egui::pos2(full.right() - margin - rail - 14.0, full.bottom() - margin - bottom_h - 28.0),
        );
        if self.tab != Tab::Home {
            glass(ui.painter(), content, 14.0, glass_fill());
            let inner = content.shrink(10.0);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            child.set_clip_rect(content);
            match self.tab {
                Tab::Home => {}
                Tab::Versions => self.page_versions(&mut child),
                Tab::Mods => self.page_mods(&mut child),
                Tab::Settings => self.page_settings(&mut child),
            }
        }
        self.paint_chrome(ui, full);
    }

    fn paint_wallpaper(&self, ui: &egui::Ui, full: egui::Rect) {
        let paper = self.paper_for(&self.selected);
        if let Some(tex) = self.papers.get(paper.title) {
            ui.painter().image(tex.id(), full, cover_uv(tex.size_vec2(), full), egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(full, 0.0, BG);
            ui.painter().text(
                full.center(),
                egui::Align2::CENTER_CENTER,
                "Fetching the collection wallpaper…",
                egui::FontId::proportional(16.0),
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
        let margin = 16.0;
        let top = 16.0;
        let bar_h = 52.0;
        self.account_bar(ui, egui::pos2(margin, top), bar_h);
        self.nav_bar(ui, egui::pos2(margin + 188.0, top), bar_h);
        let exit = egui::Rect::from_min_size(egui::pos2(full.right() - margin - 46.0, top + 3.0), egui::vec2(46.0, 46.0));
        if icon_button(ui, exit, "exit", false, "Exit") {
            self.quit(ui.ctx());
        }
        paint_exit_icon(ui.painter(), exit.center(), TEXT);

        let rail_x = full.right() - margin - 46.0;
        let rail_y = 168.0;
        let horizon_on = self.engine == "horizon";
        let hrect = egui::Rect::from_min_size(egui::pos2(rail_x, rail_y), egui::vec2(46.0, 46.0));
        if icon_button(ui, hrect, "rail-horizon", horizon_on, "Horizon") {
            self.engine = if horizon_on { "vanilla".into() } else { "horizon".into() };
            self.save_settings();
        }
        ui.painter().text(hrect.center(), egui::Align2::CENTER_CENTER, "H", egui::FontId::proportional(22.0), GOLD);

        let mrect = hrect.translate(egui::vec2(0.0, 56.0));
        if icon_button(ui, mrect, "rail-mod", self.tab == Tab::Mods, "Mod") {
            self.tab = Tab::Mods;
        }
        paint_creature(ui.painter(), mrect.center());

        let srect = mrect.translate(egui::vec2(0.0, 56.0));
        if icon_button(ui, srect, "rail-settings", self.tab == Tab::Settings, "Settings") {
            self.tab = Tab::Settings;
        }
        paint_crest(ui.painter(), srect.center());

        let card = egui::Rect::from_min_size(egui::pos2(margin, full.bottom() - margin - 58.0), egui::vec2(430.0, 58.0));
        let card_hit = ui.interact(card, egui::Id::new("instance-card"), Sense::click());
        glass(ui.painter(), card, 12.0, if card_hit.hovered() { glass_hover() } else { glass_fill() });
        if let Some(grass) = &self.grass {
            let icon = egui::Rect::from_center_size(egui::pos2(card.left() + 32.0, card.center().y), egui::vec2(30.0, 30.0));
            ui.painter().image(grass.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        let engine = if horizon_on { "HORIZON" } else { "VANILLA" };
        ui.painter().text(egui::pos2(card.left() + 58.0, card.center().y), egui::Align2::LEFT_CENTER, engine, egui::FontId::proportional(15.0), GOLD);
        ui.painter().text(egui::pos2(card.left() + 168.0, card.center().y), egui::Align2::LEFT_CENTER, &self.selected, egui::FontId::proportional(18.0), TEXT);
        ui.painter().line_segment(
            [egui::pos2(card.left() + 250.0, card.center().y - 12.0), egui::pos2(card.left() + 250.0, card.center().y + 12.0)],
            egui::Stroke::new(1.0, BORDER_BTN),
        );
        paint_clock(ui.painter(), egui::pos2(card.left() + 276.0, card.center().y), GOLD);
        let stat = format!("{} GB", (self.ram as f32 / 1024.0).round() as u32);
        ui.painter().text(egui::pos2(card.left() + 294.0, card.center().y), egui::Align2::LEFT_CENTER, &stat, egui::FontId::proportional(16.0), TEXT);
        if card_hit.clicked() {
            self.tab = Tab::Versions;
        }
        card_hit.on_hover_text("Choose a version");

        let play = egui::Rect::from_min_size(egui::pos2(full.right() - margin - 210.0, full.bottom() - margin - 54.0), egui::vec2(210.0, 54.0));
        let live = crate::play::status().get("running").and_then(|v| v.as_bool()).unwrap_or(false);
        let label = if live { "STOP" } else { "PLAY" };
        let note = if let Some((progress, line)) = self.job_line() {
            let _ = progress;
            line
        } else if self.notice != "Ready" && !self.notice.is_empty() {
            self.notice.clone()
        } else {
            self.channel_label()
        };
        ui.painter().text(egui::pos2(play.right() - 4.0, play.top() - 16.0), egui::Align2::RIGHT_CENTER, &note, egui::FontId::proportional(13.0), MUTED);
        paint_diamond(ui.painter(), egui::pos2(play.right() - note_width(ui, &note) - 16.0, play.top() - 16.0), GOLD);
        if play_button(ui, play, label) {
            self.play();
        }
        if let Some((progress, _)) = self.job_line() {
            let bar = egui::Rect::from_min_max(
                egui::pos2(play.left(), play.bottom() - 3.0),
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

    fn account_bar(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, h: f32) {
        let name = if self.username.is_empty() { "Player" } else { &self.username };
        let rect = egui::Rect::from_min_size(origin, egui::vec2(176.0, h));
        let hit = ui.interact(rect, egui::Id::new("account-chip"), Sense::click());
        glass(ui.painter(), rect, 12.0, if hit.hovered() { glass_hover() } else { glass_fill() });
        if let Some(grass) = &self.grass {
            let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 30.0, rect.center().y), egui::vec2(28.0, 28.0));
            ui.painter().image(grass.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        ui.painter().text(egui::pos2(rect.left() + 52.0, rect.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(18.0), TEXT);
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
            let galley = ui.painter().layout_no_wrap(tab.label().to_string(), egui::FontId::proportional(15.0), TEXT);
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
            match tab {
                Tab::Home => paint_house(ui.painter(), icon_at, color),
                Tab::Versions => paint_layers(ui.painter(), icon_at, color),
                Tab::Mods => paint_cube(ui.painter(), icon_at, color),
                Tab::Settings => paint_gear(ui.painter(), icon_at, color),
            }
            ui.painter().text(egui::pos2(item.left() + 30.0, item.center().y), egui::Align2::LEFT_CENTER, tab.label(), egui::FontId::proportional(15.0), color);
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
            ui.painter().text(hero.center(), egui::Align2::CENTER_CENTER, "Fetching the collection wallpaper…", egui::FontId::proportional(16.0), MUTED);
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

    fn page_versions(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap().shrink(8.0);
        let _ = ui.allocate_rect(ui.available_rect_before_wrap(), Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(outer));
        let ui = &mut child;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Version").size(28.0).color(TEXT));
                ui.label(RichText::new(&self.versions_note).size(14.0).color(MUTED));
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            field(ui, &mut self.query, 280.0, "Search versions");
            ui.add_space(8.0);
            let _ = button(ui, "Releases", 100.0, 44.0, true);
            if button(ui, "Snapshots", 110.0, 44.0, self.snapshots) {
                self.snapshots = !self.snapshots;
                self.save_settings();
                spawn_versions(self.app.clone(), self.tx.clone());
            }
            if button(ui, "Previews", 104.0, 44.0, self.previews) {
                self.previews = !self.previews;
                self.save_settings();
                spawn_versions(self.app.clone(), self.tx.clone());
            }
        });
        ui.add_space(12.0);
        let list_w = (ui.available_width() - 356.0).max(360.0);
        let body_h = ui.available_height();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(list_w);
                ui.set_min_height(body_h);
                let q = self.query.to_lowercase();
                let rows: Vec<Ver> = self.versions.iter().filter(|v| q.is_empty() || v.id.to_lowercase().contains(&q)).cloned().collect();
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .max_height(body_h)
                    .show(ui, |ui| {
                        for v in rows {
                            let on = self.selected == v.id;
                            let paper = wallpapers::for_version(&v.id);
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), Sense::click());
                            ui.painter().rect_filled(rect, 8.0, if on { SELECTED } else { SURFACE });
                            ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { ACCENT } else { BORDER }), egui::StrokeKind::Inside);
                            if let Some(tex) = self.papers.get(paper.title) {
                                let thumb = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 10.0), egui::vec2(44.0, 44.0));
                                ui.painter().image(tex.id(), thumb, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            ui.painter().text(rect.min + egui::vec2(64.0, 14.0), egui::Align2::LEFT_TOP, &v.id, egui::FontId::proportional(15.0), TEXT);
                            ui.painter().text(rect.min + egui::vec2(64.0, 34.0), egui::Align2::LEFT_TOP, format!("{} · {}", paper.title, v.channel), egui::FontId::proportional(12.0), MUTED);
                            let status = if on { "Selected" } else if v.installed { "Installed" } else { "Not installed" };
                            let accent = on || v.installed;
                            let chip_w = 108.0;
                            let chip_r = egui::Rect::from_min_size(egui::pos2(rect.right() - chip_w - 12.0, rect.center().y - 12.0), egui::vec2(chip_w, 24.0));
                            ui.painter().rect_stroke(chip_r, 12.0, egui::Stroke::new(1.0, if accent { ACCENT } else { BORDER_BTN }), egui::StrokeKind::Inside);
                            ui.painter().text(chip_r.center(), egui::Align2::CENTER_CENTER, status, egui::FontId::proportional(12.0), if accent { ACCENT } else { MUTED });
                            if resp.clicked() {
                                self.selected = v.id.clone();
                                self.save_settings();
                            }
                            if resp.has_focus() {
                                ui.painter().rect_stroke(rect.expand(2.0), 8.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Outside);
                            }
                            ui.add_space(6.0);
                        }
                    });
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_width(340.0);
                card(ui, |ui| {
                    let paper = wallpapers::for_version(&self.selected);
                    if let Some(tex) = self.papers.get(paper.title) {
                        ui.add(egui::Image::from_texture((tex.id(), tex.size_vec2())).fit_to_exact_size(egui::vec2(300.0, 72.0)));
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new(&self.selected).size(22.0).color(TEXT));
                    ui.label(RichText::new(paper.title).size(14.0).color(MUTED));
                    ui.add_space(8.0);
                    ui.label(RichText::new("INSTALL OPTIONS").size(12.0).color(ACCENT));
                    ui.add_space(4.0);
                    for p in &mut self.patches {
                        check(ui, &mut p.enabled, &p.title);
                    }
                    ui.add_space(8.0);
                    if button(ui, &format!("Install {}", self.selected), 300.0, 36.0, true) {
                        self.start_install();
                    }
                    if let Some((progress, line)) = self.job_line() {
                        ui.add_space(8.0);
                        ui.add(egui::ProgressBar::new(progress.clamp(0.0, 1.0)).text(line));
                    }
                });
            });
        });
    }

    fn page_mods(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap().shrink(8.0);
        let _ = ui.allocate_rect(ui.available_rect_before_wrap(), Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(outer));
        let ui = &mut child;
        ui.label(RichText::new("Mod").size(28.0).color(TEXT));
        ui.label(RichText::new("Enabled jars are staged into the instance when you play.").size(15.0).color(MUTED));
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
        let (zone, zresp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 160.0), Sense::click());
        let hover = zresp.hovered() || ui.input(|i| !i.raw.hovered_files.is_empty());
        ui.painter().rect_filled(zone, 12.0, if hover { SURFACE2 } else { SURFACE });
        dash_rect(ui, zone);
        ui.painter().text(zone.center() + egui::vec2(0.0, -16.0), egui::Align2::CENTER_CENTER, "Drop .jar files here", egui::FontId::proportional(18.0), TEXT);
        ui.painter().text(zone.center() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, "Choose files", egui::FontId::proportional(14.0), ACCENT);
        if zresp.clicked() {
            if let Some(path) = crate::dialog::pick_file("Add a mod", "*.jar") {
                let _ = crate::mods::add_path(&self.app, &path);
                spawn_mods(self.app.clone(), self.tx.clone());
            }
        }
        ui.add_space(12.0);
        let enabled = self.mods.iter().filter(|m| m.1).count();
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.app.p("mods").display().to_string()).size(13.0).color(MUTED));
            ui.add_space(8.0);
            if button(ui, "Copy path", 110.0, 36.0, false) {
                ui.output_mut(|o| o.copied_text = self.app.p("mods").display().to_string());
                self.notice = "Path copied".into();
            }
            ui.add_space(12.0);
            ui.label(RichText::new(format!("{} mods · {enabled} enabled", self.mods.len())).size(13.0).color(MUTED));
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
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
                        ui.label(RichText::new(format_bytes(size)).size(12.0).color(MUTED));
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
        let outer = ui.available_rect_before_wrap().shrink(8.0);
        let _ = ui.allocate_rect(ui.available_rect_before_wrap(), Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(outer));
        let ui = &mut child;
        ui.label(RichText::new("Settings").size(28.0).color(TEXT));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 300.0);
                card(ui, |ui| {
                    ui.label(RichText::new("PROFILE").size(12.0).color(ACCENT));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width() - 280.0);
                            ui.label(RichText::new("Username").size(14.0).color(TEXT));
                            if field(ui, &mut self.username, ui.available_width(), "Player").changed() { self.save_settings(); }
                            ui.add_space(8.0);
                            ui.label(RichText::new("Skin").size(14.0).color(TEXT));
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
                            ui.add_space(8.0);
                            ui.label(RichText::new("Cape").size(14.0).color(TEXT));
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
                                ui.label(RichText::new(&self.import_error).size(13.0).color(ERR));
                            }
                            ui.add_space(8.0);
                            ui.label(RichText::new("Local account only. No Microsoft login.").size(13.0).color(MUTED));
                            ui.label(RichText::new(crate::state::offline_uuid(&self.username)).size(13.0).color(MUTED).monospace());
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
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_min_width(280.0);
                card(ui, |ui| {
                    ui.label(RichText::new("GAME").size(12.0).color(ACCENT));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Memory").size(14.0).color(TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} MB", self.ram)).size(14.0).color(TEXT));
                        });
                    });
                    if slider(ui, &mut self.ram, 512, 16384) { self.save_settings(); }
                    ui.add_space(10.0);
                    ui.label(RichText::new("Game window").size(14.0).color(TEXT));
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
                    ui.add_space(8.0);
                    if check(ui, &mut self.snapshots, "Show snapshots (series 26 and up)") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    if check(ui, &mut self.previews, "Show previews (series 26 and up)") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("Data folder").size(14.0).color(TEXT));
                    ui.label(RichText::new(self.app.dir.display().to_string()).size(12.0).color(MUTED));
                    if button(ui, "Open", 88.0, 40.0, false) {
                        if let Err(e) = crate::dialog::open_dir(&self.app.dir) { self.notice = e; }
                    }
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
    let fill = if on { SELECTED } else if hit.hovered() { glass_hover() } else { glass_fill() };
    glass(ui.painter(), rect, 10.0, fill);
    if on {
        ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(1.5, ACCENT), egui::StrokeKind::Inside);
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
    let fill = if hit.hovered() { egui::Color32::from_rgb(0x5B, 0x9B, 0xFF) } else { ACCENT };
    let shadow = rect.translate(egui::vec2(0.0, 3.0));
    ui.painter().rect_filled(shadow, 10.0, egui::Color32::from_black_alpha(80));
    ui.painter().rect_filled(rect, 10.0, fill);
    let tri = rect.center() + egui::vec2(-28.0, 0.0);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![tri + egui::vec2(-6.0, -8.0), tri + egui::vec2(-6.0, 8.0), tri + egui::vec2(8.0, 0.0)],
        ON_ACCENT,
        egui::Stroke::NONE,
    ));
    ui.painter().text(rect.center() + egui::vec2(10.0, 0.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(20.0), ON_ACCENT);
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn note_width(ui: &egui::Ui, text: &str) -> f32 {
    ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(13.0), MUTED).size().x
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
    let fill = if primary { ACCENT } else if resp.hovered() { SELECTED } else { SURFACE2 };
    ui.painter().rect_filled(rect, 8.0, fill);
    if !primary { ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, BORDER_BTN), egui::StrokeKind::Inside); }
    if resp.has_focus() { ui.painter().rect_stroke(rect.expand(2.0), 8.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Outside); }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), if primary { ON_ACCENT } else { TEXT });
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
            .min_size(egui::vec2(width, 44.0))
            .margin(egui::vec2(10.0, 12.0)),
    )
}

fn check(ui: &mut egui::Ui, on: &mut bool, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(180.0), 26.0), Sense::click());
    let box_r = egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 6.0), egui::vec2(18.0, 18.0));
    ui.painter().rect_filled(box_r, 3.0, BG);
    ui.painter().rect_stroke(box_r, 3.0, egui::Stroke::new(1.5, if *on { ACCENT } else { BORDER_INPUT }), egui::StrokeKind::Inside);
    if *on { ui.painter().rect_filled(box_r.shrink(4.0), 2.0, ACCENT); }
    ui.painter().text(rect.min + egui::vec2(28.0, 8.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(14.0), TEXT);
    if resp.has_focus() { ui.painter().rect_stroke(box_r.expand(2.0), 3.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Outside); }
    if resp.clicked() { *on = !*on; true } else { false }
}

fn slider(ui: &mut egui::Ui, value: &mut u32, min: u32, max: u32) -> bool {
    let width = ui.available_width().max(160.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::click_and_drag());
    let track = egui::Rect::from_min_max(egui::pos2(rect.left(), rect.center().y - 3.0), egui::pos2(rect.right(), rect.center().y + 3.0));
    ui.painter().rect_filled(track, 3.0, BORDER_INPUT);
    let t = (*value as f32 - min as f32) / (max as f32 - min as f32);
    let fill = egui::Rect::from_min_max(track.min, egui::pos2(track.left() + track.width() * t, track.bottom()));
    ui.painter().rect_filled(fill, 3.0, ACCENT);
    let hx = track.left() + track.width() * t;
    ui.painter().circle_filled(egui::pos2(hx, track.center().y), 8.0, ACCENT);
    ui.painter().circle_stroke(egui::pos2(hx, track.center().y), 8.0, egui::Stroke::new(2.0, ON_ACCENT));
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
    egui::Frame::new().fill(SURFACE).stroke(egui::Stroke::new(1.0, BORDER)).corner_radius(12.0).inner_margin(14.0).show(ui, add);
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

fn install_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("MozillaHeadline".into(), egui::FontData::from_static(include_bytes!("../assets/MozillaHeadline-600.ttf")).into());
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "MozillaHeadline".into());
    fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "MozillaHeadline".into());
    ctx.set_fonts(fonts);
}

fn style_ui(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = BG;
    style.visuals.extreme_bg_color = BG;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = SELECTED;
    style.visuals.selection.stroke = egui::Stroke::new(2.0, ACCENT);
    let radius = egui::CornerRadius::same(8);
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.corner_radius = radius;
        w.bg_stroke = egui::Stroke::new(1.5, BORDER_INPUT);
        w.fg_stroke.color = TEXT;
    }
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE2;
    style.visuals.widgets.hovered.weak_bg_fill = SELECTED;
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.active.weak_bg_fill = ACCENT;
    style.visuals.widgets.active.fg_stroke.color = ON_ACCENT;
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
    let opts = if name == "mark" || name == "avatar" { egui::TextureOptions::NEAREST } else { egui::TextureOptions::LINEAR };
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

fn spawn_versions(app: Arc<App>, tx: Sender<Msg>) {
    std::thread::spawn(move || match crate::mojang::version_list(&app) {
        Ok(v) => {
            let mut out = Vec::new();
            if let Some(arr) = v.get("versions").and_then(|x| x.as_array()) {
                for item in arr {
                    out.push(Ver {
                        id: item.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
                        channel: item.get("channel").and_then(|x| x.as_str()).unwrap_or("").into(),
                        installed: item.get("installed").and_then(|x| x.as_bool()).unwrap_or(false),
                    });
                }
            }
            let _ = tx.send(Msg::Versions(out));
        }
        Err(e) => { let _ = tx.send(Msg::VersionsErr(e)); }
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
        for paper in wallpapers::all() {
            if let Ok(bytes) = wallpapers::load_cached(&dir, paper) {
                let _ = tx.send(Msg::Paper { title: paper.title.into(), bytes });
            }
        }
    });
}
