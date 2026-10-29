//! Settings. Each group sits in its own card: player, Java, versions.
use eframe::egui::{self, RichText, Sense};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn paint_settings_page(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let area = spec_content(full);
        let gap = 12.0;
        let left_w = 448.0;
        let player = egui::Rect::from_min_size(area.min, egui::vec2(left_w, 320.0));
        let java = egui::Rect::from_min_size(
            egui::pos2(player.right() + gap, area.top()),
            egui::vec2(area.right() - player.right() - gap, 256.0),
        );
        let versions = egui::Rect::from_min_size(
            egui::pos2(area.left(), player.bottom() + gap),
            egui::vec2(left_w, 84.0),
        );
        paint_settings_card(ui.painter(), player);
        paint_settings_card(ui.painter(), java);
        paint_settings_card(ui.painter(), versions);
        self.paint_player_settings(ui, player.shrink(14.0));
        self.paint_java_settings(ui, java.shrink(14.0));
        self.paint_version_settings(ui, versions.shrink(14.0));
    }

    fn paint_player_settings(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        child.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        settings_title(&mut child, "Player");
        let top = child.cursor().top();
        let body = egui::Rect::from_min_max(egui::pos2(rect.left(), top), rect.right_bottom());
        let portrait_w = 136.0;
        let fields = egui::Rect::from_min_max(body.left_top(), egui::pos2(body.right() - portrait_w - 10.0, body.bottom()));
        let stage = egui::Rect::from_min_max(egui::pos2(body.right() - portrait_w, body.top()), body.right_bottom());
        {
            let mut col = child.new_child(egui::UiBuilder::new().max_rect(fields));
            col.set_clip_rect(fields);
            col.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
            let ui = &mut col;
            element_label(ui, "Username", body_font(13.0), TEXT);
            if field(ui, &mut self.username, fields.width(), "Player").changed() {
                self.save_settings();
            }
            ui.add_space(4.0);
            element_label(ui, "Skin", body_font(13.0), TEXT);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if plain_action(ui, "Steve", 62.0, 28.0, self.skin == "preset:steve") {
                    self.skin = "preset:steve".into();
                    self.preview.arms = ArmModel::Classic;
                    self.preview.invalidate();
                    self.avatar_key.clear();
                    self.save_settings();
                }
                if plain_action(ui, "Alex", 56.0, 28.0, self.skin == "preset:alex") {
                    self.skin = "preset:alex".into();
                    self.preview.arms = ArmModel::Slim;
                    self.preview.invalidate();
                    self.avatar_key.clear();
                    self.save_settings();
                }
                if plain_action(ui, "Import", 72.0, 28.0, false) {
                    if let Some(path) = crate::dialog::pick_file("Import skin", "*.png") {
                        self.import_path("skin", &path);
                    } else {
                        self.import_error = "No file chosen. You can also drop a PNG here.".into();
                    }
                }
            });
            self.paint_imported_switch(ui, "skin", fields.width());
            ui.add_space(4.0);
            element_label(ui, "Cape", body_font(13.0), TEXT);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if plain_action(ui, "Rus", 52.0, 28.0, self.cape == "preset:rus") {
                    self.cape = "preset:rus".into();
                    self.preview.invalidate();
                    self.save_settings();
                }
                if plain_action(ui, "None", 56.0, 28.0, self.cape == "preset:none" || self.cape.is_empty()) {
                    self.cape = "preset:none".into();
                    self.preview.invalidate();
                    self.save_settings();
                }
                if plain_action(ui, "Import", 72.0, 28.0, false) {
                    if let Some(path) = crate::dialog::pick_file("Import cape", "*.png") {
                        self.import_path("cape", &path);
                    } else {
                        self.import_error = "Cape must be a 64×32 PNG.".into();
                    }
                }
            });
            self.paint_imported_switch(ui, "cape", fields.width());
            if !self.import_error.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&self.import_error).font(sub_font(12.0)).color(ERR));
            }
        }
        let skin = self.skin_image();
        let cape = self.cape_image();
        let portrait = egui::vec2(128.0, 156.0);
        let origin = egui::pos2(stage.center().x - portrait.x * 0.5, stage.top());
        let frame = egui::Rect::from_min_size(origin, portrait);
        let mut stage_ui = child.new_child(egui::UiBuilder::new().max_rect(frame));
        stage_ui.set_clip_rect(stage);
        if let Some(skin) = skin.as_ref() {
            self.preview.portrait(&mut stage_ui, skin, cape.as_ref(), self.detected_slim(), portrait);
        } else {
            stage_ui.label(RichText::new("Skin file missing").color(ERR));
        }
    }

    fn paint_imported_switch(&mut self, ui: &mut egui::Ui, kind: &str, width: f32) {
        let samples: Vec<(&str, &str)> = if kind == "cape" {
            vec![("preset:one", "Cape 1"), ("preset:two", "Cape 2")]
        } else {
            vec![("preset:steve", "Steve"), ("preset:alex", "Alex")]
        };
        let files = crate::skin::imported_files(&self.app, kind);
        for (id, _) in &samples {
            self.ensure_import_icon(ui.ctx(), kind, id);
        }
        for (id, _) in &files {
            self.ensure_import_icon(ui.ctx(), kind, id);
        }
        let selected = if kind == "cape" { self.cape.clone() } else { self.skin.clone() };
        let mut icons: Vec<(String, String, egui::TextureId)> = Vec::new();
        for (id, label) in &samples {
            if let Some(tex) = self.import_icons.get(&format!("{kind}:{id}")) {
                icons.push(((*id).to_string(), (*label).to_string(), tex.id()));
            }
        }
        for (id, _) in &files {
            if let Some(tex) = self.import_icons.get(&format!("{kind}:{id}")) {
                let label = id.trim_start_matches("file:").trim_end_matches(".png").to_string();
                icons.push((id.clone(), label, tex.id()));
            }
        }
        if icons.is_empty() {
            return;
        }
        ui.add_space(4.0);
        let row_h = if kind == "cape" { 36.0 } else { 32.0 };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_h), Sense::hover());
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        row.set_clip_rect(rect);
        let picked = egui::ScrollArea::horizontal()
            .id_salt(("imported", kind))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut row, |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.horizontal(|ui| {
                    let mut hit = None;
                    for (id, label, tex) in icons {
                        let size = if kind == "cape" { egui::vec2(24.0, 34.0) } else { egui::vec2(28.0, 28.0) };
                        if import_chip(ui, &label, tex, size, selected == id) {
                            hit = Some(id);
                        }
                    }
                    hit
                }).inner
            })
            .inner;
        if let Some(id) = picked {
            if kind == "cape" {
                self.choose_cape(&id);
            } else {
                self.choose_skin(&id);
            }
        }
    }

    fn paint_java_settings(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        child.spacing_mut().item_spacing.y = 4.0;
        let ui = &mut child;
        settings_title(ui, "Java");
        ui.horizontal(|ui| {
            element_label(ui, "Memory", body_font(13.0), TEXT);
            ui.add_space(8.0);
            element_label(ui, &format!("{} MB", self.ram), body_font(13.0), GOLD);
        });
        if slider(ui, &mut self.ram, 512, 16384) {
            self.save_settings();
        }
        ui.add_space(6.0);
        element_label(ui, "Game window", body_font(13.0), TEXT);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let mut w = self.width.to_string();
            let mut h = self.height.to_string();
            if field(ui, &mut w, 88.0, "1280").changed() {
                if let Ok(n) = w.parse::<u32>() {
                    self.width = n.clamp(640, 3840);
                    self.save_settings();
                }
            }
            element_label(ui, "×", body_font(14.0), MUTED);
            if field(ui, &mut h, 88.0, "720").changed() {
                if let Ok(n) = h.parse::<u32>() {
                    self.height = n.clamp(480, 2160);
                    self.save_settings();
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            element_label(ui, "JVM arguments", body_font(13.0), TEXT);
            ui.add_space(8.0);
            if plain_action(ui, "Reset", 58.0, 24.0, false) {
                self.jvm_args = crate::state::default_jvm_args();
                self.save_settings();
            }
        });
        if field(ui, &mut self.jvm_args, rect.width(), "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions").changed() {
            self.save_settings();
        }
    }

    fn paint_version_settings(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        child.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
        let ui = &mut child;
        settings_title(ui, "Versions");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if check(ui, &mut self.snapshots, "Snapshots") {
                self.save_settings();
                spawn_versions(self.app.clone(), self.tx.clone());
            }
            if check(ui, &mut self.previews, "Previews") {
                self.save_settings();
                spawn_versions(self.app.clone(), self.tx.clone());
            }
        });
    }
}

fn import_chip(ui: &mut egui::Ui, tip: &str, tex: egui::TextureId, size: egui::Vec2, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 230));
    ui.painter().image(tex, rect.shrink(3.0), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    let edge = if on { GOLD } else { egui::Color32::from_white_alpha(48) };
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(if on { 2.0 } else { 1.0 }, edge), egui::StrokeKind::Inside);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let clicked = resp.clicked();
    resp.on_hover_text(tip);
    clicked
}

fn paint_settings_card(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 12.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 228));
    painter.rect_stroke(rect, 12.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(42)), egui::StrokeKind::Inside);
    let accent = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 1.5, rect.top() + 14.0),
        egui::pos2(rect.left() + 3.5, rect.bottom() - 14.0),
    );
    painter.rect_filled(accent, 1.0, egui::Color32::from_rgba_unmultiplied(0xF4, 0xCA, 0x54, 210));
}

fn settings_title(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(40.0), 22.0), Sense::hover());
    ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, text, nav_font(13.0), GOLD);
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.bottom() - 1.0,
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(28)),
    );
    ui.add_space(6.0);
}
