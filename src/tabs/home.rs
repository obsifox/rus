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

    pub(crate) fn paint_chrome(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let horizon_on = self.engine == "horizon";
        self.paint_recent_servers(ui, full);
        self.paint_title_logo(ui, full);
        self.paint_instance_card(ui, full);

        let play_h = us(full, 52.0);
        let play_w = 112.0;
        let play = egui::Rect::from_min_size(
            egui::pos2(full.right() - 22.0 - play_w, uy(full, 640.0)),
            egui::vec2(play_w, play_h),
        );
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
        let note_y = play.top() - 16.0;
        paint_diamond(ui.painter(), egui::pos2(play.right() - note_w - 14.0, note_y), GOLD);
        ui.painter().text(egui::pos2(play.right() - 2.0, note_y), egui::Align2::RIGHT_CENTER, &note, note_font, GOLD);
        let horizon_here = self.selected.starts_with("26.3");
        if horizon_here && self.paint_horizon_option(ui, play, horizon_on) {
            self.engine = if horizon_on { "vanilla".into() } else { "horizon".into() };
            self.save_settings();
        }
        if play_button(ui, play, label, live, horizon_on && !live) {
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

    pub(crate) fn paint_title_logo(&self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(tex) = &self.logo_title else { return };
        let card_top = uy(full, 640.0);
        let h = 104.0 * 0.98;
        let size = tex.size();
        let aspect = size[0] as f32 / size[1] as f32;
        let w = h * aspect;
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, card_top - 50.0 - h + 4.0), egui::vec2(w, h));
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
        let ver_font = title_font(us(full, 15.0));
        let sub_f = sub_font(10.0);
        let time_font = title_font(us(full, 14.0));
        let white = egui::Color32::WHITE;
        let ver_w = ui.painter().layout_no_wrap(version.clone(), ver_font.clone(), GOLD).size().x;
        let loader_w = ui.painter().layout_no_wrap(loader.clone(), sub_f.clone(), white).size().x;
        let time_w = ui.painter().layout_no_wrap(time.clone(), time_font.clone(), TEXT).size().x;
        let tail = self.status_tail();
        let tail_w = ui.painter().layout_no_wrap(tail.clone(), time_font.clone(), TEXT).size().x;
        let text_w = ver_w.max(loader_w);
        let h = us(full, 52.0);
        let icon = 22.0;
        let pad = 10.0;
        let right_gap = 12.0;
        let clock_r = 8.0;
        let clock_gap = 10.0;
        let tail_gap = 8.0;
        let tail_pad = 10.0;
        let w = pad + icon + 8.0 + text_w + 8.0 + 2.0 + clock_gap + clock_r * 2.0 + 9.0 + time_w + tail_gap + 2.0 + tail_pad + tail_w + right_gap;
        let card = egui::Rect::from_min_size(egui::pos2(ux(full, 26.0), uy(full, 640.0)), egui::vec2(w, h));
        paint_hytale_bar(ui.painter(), card);
        let icon_rect = egui::Rect::from_center_size(egui::pos2(card.left() + pad + icon * 0.5, card.center().y), egui::vec2(icon, icon));
        if let Some(tex) = self.loader_icon() {
            ui.painter().image(tex.id(), icon_rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        let text_x = icon_rect.right() + 8.0;
        let ver_y = card.center().y - 7.0;
        ui.painter().text(egui::pos2(text_x, ver_y), egui::Align2::LEFT_CENTER, &version, ver_font, GOLD);
        ui.painter().text(egui::pos2(text_x, ver_y + 13.0), egui::Align2::LEFT_CENTER, &loader, sub_f, white);
        let text_y = card.center().y;
        let text_right = text_x + text_w;
        let bar_x = text_right + 8.0;
        paint_faded_bar(ui.painter(), bar_x, card.top() + card.height() * 0.28, card.height() * 0.44);
        let clock_x = bar_x + 2.0 + clock_gap + clock_r;
        paint_clock(ui.painter(), egui::pos2(clock_x, text_y), GOLD);
        let time_x = clock_x + 16.0;
        ui.painter().text(egui::pos2(time_x, text_y), egui::Align2::LEFT_CENTER, &time, time_font.clone(), TEXT);
        let tail_bar_x = time_x + time_w + tail_gap;
        paint_faded_bar(ui.painter(), tail_bar_x, card.top() + card.height() * 0.28, card.height() * 0.44);
        ui.painter().text(egui::pos2(tail_bar_x + 2.0 + tail_pad, text_y), egui::Align2::LEFT_CENTER, &tail, time_font.clone(), TEXT);
        card
    }

    pub(crate) fn paint_horizon_option(&mut self, ui: &mut egui::Ui, play: egui::Rect, on: bool) -> bool {
        let font = if on { title_font(12.5) } else { sub_font(12.0) };
        let label = "Horizon";
        let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), GOLD).size().x;
        let row = egui::Rect::from_center_size(egui::pos2(play.center().x, play.bottom() + 11.0), egui::vec2(text_w + 16.0, 16.0));
        let hit = ui.interact(row, egui::Id::new("horizon-option"), Sense::click());
        let pos = row.center();
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
