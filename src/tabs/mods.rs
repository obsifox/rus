//! Mod tab.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn paint_mods_page(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let content = spec_content(full);
        paint_blend_stage(ui.painter(), content);
        let right = egui::Rect::from_min_max(egui::pos2(content.right() - 290.0, content.top()), content.right_bottom());
        let left = egui::Rect::from_min_max(content.left_top(), egui::pos2(right.left() - 12.0, content.bottom()));
        spec_panel(ui.painter(), left);
        spec_panel(ui.painter(), right);
        let inst = self.current_instance().map(|i| i.name.clone()).unwrap_or_else(|| "No instance".into());
        ui.painter().text(egui::pos2(left.left() + 16.0, left.top() + 22.0), egui::Align2::LEFT_CENTER, &inst, title_font(18.0), SPEC_TEXT);
        let tabs = ["Mods", "Resource packs", "Shader packs"];
        let counts = [
            self.scan_mods().len(),
            self.scan_packs("resourcepacks").len(),
            self.scan_packs("shaderpacks").len(),
        ];
        let mut x = left.left() + 16.0;
        let tab_y = left.top() + 48.0;
        for (i, label) in tabs.iter().enumerate() {
            let text = format!("{label} {}", counts[i]);
            let w = ui.painter().layout_no_wrap(text.clone(), nav_font(13.0), SPEC_TEXT).size().x + 20.0;
            let rect = egui::Rect::from_min_size(egui::pos2(x, tab_y), egui::vec2(w, 32.0));
            let hit = ui.interact(rect, egui::Id::new("pack-tab").with(i), Sense::click());
            let on = self.pack_tab as usize == i;
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, &text, nav_font(13.0), if on { SPEC_YELLOW } else { SPEC_TEXT });
            if on {
                ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 3.0), rect.right_bottom()), 1.0, SPEC_YELLOW);
            }
            if hit.clicked() {
                self.pack_tab = i as u8;
            }
            x += w + 8.0;
        }
        let loader = crate::state::loader_label(&self.loader);
        let enabled = self.scan_mods().iter().filter(|m| m.1).count();
        let summary = format!("{inst} · {loader} · {enabled} enabled");
        ui.painter().text(egui::pos2(left.left() + 16.0, left.top() + 96.0), egui::Align2::LEFT_CENTER, &summary, sub_font(13.0), SPEC_MUTED);
        let path_bar = egui::Rect::from_min_max(egui::pos2(left.left() + 12.0, left.bottom() - 48.0), egui::pos2(left.right() - 12.0, left.bottom() - 10.0));
        let list = egui::Rect::from_min_max(egui::pos2(left.left() + 12.0, left.top() + 112.0), egui::pos2(left.right() - 12.0, path_bar.top() - 8.0));
        if self.pack_tab == 0 && (self.loader == "vanilla" || self.loader.is_empty()) {
            ui.painter().text(egui::pos2(list.left() + 8.0, list.top() + 8.0), egui::Align2::LEFT_TOP, "Mods need Forge, Fabric, or NeoForge.", sub_font(13.0), SPEC_YELLOW);
            let go = egui::Rect::from_min_size(egui::pos2(list.left() + 8.0, list.top() + 28.0), egui::vec2(140.0, 32.0));
            if spec_button(ui, go, "Choose loader", true) {
                self.tab = Tab::Versions;
            }
        }
        self.take_drops(ui);
        let entries = self.pack_entries();
        if !entries.is_empty() {
            let n = entries.len();
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.row_focus = (self.row_focus + 1).min(n - 1);
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.row_focus = self.row_focus.saturating_sub(1);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                if let Some((name, on, _)) = entries.get(self.row_focus) {
                    self.set_pack_enabled(name, !on);
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                if let Some((name, _, _)) = entries.get(self.row_focus) {
                    self.confirm_remove = Some(name.clone());
                }
            }
        }
        let mut toggled: Option<String> = None;
        let mut removed: Option<String> = None;
        let mut pick = false;
        if entries.is_empty() {
            paint_dashed_rect(ui.painter(), list.shrink(8.0), SPEC_MUTED);
            ui.painter().text(list.center() + egui::vec2(0.0, -12.0), egui::Align2::CENTER_CENTER, "Drop files here", body_font(16.0), SPEC_TEXT);
            let choose = egui::Rect::from_center_size(list.center() + egui::vec2(0.0, 18.0), egui::vec2(132.0, 32.0));
            if spec_button(ui, choose, "Choose files", false) {
                pick = true;
            }
        } else {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
            child.set_clip_rect(list);
            let focus = self.row_focus;
            egui::ScrollArea::vertical().id_salt("spec-packs").auto_shrink([false, false]).show_rows(&mut child, 44.0, entries.len(), |ui, range| {
                for i in range {
                    let (name, on, size) = &entries[i];
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(list.width(), 40.0), Sense::hover());
                    spec_card(ui.painter(), rect, i == focus);
                    let shown = display_pack_name(name);
                    let shown = ellipsize(ui.painter(), &shown, body_font(14.0), rect.width() - 210.0);
                    let ink = if *on { SPEC_TEXT } else { SPEC_MUTED };
                    let tick = egui::Rect::from_center_size(egui::pos2(rect.left() + 20.0, rect.center().y), egui::vec2(16.0, 16.0));
                    paint_check(ui.painter(), tick, *on);
                    let hit = ui.interact(tick.expand(6.0), egui::Id::new("mod-on").with(name), Sense::click());
                    if hit.clicked() {
                        toggled = Some(name.clone());
                    }
                    ui.painter().text(egui::pos2(rect.left() + 40.0, rect.center().y), egui::Align2::LEFT_CENTER, &shown, body_font(14.0), ink);
                    ui.painter().text(egui::pos2(rect.right() - 96.0, rect.center().y), egui::Align2::RIGHT_CENTER, &format_bytes(*size), sub_font(12.0), SPEC_MUTED);
                    let rm = egui::Rect::from_min_size(egui::pos2(rect.right() - 78.0, rect.top() + 6.0), egui::vec2(66.0, 28.0));
                    if spec_button(ui, rm, "Remove", false) {
                        removed = Some(name.clone());
                    }
                    let row_hit = ui.interact(rect, egui::Id::new("pack-row").with(name), Sense::click());
                    if row_hit.hovered() {
                        row_hit.on_hover_text(name);
                    }
                }
            });
        }
        if let Some(name) = toggled {
            let on = entries.iter().find(|(n, _, _)| n == &name).map(|e| e.1).unwrap_or(true);
            self.set_pack_enabled(&name, !on);
        }
        if let Some(name) = removed {
            self.confirm_remove = Some(name);
        }
        if pick {
            self.pick_pack_file();
        }
        let folder = match self.pack_tab {
            1 => "resourcepacks",
            2 => "shaderpacks",
            _ => "mods",
        };
        let path = self.pack_dir(folder);
        ui.painter().text(egui::pos2(path_bar.left() + 8.0, path_bar.center().y), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &path.display().to_string(), sub_font(12.0), path_bar.width() - 210.0), sub_font(12.0), SPEC_MUTED);
        let copy = egui::Rect::from_min_size(egui::pos2(path_bar.right() - 188.0, path_bar.top() + 4.0), egui::vec2(88.0, 28.0));
        let open = egui::Rect::from_min_size(egui::pos2(path_bar.right() - 92.0, path_bar.top() + 4.0), egui::vec2(84.0, 28.0));
        if spec_button(ui, copy, "Copy path", false) {
            ui.output_mut(|o| o.copied_text = path.display().to_string());
            self.notice = "Path copied".into();
        }
        if spec_button(ui, open, "Open folder", false) {
            let _ = std::fs::create_dir_all(&path);
            if let Err(e) = crate::dialog::open_dir(&path) { self.notice = e; }
        }
        let kinds = ["Modpack", "Mod", "Resource pack", "Shader pack"];
        let mut y = right.top() + 16.0;
        for (i, label) in kinds.iter().enumerate() {
            let btn = egui::Rect::from_min_size(egui::pos2(right.left() + 16.0, y), egui::vec2(right.width() - 32.0, 56.0));
            if spec_button(ui, btn, label, false) {
                self.install_kind(i as u8);
            }
            y += 68.0;
        }
        ui.painter().text(
            egui::pos2(right.center().x, y + 8.0),
            egui::Align2::CENTER_TOP,
            "Drop files anywhere on this page",
            sub_font(13.0),
            SPEC_MUTED,
        );
    }

    pub(crate) fn paint_remove_confirm(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(name) = self.confirm_remove.clone() else { return };
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(120));
        let box_rect = egui::Rect::from_center_size(full.center(), egui::vec2(420.0, 160.0));
        spec_panel(ui.painter(), box_rect);
        ui.painter().text(egui::pos2(box_rect.left() + 20.0, box_rect.top() + 28.0), egui::Align2::LEFT_CENTER, "Move this file to trash?", title_font(16.0), SPEC_TEXT);
        ui.painter().text(egui::pos2(box_rect.left() + 20.0, box_rect.top() + 58.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &name, body_font(14.0), 380.0), body_font(14.0), SPEC_MUTED);
        let cancel = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 220.0, box_rect.bottom() - 52.0), egui::vec2(96.0, 36.0));
        let ok = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 112.0, box_rect.bottom() - 52.0), egui::vec2(96.0, 36.0));
        if spec_button(ui, cancel, "Cancel", false) || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.confirm_remove = None;
        }
        if spec_button(ui, ok, "Remove", true) || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.trash_pack(&name);
            self.confirm_remove = None;
        }
    }

    pub(crate) fn paint_pack_prompt(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(path) = self.pending_pack.clone() else { return };
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(120));
        let box_rect = egui::Rect::from_center_size(full.center(), egui::vec2(460.0, 180.0));
        spec_panel(ui.painter(), box_rect);
        ui.painter().text(egui::pos2(box_rect.left() + 20.0, box_rect.top() + 28.0), egui::Align2::LEFT_CENTER, "Install this modpack?", title_font(16.0), SPEC_TEXT);
        ui.painter().text(egui::pos2(box_rect.left() + 20.0, box_rect.top() + 56.0), egui::Align2::LEFT_CENTER, "New instance is the safe default. Nothing is overwritten.", sub_font(13.0), SPEC_MUTED);
        let cancel = egui::Rect::from_min_size(egui::pos2(box_rect.left() + 20.0, box_rect.bottom() - 52.0), egui::vec2(96.0, 36.0));
        let apply = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 292.0, box_rect.bottom() - 52.0), egui::vec2(140.0, 36.0));
        let fresh = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 140.0, box_rect.bottom() - 52.0), egui::vec2(120.0, 36.0));
        if spec_button(ui, cancel, "Cancel", false) {
            self.pending_pack = None;
        }
        if spec_button(ui, apply, "Apply here", false) {
            self.apply_pack(&path);
            self.pending_pack = None;
        }
        if spec_button(ui, fresh, "New instance", true) {
            self.store_pack_as_instance(&path);
            self.pending_pack = None;
        }
    }

    pub(crate) fn page_mods(&mut self, ui: &mut egui::Ui) {
        let title_id = title_font(22.0);
        let title_w = ui.painter().layout_no_wrap("Mod".into(), title_id.clone(), TEXT).size().x;
        let (title, _) = ui.allocate_exact_size(egui::vec2(40.0 + title_w + 14.0, 36.0), Sense::hover());
        paint_element_template(ui.painter(), title);
        paint_material(ui.painter(), egui::pos2(title.left() + 18.0, title.center().y), 18.0, I_CUBE, TEXT);
        ui.painter().text(egui::pos2(title.left() + 34.0, title.center().y), egui::Align2::LEFT_CENTER, "Mod", title_id, TEXT);
        ui.add_space(6.0);
        element_label(ui, "The home card icon is the loader. Jars are staged when you play.", sub_font(13.0), MUTED);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (i, (id, label)) in [("vanilla", "Vanilla"), ("forge", "Forge"), ("fabric", "Fabric"), ("neoforge", "NeoForge"), ("optifine", "OptiFine")].into_iter().enumerate() {
                if i > 0 {
                    gold_slot(ui, 40.0);
                }
                if plain_action(ui, label, 96.0, 40.0, self.loader_active(id)) {
                    self.choose_loader(id);
                }
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if plain_action(ui, "Open folder", 120.0, 36.0, false) {
                if let Err(e) = crate::dialog::open_dir(&self.app.p("mods")) { self.notice = e; }
            }
            gold_slot(ui, 36.0);
            if plain_action(ui, "Add .jar", 90.0, 36.0, true) {
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
        let (zone, zresp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), Sense::click());
        paint_element_template(ui.painter(), zone);
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
            element_label(ui, &self.app.p("mods").display().to_string(), sub_font(13.0), MUTED);
            gold_slot(ui, 32.0);
            if plain_action(ui, "Copy path", 96.0, 28.0, false) {
                ui.output_mut(|o| o.copied_text = self.app.p("mods").display().to_string());
                self.notice = "Path copied".into();
            }
            gold_slot(ui, 32.0);
            element_label(ui, &format!("{} mods · {enabled} enabled", self.mods.len()), sub_font(13.0), MUTED);
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
                let width = ui.available_width();
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 36.0), Sense::hover());
                let chip = rect.shrink2(egui::vec2(0.0, 3.0));
                paint_element_template(ui.painter(), chip);
                if on {
                    ui.painter().rect_stroke(chip.shrink(1.5), 1.5, egui::Stroke::new(1.0, GOLD), egui::StrokeKind::Inside);
                }
                let inner = chip.shrink2(egui::vec2(8.0, 2.0));
                let mut row = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                row.set_clip_rect(chip.shrink(1.5));
                row.horizontal(|ui| {
                    let mut enabled = on;
                    if check_inline(ui, &mut enabled, &name, 280.0) {
                        let _ = crate::mods::toggle(&self.app, &name);
                        spawn_mods(self.app.clone(), self.tx.clone());
                    }
                    gold_slot(ui, 28.0);
                    ui.label(RichText::new(format_bytes(size)).font(sub_font(12.0)).color(MUTED));
                    gold_slot(ui, 28.0);
                    if item_action(ui, "Remove", 72.0, 28.0, false) {
                        let _ = crate::mods::remove(&self.app, &name);
                        spawn_mods(self.app.clone(), self.tx.clone());
                    }
                });
                ui.add_space(2.0);
            }
        });
    }

}
