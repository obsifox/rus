//! Settings tab.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn page_settings(&mut self, ui: &mut egui::Ui) {
        let title_id = title_font(22.0);
        let title_w = ui.painter().layout_no_wrap("Settings".into(), title_id.clone(), TEXT).size().x;
        let (title, _) = ui.allocate_exact_size(egui::vec2(40.0 + title_w + 14.0, 36.0), Sense::hover());
        paint_element_template(ui.painter(), title);
        paint_material(ui.painter(), egui::pos2(title.left() + 18.0, title.center().y), 18.0, I_FLARE, TEXT);
        ui.painter().text(egui::pos2(title.left() + 34.0, title.center().y), egui::Align2::LEFT_CENTER, "Settings", title_id, TEXT);
        ui.add_space(4.0);
        let width = ui.available_width();
        let body_h = ui.available_height();
        egui::ScrollArea::vertical()
            .max_height(body_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
        ui.set_min_width(width);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((width - 40.0) * 0.52);
                card(ui, |ui| {
                    element_label(ui, "PROFILE", sub_font(12.0), GOLD);
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 180.0).max(140.0));
                            element_label(ui, "Username", body_font(14.0), TEXT);
                            if field(ui, &mut self.username, ui.available_width(), "Player").changed() { self.save_settings(); }
                            ui.add_space(4.0);
                            element_label(ui, "Skin", body_font(14.0), TEXT);
                            ui.horizontal(|ui| {
                                if plain_action(ui, "Steve", 64.0, 32.0, self.skin == "preset:steve") {
                                    self.skin = "preset:steve".into();
                                    self.preview.arms = ArmModel::Classic;
                                    self.preview.invalidate();
                                    self.avatar_key.clear();
                                    self.save_settings();
                                }
                                gold_slot(ui, 32.0);
                                if plain_action(ui, "Alex", 64.0, 32.0, self.skin == "preset:alex") {
                                    self.skin = "preset:alex".into();
                                    self.preview.arms = ArmModel::Slim;
                                    self.preview.invalidate();
                                    self.avatar_key.clear();
                                    self.save_settings();
                                }
                                gold_slot(ui, 32.0);
                                if plain_action(ui, "Import skin", 108.0, 32.0, false) {
                                    if let Some(path) = crate::dialog::pick_file("Import skin", "*.png") {
                                        self.import_path("skin", &path);
                                    } else {
                                        self.import_error = "No file chosen. You can also drop a PNG here.".into();
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            element_label(ui, "Cape", body_font(14.0), TEXT);
                            ui.horizontal(|ui| {
                                if plain_action(ui, "Rus", 52.0, 32.0, self.cape == "preset:rus") {
                                    self.cape = "preset:rus".into();
                                    self.preview.invalidate();
                                    self.save_settings();
                                }
                                gold_slot(ui, 32.0);
                                if plain_action(ui, "None", 56.0, 32.0, self.cape == "preset:none" || self.cape.is_empty()) {
                                    self.cape = "preset:none".into();
                                    self.preview.invalidate();
                                    self.save_settings();
                                }
                                gold_slot(ui, 32.0);
                                if plain_action(ui, "Import cape", 108.0, 32.0, false) {
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
                            element_label(ui, "Local account only. No Microsoft login.", sub_font(12.0), MUTED);
                            ui.add_space(4.0);
                            element_label(ui, &crate::state::offline_uuid(&self.username), sub_font(12.0), MUTED);
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
            gold_slot(ui, 420.0);
            ui.vertical(|ui| {
                ui.set_width((width - 48.0) * 0.38);
                egui::Frame::new()
                    .inner_margin(egui::Margin { left: 16, right: 0, top: 0, bottom: 0 })
                    .show(ui, |ui| {
                card(ui, |ui| {
                    element_label(ui, "GAME", sub_font(12.0), GOLD);
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        element_label(ui, "Memory", body_font(14.0), TEXT);
                        gold_slot(ui, 28.0);
                        element_label(ui, &format!("{} MB", self.ram), body_font(14.0), TEXT);
                    });
                    if slider(ui, &mut self.ram, 512, 16384) { self.save_settings(); }
                    ui.add_space(2.0);
                    element_label(ui, "Game window", body_font(14.0), TEXT);
                    ui.horizontal(|ui| {
                        let mut w = self.width.to_string();
                        let mut h = self.height.to_string();
                        if field(ui, &mut w, 90.0, "1280").changed() {
                            if let Ok(n) = w.parse::<u32>() { self.width = n.clamp(640, 3840); self.save_settings(); }
                        }
                        gold_slot(ui, 32.0);
                        if field(ui, &mut h, 90.0, "720").changed() {
                            if let Ok(n) = h.parse::<u32>() { self.height = n.clamp(480, 2160); self.save_settings(); }
                        }
                    });
                    ui.add_space(4.0);
                    if check(ui, &mut self.snapshots, "Snapshots") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    if check(ui, &mut self.previews, "Previews") {
                        self.save_settings();
                        spawn_versions(self.app.clone(), self.tx.clone());
                    }
                    ui.add_space(2.0);
                    element_label(ui, "Servers", body_font(14.0), TEXT);
                    ui.add_space(4.0);
                    element_label(ui, "Played servers show on Home.", sub_font(12.0), MUTED);
                    if field(ui, &mut self.server_name, 250.0, "Name").changed() {}
                    if field(ui, &mut self.server_addr, 250.0, "host or host:port").changed() {}
                    ui.horizontal(|ui| {
                        if plain_action(ui, "Add", 48.0, 32.0, false) {
                            self.add_server();
                        }
                        gold_slot(ui, 32.0);
                        if plain_action(ui, "Join", 48.0, 32.0, true) {
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
                        let width = ui.available_width();
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 34.0), Sense::hover());
                        let chip = rect.shrink2(egui::vec2(0.0, 3.0));
                        paint_element_template(ui.painter(), chip);
                        let inner = chip.shrink2(egui::vec2(8.0, 1.0));
                        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                        row.set_clip_rect(chip.shrink(1.5));
                        row.horizontal(|ui| {
                            let label = if name.is_empty() { addr.clone() } else { name.clone() };
                            ui.label(RichText::new(label).font(body_font(13.0)).color(if icon.is_empty() { MUTED } else { TEXT }));
                            gold_slot(ui, 28.0);
                            if item_action(ui, "Play", 48.0, 28.0, false) {
                                self.join_server(name.clone(), addr.clone());
                            }
                            gold_slot(ui, 28.0);
                            if item_action(ui, "Icon", 44.0, 28.0, false) {
                                if let Some(path) = crate::dialog::pick_file("Server icon", "*.png") {
                                    self.set_server_icon(&addr, &path);
                                }
                            }
                        });
                        ui.add_space(2.0);
                    }
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        element_label(ui, "Data", body_font(13.0), TEXT);
                        gold_slot(ui, 32.0);
                        element_label(ui, &self.app.dir.display().to_string(), sub_font(12.0), MUTED);
                        gold_slot(ui, 32.0);
                        if plain_action(ui, "Open", 52.0, 28.0, false) {
                            if let Err(e) = crate::dialog::open_dir(&self.app.dir) { self.notice = e; }
                        }
                    });
                });
                });
            });
        });
        });
    }
}
