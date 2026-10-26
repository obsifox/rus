//! Native window. No browser, no web view, no local page.

use crate::state::{now_ms, App, Job};
use crate::wallpapers::{self, Paper};
use eframe::egui;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(198, 242, 92);
const INK: egui::Color32 = egui::Color32::from_rgb(16, 20, 10);
const BG: egui::Color32 = egui::Color32::from_rgb(7, 7, 7);
const SIDE: egui::Color32 = egui::Color32::from_rgb(14, 14, 14);
const PANEL: egui::Color32 = egui::Color32::from_rgb(18, 18, 18);
const LINE: egui::Color32 = egui::Color32::from_rgb(44, 44, 44);
const DIM: egui::Color32 = egui::Color32::from_rgb(163, 163, 163);
const MUTED: egui::Color32 = egui::Color32::from_rgb(124, 124, 124);

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
    let icon = icon_data();
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([980.0, 640.0])
        .with_title("Rus")
        .with_icon(icon);
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
            self.notice = "Preparing Horizon inside the sandbox…".into();
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
        self.notice = format!("Starting {id} in the sandbox…");
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
}

impl eframe::App for Desktop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
        ctx.request_repaint_after(Duration::from_millis(250));

        egui::SidePanel::left("side")
            .exact_width(248.0)
            .frame(egui::Frame::none().fill(SIDE).inner_margin(14.0))
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(22.0))
            .show(ctx, |ui| match self.tab {
                Tab::Home => self.page_home(ui),
                Tab::Versions => self.page_versions(ui),
                Tab::Mods => self.page_mods(ui),
                Tab::Settings => self.page_settings(ui),
            });

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
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let shot = self.shot.clone();
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(path) = &shot {
                        let _ = save_png(path, image);
                        std::process::exit(0);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });
    }
}

