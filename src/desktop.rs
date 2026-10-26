//! Native desktop window. No title bar, no browser, no web view.

use crate::state::{now_ms, App, Job};
use crate::wallpapers::{self, Paper};
use eframe::egui::{self, CursorIcon, ResizeDirection, Sense, ViewportCommand};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(198, 242, 92);
const INK: egui::Color32 = egui::Color32::from_rgb(16, 20, 10);
const BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 8);
const SIDE: egui::Color32 = egui::Color32::from_rgb(12, 12, 12);
const PANEL: egui::Color32 = egui::Color32::from_rgb(18, 18, 18);
const LINE: egui::Color32 = egui::Color32::from_rgb(42, 42, 42);
const DIM: egui::Color32 = egui::Color32::from_rgb(176, 176, 176);
const MUTED: egui::Color32 = egui::Color32::from_rgb(118, 118, 118);
const EXIT_HOT: egui::Color32 = egui::Color32::from_rgb(240, 176, 172);

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
            Tab::Versions => "Versions",
            Tab::Mods => "Mods",
            Tab::Settings => "Settings",
        }
    }
}

#[derive(Clone)]
struct Ver {
    id: String,
    channel: String,
    released: String,
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
    job_id: Option<String>,
    patches: Vec<crate::state::PatchState>,
    shot: Option<String>,
    frames: u32,
    shot_requested: bool,
}

pub fn run(shot: Option<String>, tab: String) -> Result<(), String> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 760.0])
        .with_min_inner_size([1040.0, 680.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_title("Rus")
        .with_icon(icon_data());
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Rus",
        options,
        Box::new(move |cc| Ok(Box::new(Desktop::new(cc, shot, tab)))),
    )
    .map_err(|e| e.to_string())
}

fn icon_data() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/mark-32.png"))
        .expect("mark")
        .to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
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
        let selected = if settings.last_version.is_empty() {
            "26.3".into()
        } else {
            settings.last_version.clone()
        };
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
            job_id: None,
            patches: crate::install::patch_defs(),
            shot,
            frames: 0,
            shot_requested: false,
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
                        if let Some(first) = v.first() {
                            self.selected = first.id.clone();
                        }
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
        self.app.update_settings(s);
    }

    fn paper_for(&self, id: &str) -> &'static Paper {
        wallpapers::for_version(id)
    }

    fn play(&mut self) {
        let running = crate::play::status();
        if running.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
            let _ = crate::play::kill();
            self.notice = "Stopped".into();
            return;
        }
        self.save_settings();
        if self.engine == "horizon" {
            let app = self.app.clone();
            let tx = self.tx.clone();
            self.notice = "Preparing Horizon…".into();
            std::thread::spawn(move || {
                let msg = match crate::horizon::start(app) {
                    Ok(v) => v
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Horizon is starting")
                        .to_string(),
                    Err(e) => e,
                };
                let _ = tx.send(Msg::Notice(msg));
            });
            return;
        }
        let id = self.selected.clone();
        if !self.app.is_installed(&id) {
            self.tab = Tab::Versions;
            self.start_install();
            return;
        }
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

    fn start_install(&mut self) {
        let version = self.selected.clone();
        if version.is_empty() {
            self.notice = "Select a version".into();
            return;
        }
        let id = format!("job-{}", now_ms());
        let mut patches = self.patches.clone();
        for p in patches.iter_mut() {
            p.status = "pending".into();
        }
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

    fn quit(&self, ctx: &egui::Context) {
        let _ = crate::play::kill();
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}

impl eframe::App for Desktop {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [8.0 / 255.0, 8.0 / 255.0, 8.0 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::SidePanel::left("side")
            .exact_width(248.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDE).inner_margin(14.0))
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show(ctx, |ui| match self.tab {
                Tab::Home => self.page_home(ui),
                Tab::Versions => self.page_versions(ui),
                Tab::Mods => self.page_mods(ui),
                Tab::Settings => self.page_settings(ui),
            });

        let screen = ctx.screen_rect();
        let seam = egui::Rect::from_min_max(
            egui::pos2(248.0, 0.0),
            egui::pos2(249.0, screen.height()),
        );
        ctx.layer_painter(egui::LayerId::background()).rect_filled(seam, 0.0, LINE);
        window_edges(ctx);
        paint_frame(ctx);

        self.frames += 1;
        let paper_ready = self.papers.contains_key(self.paper_for(&self.selected).title);
        let versions_ready = !self.versions.is_empty() || self.frames > 40;
        if self.shot.is_some()
            && !self.shot_requested
            && self.frames > 6
            && (paper_ready || self.frames > 90)
            && versions_ready
        {
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
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        });
    }
}

