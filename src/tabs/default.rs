//! Shared tab chrome: player bar, nav, exit, and the panel frame.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn page_in_panel(&mut self, ui: &mut egui::Ui, full: egui::Rect, page: Page) {
        let panel = drect(full, 26.0, 96.0, 1228.0, 608.0);
        page_veil(ui.painter(), panel);
        paint_open_frame(ui.painter(), panel);
        let inner = panel.shrink(us(full, 14.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        style_page_widgets(&mut child);
        child.set_clip_rect(inner);
        match page {
            Page::Mods => self.page_mods(&mut child),
            Page::Settings => self.page_settings(&mut child),
        }
    }

    pub(crate) fn paint_shared_tabs(&mut self, ui: &mut egui::Ui, full: egui::Rect, left: f32) -> egui::Rect {
        let tabs = [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings];
        let h = us(full, 40.0);
        let font = nav_font(us(full, 14.5));
        let track = us(full, 0.7);
        let pad = us(full, 16.0);
        let mut widths = Vec::with_capacity(tabs.len());
        for tab in tabs {
            if tab == Tab::Home {
                widths.push(h * 0.28 + pad * 2.0);
            } else {
                let label = tab.label().to_uppercase();
                widths.push(tracked_width(ui.painter(), &label, font.clone(), track) + pad * 2.0);
            }
        }
        let total: f32 = widths.iter().sum();
        let bar = egui::Rect::from_min_size(egui::pos2(left, uy(full, 16.0)), egui::vec2(total, h));
        paint_hytale_bar(ui.painter(), bar);
        let mut x = bar.left();
        let mut rects = Vec::with_capacity(tabs.len());
        for (i, tab) in tabs.into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(widths[i], h));
            rects.push(rect);
            let hit = ui.interact(rect, egui::Id::new("hynav").with(tab.label()), Sense::click());
            let on = self.tab == tab;
            let active_t = ease_out(ui.ctx().animate_bool_with_time(egui::Id::new("nav-on").with(tab.label()), on, 0.16));
            let hover_t = ui.ctx().animate_bool_with_time(egui::Id::new("nav-hov").with(tab.label()), hit.hovered(), 0.1);
            let color = lerp_color(HY_TEXT, HY_ACCENT, active_t.max(hover_t * 0.4));
            if tab == Tab::Home {
                paint_nav_house(ui.painter(), rect.center(), h * 0.28, color);
            } else {
                let label = tab.label().to_uppercase();
                paint_tracked(
                    ui.painter(),
                    rect.center() + egui::vec2(0.0, 1.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    font.clone(),
                    egui::Color32::from_black_alpha(80),
                    track,
                );
                paint_tracked(ui.painter(), rect.center(), egui::Align2::CENTER_CENTER, &label, font.clone(), color, track);
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.tab = tab;
            }
            hit.on_hover_text(tab.label());
            x += widths[i];
        }
        let index = tabs.iter().position(|t| *t == self.tab).unwrap_or(0) as f32;
        let slide = ui.ctx().animate_value_with_time(egui::Id::new("nav-slide"), index, 0.2);
        let i0 = slide.floor() as usize;
        let i1 = (i0 + 1).min(rects.len().saturating_sub(1));
        if let (Some(a), Some(b)) = (rects.get(i0), rects.get(i1)) {
            let frac = slide - i0 as f32;
            let left = a.left() + (b.left() - a.left()) * frac + 10.0;
            let right = a.right() + (b.right() - a.right()) * frac - 10.0;
            let underline = egui::Rect::from_min_max(egui::pos2(left, a.bottom() - 3.0), egui::pos2(right, a.bottom() - 1.0));
            ui.painter().rect_filled(underline, 0.0, GOLD);
        }
        let sep_h = us(full, 52.0) * 0.44;
        let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("tab-seps")));
        let mut sx = bar.left();
        for (i, width) in widths.iter().enumerate() {
            if i > 0 {
                paint_tab_sep(&fg, sx, bar.center().y - sep_h * 0.5, sep_h);
            }
            sx += width;
        }
        bar
    }

    pub(crate) fn paint_player_bar(&mut self, ui: &mut egui::Ui, full: egui::Rect) -> egui::Rect {
        let name = if self.username.is_empty() { "Player".to_string() } else { self.username.clone() };
        let font = title_font(13.0);
        let text_w = ui.painter().layout_no_wrap(name.clone(), font.clone(), TEXT).size().x;
        let h = us(full, 40.0);
        let head_s = (h - 10.0).clamp(16.0, 20.0);
        let pad = 8.0;
        let bar = egui::Rect::from_min_size(
            egui::pos2(ux(full, 22.0), uy(full, 16.0)),
            egui::vec2(pad + head_s + 8.0 + text_w + 12.0, h),
        );
        paint_hytale_bar(ui.painter(), bar);
        let head = egui::Rect::from_center_size(
            egui::pos2(bar.left() + pad + head_s * 0.5, bar.center().y),
            egui::vec2(head_s, head_s),
        );
        let hit = ui.interact(bar, egui::Id::new("player-card"), Sense::click());
        if let Some(face) = &self.avatar {
            ui.painter().add(
                egui::epaint::RectShape::filled(head, 3.0, egui::Color32::WHITE)
                    .with_texture(face.id(), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))),
            );
        } else {
            ui.painter().rect_filled(head, 3.0, egui::Color32::from_rgb(0x1A, 0x14, 0x10));
        }
        ui.painter().text(
            egui::pos2(head.right() + 8.0, bar.center().y),
            egui::Align2::LEFT_CENTER,
            &name,
            font,
            egui::Color32::from_rgb(236, 228, 214),
        );
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if hit.clicked() {
            self.tab = Tab::Settings;
        }
        bar
    }

    /// Title mark, left of the window and above the version / play-time card.
    pub(crate) fn paint_exit(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        // 3px clear of the 4px rounded corner, on both axes.
        let h = 4.0;
        let corner = egui::pos2(full.right() - 4.0 - 3.0, full.top() + 4.0 + 3.0);
        let center = corner + egui::vec2(-h, h);
        let size = h * 2.0;
        let exit = egui::Rect::from_center_size(center, egui::vec2(16.0, 16.0));
        let hit = ui.interact(exit, egui::Id::new("exit-version"), Sense::click());
        paint_empty_cross(ui.painter(), center, size, empty_gold(hit.hovered()));
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if hit.clicked() {
            self.quit(ui.ctx());
        }
        hit.on_hover_text("Exit");
    }

}