impl Desktop {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(mark) = &self.mark {
                ui.image((mark.id(), egui::vec2(36.0, 36.0)));
            }
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("RUS").size(26.0).strong().color(egui::Color32::WHITE));
                ui.label(
                    egui::RichText::new("JAVA EDITION")
                        .size(10.0)
                        .color(MUTED)
                        .strong(),
                );
            });
        });
        ui.add_space(16.0);
        for tab in [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings] {
            if nav_button(ui, tab.label(), self.tab == tab) {
                self.tab = tab;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("LOCAL ACCOUNT").size(10.0).color(ACCENT));
                ui.label(egui::RichText::new(&self.username).strong());
                ui.add_space(8.0);
                let box_ = crate::sandbox::describe(&self.app);
                let label = box_.get("label").and_then(|v| v.as_str()).unwrap_or("Sandbox");
                ui.label(egui::RichText::new(label).size(11.0).strong().color(ACCENT));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("LAUNCH WITH").size(10.0).color(MUTED));
                ui.horizontal(|ui| {
                    if engine_button(ui, "Vanilla", self.engine != "horizon") {
                        self.engine = "vanilla".into();
                        self.save_settings();
                    }
                    if engine_button(ui, "Horizon", self.engine == "horizon") {
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
                let btn = egui::Button::new(
                    egui::RichText::new(play_label).size(16.0).strong().color(if live {
                        egui::Color32::from_rgb(240, 180, 176)
                    } else {
                        INK
                    }),
                )
                .min_size(egui::vec2(ui.available_width(), 46.0))
                .fill(if live {
                    egui::Color32::from_rgb(42, 24, 24)
                } else {
                    ACCENT
                });
                if ui.add(btn).clicked() {
                    self.play();
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new(&self.notice).size(12.0).color(DIM));
            });
        });
    }

    fn page_home(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let paper = self.paper_for(&self.selected);
            ui.label(egui::RichText::new("COLLECTION").size(11.0).strong().color(ACCENT));
            ui.label(egui::RichText::new(paper.title).size(32.0).strong());
            ui.label(
                egui::RichText::new(format!(
                    "{} · official wallpaper from minecraft.net/collectibles",
                    self.selected
                ))
                .color(DIM),
            );
            ui.add_space(10.0);
            let tex = self.papers.get(paper.title).cloned();
            let installed = self.app.is_installed(&self.selected);
            ui.horizontal(|ui| {
                let size = egui::vec2(360.0, 360.0);
                if let Some(tex) = tex {
                    ui.add(
                        egui::Image::from_texture((tex.id(), tex.size_vec2())).fit_to_exact_size(size),
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, PANEL);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Fetching wallpaper…",
                        egui::FontId::proportional(15.0),
                        DIM,
                    );
                }
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("VERSION").size(11.0).color(MUTED));
                    ui.label(egui::RichText::new(&self.selected).size(28.0).strong());
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(if installed { "Installed" } else { "Not installed" })
                            .color(if installed { ACCENT } else { DIM }),
                    );
                    ui.add_space(12.0);
                    let box_ = crate::sandbox::describe(&self.app);
                    ui.label(
                        egui::RichText::new(box_.get("detail").and_then(|v| v.as_str()).unwrap_or(""))
                            .color(DIM),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Play starts this version in the sandbox. Horizon is only for 26.3.")
                            .color(DIM),
                    );
                });
            });
            ui.add_space(14.0);
            ui.label(egui::RichText::new("MINECRAFT COLLECTION").size(11.0).strong().color(MUTED));
            ui.add_space(6.0);
            egui::ScrollArea::horizontal().id_salt("collection").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for paper in wallpapers::all() {
                        ui.vertical(|ui| {
                            let size = egui::vec2(148.0, 148.0);
                            if let Some(tex) = self.papers.get(paper.title) {
                                let resp = ui
                                    .add(egui::Image::from_texture((tex.id(), tex.size_vec2())).fit_to_exact_size(size))
                                    .interact(egui::Sense::click());
                                if resp.clicked() {
                                    self.select_paper(paper);
                                }
                            } else {
                                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                ui.painter().rect_filled(rect, 0.0, PANEL);
                            }
                            ui.label(egui::RichText::new(paper.title).size(11.0).color(DIM));
                        });
                    }
                });
            });
            ui.add_space(12.0);
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
        ui.label(egui::RichText::new("Versions").size(28.0).strong());
        ui.label(egui::RichText::new(&self.versions_note).color(DIM));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search versions").desired_width(280.0));
            if ui.button("Install").clicked() {
                self.start_install();
            }
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Patches").size(11.0).color(MUTED));
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
                let frame = egui::Frame::none()
                    .fill(if selected { egui::Color32::from_rgb(24, 28, 18) } else { PANEL })
                    .stroke(egui::Stroke::new(1.0, if selected { ACCENT } else { LINE }))
                    .inner_margin(8.0);
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(tex) = self.papers.get(paper.title) {
                            ui.image((tex.id(), egui::vec2(48.0, 48.0)));
                        } else {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
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
                            if ui.button(if v.installed { "Repair" } else { "Install" }).clicked() {
                                self.selected = v.id.clone();
                                self.save_settings();
                                self.start_install();
                            }
                            if ui.button("Select").clicked() {
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
        ui.label(egui::RichText::new("Mods").size(28.0).strong());
        ui.label(egui::RichText::new("Jars in the Rus mods folder. They are staged when you play.").color(DIM));
        ui.add_space(10.0);
        if self.mods.is_empty() {
            ui.label("No mods yet. Drop a .jar into the mods folder.");
            ui.monospace(self.app.p("mods").display().to_string());
            return;
        }
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

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Settings").size(28.0).strong());
        ui.add_space(8.0);
        ui.label("Username");
        if ui.text_edit_singleline(&mut self.username).changed() {
            self.save_settings();
        }
        ui.add_space(8.0);
        ui.label(format!("Memory  {} MB", self.ram));
        if ui.add(egui::Slider::new(&mut self.ram, 512..=16384).step_by(256.0)).changed() {
            self.save_settings();
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Window");
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
        ui.add_space(12.0);
        ui.add_space(8.0);
        ui.label("Skin");
        ui.horizontal(|ui| {
            for (id, label) in [("preset:steve", "Steve"), ("preset:alex", "Alex")] {
                if ui.selectable_label(self.skin == id, label).clicked() {
                    self.skin = id.into();
                    self.save_settings();
                }
            }
        });
        ui.label("Cape");
        if ui.selectable_label(self.cape == "preset:rus", "Rus").clicked() {
            self.cape = "preset:rus".into();
            self.save_settings();
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Local account only. No Microsoft login.").color(DIM));
        ui.monospace(self.app.dir.display().to_string());
        let uuid = crate::state::offline_uuid(&self.username);
        ui.label(egui::RichText::new(uuid).color(MUTED).size(12.0));
    }
}

fn nav_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 42.0), egui::Sense::click());
    let fill = if on {
        egui::Color32::from_rgb(22, 26, 16)
    } else if resp.hovered() {
        egui::Color32::from_rgb(22, 22, 22)
    } else {
        egui::Color32::from_rgb(16, 16, 16)
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    if on {
        let mut bar = rect;
        bar.set_width(3.0);
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

fn engine_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(12.0)
            .color(if on { INK } else { DIM }),
    )
    .fill(if on { ACCENT } else { PANEL })
    .min_size(egui::vec2(100.0, 30.0));
    ui.add(btn).clicked()
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
    ctx.set_style(style);
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let mut opts = egui::TextureOptions::NEAREST;
    if name != "mark" {
        opts = egui::TextureOptions::LINEAR;
    }
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
        if let Ok(bytes) = wallpapers::load_cached(&dir, wallpapers::for_version("")) {
            let _ = tx.send(Msg::Paper {
                title: wallpapers::for_version("").title.into(),
                bytes,
            });
        }
    });
}