impl Desktop {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        drag_block(ui, 58.0, |ui, rect| {
            if let Some(mark) = &self.mark {
                let mark_rect = egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 8.0), egui::vec2(40.0, 40.0));
                ui.painter().image(mark.id(), mark_rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            ui.painter().text(
                rect.min + egui::vec2(52.0, 16.0),
                egui::Align2::LEFT_TOP,
                "RUS",
                egui::FontId::proportional(26.0),
                egui::Color32::WHITE,
            );
            ui.painter().text(
                rect.min + egui::vec2(52.0, 40.0),
                egui::Align2::LEFT_TOP,
                "JAVA EDITION",
                egui::FontId::proportional(10.0),
                MUTED,
            );
        });
        ui.add_space(10.0);
        for tab in [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings] {
            if nav_button(ui, tab.label(), self.tab == tab) {
                self.tab = tab;
            }
            ui.add_space(4.0);
        }

        let footer = 292.0;
        let gap = (ui.available_height() - footer).max(8.0);
        drag_block(ui, gap, |_, _| {});

        ui.label(egui::RichText::new("LOCAL ACCOUNT").size(10.0).color(ACCENT));
        ui.label(egui::RichText::new(&self.username).strong());
        ui.add_space(6.0);
        let box_ = crate::sandbox::describe(&self.app);
        let label = box_.get("label").and_then(|v| v.as_str()).unwrap_or("Sandbox");
        ui.label(egui::RichText::new(label).size(11.0).strong().color(ACCENT));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("LAUNCH WITH").size(10.0).color(MUTED));
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 8.0) / 2.0;
            if engine_button(ui, "Vanilla", w, self.engine != "horizon") {
                self.engine = "vanilla".into();
                self.save_settings();
            }
            if engine_button(ui, "Horizon", w, self.engine == "horizon") {
                self.engine = "horizon".into();
                self.save_settings();
            }
        });
        ui.add_space(8.0);
        ui.label(egui::RichText::new("SELECTED").size(10.0).color(MUTED));
        ui.label(
            egui::RichText::new(if self.engine == "horizon" {
                "26.3"
            } else {
                self.selected.as_str()
            })
            .strong()
            .color(egui::Color32::WHITE),
        );
        ui.add_space(8.0);
        let running = crate::play::status();
        let live = running.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
        let play_label = if live {
            "STOP"
        } else if self.engine == "horizon" {
            "PLAY HORIZON"
        } else {
            "PLAY"
        };
        let full = ui.available_width();
        if solid_button(
            ui,
            play_label,
            full,
            44.0,
            if live { egui::Color32::from_rgb(42, 24, 24) } else { ACCENT },
            if live { EXIT_HOT } else { INK },
        ) {
            self.play();
        }
        ui.add_space(6.0);
        if solid_button(ui, "Exit", full, 36.0, egui::Color32::from_rgb(22, 22, 22), egui::Color32::WHITE) {
            self.quit(ui.ctx());
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new(&self.notice).size(11.0).color(DIM));
    }

    fn page_home(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if resp.double_clicked() {
            toggle_max(ui.ctx());
        }
        let paper = self.paper_for(&self.selected);
        if let Some(tex) = self.papers.get(paper.title) {
            let uv = cover_uv(tex.size_vec2(), rect);
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(rect, 0.0, PANEL);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Fetching the official collection wallpaper…",
                egui::FontId::proportional(16.0),
                DIM,
            );
        }
        paint_fade(ui, rect);
        let installed = self.app.is_installed(&self.selected);
        let status = if installed { "Installed" } else { "Not installed" };
        ui.painter().text(
            egui::pos2(rect.left() + 22.0, rect.bottom() - 196.0),
            egui::Align2::LEFT_TOP,
            paper.title,
            egui::FontId::proportional(36.0),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 22.0, rect.bottom() - 152.0),
            egui::Align2::LEFT_TOP,
            format!("{}   ·   {}   ·   sandbox", self.selected, status),
            egui::FontId::proportional(14.0),
            DIM,
        );

        let strip = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 16.0, rect.bottom() - 128.0),
            egui::pos2(rect.right() - 8.0, rect.bottom() - 12.0),
        );
        let ctx = ui.ctx().clone();
        let papers = self.papers.clone();
        let selected = self.selected.clone();
        let mut pick: Option<String> = None;
        egui::Area::new(egui::Id::new("collection-strip"))
            .order(egui::Order::Middle)
            .fixed_pos(strip.min)
            .show(&ctx, |ui| {
                ui.set_width(strip.width());
                ui.set_height(strip.height());
                egui::ScrollArea::horizontal().id_salt("films").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for paper in wallpapers::all() {
                            let size = egui::vec2(100.0, 100.0);
                            let on = paper.prefixes.iter().any(|p| version_hit(&selected, p));
                            if let Some(tex) = papers.get(paper.title) {
                                let img = ui.add(
                                    egui::Image::from_texture((tex.id(), tex.size_vec2()))
                                        .fit_to_exact_size(size)
                                        .sense(Sense::click()),
                                );
                                if on {
                                    ui.painter().rect_stroke(
                                        img.rect.expand(1.0),
                                        0.0,
                                        egui::Stroke::new(2.0_f32, ACCENT),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                img.clone().on_hover_text(paper.title);
                                if img.clicked() {
                                    if let Some(prefix) = paper.prefixes.first() {
                                        pick = Some((*prefix).to_string());
                                    }
                                }
                            } else {
                                let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                                ui.painter().rect_filled(r, 0.0, PANEL);
                            }
                        }
                    });
                });
            });
        if let Some(prefix) = pick {
            if let Some(v) = self.versions.iter().find(|v| version_hit(&v.id, &prefix)) {
                self.selected = v.id.clone();
                self.save_settings();
            }
        }
    }

    fn page_versions(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap();
        ui.allocate_rect(outer, Sense::hover());
        let inner = outer.shrink(18.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        let ui = &mut child;
        ui.label(egui::RichText::new("Versions").size(26.0).strong());
        ui.label(egui::RichText::new(&self.versions_note).color(DIM));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search versions")
                    .desired_width(280.0),
            );
            if solid_button(ui, "Install selected", 168.0, 32.0, ACCENT, INK) {
                self.start_install();
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for p in &mut self.patches {
                ui.checkbox(&mut p.enabled, &p.title);
            }
        });
        if let Some((progress, line)) = self.job_line() {
            ui.add(egui::ProgressBar::new(progress.clamp(0.0, 1.0)).text(line));
        }
        ui.add_space(8.0);
        let q = self.query.to_lowercase();
        let rows: Vec<Ver> = self
            .versions
            .iter()
            .filter(|v| q.is_empty() || v.id.to_lowercase().contains(&q))
            .take(80)
            .cloned()
            .collect();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for v in rows {
                let paper = wallpapers::for_version(&v.id);
                let selected = self.selected == v.id;
                let frame = egui::Frame::new()
                    .fill(if selected {
                        egui::Color32::from_rgb(24, 28, 18)
                    } else {
                        PANEL
                    })
                    .stroke(egui::Stroke::new(1.0_f32, if selected { ACCENT } else { LINE }))
                    .inner_margin(8.0);
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(tex) = self.papers.get(paper.title) {
                            ui.add(
                                egui::Image::from_texture((tex.id(), tex.size_vec2()))
                                    .fit_to_exact_size(egui::vec2(46.0, 46.0)),
                            );
                        } else {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, SIDE);
                        }
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&v.id).strong());
                            ui.label(
                                egui::RichText::new(format!("{} · {}", paper.title, v.channel))
                                    .size(12.0)
                                    .color(DIM),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if solid_button(
                                ui,
                                if v.installed { "Repair" } else { "Install" },
                                84.0,
                                30.0,
                                PANEL,
                                egui::Color32::WHITE,
                            ) {
                                self.selected = v.id.clone();
                                self.save_settings();
                                self.start_install();
                            }
                            ui.add_space(6.0);
                            if solid_button(ui, "Select", 72.0, 30.0, if selected { ACCENT } else { PANEL }, if selected { INK } else { DIM }) {
                                self.selected = v.id.clone();
                                self.save_settings();
                            }
                        });
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

    fn page_mods(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap();
        ui.allocate_rect(outer, Sense::hover());
        let inner = outer.shrink(18.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        let ui = &mut child;
        ui.label(egui::RichText::new("Mods").size(26.0).strong());
        ui.label(egui::RichText::new("Jars in the Rus mods folder. Enabled ones are staged when you play.").color(DIM));
        ui.add_space(12.0);
        group(ui, |ui| {
            if self.mods.is_empty() {
                ui.label(egui::RichText::new("No mods yet").size(18.0).strong());
                ui.add_space(4.0);
                ui.label("Drop a .jar into this folder.");
                ui.add_space(6.0);
                ui.monospace(self.app.p("mods").display().to_string());
            } else {
                for (name, enabled, size) in self.mods.clone() {
                    ui.horizontal(|ui| {
                        let mut on = enabled;
                        if ui.checkbox(&mut on, &name).changed() {
                            let _ = crate::mods::toggle(&self.app, &name);
                            spawn_mods(self.app.clone(), self.tx.clone());
                        }
                        ui.label(egui::RichText::new(format_bytes(size)).color(MUTED));
                    });
                }
            }
        });
    }

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_rect_before_wrap();
        ui.allocate_rect(outer, Sense::hover());
        let inner = outer.shrink(18.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        let ui = &mut child;
        ui.label(egui::RichText::new("Settings").size(26.0).strong());
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                group(ui, |ui| {
                    ui.set_min_width(420.0);
                    ui.label(egui::RichText::new("ACCOUNT").size(11.0).color(ACCENT));
                    ui.add_space(6.0);
                    ui.label("Username");
                    if ui.text_edit_singleline(&mut self.username).changed() {
                        self.save_settings();
                    }
                    ui.add_space(8.0);
                    ui.label("Skin");
                    ui.horizontal(|ui| {
                        for (id, label) in [("preset:steve", "Steve"), ("preset:alex", "Alex")] {
                            if solid_button(ui, label, 88.0, 30.0, if self.skin == id { ACCENT } else { PANEL }, if self.skin == id { INK } else { DIM }) {
                                self.skin = id.into();
                                self.save_settings();
                            }
                            ui.add_space(6.0);
                        }
                    });
                    ui.label("Cape");
                    if solid_button(ui, "Rus", 88.0, 30.0, if self.cape == "preset:rus" { ACCENT } else { PANEL }, INK) {
                        self.cape = "preset:rus".into();
                        self.save_settings();
                    }
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Local account only. No Microsoft login.").color(DIM));
                    ui.label(egui::RichText::new(crate::state::offline_uuid(&self.username)).size(12.0).color(MUTED));
                });
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                group(ui, |ui| {
                    ui.set_min_width(420.0);
                    ui.label(egui::RichText::new("GAME").size(11.0).color(ACCENT));
                    ui.add_space(6.0);
                    ui.label(format!("Memory  {} MB", self.ram));
                    if ui.add(egui::Slider::new(&mut self.ram, 512..=16384).step_by(256.0)).changed() {
                        self.save_settings();
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Game window");
                        let mut w = self.width as i32;
                        let mut h = self.height as i32;
                        if ui.add(egui::DragValue::new(&mut w).range(640..=3840)).changed() {
                            self.width = w as u32;
                            self.save_settings();
                        }
                        ui.label("×");
                        if ui.add(egui::DragValue::new(&mut h).range(480..=2160)).changed() {
                            self.height = h as u32;
                            self.save_settings();
                        }
                    });
                    ui.add_space(8.0);
                    if ui.checkbox(&mut self.snapshots, "Show snapshots from series 26 up").changed() {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    if ui.checkbox(&mut self.previews, "Show previews from series 26 up").changed() {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(self.app.dir.display().to_string()).size(12.0).color(MUTED));
                });
            });
        });
    }
}

