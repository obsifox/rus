//! Instance tab.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn paint_version_page(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let content = spec_content(full);
        let gap = 10.0;
        let foot_h = 68.0;
        let loader_w = 236.0;
        let top_h = content.height() - foot_h - gap;
        let left = egui::Rect::from_min_size(content.min, egui::vec2(content.width() - gap - loader_w, top_h));
        let right = egui::Rect::from_min_size(egui::pos2(left.right() + gap, content.top()), egui::vec2(loader_w, top_h));
        let foot = egui::Rect::from_min_size(egui::pos2(content.left(), left.bottom() + gap), egui::vec2(content.width(), foot_h));
        paint_main_box(ui.painter(), left);
        paint_main_box(ui.painter(), right);
        paint_main_box(ui.painter(), foot);
        self.paint_instance_column(ui, left);
        self.paint_loader_column(ui, right);
        self.paint_instance_foot(ui, foot);
        if !self.instances.is_empty() && self.confirm_remove.is_none() && self.pending_pack.is_none() {
            let ids: Vec<String> = self.instances.iter().map(|i| i.id.clone()).collect();
            let mut idx = ids.iter().position(|id| id == &self.instance_id).unwrap_or(0);
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            if down { idx = (idx + 1).min(ids.len() - 1); }
            if up { idx = idx.saturating_sub(1); }
            if (down || up) && ids.get(idx).is_some() {
                self.select_instance(&ids[idx]);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.on_install();
            }
        }
    }

    pub(crate) fn paint_instance_column(&mut self, ui: &mut egui::Ui, col: egui::Rect) {
        let head = egui::Rect::from_min_size(col.min + egui::vec2(12.0, 8.0), egui::vec2(col.width() - 24.0, 36.0));
        paint_tab_glyph(ui.painter(), Tab::Versions, egui::pos2(head.left() + 8.0, head.center().y), 15.0, HY_ACCENT);
        ui.painter().text(egui::pos2(head.left() + 24.0, head.center().y), egui::Align2::LEFT_CENTER, "INSTANCES", nav_font(13.0), HY_TEXT);
        let count = format!("{}", self.instances.len());
        let badge = egui::Rect::from_center_size(egui::pos2(head.left() + 118.0, head.center().y), egui::vec2(22.0, 18.0));
        ui.painter().rect_filled(badge, 6.0, egui::Color32::from_rgba_unmultiplied(0xF2, 0xB8, 0x4B, 36));
        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, &count, nav_font(11.0), HY_ACCENT);
        let new_btn = egui::Rect::from_min_size(egui::pos2(head.right() - 86.0, head.top() + 3.0), egui::vec2(86.0, 28.0));
        if opaque_button(ui, new_btn, "NEW", true) {
            self.draft_name.clear();
            self.draft_version = if self.selected.is_empty() { "26.3".into() } else { self.selected.clone() };
            self.draft_open = true;
        }
        let list = egui::Rect::from_min_max(egui::pos2(col.left() + 10.0, head.bottom() + 6.0), egui::pos2(col.right() - 10.0, col.bottom() - 8.0));
        if self.instances.is_empty() {
            let btn = egui::Rect::from_center_size(list.center(), egui::vec2(220.0, 36.0));
            ui.painter().text(egui::pos2(list.center().x, list.center().y - 28.0), egui::Align2::CENTER_CENTER, "No instances yet", body_font(15.0), HY_MUTED);
            if opaque_button(ui, btn, "Create instance", true) {
                self.draft_open = true;
            }
            return;
        }
        let rows: Vec<crate::instances::Instance> = self.instances.clone();
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
        child.set_clip_rect(list);
        child.style_mut().spacing.window_margin = egui::Margin::ZERO;
        child.style_mut().spacing.item_spacing.x = 0.0;
        let mut pick: Option<String> = None;
        let mut drop_id: Option<String> = None;
        egui::ScrollArea::vertical().id_salt("spec-instances").auto_shrink([false, false]).show(&mut child, |ui| {
            ui.set_width(list.width());
            for inst in &rows {
                let (slot, _) = ui.allocate_exact_size(egui::vec2(list.width(), 64.0), Sense::hover());
                ui.add_space(8.0);
                let rect = slot;
                let on = self.instance_id == inst.id;
                let picked = ease_out(ui.ctx().animate_bool_with_time(egui::Id::new("inst-card").with(&inst.id), on, 0.18));
                paint_opaque_card(ui.painter(), rect, picked);
                let icon = egui::Rect::from_min_size(rect.min + egui::vec2(12.0, 14.0), egui::vec2(36.0, 36.0));
                if let Some(tex) = self.loader_icon_for(&inst.loader) {
                    ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
                let select = egui::Rect::from_min_size(egui::pos2(rect.right() - 196.0, rect.top() + 16.0), egui::vec2(104.0, 32.0));
                let delete = egui::Rect::from_min_size(egui::pos2(rect.right() - 84.0, rect.top() + 16.0), egui::vec2(72.0, 32.0));
                let name = ellipsize(ui.painter(), &inst.name, title_font(16.0), select.left() - icon.right() - 20.0);
                let name_ink = lerp_color(HY_TEXT, HY_ACCENT, picked);
                ui.painter().text(egui::pos2(icon.right() + 10.0, rect.top() + 22.0), egui::Align2::LEFT_CENTER, &name, title_font(16.0), name_ink);
                let sub = format!("{} · {}", inst.version, crate::state::loader_label(&inst.loader).to_uppercase());
                ui.painter().text(egui::pos2(icon.right() + 10.0, rect.top() + 42.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &sub, sub_font(12.0), select.left() - icon.right() - 20.0), sub_font(12.0), HY_MUTED);
                if action_button(ui, select, if on { "SELECTED" } else { "SELECT" }, if on { BtnKind::Outline } else { BtnKind::Quiet }) {
                    pick = Some(inst.id.clone());
                }
                if red_button(ui, delete, "DELETE") {
                    drop_id = Some(inst.id.clone());
                }
            }
        });
        if let Some(id) = pick {
            self.select_instance(&id);
        }
        if let Some(id) = drop_id {
            self.confirm_instance = Some(id);
        }
    }

    pub(crate) fn paint_loader_column(&mut self, ui: &mut egui::Ui, col: egui::Rect) {
        let head = egui::Rect::from_min_size(col.min + egui::vec2(10.0, 8.0), egui::vec2(col.width() - 18.0, 36.0));
        paint_tab_glyph(ui.painter(), Tab::Versions, egui::pos2(head.left() + 8.0, head.center().y), 14.0, HY_ACCENT);
        ui.painter().text(egui::pos2(head.left() + 24.0, head.center().y), egui::Align2::LEFT_CENTER, "LOADER", nav_font(13.0), HY_TEXT);
        let mc = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        ui.painter().text(head.right_center(), egui::Align2::RIGHT_CENTER, &mc, sub_font(12.0), HY_MUTED);
        let list = egui::Rect::from_min_max(egui::pos2(col.left() + 8.0, head.bottom() + 4.0), egui::pos2(col.right() - 8.0, col.bottom() - 8.0));
        let rows = crate::loaders::rows_for(&mc, &self.loaders);
        if !self.loaders.ready && rows.iter().all(|r| r.id == "vanilla" || r.version.is_empty()) {
            for i in 0..5 {
                let y = list.top() + i as f32 * 48.0;
                let row = egui::Rect::from_min_size(egui::pos2(list.left(), y), egui::vec2(list.width(), 42.0));
                ui.painter().rect_filled(row, 1.5, HY_SURFACE);
            }
            return;
        }
        let forge_on = self.loader == "forge" || self.loader == "forge+optifine";
        let any_on = rows.iter().any(|r| self.loader_active(r.id));
        let mut clicked: Option<String> = None;
        let stride = (list.height() / rows.len().max(1) as f32).clamp(46.0, 62.0);
        for (i, row) in rows.iter().enumerate() {
            let y = list.top() + i as f32 * stride;
            let rect = egui::Rect::from_min_size(egui::pos2(list.left(), y), egui::vec2(list.width(), stride - 6.0));
            let missing = matches!(row.kind, crate::loaders::Badge::Preview) && row.version.is_empty();
            let on = self.loader_active(row.id);
            let pair = row.id == "optifine" && forge_on && !on;
            let fade = any_on && !on && !pair && !missing;
            let on_t = ease_out(ui.ctx().animate_bool_with_time(egui::Id::new("ld-on").with(row.id), on && !missing, 0.16));
            let fade_t = ease_out(ui.ctx().animate_bool_with_time(egui::Id::new("ld-fade").with(row.id), fade, 0.2));
            if pair {
                paint_purple_card(ui.painter(), rect);
            } else if missing {
                ui.painter().rect_filled(rect, 1.5, HY_BG);
                ui.painter().rect_stroke(rect, 1.5, egui::Stroke::new(1.0, HY_BORDER), egui::StrokeKind::Inside);
            } else {
                paint_mixed_card(ui.painter(), rect, on_t, fade_t);
            }
            let switch = egui::Rect::from_min_size(egui::pos2(rect.right() - 52.0, rect.center().y - 9.0), egui::vec2(42.0, 18.0));
            let warn = egui::Rect::from_min_size(egui::pos2(switch.left() - 4.0 - 16.0, rect.center().y - 8.0), egui::vec2(16.0, 16.0));
            let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 20.0, rect.center().y), egui::vec2(22.0, 22.0));
            if let Some(tex) = self.loader_icon_id(row.id) {
                ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            if missing {
                paint_warn(ui.painter(), warn.center());
            }
            let can_switch = !missing || pair;
            if paint_slide_switch(ui, switch, on && can_switch, egui::Id::new("loader-slide").with(row.id), can_switch) {
                clicked = Some(row.id.to_string());
            }
            let text_right = if missing { warn.left() - 8.0 } else { switch.left() - 8.0 };
            let ink = if pair {
                egui::Color32::from_rgb(0xC4, 0xB0, 0xFF)
            } else if missing {
                HY_MUTED
            } else {
                lerp_color(lerp_color(HY_TEXT, HY_ACCENT, on_t), HY_MUTED, fade_t)
            };
            let detail = if row.version.is_empty() { row.detail.clone() } else { row.version.clone() };
            let detail_ink = if pair { egui::Color32::from_rgb(0xA7, 0x8B, 0xFA) } else { HY_MUTED };
            ui.painter().text(egui::pos2(rect.left() + 38.0, rect.center().y - 8.0), egui::Align2::LEFT_CENTER, row.name, title_font(13.0), ink);
            ui.painter().text(egui::pos2(rect.left() + 38.0, rect.center().y + 8.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &detail, sub_font(11.0), (text_right - rect.left() - 42.0).max(24.0)), sub_font(11.0), detail_ink);
        }
        if let Some(id) = clicked {
            self.toggle_loader(&id);
        }
    }

    pub(crate) fn paint_instance_foot(&mut self, ui: &mut egui::Ui, foot: egui::Rect) {
        let name = self.current_instance().map(|i| i.name.clone()).unwrap_or_else(|| "None".into());
        let mc = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        let rows = crate::loaders::rows_for(&mc, &self.loaders);
        let chosen: Vec<_> = rows.iter().filter(|r| self.loader_active(r.id)).cloned().collect();
        let plan = if chosen.is_empty() {
            format!("Nothing selected on {name}")
        } else {
            let parts: Vec<String> = chosen.iter().map(|r| if r.version.is_empty() { r.name.to_string() } else { format!("{} {}", r.name, r.version) }).collect();
            format!("{} on {name} · Minecraft {mc}", parts.join(" + "))
        };
        paint_tab_glyph(ui.painter(), Tab::Versions, egui::pos2(foot.left() + 22.0, foot.center().y), 16.0, HY_ACCENT);
        ui.painter().text(egui::pos2(foot.left() + 40.0, foot.top() + 22.0), egui::Align2::LEFT_CENTER, "WILL INSTALL", nav_font(11.0), HY_ACCENT);
        ui.painter().text(egui::pos2(foot.left() + 40.0, foot.top() + 42.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &plan, body_font(14.0), foot.width() - 230.0), body_font(14.0), HY_TEXT);
        let btn = egui::Rect::from_min_size(egui::pos2(foot.right() - 168.0, foot.center().y - 18.0), egui::vec2(150.0, 36.0));
        let active = chosen.iter().find(|r| r.id != "optifine").cloned();
        let raw = self.install_label(active.as_ref());
        let (label, enabled) = if raw == "INSTALL" {
            ("INSTALL", true)
        } else if raw == "INSTALLED" {
            ("INSTALLED", false)
        } else {
            ("UNAVAILABLE", false)
        };
        if opaque_button(ui, btn, label, enabled) && enabled {
            self.on_install();
        }
    }

    pub(crate) fn paint_instance_delete(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(id) = self.confirm_instance.clone() else { return };
        let name = self.instances.iter().find(|i| i.id == id).map(|i| i.name.clone()).unwrap_or_else(|| "Instance".into());
        let dir = crate::instances::dir(&self.app, &id);
        let files = count_files(&dir);
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(120));
        let box_rect = egui::Rect::from_center_size(full.center(), egui::vec2(440.0, 168.0));
        paint_main_box(ui.painter(), box_rect);
        ui.painter().text(egui::pos2(box_rect.left() + 18.0, box_rect.top() + 28.0), egui::Align2::LEFT_CENTER, "Delete this instance?", title_font(16.0), HY_TEXT);
        ui.painter().text(egui::pos2(box_rect.left() + 18.0, box_rect.top() + 56.0), egui::Align2::LEFT_CENTER, &format!("{name} · {files} files"), body_font(14.0), HY_MUTED);
        let cancel = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 220.0, box_rect.bottom() - 52.0), egui::vec2(96.0, 34.0));
        let ok = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 112.0, box_rect.bottom() - 52.0), egui::vec2(96.0, 34.0));
        if opaque_button(ui, cancel, "Cancel", false) || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.confirm_instance = None;
        }
        if red_button(ui, ok, "Delete") || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.delete_instance(&id);
            self.confirm_instance = None;
        }
    }

    pub(crate) fn paint_version_list(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let count = format!("{}", self.instances.len());
        let head_font = sub_font(us(full, 12.0));
        let count_w = tracked_width(ui.painter(), &count, head_font.clone(), us(full, 1.0));
        let head = egui::Rect::from_min_max(
            egui::pos2(ux(full, 40.0), uy(full, 118.0)),
            egui::pos2(ux(full, 164.0) + count_w + us(full, 14.0), uy(full, 162.0)),
        );
        paint_version_corners(ui.painter(), head, false);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 52.0), head.center().y), egui::Align2::LEFT_CENTER, "INSTANCES", head_font.clone(), MUTED, us(full, 1.2));
        paint_gold_bar(ui.painter(), ux(full, 148.0), head.top() + us(full, 6.0), head.height() - us(full, 12.0));
        paint_tracked(ui.painter(), egui::pos2(ux(full, 164.0), head.center().y), egui::Align2::LEFT_CENTER, &count, head_font, MUTED, us(full, 1.0));
        paint_gold_bar(ui.painter(), ux(full, 572.0), uy(full, 124.0), us(full, 36.0));
        let new_btn = drect(full, 590.0, 118.0, 128.0, 44.0);
        if pill_action(ui, new_btn, "New", true) {
            self.draft_name.clear();
            let release = self.versions.iter().find(|v| v.channel == "release").map(|v| v.id.clone());
            self.draft_version = if self.versions.iter().any(|v| v.id == self.selected && v.channel == "release") {
                self.selected.clone()
            } else {
                release.unwrap_or_else(|| "26.3".into())
            };
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
            let msg = "No instances yet";
            let font = body_font(us(full, 15.0));
            let w = ui.painter().layout_no_wrap(msg.to_string(), font.clone(), MUTED).size().x;
            let chip = egui::Rect::from_center_size(list.center(), egui::vec2(w + us(full, 28.0), us(full, 36.0)));
            paint_version_corners(ui.painter(), chip, false);
            ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, msg, font, MUTED);
            let _ = ui.allocate_rect(list, Sense::hover());
        } else {
            egui::ScrollArea::vertical().id_salt("instance-list").auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.set_width(list.width());
                for inst in &rows {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(list.width(), row_h), Sense::click());
                    let on = self.instance_id == inst.id;
                    let chip = rect.shrink2(egui::vec2(us(full, 6.0), us(full, 5.0)));
                    paint_version_corners(ui.painter(), chip, on);
                    let name_ink = if on { GOLD } else { V_TEXT };
                    let mid_y = rect.center().y;
                    let bar_h = rect.height() * 0.42;
                    let bar_y = mid_y - bar_h * 0.5;
                    paint_gold_bar(ui.painter(), rect.left() + us(full, 200.0), bar_y, bar_h);
                    paint_gold_bar(ui.painter(), rect.left() + us(full, 370.0), bar_y, bar_h);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 24.0), mid_y), egui::Align2::LEFT_CENTER, &inst.name, title_font(us(full, 18.0)), name_ink);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 220.0), mid_y), egui::Align2::LEFT_CENTER, &inst.version, body_font(us(full, 15.0)), V_TEXT);
                    ui.painter().text(egui::pos2(rect.left() + us(full, 390.0), mid_y), egui::Align2::LEFT_CENTER, crate::state::loader_label(&inst.loader), sub_font(us(full, 13.0)), GOLD);
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

        let foot = drect(full, 40.0, 636.0, 690.0, 52.0);
        paint_version_corners(&painter, foot, false);
        let bar_h = foot.height() * 0.42;
        let bar_y = foot.center().y - bar_h * 0.5;
        paint_gold_bar(&painter, foot.left() + us(full, 92.0), bar_y, bar_h);
        paint_tracked(&painter, egui::pos2(foot.left() + us(full, 14.0), foot.center().y), egui::Align2::LEFT_CENTER, "SELECTED", sub_font(us(full, 12.0)), MUTED, us(full, 1.2));
        let selected_name = self.current_instance().map(|i| i.name.clone()).unwrap_or_else(|| self.selected.clone());
        let name_font = title_font(us(full, 16.0));
        painter.text(egui::pos2(foot.left() + us(full, 108.0), foot.center().y), egui::Align2::LEFT_CENTER, &selected_name, name_font.clone(), GOLD);
        let name_w = painter.layout_no_wrap(selected_name, name_font, GOLD).size().x;
        paint_gold_bar(&painter, foot.left() + us(full, 108.0) + name_w + us(full, 14.0), bar_y, bar_h);
        let drop = self.paper_for(&self.selected).title.to_uppercase();
        let drop_font = sub_font(us(full, 12.0));
        let drop_w = painter.layout_no_wrap(drop.clone(), drop_font, MUTED).size().x;
        paint_gold_bar(&painter, foot.right() - us(full, 16.0) - drop_w - us(full, 14.0), bar_y, bar_h);
        paint_tracked(&painter, egui::pos2(foot.right() - us(full, 16.0), foot.center().y), egui::Align2::RIGHT_CENTER, &drop, sub_font(us(full, 12.0)), MUTED, us(full, 1.0));
    }

    pub(crate) fn paint_instance_popup(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let dim = ui.interact(full, egui::Id::new("instance-overlay"), Sense::click());
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(150));
        // Room under the buttons so the bar stroke is not clipped by the card.
        let card = egui::Rect::from_center_size(full.center(), egui::vec2(680.0, 404.0));
        paint_hytale_bar(ui.painter(), card);
        if dim.clicked() {
            if let Some(at) = dim.interact_pointer_pos() {
                if !card.contains(at) {
                    self.draft_open = false;
                    return;
                }
            }
        }
        let left = card.left() + 22.0;
        let right = card.right() - 22.0;
        let width = right - left;
        ui.painter().text(egui::pos2(left, card.top() + 30.0), egui::Align2::LEFT_CENTER, "New instance", title_font(20.0), TEXT);
        ui.painter().text(
            egui::pos2(left, card.top() + 50.0),
            egui::Align2::LEFT_CENTER,
            "A separate world, options, and mods folder.",
            sub_font(12.0),
            MUTED,
        );
        paint_faded_rule(ui.painter(), left, card.top() + 66.0, width);
        ui.painter().text(egui::pos2(left, card.top() + 84.0), egui::Align2::LEFT_CENTER, "Name", sub_font(12.0), MUTED);
        let field_rect = egui::Rect::from_min_size(egui::pos2(left, card.top() + 96.0), egui::vec2(width, 36.0));
        paint_popup_field(ui, field_rect, &mut self.draft_name, "Survival, creative, test…");

        ui.painter().text(egui::pos2(left, card.top() + 152.0), egui::Align2::LEFT_CENTER, "Version", sub_font(12.0), MUTED);
        let gap = 8.0;
        let row_h = 36.0;
        let grid = egui::Rect::from_min_size(egui::pos2(left, card.top() + 166.0), egui::vec2(width, row_h * 3.0 + gap * 2.0));
        let versions: Vec<String> = self.versions.iter().filter(|v| v.channel == "release").map(|v| v.id.clone()).collect();
        let picked = self.draft_version.clone();
        let mut chosen: Option<String> = None;
        if versions.is_empty() {
            ui.painter().text(grid.center(), egui::Align2::CENTER_CENTER, "Loading releases…", body_font(14.0), MUTED);
        } else {
            let mut grid_ui = ui.new_child(egui::UiBuilder::new().max_rect(grid));
            grid_ui.set_clip_rect(grid);
            style_popup_scroll(&mut grid_ui);
            let content_w = (grid.width() - 14.0).max(120.0);
            let vw = (content_w - gap * 3.0) / 4.0;
            egui::ScrollArea::vertical()
                .id_salt("popup-versions")
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(&mut grid_ui, |ui| {
                    ui.set_width(content_w);
                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                    for chunk in versions.chunks(4) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            ui.set_height(row_h);
                            for id in chunk {
                                if button(ui, id, vw, row_h, id == &picked) {
                                    chosen = Some(id.clone());
                                }
                            }
                        });
                    }
                });
        }
        if let Some(id) = chosen {
            self.draft_version = id;
        }

        paint_faded_rule(ui.painter(), left, card.top() + 308.0, width);
        let btn_y = card.top() + 324.0;
        let create = egui::Rect::from_min_size(egui::pos2(right - 128.0, btn_y), egui::vec2(128.0, 40.0));
        let cancel = egui::Rect::from_min_size(egui::pos2(create.left() - 10.0 - 112.0, btn_y), egui::vec2(112.0, 40.0));
        if bar_button(ui, "popup-create", create, "Create", true) {
            let version = if self.draft_version.is_empty() {
                versions.first().cloned().unwrap_or_else(|| "26.3".into())
            } else {
                self.draft_version.clone()
            };
            let inst = crate::instances::create(&self.draft_name, &version, "vanilla");
            let _ = std::fs::create_dir_all(crate::instances::dir(&self.app, &inst.id));
            self.instance_id = inst.id.clone();
            self.selected = inst.version.clone();
            self.loader = inst.loader.clone();
            self.instances.push(inst);
            crate::instances::save(&self.app, &self.instances);
            self.save_settings();
            self.draft_open = false;
        }
        if bar_button(ui, "popup-cancel", cancel, "Cancel", false) {
            self.draft_open = false;
        }
    }

    pub(crate) fn paint_loader_panel(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let loader_font = sub_font(us(full, 12.0));
        let loader_w = tracked_width(ui.painter(), "LOADER", loader_font.clone(), us(full, 1.3));
        let loader_chip = egui::Rect::from_min_max(
            egui::pos2(ux(full, 778.0), uy(full, 118.0)),
            egui::pos2(ux(full, 790.0) + loader_w + us(full, 14.0), uy(full, 162.0)),
        );
        paint_version_corners(ui.painter(), loader_chip, false);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 790.0), loader_chip.center().y), egui::Align2::LEFT_CENTER, "LOADER", loader_font, MUTED, us(full, 1.3));
        let for_mc = format!("FOR MC {}", self.selected);
        let for_font = sub_font(us(full, 13.0));
        let for_w = tracked_width(ui.painter(), &for_mc, for_font.clone(), us(full, 0.8));
        let for_right = ux(full, 1236.0);
        let for_chip = egui::Rect::from_min_max(
            egui::pos2(for_right - for_w - us(full, 22.0), uy(full, 118.0)),
            egui::pos2(for_right, uy(full, 162.0)),
        );
        paint_gold_bar(ui.painter(), for_chip.left() - us(full, 14.0), uy(full, 124.0), us(full, 36.0));
        paint_version_corners(ui.painter(), for_chip, false);
        paint_tracked(ui.painter(), egui::pos2(for_chip.right() - us(full, 10.0), for_chip.center().y), egui::Align2::RIGHT_CENTER, &for_mc, for_font, GOLD, us(full, 0.8));

        let mc = self.selected.clone();
        let rows = crate::loaders::rows_for(&mc, &self.loaders);
        let mut clicked: Option<String> = None;
        for (i, row) in rows.iter().enumerate() {
            let y = 168.0 + i as f32 * 78.0;
            let rect = drect(full, 768.0, y, 483.0, 78.0);
            let on = self.loader_active(row.id);
            let chip = rect.shrink2(egui::vec2(us(full, 4.0), us(full, 6.0)));
            paint_version_corners(ui.painter(), chip, on);
            let hit = ui.interact(rect, egui::Id::new("loader-row").with(row.id), Sense::click());
            let bar_h = rect.height() * 0.36;
            let bar_y = rect.center().y - bar_h * 0.5;
            let icon = egui::Rect::from_center_size(egui::pos2(ux(full, 814.0), rect.center().y), egui::vec2(us(full, 32.0), us(full, 32.0)));
            if let Some(tex) = self.loader_icon_id(row.id) {
                ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            paint_gold_bar(ui.painter(), ux(full, 968.0), bar_y, bar_h);
            let name_ink = if on { GOLD } else { V_TEXT };
            ui.painter().text(egui::pos2(ux(full, 852.0), rect.center().y), egui::Align2::LEFT_CENTER, row.name, title_font(us(full, 18.0)), name_ink);
            let detail = loader_detail(row, &mc, self.loaders.ready);
            let detail_color = if row.kind == crate::loaders::Badge::Preview { GOLD } else { MUTED };
            ui.painter().text(egui::pos2(ux(full, 988.0), rect.center().y), egui::Align2::LEFT_CENTER, &detail, body_font(us(full, 15.0)), detail_color);
            let badge = loader_badge_rect(full, rect);
            paint_gold_bar(ui.painter(), badge.left() - us(full, 10.0), bar_y, bar_h);
            let badge_ink = match row.kind {
                crate::loaders::Badge::BuiltIn | crate::loaders::Badge::Ready => V_GREEN,
                crate::loaders::Badge::Preview => GOLD,
            };
            ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, row.badge, sub_font(us(full, 11.0)), badge_ink);
            if hit.clicked() {
                clicked = Some(row.id.to_string());
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
        if let Some(id) = clicked {
            self.choose_loader(&id);
        }

        let primary = self.install_loader_id();
        let selected = rows.iter().find(|r| r.id == primary).cloned();
        let foot_y = uy(full, 578.0);
        let sel_font = sub_font(us(full, 12.0));
        let sel_w = tracked_width(ui.painter(), "SELECTED LOADER", sel_font.clone(), us(full, 1.1));
        let sel_chip = egui::Rect::from_min_max(
            egui::pos2(ux(full, 778.0), foot_y - us(full, 16.0)),
            egui::pos2(ux(full, 790.0) + sel_w + us(full, 14.0), foot_y + us(full, 16.0)),
        );
        paint_version_corners(ui.painter(), sel_chip, false);
        paint_tracked(ui.painter(), egui::pos2(ux(full, 790.0), foot_y), egui::Align2::LEFT_CENTER, "SELECTED LOADER", sel_font, MUTED, us(full, 1.1));
        if self.loader == "forge+optifine" {
            let ver = selected.as_ref().map(|r| r.version.as_str()).unwrap_or("");
            let right = if ver.is_empty() {
                "FORGE + OPTIFINE".to_string()
            } else {
                format!("FORGE {ver} + OPTIFINE")
            };
            let right_font = sub_font(us(full, 13.0));
            let right_w = tracked_width(ui.painter(), &right, right_font.clone(), us(full, 0.7));
            let right_edge = ux(full, 1236.0);
            let right_chip = egui::Rect::from_min_max(
                egui::pos2(right_edge - right_w - us(full, 22.0), foot_y - us(full, 16.0)),
                egui::pos2(right_edge, foot_y + us(full, 16.0)),
            );
            paint_version_corners(ui.painter(), right_chip, false);
            paint_tracked(ui.painter(), egui::pos2(right_chip.right() - us(full, 10.0), foot_y), egui::Align2::RIGHT_CENTER, &right, right_font, V_GOLD, us(full, 0.7));
        } else if let Some(row) = &selected {
            let right = if row.version.is_empty() {
                row.name.to_uppercase()
            } else {
                format!("{} {}", row.name.to_uppercase(), row.version)
            };
            let right_font = sub_font(us(full, 13.0));
            let right_w = tracked_width(ui.painter(), &right, right_font.clone(), us(full, 0.7));
            let right_edge = ux(full, 1236.0);
            let right_chip = egui::Rect::from_min_max(
                egui::pos2(right_edge - right_w - us(full, 22.0), foot_y - us(full, 16.0)),
                egui::pos2(right_edge, foot_y + us(full, 16.0)),
            );
            paint_version_corners(ui.painter(), right_chip, false);
            paint_tracked(ui.painter(), egui::pos2(right_chip.right() - us(full, 10.0), foot_y), egui::Align2::RIGHT_CENTER, &right, right_font, V_GOLD, us(full, 0.7));
        }
        let install = drect(full, 786.0, 612.0, 448.0, 52.0);
        let label = self.install_label(selected.as_ref());
        let enabled = label == "INSTALL";
        if install_button(ui, install, &label, enabled) && enabled {
            self.on_install();
        }
    }

}
