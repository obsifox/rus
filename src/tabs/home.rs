//! Home tab.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn paint_recent_servers(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
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

    pub(crate) fn loader_hit_rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(HOME_RAIL + 10.0, 12.0), egui::vec2(32.0, 32.0))
    }

    pub(crate) fn home_popup_rect(&self, full: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(HOME_RAIL + 10.0, 90.0),
            egui::pos2((HOME_RAIL + 520.0).min(full.right() - 16.0), full.bottom() - 96.0),
        )
    }

    /// Black rail. Uninstalled versions stay gray on the left. Installed ones sit on the right.
    pub(crate) fn paint_home_rail(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let rail = egui::Rect::from_min_size(full.left_top(), egui::vec2(HOME_RAIL, full.height()));
        ui.painter().rect_filled(rail, 0.0, egui::Color32::from_rgb(8, 8, 10));
        ui.painter().vline(
            rail.right() - 0.5,
            rail.top()..=rail.bottom(),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(32, 32, 36)),
        );
        let name = if self.username.is_empty() { "Player".to_string() } else { self.username.clone() };
        let head_s = 28.0;
        let head = egui::Rect::from_center_size(egui::pos2(rail.center().x, 16.0 + head_s * 0.5), egui::vec2(head_s, head_s));
        let profile = ui.interact(head.expand(4.0), egui::Id::new("rail-profile"), Sense::click());
        if let Some(face) = &self.avatar {
            ui.painter().add(
                egui::epaint::RectShape::filled(head, 6.0, egui::Color32::WHITE)
                    .with_texture(face.id(), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))),
            );
        } else {
            ui.painter().rect_filled(head, 6.0, egui::Color32::from_rgb(0x1A, 0x14, 0x10));
        }
        if profile.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if profile.clicked() {
            self.tab = Tab::Settings;
        }
        profile.on_hover_text(name);

        let btn = 30.0;
        let gap = 6.0;
        let x0 = rail.center().x - (btn * 2.0 + gap) * 0.5;
        let y0 = 50.0;
        for (i, kind) in [0u8, 1u8].into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x0 + i as f32 * (btn + gap), y0), egui::vec2(btn, btn));
            let on = self.rail_view == kind;
            let hit = ui.interact(rect, egui::Id::new("rail-mode").with(i), Sense::click());
            let hot = ui.ctx().animate_bool_with_time(hit.id.with("hot"), hit.hovered() || on, 0.14);
            let fill = if on {
                egui::Color32::from_rgba_unmultiplied(0x2C, 0x24, 0x12, 190 + (hot * 40.0) as u8)
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, (16.0 + hot * 28.0) as u8)
            };
            ui.painter().rect_filled(rect, 8.0, fill);
            if on {
                ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, GOLD), egui::StrokeKind::Inside);
            }
            let ink = if on { GOLD } else { HY_TEXT };
            if kind == 0 {
                paint_installed_icon(ui.painter(), rect.center(), 15.0, ink);
            } else {
                paint_globe_icon(ui.painter(), rect.center(), 15.0, ink);
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() && self.rail_view != kind {
                self.rail_view = kind;
                crate::sfx::toggle();
            }
            hit.on_hover_text(if kind == 0 { "Installed" } else { "Explore" });
        }
        let typing = ui.memory(|m| m.focused().is_some());
        if !typing {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.rail_nudge += 68.0;
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.rail_nudge -= 68.0;
            }
        }
        let list = egui::Rect::from_min_max(egui::pos2(rail.left() + 4.0, 88.0), egui::pos2(rail.right() - 4.0, full.bottom() - 46.0));
        let versions = self.rail_versions();
        let ids: Vec<String> = if self.rail_view == 0 {
            versions.into_iter().filter(|id| self.app.is_installed(id)).collect()
        } else {
            versions
        };
        let nudge = self.rail_nudge;
        self.rail_nudge = 0.0;
        if ids.is_empty() {
            ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, "None yet", sub_font(10.0), HY_MUTED);
        }
        if let Some(id) = self.paint_version_column(ui, list, &ids, nudge, "rail-one") {
            crate::sfx::select();
            self.select_version_art(&id);
        }

        let gear = egui::Rect::from_center_size(egui::pos2(rail.center().x, full.bottom() - 22.0), egui::vec2(32.0, 32.0));
        let settings = ui.interact(gear, egui::Id::new("rail-settings"), Sense::click());
        let gear_ink = if settings.hovered() { GOLD } else { egui::Color32::from_rgb(210, 214, 220) };
        paint_tab_glyph(ui.painter(), Tab::Settings, gear.center(), 15.0, gear_ink);
        if settings.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if settings.clicked() {
            self.tab = Tab::Settings;
        }
        settings.on_hover_text("Settings");
    }

    /// Loader only, 10px from the rail. Pinned copies appear beside it, and only after a pin.
    pub(crate) fn paint_version_bar(&mut self, ui: &mut egui::Ui, _full: egui::Rect) {
        let icon = self.loader_hit_rect();
        let open = self.home_menu;
        ui.painter().rect_filled(icon, 6.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 220));
        ui.painter().rect_stroke(
            icon,
            6.0,
            egui::Stroke::new(if open { 1.6 } else { 1.0 }, if open { GOLD } else { egui::Color32::from_white_alpha(36) }),
            egui::StrokeKind::Inside,
        );
        let loader = self.current_instance().map(|i| i.loader.clone()).unwrap_or_else(|| self.loader.clone());
        if let Some(tex) = self.loader_icon_id(&loader) {
            ui.painter().image(
                tex.id(),
                icon.shrink(6.0),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        let hit = ui.interact(icon, egui::Id::new("loader-menu"), Sense::click());
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if hit.clicked() {
            self.home_menu = !self.home_menu;
            if self.home_menu { crate::sfx::open(); } else { crate::sfx::back(); }
        }
        hit.on_hover_text("Instance and Mods");

        let pinned: Vec<crate::instances::Instance> = self.instances.iter().filter(|i| i.pinned).take(3).cloned().collect();
        let mut x = icon.right() + 6.0;
        let mut pick = None;
        for inst in &pinned {
            let slot = egui::Rect::from_min_size(egui::pos2(x, icon.top()), egui::vec2(32.0, 32.0));
            let on = inst.id == self.instance_id;
            ui.painter().rect_filled(slot, 6.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 220));
            ui.painter().rect_stroke(slot, 6.0, egui::Stroke::new(if on { 1.6 } else { 1.0 }, if on { GOLD } else { egui::Color32::from_white_alpha(50) }), egui::StrokeKind::Inside);
            if let Some(tex) = self.loader_icon_id(&inst.loader) {
                ui.painter().image(
                    tex.id(),
                    slot.shrink(7.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            let chip = ui.interact(slot, egui::Id::new("pin-slot").with(&inst.id), Sense::click());
            if chip.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if chip.clicked() {
                pick = Some(inst.id.clone());
            }
            chip.on_hover_text(&inst.name);
            x += 32.0 + 6.0;
        }
        if let Some(id) = pick {
            self.select_instance(&id);
        }
    }

    pub(crate) fn paint_home_switch(&mut self, ui: &mut egui::Ui, _full: egui::Rect) {
        let y = 52.0;
        let h = 32.0;
        let labels = ["Instance", "Mods"];
        let mut widths = Vec::new();
        for label in labels {
            widths.push(ui.painter().layout_no_wrap(label.to_string(), nav_font(12.0), HY_TEXT).size().x + 22.0);
        }
        let bar_w: f32 = widths.iter().sum();
        let bar = egui::Rect::from_min_size(egui::pos2(HOME_RAIL + 10.0, y), egui::vec2(bar_w, h));
        ui.painter().rect_filled(bar, 8.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 235));
        ui.painter().rect_stroke(bar, 8.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(36)), egui::StrokeKind::Inside);
        let mut sx = bar.left();
        for (i, (label, w)) in labels.into_iter().zip(widths).enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(sx, bar.top()), egui::vec2(w, h));
            let on = self.home_pane as usize == i;
            let hit = ui.interact(rect, egui::Id::new("home-pane").with(i), Sense::click());
            if on {
                ui.painter().rect_filled(rect.shrink2(egui::vec2(3.0, 4.0)), 6.0, egui::Color32::from_rgb(0x2C, 0x24, 0x12));
            }
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(12.0), if on { GOLD } else { HY_TEXT });
            if on {
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(egui::pos2(rect.left() + 8.0, rect.bottom() - 3.0), egui::pos2(rect.right() - 8.0, rect.bottom() - 1.0)),
                    1.0,
                    GOLD,
                );
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.home_pane = i as u8;
            }
            sx += w;
        }
    }

    pub(crate) fn paint_version_instances(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let version = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        let panel = self.home_popup_rect(full);
        ui.painter().rect_filled(panel, 12.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 196));
        ui.painter().rect_stroke(panel, 12.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(32)), egui::StrokeKind::Inside);
        ui.painter().text(
            egui::pos2(panel.left() + 14.0, panel.top() + 20.0),
            egui::Align2::LEFT_CENTER,
            &format!("Instances · {version}"),
            nav_font(13.0),
            HY_TEXT,
        );
        let new_btn = egui::Rect::from_min_size(egui::pos2(panel.right() - 92.0, panel.top() + 8.0), egui::vec2(78.0, 26.0));
        let mut make = false;
        if opaque_button(ui, new_btn, "NEW", true) {
            make = true;
        }
        let rows: Vec<crate::instances::Instance> = self.instances.iter().filter(|i| i.version == version).cloned().collect();
        let list = egui::Rect::from_min_max(egui::pos2(panel.left() + 10.0, panel.top() + 42.0), egui::pos2(panel.right() - 10.0, panel.bottom() - 8.0));
        let mut pick = None;
        let mut pin = None;
        let mut drop_id = None;
        if rows.is_empty() {
            ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, "No copies of this version yet", body_font(14.0), HY_MUTED);
        } else {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
            child.set_clip_rect(list);
            quiet_scroll("ver-copies").show(&mut child, |ui| {
                ui.set_width(list.width());
                for inst in &rows {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(list.width(), 44.0), Sense::hover());
                    ui.add_space(6.0);
                    let on = inst.id == self.instance_id;
                    paint_opaque_card(ui.painter(), rect, if on { 1.0 } else { 0.0 });
                    let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 20.0, rect.center().y), egui::vec2(22.0, 22.0));
                    if let Some(tex) = self.loader_icon_id(&inst.loader) {
                        ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    }
                    let pin_btn = egui::Rect::from_min_size(egui::pos2(rect.right() - 132.0, rect.top() + 8.0), egui::vec2(58.0, 28.0));
                    let del = egui::Rect::from_min_size(egui::pos2(rect.right() - 68.0, rect.top() + 8.0), egui::vec2(58.0, 28.0));
                    let name = ellipsize(ui.painter(), &inst.name, title_font(14.0), pin_btn.left() - icon.right() - 16.0);
                    ui.painter().text(egui::pos2(icon.right() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, &name, title_font(14.0), if on { GOLD } else { HY_TEXT });
                    if action_button(ui, pin_btn, if inst.pinned { "On" } else { "Pin" }, if inst.pinned { BtnKind::Outline } else { BtnKind::Quiet }) {
                        pin = Some(inst.id.clone());
                    }
                    if red_button(ui, del, "DEL") {
                        drop_id = Some(inst.id.clone());
                    }
                    let hit = ui.interact(egui::Rect::from_min_max(rect.left_top(), egui::pos2(pin_btn.left() - 4.0, rect.bottom())), egui::Id::new("copy-pick").with(&inst.id), Sense::click());
                    if hit.clicked() {
                        pick = Some(inst.id.clone());
                    }
                }
            });
        }
        if make {
            self.create_version_copy(&version);
        }
        if let Some(id) = pick {
            self.select_instance(&id);
        }
        if let Some(id) = pin {
            self.toggle_pin(&id);
        }
        if let Some(id) = drop_id {
            self.confirm_instance = Some(id);
        }
    }

    fn rail_versions(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.versions.iter().filter(|v| v.channel == "release").map(|v| v.id.clone()).collect();
        if ids.is_empty() {
            for paper in crate::wallpapers::all() {
                if let Some(prefix) = paper.prefixes.first() {
                    if !ids.iter().any(|id| id == prefix) {
                        ids.push((*prefix).to_string());
                    }
                }
            }
        }
        if ids.is_empty() {
            ids.push("26.3".into());
        }
        ids
    }

    fn select_version_art(&mut self, version: &str) {
        self.selected = version.to_string();
        if self.current_instance().map(|i| i.version.as_str()) != Some(version) {
            if let Some(inst) = self.instances.iter().find(|i| i.version == version).cloned() {
                self.select_instance(&inst.id);
                return;
            }
            self.instance_id.clear();
        }
        self.save_settings();
    }

    fn create_version_copy(&mut self, version: &str) {
        let inst = crate::instances::create(version, version, &self.loader);
        let _ = std::fs::create_dir_all(crate::instances::path_of(&self.app, &inst));
        crate::sfx::open();
        self.instances.push(inst.clone());
        crate::instances::save(&self.app, &self.instances);
        self.select_instance(&inst.id);
        self.notice = "Instance created".into();
    }

    fn version_cover(&self, id: &str) -> f32 {
        let installed = self.app.is_installed(id);
        let downloading = self.job_state().as_deref() == Some("running") && self.selected == id;
        if downloading {
            1.0 - self.job_line().map(|(p, _)| p).unwrap_or(0.0)
        } else if installed {
            0.0
        } else {
            1.0
        }
    }

    fn paint_version_column(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ids: &[String], nudge: f32, salt: &str) -> Option<String> {
        let mut pick = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        quiet_scroll(salt).show(&mut child, |ui| {
            ui.set_width(rect.width());
            if nudge != 0.0 {
                ui.scroll_with_delta(egui::vec2(0.0, nudge));
            }
            for id in ids {
                let (slot, _) = ui.allocate_exact_size(egui::vec2(rect.width(), 68.0), Sense::hover());
                let hit = ui.interact(slot, egui::Id::new(salt).with(id), Sense::click());
                let hot = ui.ctx().animate_bool_with_time(hit.id.with("hot"), hit.hovered(), 0.14);
                let side = 44.0 + hot * 3.0;
                let square = egui::Rect::from_center_size(egui::pos2(slot.center().x, slot.top() + 24.0), egui::vec2(side, side));
                let on = self.selected == *id || (self.selected.is_empty() && id == "26.3");
                let paper = self.paper_for(id);
                let sq_key = format!("{}#sq", paper.title);
                if let Some(tex) = self.papers.get(&sq_key).or_else(|| self.papers.get(paper.title)) {
                    let uv = if self.papers.contains_key(&sq_key) {
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
                    } else {
                        cover_focus(tex.size_vec2(), square, 0.5, 0.42)
                    };
                    ui.painter().add(
                        egui::epaint::RectShape::filled(square, 4.0, egui::Color32::WHITE)
                            .with_texture(tex.id(), uv),
                    );
                } else {
                    ui.painter().rect_filled(square, 4.0, egui::Color32::from_rgb(28, 28, 32));
                }
                let cover = ui.ctx().animate_value_with_time(egui::Id::new("sq-veil").with(id), self.version_cover(id), 0.22);
                if cover > 0.02 {
                    let veil = egui::Rect::from_min_size(square.left_top(), egui::vec2(square.width(), square.height() * cover));
                    ui.painter().rect_filled(veil, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128));
                }
                let edge = if on { GOLD } else { egui::Color32::from_white_alpha(40 + (hot * 40.0) as u8) };
                ui.painter().rect_stroke(square, 4.0, egui::Stroke::new(if on { 2.0 } else { 1.0 }, edge), egui::StrokeKind::Inside);
                let label = ellipsize(ui.painter(), id, sub_font(9.0), rect.width() - 4.0);
                ui.painter().text(
                    egui::pos2(slot.center().x, slot.bottom() - 7.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    sub_font(9.0),
                    if on { egui::Color32::WHITE } else { HY_MUTED },
                );
                if hit.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if hit.clicked() {
                    pick = Some(id.clone());
                }
                let tip = if self.app.is_installed(id) { id.clone() } else { format!("{id} · not installed") };
                hit.on_hover_text(tip);
            }
        });
        pick
    }

    pub(crate) fn paint_version_veil(&self, ui: &egui::Ui, full: egui::Rect) {
        let version = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        let cover = ui.ctx().animate_value_with_time(egui::Id::new("home-veil"), self.version_cover(&version), 0.28);
        if cover < 0.01 {
            return;
        }
        let veil = egui::Rect::from_min_size(full.left_top(), egui::vec2(full.width(), full.height() * cover));
        ui.painter().rect_filled(veil, 0.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 148));
    }

    fn toggle_pin(&mut self, id: &str) {
        let others = self.instances.iter().filter(|i| i.pinned && i.id != id).count();
        let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) else { return };
        if inst.pinned {
            inst.pinned = false;
        } else if others >= 3 {
            self.notice = "Only three instances can be pinned".into();
            return;
        } else {
            inst.pinned = true;
        }
        crate::instances::save(&self.app, &self.instances);
    }

    pub(crate) fn paint_chrome(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let horizon_on = self.engine == "horizon";
        self.paint_instance_card(ui, full);

        let play_h = 50.0;
        let play_w = 156.0;
        let version = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        let installed = self.app.is_installed(&version);
        let live = crate::play::status().get("running").and_then(|v| v.as_bool()).unwrap_or(false);
        let installing = self.job_state().as_deref() == Some("running");
        let bar_h = if installing { 16.0 } else { 0.0 };
        let bar_gap = if installing { 8.0 } else { 0.0 };
        let play = egui::Rect::from_min_size(
            egui::pos2(full.right() - 18.0 - play_w, full.bottom() - 28.0 - bar_h - bar_gap - play_h),
            egui::vec2(play_w, play_h),
        );
        let label = if live { "STOP" } else { "PLAY" };
        let note = if installing {
            self.job_message().filter(|s| !s.is_empty()).unwrap_or_else(|| "Downloading…".into())
        } else if let Some((_, line)) = self.job_line() {
            line
        } else if self.notice != "Ready" && !self.notice.is_empty() {
            self.notice.clone()
        } else {
            format!("Patch {}", crate::product::VERSION)
        };
        let note = if note.chars().count() > 48 {
            format!("{}…", note.chars().take(47).collect::<String>())
        } else {
            note
        };
        let note_font = lexend(us(full, 14.0));
        let note_w = ui.painter().layout_no_wrap(note.clone(), note_font.clone(), GOLD).size().x;
        let note_y = if installing { play.top() - 36.0 } else { play.top() - 16.0 };
        paint_diamond(ui.painter(), egui::pos2(play.right() - note_w - 14.0, note_y), GOLD);
        ui.painter().text(egui::pos2(play.right() - 2.0, note_y), egui::Align2::RIGHT_CENTER, &note, note_font, GOLD);
        if installing {
            let progress = self.job_line().map(|(p, _)| p).unwrap_or(0.0);
            let track = egui::Rect::from_min_size(egui::pos2(play.right() - 280.0, play.bottom() + bar_gap), egui::vec2(280.0, bar_h));
            paint_download_bar(ui.painter(), track, progress);
        }
        let horizon_here = self.selected.starts_with("26.3");
        let horizon_anchor = if installing {
            egui::Rect::from_min_max(play.min, egui::pos2(play.right(), play.bottom() + bar_gap + bar_h - 2.0))
        } else {
            play
        };
        if horizon_here && self.paint_horizon_option(ui, horizon_anchor, horizon_on) {
            self.engine = if horizon_on { "vanilla".into() } else { "horizon".into() };
            self.save_settings();
        }
        if !installed || installing {
            let act = egui::Rect::from_min_size(egui::pos2(play.left() - 8.0 - 148.0, play.top()), egui::vec2(148.0, play_h));
            let act_label = if installing { "STOP" } else { "INSTALL" };
            if play_button(ui, "install", act, act_label, installing, false) {
                if installing {
                    crate::sfx::back();
                    self.cancel_install();
                } else {
                    crate::sfx::ok();
                    self.play_after = false;
                    self.start_install();
                }
            }
        }
        if play_button(ui, "play", play, label, live, horizon_on && !live) {
            crate::sfx::click();
            self.join = None;
            if live {
                self.play();
            } else if installing {
                self.notice = "Download in progress".into();
            } else if !installed && self.engine != "horizon" {
                self.notice = "Install this version first".into();
            } else {
                self.play();
            }
        }
    }

    pub(crate) fn paint_title_logo(&self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(tex) = &self.logo_title else { return };
        let size = tex.size();
        let aspect = if size[1] == 0 { 1080.0 / 481.0 } else { size[0] as f32 / size[1] as f32 };
        let h = 78.0;
        let w = h * aspect;
        let rect = egui::Rect::from_min_size(
            egui::pos2(full.right() - 48.0 - w, full.top() + 16.0),
            egui::vec2(w, h),
        );
        ui.painter().image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    /// Icon, version, loader, then play time. Display only — not a control.
    pub(crate) fn paint_instance_card(&self, ui: &mut egui::Ui, full: egui::Rect) -> egui::Rect {
        let version = if self.selected.is_empty() { "26.3".to_string() } else { self.selected.clone() };
        let loader = self.loader_line(&version);
        let (number, unit, _) = crate::state::play_label(self.shown_play_ms());
        let time = format!("{number}{unit}");
        let ver_font = title_font(15.0);
        let sub_f = sub_font(11.0);
        let time_font = title_font(13.0);
        let white = egui::Color32::WHITE;
        let ver_w = ui.painter().layout_no_wrap(version.clone(), ver_font.clone(), GOLD).size().x;
        let loader_w = ui.painter().layout_no_wrap(loader.clone(), sub_f.clone(), white).size().x;
        let time_w = ui.painter().layout_no_wrap(time.clone(), time_font.clone(), TEXT).size().x;
        let tail = self.status_tail();
        let tail_w = ui.painter().layout_no_wrap(tail.clone(), time_font.clone(), TEXT).size().x;
        let text_w = ver_w.max(loader_w);
        let h = 50.0;
        let icon = 26.0;
        let w = 12.0 + icon + 10.0 + text_w + 16.0 + 18.0 + 8.0 + time_w + 16.0 + 18.0 + 8.0 + tail_w + 14.0;
        let card = egui::Rect::from_min_size(egui::pos2(HOME_RAIL + 10.0, full.bottom() - 22.0 - h), egui::vec2(w, h));
        ui.painter().rect_filled(card, 10.0, egui::Color32::from_rgba_unmultiplied(8, 10, 14, 214));
        ui.painter().rect_stroke(card, 10.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(28)), egui::StrokeKind::Inside);
        let icon_rect = egui::Rect::from_center_size(egui::pos2(card.left() + 12.0 + icon * 0.5, card.center().y), egui::vec2(icon, icon));
        if let Some(tex) = self.loader_icon() {
            ui.painter().image(tex.id(), icon_rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        let text_x = icon_rect.right() + 10.0;
        ui.painter().text(egui::pos2(text_x, card.center().y - 8.0), egui::Align2::LEFT_CENTER, &version, ver_font, GOLD);
        ui.painter().text(egui::pos2(text_x, card.center().y + 8.0), egui::Align2::LEFT_CENTER, &loader, sub_f, white);
        let sep1 = text_x + text_w + 12.0;
        paint_faded_bar(ui.painter(), sep1, card.top() + 12.0, card.height() - 24.0);
        paint_clock(ui.painter(), egui::pos2(sep1 + 16.0, card.center().y), egui::Color32::from_rgb(120, 186, 255));
        let time_x = sep1 + 28.0;
        ui.painter().text(egui::pos2(time_x, card.center().y), egui::Align2::LEFT_CENTER, &time, time_font.clone(), TEXT);
        let sep2 = time_x + time_w + 12.0;
        paint_faded_bar(ui.painter(), sep2, card.top() + 12.0, card.height() - 24.0);
        paint_material(ui.painter(), egui::pos2(sep2 + 16.0, card.center().y), 14.0, I_FOLDER, egui::Color32::from_rgb(120, 186, 255));
        ui.painter().text(egui::pos2(sep2 + 28.0, card.center().y), egui::Align2::LEFT_CENTER, &tail, time_font, TEXT);
        card
    }

    pub(crate) fn paint_horizon_option(&mut self, ui: &mut egui::Ui, play: egui::Rect, on: bool) -> bool {
        let font = if on { title_font(12.0) } else { sub_font(12.0) };
        let label = "Horizon";
        let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), GOLD).size().x;
        let row = egui::Rect::from_center_size(egui::pos2(play.center().x, play.bottom() + 14.0), egui::vec2(text_w + 56.0, 16.0));
        let hit = ui.interact(row, egui::Id::new("horizon-option"), Sense::click());
        let pos = row.center();
        let line = if on { GOLD } else { egui::Color32::from_rgba_unmultiplied(200, 214, 230, 140) };
        ui.painter().hline(pos.x - text_w * 0.5 - 28.0..=pos.x - text_w * 0.5 - 8.0, pos.y, egui::Stroke::new(1.0, line));
        ui.painter().hline(pos.x + text_w * 0.5 + 8.0..=pos.x + text_w * 0.5 + 28.0, pos.y, egui::Stroke::new(1.0, line));
        if on {
            for (dx, dy, a) in [(0.0, 0.0, 42u8), (0.0, 1.0, 28), (-1.0, 0.0, 22), (1.0, 0.0, 22)] {
                ui.painter().text(
                    pos + egui::vec2(dx, dy),
                    egui::Align2::CENTER_CENTER,
                    label,
                    font.clone(),
                    egui::Color32::from_rgba_unmultiplied(232, 186, 78, a),
                );
            }
            ui.painter().text(pos, egui::Align2::CENTER_CENTER, label, font, egui::Color32::from_rgb(255, 228, 150));
        } else {
            let ink = if hit.hovered() { egui::Color32::from_rgb(255, 236, 196) } else { egui::Color32::from_rgb(220, 204, 168) };
            ui.painter().text(pos, egui::Align2::CENTER_CENTER, label, font, ink);
        }
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        hit.clicked()
    }

    pub(crate) fn account_bar(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, h: f32) {
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

    pub(crate) fn nav_bar(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, h: f32) {
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

    pub(crate) fn sidebar(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn page_home(&mut self, ui: &mut egui::Ui) {
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

}

/// Installed: a package with a downward arrow. Same weight as the globe beside it.
fn paint_installed_icon(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let box_r = egui::Rect::from_center_size(c + egui::vec2(0.0, s * 0.22), egui::vec2(s * 0.72, s * 0.46));
    painter.rect_stroke(box_r, 1.5, stroke, egui::StrokeKind::Inside);
    let lid_y = box_r.top() + s * 0.14;
    painter.line_segment(
        [egui::pos2(box_r.left() + 1.0, lid_y), egui::pos2(box_r.right() - 1.0, lid_y)],
        stroke,
    );
    let tip = egui::pos2(c.x, box_r.top() - s * 0.06);
    painter.line_segment([egui::pos2(c.x, tip.y - s * 0.26), tip], stroke);
    painter.line_segment([tip + egui::vec2(-s * 0.13, -s * 0.12), tip], stroke);
    painter.line_segment([tip + egui::vec2(s * 0.13, -s * 0.12), tip], stroke);
}

/// Explore: a globe, not a word.
fn paint_globe_icon(painter: &egui::Painter, c: egui::Pos2, s: f32, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let r = s * 0.40;
    painter.circle_stroke(c, r, stroke);
    painter.line_segment([c + egui::vec2(-r, 0.0), c + egui::vec2(r, 0.0)], stroke);
    painter.line_segment([c + egui::vec2(0.0, -r), c + egui::vec2(0.0, r)], stroke);
    let mut left = Vec::new();
    let mut right = Vec::new();
    for i in 0..=8 {
        let t = i as f32 / 8.0;
        let y = -r + t * 2.0 * r;
        let k = (1.0 - (y / r).powi(2)).max(0.0).sqrt();
        let x = r * 0.46 * k;
        left.push(c + egui::vec2(-x, y));
        right.push(c + egui::vec2(x, y));
    }
    painter.add(egui::Shape::line(left, stroke));
    painter.add(egui::Shape::line(right, stroke));
}

fn quiet_scroll(id: impl std::hash::Hash) -> egui::ScrollArea {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .drag_to_scroll(true)
        .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
}