fn group(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0_f32, LINE))
        .inner_margin(14.0)
        .show(ui, add);
}

fn drag_block(ui: &mut egui::Ui, height: f32, paint: impl FnOnce(&mut egui::Ui, egui::Rect)) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click_and_drag());
    paint(ui, rect);
    if resp.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if resp.double_clicked() {
        toggle_max(ui.ctx());
    }
}

fn toggle_max(ctx: &egui::Context) {
    let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maxed));
}

fn nav_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 40.0), Sense::click());
    let fill = if on {
        egui::Color32::from_rgb(22, 26, 16)
    } else if resp.hovered() {
        egui::Color32::from_rgb(22, 22, 22)
    } else {
        SIDE
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    if on {
        let bar = egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 3.0, rect.max.y));
        ui.painter().rect_filled(bar, 0.0, ACCENT);
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(15.0),
        if on { egui::Color32::WHITE } else { DIM },
    );
    resp.clicked()
}

fn engine_button(ui: &mut egui::Ui, label: &str, width: f32, on: bool) -> bool {
    solid_button(ui, label, width, 30.0, if on { ACCENT } else { PANEL }, if on { INK } else { DIM })
}

fn solid_button(ui: &mut egui::Ui, label: &str, width: f32, height: f32, fill: egui::Color32, text: egui::Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width.max(64.0), height), Sense::click());
    let mut bg = fill;
    if resp.hovered() && fill != ACCENT {
        bg = egui::Color32::from_rgb(32, 32, 32);
    }
    if label == "Exit" && resp.hovered() {
        bg = egui::Color32::from_rgb(42, 24, 24);
    }
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, if label == "Exit" && resp.hovered() { EXIT_HOT } else { LINE }), egui::StrokeKind::Inside);
    let color = if label == "Exit" && resp.hovered() { EXIT_HOT } else { text };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        color,
    );
    resp.clicked()
}

