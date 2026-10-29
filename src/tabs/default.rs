//! Shared tab chrome: player bar, nav, exit, and the panel frame.
use eframe::egui::{self, CursorIcon, RichText, Sense, ViewportCommand};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn page_in_panel(&mut self, ui: &mut egui::Ui, full: egui::Rect, page: Page) {
        let panel = spec_content(full);
        paint_glass_panel(ui.painter(), panel, 12.0);
        let inner = panel.shrink(us(full, 14.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        style_page_widgets(&mut child);
        child.set_clip_rect(inner);
        match page {
            Page::Mods => self.page_mods(&mut child),
            Page::Settings => {}
        }
    }

    /// Same icon buttons as the rail, so every page can move without the old word bar.
    pub(crate) fn paint_page_icons(&mut self, ui: &mut egui::Ui) {
        let tabs = [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings];
        let size = 32.0;
        let gap = 6.0;
        let x0 = HOME_RAIL + 10.0;
        let y0 = 12.0;
        for (i, tab) in tabs.into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x0 + i as f32 * (size + gap), y0), egui::vec2(size, size));
            let on = self.tab == tab;
            let hit = ui.interact(rect, egui::Id::new("page-icon").with(tab.label()), Sense::click());
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
            paint_tab_glyph(ui.painter(), tab, rect.center(), 15.0, if on { GOLD } else { HY_TEXT });
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() && self.tab != tab {
                self.tab = tab;
                crate::sfx::toggle();
            }
            hit.on_hover_text(tab.label());
        }
    }

    pub(crate) fn paint_shared_tabs(&mut self, ui: &mut egui::Ui, full: egui::Rect, left: f32) -> egui::Rect {
        let tabs = [Tab::Home, Tab::Versions, Tab::Mods, Tab::Settings];
        let h = 46.0;
        let font = nav_font(12.0);
        let mut widths = Vec::with_capacity(tabs.len());
        for tab in tabs {
            let label = tab.nav_label();
            widths.push(22.0 + 16.0 + tracked_width(ui.painter(), label, font.clone(), 0.6) + 16.0);
        }
        let total: f32 = widths.iter().sum();
        let bar = egui::Rect::from_min_size(egui::pos2(left, 14.0), egui::vec2(total, h));
        paint_glass_chip(ui.painter(), bar, 10.0, 236);
        let mut x = bar.left();
        let mut rects = Vec::with_capacity(tabs.len());
        for (i, tab) in tabs.into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(widths[i], h));
            rects.push(rect);
            let hit = ui.interact(rect, egui::Id::new("hynav").with(tab.label()), Sense::click());
            let on = self.tab == tab;
            let active_t = ease_out(ui.ctx().animate_bool_with_time(egui::Id::new("nav-on").with(tab.label()), on, 0.16));
            let hover_t = ui.ctx().animate_bool_with_time(egui::Id::new("nav-hov").with(tab.label()), hit.hovered(), 0.1);
            let color = lerp_color(HY_TEXT, HY_ACCENT, active_t.max(hover_t * 0.35));
            let icon_at = egui::pos2(rect.left() + 20.0, rect.center().y);
            paint_tab_glyph(ui.painter(), tab, icon_at, 15.0, color);
            paint_tracked(
                ui.painter(),
                egui::pos2(icon_at.x + 14.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                tab.nav_label(),
                font.clone(),
                color,
                0.6,
            );
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
        let slide = ui.ctx().animate_value_with_time(egui::Id::new("nav-slide"), index, 0.18);
        let i0 = slide.floor() as usize;
        let i1 = (i0 + 1).min(rects.len().saturating_sub(1));
        if let (Some(a), Some(b)) = (rects.get(i0), rects.get(i1)) {
            let frac = slide - i0 as f32;
            let left = a.left() + (b.left() - a.left()) * frac + 14.0;
            let right = a.right() + (b.right() - a.right()) * frac - 14.0;
            let underline = egui::Rect::from_min_max(egui::pos2(left, a.bottom() - 4.0), egui::pos2(right, a.bottom() - 1.0));
            ui.painter().rect_filled(underline, 1.0, GOLD);
        }
        let _ = full;
        bar
    }

    pub(crate) fn paint_player_bar(&mut self, ui: &mut egui::Ui, full: egui::Rect) -> egui::Rect {
        let name = if self.username.is_empty() { "Player".to_string() } else { self.username.clone() };
        let sub = match self.loader.as_str() {
            "forge" | "forge+optifine" => "Forge",
            "fabric" => "Fabric",
            "neoforge" => "NeoForge",
            "optifine" => "OptiFine",
            _ => "Vanilla",
        };
        let name_font = title_font(13.0);
        let sub_f = sub_font(10.0);
        let name_w = ui.painter().layout_no_wrap(name.clone(), name_font.clone(), TEXT).size().x;
        let sub_w = ui.painter().layout_no_wrap(sub.to_string(), sub_f.clone(), HY_MUTED).size().x;
        let h = 46.0;
        let head_s = 28.0;
        let text_w = name_w.max(sub_w);
        let bar = egui::Rect::from_min_size(egui::pos2(16.0, 14.0), egui::vec2(12.0 + head_s + 8.0 + text_w + 28.0, h));
        paint_glass_chip(ui.painter(), bar, 10.0, 236);
        let head = egui::Rect::from_center_size(egui::pos2(bar.left() + 8.0 + head_s * 0.5, bar.center().y), egui::vec2(head_s, head_s));
        let hit = ui.interact(bar, egui::Id::new("player-card"), Sense::click());
        if let Some(face) = &self.avatar {
            ui.painter().add(
                egui::epaint::RectShape::filled(head, 3.0, egui::Color32::WHITE)
                    .with_texture(face.id(), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))),
            );
        } else {
            ui.painter().rect_filled(head, 3.0, egui::Color32::from_rgb(0x1A, 0x14, 0x10));
        }
        let text_x = head.right() + 8.0;
        ui.painter().text(egui::pos2(text_x, bar.center().y - 7.0), egui::Align2::LEFT_CENTER, &name, name_font, egui::Color32::WHITE);
        ui.painter().text(egui::pos2(text_x, bar.center().y + 8.0), egui::Align2::LEFT_CENTER, sub, sub_f, HY_MUTED);
        paint_chevron(ui.painter(), egui::pos2(bar.right() - 14.0, bar.center().y), 4.0, HY_MUTED, true);
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if hit.clicked() {
            self.tab = Tab::Settings;
        }
        let _ = full;
        bar
    }

    /// Title mark, left of the window and above the version / play-time card.
    pub(crate) fn paint_exit(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let center = egui::pos2(full.right() - 16.0, full.top() + 16.0);
        let size = 9.0;
        let exit = egui::Rect::from_center_size(center, egui::vec2(18.0, 18.0));
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