fn paint_fade(ui: &egui::Ui, rect: egui::Rect) {
    let fade_top = rect.top() + rect.height() * 0.38;
    let steps = 18;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let y0 = fade_top + (rect.bottom() - fade_top) * t0;
        let y1 = fade_top + (rect.bottom() - fade_top) * t1;
        let band = egui::Rect::from_min_max(egui::pos2(rect.left(), y0), egui::pos2(rect.right(), y1));
        ui.painter().rect_filled(band, 0.0, egui::Color32::from_black_alpha((t1 * 220.0) as u8));
    }
}

fn paint_frame(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("frame")))
        .rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, LINE), egui::StrokeKind::Inside);
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
    egui::Area::new(egui::Id::new("window-edges"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .interactable(true)
        .show(ctx, |ui| {
            for (i, (dir, zone, cursor)) in zones.iter().enumerate() {
                let resp = ui.interact(*zone, egui::Id::new("edge").with(i), Sense::click_and_drag());
                if resp.hovered() || resp.dragged() {
                    ui.ctx().set_cursor_icon(*cursor);
                }
                if resp.drag_started() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(*dir));
                }
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
    fonts.font_data.insert(
        "MozillaHeadline".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/MozillaHeadline-600.ttf")).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "MozillaHeadline".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "MozillaHeadline".to_owned());
    ctx.set_fonts(fonts);
}

fn style_ui(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = BG;
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(10, 10, 10);
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.inactive.bg_fill = PANEL;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(32, 32, 32);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(243, 243, 243));
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(36, 48, 20);
    let zero = egui::CornerRadius::ZERO;
    style.visuals.widgets.noninteractive.corner_radius = zero;
    style.visuals.widgets.inactive.corner_radius = zero;
    style.visuals.widgets.hovered.corner_radius = zero;
    style.visuals.widgets.active.corner_radius = zero;
    style.visuals.widgets.open.corner_radius = zero;
    style.visuals.window_corner_radius = zero;
    style.visuals.menu_corner_radius = zero;
    ctx.set_style(style);
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let opts = if name == "mark" {
        egui::TextureOptions::NEAREST
    } else {
        egui::TextureOptions::LINEAR
    };
    Some(ctx.load_texture(name, color, opts))
}

fn save_png(path: &str, image: &egui::ColorImage) -> Result<(), String> {
    let img = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, image.as_raw().to_vec())
        .ok_or("bad image")?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    img.save(path).map_err(|e| e.to_string())
}

fn version_hit(id: &str, prefix: &str) -> bool {
    id == prefix || id.starts_with(&format!("{prefix}.")) || id.starts_with(&format!("{prefix}-"))
}

fn format_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{} KB", n / 1024)
    } else {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    }
}

fn spawn_versions(app: Arc<App>, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        match crate::mojang::version_list(&app) {
            Ok(v) => {
                let mut out = Vec::new();
                if let Some(arr) = v.get("versions").and_then(|x| x.as_array()) {
                    for item in arr {
                        out.push(Ver {
                            id: item.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
                            channel: item.get("channel").and_then(|x| x.as_str()).unwrap_or("").into(),
                            released: item.get("released").and_then(|x| x.as_str()).unwrap_or("").into(),
                            installed: item.get("installed").and_then(|x| x.as_bool()).unwrap_or(false),
                        });
                    }
                }
                let _ = tx.send(Msg::Versions(out));
            }
            Err(e) => {
                let _ = tx.send(Msg::VersionsErr(e));
            }
        }
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
                let _ = tx.send(Msg::Paper {
                    title: paper.title.into(),
                    bytes,
                });
            }
        }
    });
}
