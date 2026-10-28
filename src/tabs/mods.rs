//! Mod tab. Installed files, and Explore on Modrinth.
use eframe::egui::{self, CursorIcon, RichText, Sense};
use super::super::*;

impl super::super::Desktop {
    pub(crate) fn paint_mods_page(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let content = spec_content(full);
        spec_panel(ui.painter(), content);
        let inner = content.shrink(14.0);
        let mode_y = inner.top();
        let mut x = inner.left() + 4.0;
        for (i, label) in ["Installed", "Explore"].iter().enumerate() {
            let on = self.mod_view as usize == i;
            let w = if on { chip_w(ui, label) + 8.0 } else { chip_w(ui, label) };
            let rect = egui::Rect::from_min_size(egui::pos2(x, mode_y), egui::vec2(w, 30.0));
            let hit = ui.interact(rect, egui::Id::new("mod-view").with(i), Sense::click());
            if on {
                ui.painter().rect_filled(rect, 8.0, egui::Color32::from_rgb(0x2C, 0x24, 0x12));
                ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, HY_ACCENT), egui::StrokeKind::Inside);
            }
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, *label, nav_font(13.0), if on { HY_ACCENT } else { HY_TEXT });
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.mod_view = i as u8;
                self.explore_ver_open = false;
                self.explore_open = None;
                if i == 1 {
                    self.explore_dirty = true;
                }
            }
            x += w + 6.0;
        }

        let cats = [("Modpack", I_LAYERS), ("Mods", '\u{E87B}'), ("Resource packs", I_FOLDER), ("Shader packs", I_FLARE)];
        let mut widths = Vec::new();
        for (label, _) in cats {
            widths.push(chip_w(ui, label) + 16.0);
        }
        let bar_w: f32 = widths.iter().sum();
        let cat_y = mode_y + 38.0;
        let cat_bar = egui::Rect::from_min_size(egui::pos2(inner.left(), cat_y), egui::vec2(bar_w, 34.0));
        paint_glass_chip(ui.painter(), cat_bar, 8.0, 200);
        let mut x = cat_bar.left();
        for (i, ((label, icon), w)) in cats.into_iter().zip(widths).enumerate() {
            let rect = egui::Rect::from_min_size(egui::pos2(x, cat_bar.top()), egui::vec2(w, cat_bar.height()));
            let on = self.pack_tab as usize == i;
            let hit = ui.interact(rect, egui::Id::new("pack-tab").with(i), Sense::click());
            if on {
                ui.painter().rect_filled(rect.shrink2(egui::vec2(3.0, 4.0)), 7.0, egui::Color32::from_rgb(0x2C, 0x24, 0x12));
            }
            let ink = if on { HY_ACCENT } else { HY_TEXT };
            paint_material(ui.painter(), egui::pos2(rect.left() + 16.0, rect.center().y), 14.0, icon, ink);
            ui.painter().text(egui::pos2(rect.left() + 28.0, rect.center().y), egui::Align2::LEFT_CENTER, label, nav_font(12.0), ink);
            if on {
                ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(rect.left() + 12.0, rect.bottom() - 3.0), egui::pos2(rect.right() - 8.0, rect.bottom() - 1.0)), 1.0, HY_ACCENT);
            }
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.pack_tab = i as u8;
                self.row_focus = 0;
                self.explore_ver_open = false;
                self.explore_open = None;
                if self.mod_view == 1 {
                    self.explore_dirty = true;
                }
            }
            x += w;
        }

        let list_top = if self.mod_view == 1 { cat_y + 92.0 } else { cat_y + 42.0 };
        let path_bar = egui::Rect::from_min_max(egui::pos2(inner.left(), inner.bottom() - 36.0), inner.right_bottom());
        let list_bottom = if self.mod_view == 1 { inner.bottom() } else { path_bar.top() - 8.0 };
        let list = egui::Rect::from_min_max(egui::pos2(inner.left(), list_top), egui::pos2(inner.right(), list_bottom));

        if self.mod_view == 1 {
            self.paint_explore_filters(ui, inner, cat_y + 34.0);
            self.paint_explore_list(ui, list);
            if self.explore_ver_open {
                self.paint_version_menu(ui, inner, cat_y + 34.0);
            }
        } else {
            self.paint_installed_list(ui, list);
            self.paint_installed_bar(ui, path_bar);
        }

        if self.mod_view == 1 && self.explore_dirty {
            self.explore_dirty = false;
            self.start_explore();
        }
    }

    fn paint_explore_filters(&mut self, ui: &mut egui::Ui, inner: egui::Rect, y: f32) {
        ui.painter().text(egui::pos2(inner.left(), y), egui::Align2::LEFT_TOP, "Minecraft Version", sub_font(11.0), HY_MUTED);
        ui.painter().text(egui::pos2(inner.left() + 132.0, y), egui::Align2::LEFT_TOP, "Loader", sub_font(11.0), HY_MUTED);
        let controls = y + 16.0;
        let ver = if self.explore_version.is_empty() { "26.3".to_string() } else { self.explore_version.clone() };
        let ver_rect = egui::Rect::from_min_size(egui::pos2(inner.left(), controls), egui::vec2(118.0, 32.0));
        paint_glass_chip(ui.painter(), ver_rect, 8.0, 210);
        ui.painter().text(egui::pos2(ver_rect.left() + 12.0, ver_rect.center().y), egui::Align2::LEFT_CENTER, &ver, body_font(13.0), HY_TEXT);
        paint_chevron(ui.painter(), egui::pos2(ver_rect.right() - 14.0, ver_rect.center().y), 4.0, HY_MUTED, true);
        let ver_hit = ui.interact(ver_rect, egui::Id::new("ex-ver-open"), Sense::click());
        if ver_hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if ver_hit.clicked() {
            self.explore_ver_open = !self.explore_ver_open;
            self.explore_ver_skip = self.explore_ver_open;
        }
        let mut x = inner.left() + 132.0;
        for loader in ["forge", "fabric", "neoforge"] {
            let label = match loader {
                "forge" => "Forge",
                "fabric" => "Fabric",
                _ => "NeoForge",
            };
            let w = chip_w(ui, label) + 10.0;
            let rect = egui::Rect::from_min_size(egui::pos2(x, controls), egui::vec2(w, 32.0));
            let on = self.explore_loader == loader;
            let hit = ui.interact(rect, egui::Id::new("ex-loader").with(loader), Sense::click());
            if on {
                ui.painter().rect_filled(rect, 8.0, egui::Color32::from_rgba_unmultiplied(0x1A, 0x2A, 0x48, 230));
                ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.2, HY_ACCENT), egui::StrokeKind::Inside);
            } else {
                paint_glass_chip(ui.painter(), rect, 8.0, 190);
            }
            let ink = if on { HY_ACCENT } else { HY_TEXT };
            let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 16.0, rect.center().y), egui::vec2(14.0, 14.0));
            if let Some(tex) = self.loader_icon_id(loader) {
                ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            } else if on {
                paint_tick(ui.painter(), icon.center(), 7.0, ink);
            }
            ui.painter().text(egui::pos2(rect.left() + 28.0, rect.center().y), egui::Align2::LEFT_CENTER, label, nav_font(12.0), ink);
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                self.explore_loader = loader.into();
                self.explore_ver_open = false;
                self.explore_dirty = true;
            }
            x += w + 6.0;
        }
        let search_btn = egui::Rect::from_min_size(egui::pos2(inner.right() - 96.0, controls), egui::vec2(96.0, 32.0));
        let field = egui::Rect::from_min_max(egui::pos2(x + 8.0, controls), egui::pos2(search_btn.left() - 8.0, controls + 32.0));
        if field.width() > 40.0 {
            paint_glass_chip(ui.painter(), field, 8.0, 200);
            paint_search_icon(ui.painter(), egui::pos2(field.left() + 16.0, field.center().y), 7.0, HY_MUTED);
            let mut edit = ui.new_child(egui::UiBuilder::new().max_rect(field.shrink2(egui::vec2(28.0, 4.0))));
            edit.set_clip_rect(field.shrink(2.0));
            let resp = edit.add(
                egui::TextEdit::singleline(&mut self.explore_query)
                    .id(egui::Id::new("explore-query"))
                    .hint_text("Search mods, authors, or keywords...")
                    .font(body_font(13.0))
                    .desired_width(field.width() - 36.0)
                    .frame(false),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.explore_dirty = true;
                self.explore_ver_open = false;
            }
        }
        if spec_button(ui, search_btn, "Search", true) {
            self.explore_dirty = true;
            self.explore_ver_open = false;
        }
    }

    fn paint_version_menu(&mut self, ui: &mut egui::Ui, inner: egui::Rect, y: f32) {
        let versions = self.explore_versions();
        let menu = egui::Rect::from_min_size(egui::pos2(inner.left(), y + 52.0), egui::vec2(168.0, 180.0));
        let closer = ui.interact(inner, egui::Id::new("ver-closer"), Sense::click());
        spec_panel(ui.painter(), menu);
        let mut pick = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(menu.shrink(6.0)));
        child.set_clip_rect(menu.shrink(4.0));
        egui::ScrollArea::vertical().id_salt("explore-versions").auto_shrink([false, false]).show_rows(&mut child, 28.0, versions.len(), |ui, range| {
            for i in range {
                let id = &versions[i];
                let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 26.0), Sense::hover());
                let on = id == &self.explore_version;
                if on {
                    ui.painter().rect_filled(rect, 1.5, HY_HIGH);
                }
                ui.painter().text(egui::pos2(rect.left() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, id, body_font(13.0), if on { SPEC_YELLOW } else { SPEC_TEXT });
                let hit = ui.interact(rect, egui::Id::new("ex-ver").with(id), Sense::click());
                if hit.clicked() {
                    pick = Some(id.clone());
                }
            }
        });
        if let Some(id) = pick {
            self.explore_version = id;
            self.explore_ver_open = false;
            self.explore_dirty = true;
        } else if closer.clicked() && !self.explore_ver_skip {
            self.explore_ver_open = false;
        }
        self.explore_ver_skip = false;
    }

    fn paint_explore_list(&mut self, ui: &mut egui::Ui, list: egui::Rect) {
        if self.explore_busy && self.explore_hits.is_empty() {
            ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, "Looking on Modrinth…", body_font(15.0), SPEC_MUTED);
            return;
        }
        if self.explore_hits.is_empty() {
            let note = if self.explore_note.is_empty() { "Nothing on Modrinth for this version.".to_string() } else { self.explore_note.clone() };
            ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, &ellipsize(ui.painter(), &note, body_font(14.0), list.width() - 24.0), body_font(14.0), SPEC_MUTED);
            return;
        }
        if !self.explore_note.is_empty() && !self.explore_busy {
            ui.painter().text(egui::pos2(list.left() + 4.0, list.top() + 8.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &self.explore_note, sub_font(12.0), list.width() - 8.0), sub_font(12.0), SPEC_YELLOW);
        }
        let hits = self.explore_hits.clone();
        let mut open = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
        child.set_clip_rect(list);
        egui::ScrollArea::vertical().id_salt("explore-hits").auto_shrink([false, false]).show_rows(&mut child, 76.0, hits.len(), |ui, range| {
            for i in range {
                let hit = &hits[i];
                let (rect, _) = ui.allocate_exact_size(egui::vec2(list.width(), 72.0), Sense::hover());
                let row = ui.interact(rect, egui::Id::new("ex-hit").with(&hit.id), Sense::click());
                let card = rect.shrink2(egui::vec2(0.0, 4.0));
                spec_card(ui.painter(), card, row.hovered() || i == 0);
                let icon = egui::Rect::from_center_size(egui::pos2(card.left() + 26.0, card.center().y), egui::vec2(40.0, 40.0));
                ui.painter().rect_filled(icon, 6.0, egui::Color32::from_rgba_unmultiplied(0x12, 0x1C, 0x34, 230));
                if let Some(tex) = self.explore_icons.get(&hit.id) {
                    ui.painter().image(tex.id(), icon.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                } else {
                    let letter = hit.title.chars().next().unwrap_or('?').to_string();
                    ui.painter().text(icon.center(), egui::Align2::CENTER_CENTER, &letter, title_font(14.0), SPEC_MUTED);
                }
                let ver = display_version(&hit.latest_version);
                let loader = loader_name(&self.explore_loader);
                let count = compact_count(hit.downloads);
                let chev_x = card.right() - 14.0;
                paint_chevron(ui.painter(), egui::pos2(chev_x, card.center().y), 5.0, HY_MUTED, false);
                let meta_x = chev_x - 14.0;
                ui.painter().text(egui::pos2(meta_x, card.center().y - 9.0), egui::Align2::RIGHT_CENTER, loader, sub_font(12.0), HY_TEXT);
                if !ver.is_empty() {
                    ui.painter().text(egui::pos2(meta_x, card.center().y + 9.0), egui::Align2::RIGHT_CENTER, &ver, sub_font(10.0), HY_MUTED);
                }
                let count_x = meta_x - 108.0;
                ui.painter().text(egui::pos2(count_x, card.center().y - 9.0), egui::Align2::RIGHT_CENTER, &count, title_font(12.0), HY_TEXT);
                ui.painter().text(egui::pos2(count_x, card.center().y + 8.0), egui::Align2::RIGHT_CENTER, "Downloads", sub_font(9.0), HY_MUTED);
                paint_material(ui.painter(), egui::pos2(count_x - ui.painter().layout_no_wrap(count.clone(), title_font(12.0), HY_TEXT).size().x - 12.0, card.center().y - 9.0), 12.0, I_DOWN, HY_MUTED);
                let text_right = count_x - 78.0;
                let text_x = icon.right() + 10.0;
                let text_w = (text_right - text_x).max(48.0);
                let title = ellipsize(ui.painter(), &hit.title, title_font(13.0), text_w * 0.7);
                ui.painter().text(egui::pos2(text_x, card.top() + 16.0), egui::Align2::LEFT_CENTER, &title, title_font(13.0), HY_TEXT);
                if let Some(tag) = hit.categories.first() {
                    let badge = pretty_tag(tag);
                    let bw = ui.painter().layout_no_wrap(badge.clone(), sub_font(9.0), HY_ACCENT).size().x + 12.0;
                    let title_w = ui.painter().layout_no_wrap(title.clone(), title_font(13.0), HY_TEXT).size().x;
                    let badge_r = egui::Rect::from_min_size(egui::pos2(text_x + title_w + 8.0, card.top() + 8.0), egui::vec2(bw, 15.0));
                    if badge_r.right() < text_right {
                        ui.painter().rect_filled(badge_r, 7.0, egui::Color32::from_rgba_unmultiplied(0xF2, 0xB8, 0x4B, 48));
                        ui.painter().text(badge_r.center(), egui::Align2::CENTER_CENTER, &badge, sub_font(9.0), HY_ACCENT);
                    }
                }
                let desc = ellipsize(ui.painter(), &hit.description, sub_font(11.0), text_w);
                ui.painter().text(egui::pos2(text_x, card.top() + 34.0), egui::Align2::LEFT_CENTER, &desc, sub_font(11.0), HY_MUTED);
                let mut tx = text_x;
                for tag in hit.categories.iter().take(3) {
                    let label = pretty_tag(tag);
                    let tw = ui.painter().layout_no_wrap(label.clone(), sub_font(9.0), HY_MUTED).size().x + 12.0;
                    if tx + tw > text_right {
                        break;
                    }
                    let chip = egui::Rect::from_min_size(egui::pos2(tx, card.top() + 46.0), egui::vec2(tw, 14.0));
                    ui.painter().rect_filled(chip, 6.0, egui::Color32::from_rgba_unmultiplied(0x1C, 0x30, 0x52, 230));
                    ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, &label, sub_font(9.0), HY_MUTED);
                    tx += tw + 4.0;
                }
                if row.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if row.clicked() {
                    open = Some(hit.clone());
                }
            }
        });
        if let Some(hit) = open {
            self.explore_ver_open = false;
            self.open_explore_hit(&hit);
        }
    }

    fn paint_installed_list(&mut self, ui: &mut egui::Ui, list: egui::Rect) {
        if self.pack_tab == 1 && (self.loader == "vanilla" || self.loader.is_empty()) {
            ui.painter().text(egui::pos2(list.left() + 8.0, list.top() + 8.0), egui::Align2::LEFT_TOP, "Mods need Forge, Fabric, or NeoForge.", sub_font(13.0), SPEC_YELLOW);
        }
        if self.mod_view == 0 {
            self.take_drops(ui);
        }
        let entries = self.pack_entries();
        if !entries.is_empty() && self.explore_open.is_none() {
            let n = entries.len();
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.row_focus = (self.row_focus + 1).min(n - 1);
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.row_focus = self.row_focus.saturating_sub(1);
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
            let focus = self.row_focus;
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
            child.set_clip_rect(list);
            egui::ScrollArea::vertical().id_salt("spec-packs").auto_shrink([false, false]).show_rows(&mut child, 44.0, entries.len(), |ui, range| {
                for i in range {
                    let (name, on, size) = &entries[i];
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(list.width(), 40.0), Sense::hover());
                    spec_card(ui.painter(), rect, i == focus);
                    let shown = ellipsize(ui.painter(), &display_pack_name(name), body_font(14.0), rect.width() - 210.0);
                    let ink = if *on { SPEC_TEXT } else { SPEC_MUTED };
                    let tick = egui::Rect::from_center_size(egui::pos2(rect.left() + 20.0, rect.center().y), egui::vec2(16.0, 16.0));
                    if self.pack_tab != 0 {
                        paint_check(ui.painter(), tick, *on);
                        let hit = ui.interact(tick.expand(6.0), egui::Id::new("mod-on").with(name), Sense::click());
                        if hit.clicked() {
                            toggled = Some(name.clone());
                        }
                    }
                    ui.painter().text(egui::pos2(rect.left() + 40.0, rect.center().y), egui::Align2::LEFT_CENTER, &shown, body_font(14.0), ink);
                    ui.painter().text(egui::pos2(rect.right() - 96.0, rect.center().y), egui::Align2::RIGHT_CENTER, &format_bytes(*size), sub_font(12.0), SPEC_MUTED);
                    let rm = egui::Rect::from_min_size(egui::pos2(rect.right() - 78.0, rect.top() + 6.0), egui::vec2(66.0, 28.0));
                    if spec_button(ui, rm, "Remove", false) {
                        removed = Some(name.clone());
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
    }

    fn paint_installed_bar(&mut self, ui: &mut egui::Ui, path_bar: egui::Rect) {
        let path = self.pack_dir(self.pack_folder());
        let add = egui::Rect::from_min_size(egui::pos2(path_bar.right() - 268.0, path_bar.top() + 4.0), egui::vec2(64.0, 28.0));
        let copy = egui::Rect::from_min_size(egui::pos2(path_bar.right() - 196.0, path_bar.top() + 4.0), egui::vec2(88.0, 28.0));
        let open = egui::Rect::from_min_size(egui::pos2(path_bar.right() - 100.0, path_bar.top() + 4.0), egui::vec2(96.0, 28.0));
        ui.painter().text(
            egui::pos2(path_bar.left() + 4.0, path_bar.center().y),
            egui::Align2::LEFT_CENTER,
            &ellipsize(ui.painter(), &path.display().to_string(), sub_font(12.0), (add.left() - path_bar.left() - 12.0).max(40.0)),
            sub_font(12.0),
            SPEC_MUTED,
        );
        if spec_button(ui, add, "Add", true) {
            self.pick_pack_file();
        }
        if spec_button(ui, copy, "Copy path", false) {
            ui.output_mut(|o| o.copied_text = path.display().to_string());
            self.notice = "Path copied".into();
        }
        if spec_button(ui, open, "Open folder", false) {
            let _ = std::fs::create_dir_all(&path);
            if let Err(e) = crate::dialog::open_dir(&path) {
                self.notice = e;
            }
        }
    }

    pub(crate) fn paint_explore_popup(&mut self, ui: &mut egui::Ui, full: egui::Rect) {
        let Some(detail) = self.explore_open.clone() else { return };
        ui.painter().rect_filled(full, 0.0, egui::Color32::from_black_alpha(140));
        let _ = ui.interact(full, egui::Id::new("explore-block"), Sense::click());
        let box_rect = egui::Rect::from_center_size(full.center(), egui::vec2(680.0, 430.0));
        spec_panel(ui.painter(), box_rect);
        let icon = egui::Rect::from_min_size(egui::pos2(box_rect.left() + 18.0, box_rect.top() + 16.0), egui::vec2(56.0, 56.0));
        let shot_key = format!("{}#shot", detail.id);
        let tex = self.explore_icons.get(&shot_key).or_else(|| self.explore_icons.get(&detail.id));
        if let Some(tex) = tex {
            let size = tex.size_vec2();
            let aspect = size.x / size.y.max(1.0);
            let w = (56.0 * aspect).clamp(56.0, 96.0);
            let shown = egui::Rect::from_min_size(icon.min, egui::vec2(w, 56.0));
            ui.painter().image(tex.id(), shown, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        } else {
            ui.painter().rect_filled(icon, 1.5, HY_HIGH);
        }
        let title_x = icon.right() + 14.0;
        ui.painter().text(egui::pos2(title_x, box_rect.top() + 32.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &detail.title, title_font(18.0), box_rect.right() - title_x - 36.0), title_font(18.0), SPEC_TEXT);
        let loader = match self.explore_loader.as_str() {
            "forge" => "Forge",
            "neoforge" => "NeoForge",
            _ => "Fabric",
        };
        let ver = if self.explore_version.is_empty() { self.selected.clone() } else { self.explore_version.clone() };
        ui.painter().text(egui::pos2(title_x, box_rect.top() + 54.0), egui::Align2::LEFT_CENTER, &format!("{loader} · {ver}"), sub_font(12.0), SPEC_MUTED);
        let cross = egui::pos2(box_rect.right() - 18.0, box_rect.top() + 18.0);
        let xhit = ui.interact(egui::Rect::from_center_size(cross, egui::vec2(16.0, 16.0)), egui::Id::new("explore-x"), Sense::click());
        paint_empty_cross(ui.painter(), cross, 8.0, empty_gold(xhit.hovered()));
        if xhit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        let warn = self.explore_warn();
        let body_bottom = if warn.is_empty() { box_rect.bottom() - 76.0 } else { box_rect.bottom() - 98.0 };
        let body = egui::Rect::from_min_max(egui::pos2(box_rect.left() + 18.0, box_rect.top() + 86.0), egui::pos2(box_rect.right() - 18.0, body_bottom));
        let mut toggle = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
        child.set_clip_rect(body);
        egui::ScrollArea::vertical().id_salt("explore-detail").auto_shrink([false, false]).show(&mut child, |ui| {
            ui.set_width(body.width() - 8.0);
            if !detail.ready {
                ui.label(RichText::new("Loading…").font(body_font(14.0)).color(SPEC_MUTED));
                return;
            }
            if !detail.missing.is_empty() {
                ui.label(RichText::new(&detail.missing).font(body_font(14.0)).color(SPEC_YELLOW));
                ui.add_space(8.0);
            }
            if !detail.description.is_empty() {
                ui.label(RichText::new(&detail.description).font(body_font(14.0)).color(SPEC_TEXT));
            }
            let required: Vec<_> = detail.deps.iter().filter(|d| d.kind == "required").cloned().collect();
            let optional: Vec<_> = detail.deps.iter().filter(|d| d.kind == "optional").cloned().collect();
            if !required.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new("These mods are required.").font(nav_font(13.0)).color(SPEC_YELLOW));
                ui.add_space(4.0);
                for dep in &required {
                    ui.label(RichText::new(&dep.title).font(body_font(14.0)).color(SPEC_TEXT));
                }
            }
            if !optional.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new("Optional. Choose which of these to install.").font(nav_font(13.0)).color(SPEC_TEXT));
                ui.add_space(4.0);
                for dep in &optional {
                    ui.add_space(4.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(body.width() - 12.0, 28.0), Sense::hover());
                    let on = self.explore_optional.iter().any(|id| id == &dep.project_id);
                    let tick = egui::Rect::from_center_size(egui::pos2(rect.left() + 10.0, rect.center().y), egui::vec2(16.0, 16.0));
                    paint_check(ui.painter(), tick, on);
                    ui.painter().text(egui::pos2(rect.left() + 28.0, rect.center().y), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &dep.title, body_font(14.0), rect.width() - 36.0), body_font(14.0), SPEC_TEXT);
                    let hit = ui.interact(rect, egui::Id::new("ex-opt").with(&dep.project_id), Sense::click());
                    if hit.clicked() {
                        toggle = Some(dep.project_id.clone());
                    }
                }
            }
            if !detail.body.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new(&detail.body).font(sub_font(13.0)).color(SPEC_MUTED));
            }
        });

        if !warn.is_empty() {
            ui.painter().text(egui::pos2(box_rect.left() + 18.0, box_rect.bottom() - 78.0), egui::Align2::LEFT_CENTER, &ellipsize(ui.painter(), &warn, sub_font(12.0), box_rect.width() - 36.0), sub_font(12.0), SPEC_YELLOW);
        }
        let cancel = egui::Rect::from_min_size(egui::pos2(box_rect.left() + 18.0, box_rect.bottom() - 50.0), egui::vec2(96.0, 34.0));
        let install = egui::Rect::from_min_size(egui::pos2(box_rect.right() - 132.0, box_rect.bottom() - 50.0), egui::vec2(114.0, 34.0));
        let can_install = detail.ready && !detail.file_url.is_empty() && !self.explore_installing;
        if spec_button(ui, cancel, "Close", false) || xhit.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.explore_open = None;
            return;
        }
        if self.explore_installing {
            paint_download_bar(ui.painter(), install, self.explore_progress);
        } else if can_install && spec_button(ui, install, "Install", true) {
            self.install_open_explore();
        } else if !can_install {
            spec_button(ui, install, "Install", false);
        }
        if let Some(id) = toggle {
            if let Some(i) = self.explore_optional.iter().position(|x| x == &id) {
                self.explore_optional.remove(i);
            } else {
                self.explore_optional.push(id);
            }
        }
    }

    fn explore_warn(&self) -> String {
        if self.pack_tab != 1 {
            return String::new();
        }
        let ver = if self.explore_version.is_empty() { self.selected.as_str() } else { self.explore_version.as_str() };
        let inst_loader = match self.loader.as_str() {
            "forge" | "forge+optifine" => "forge",
            "fabric" => "fabric",
            "neoforge" => "neoforge",
            _ => "vanilla",
        };
        let mut parts = Vec::new();
        if !self.selected.is_empty() && ver != self.selected {
            parts.push(format!("This file is for {ver}. This instance is {}.", self.selected));
        }
        if inst_loader == "vanilla" {
            parts.push("This instance has no loader. The file is saved, and will not run until you choose one.".into());
        } else if inst_loader != self.explore_loader {
            parts.push(format!("This file is for {}. This instance is {}.", loader_name(&self.explore_loader), loader_name(inst_loader)));
        }
        parts.join(" ")
    }

    fn explore_versions(&self) -> Vec<String> {
        let mut out = Vec::new();
        for v in &self.versions {
            if v.channel == "release" || (self.snapshots && v.channel == "snapshot") || (self.previews && v.channel == "preview") {
                out.push(v.id.clone());
            }
        }
        if out.is_empty() {
            out.push(if self.selected.is_empty() { "26.3".into() } else { self.selected.clone() });
        }
        out
    }

    fn start_explore(&mut self) {
        if self.explore_version.is_empty() {
            self.explore_version = if self.selected.is_empty() { "26.3".into() } else { self.selected.clone() };
        }
        if self.explore_loader.is_empty() {
            self.explore_loader = match self.loader.as_str() {
                "forge" | "forge+optifine" => "forge".into(),
                "neoforge" => "neoforge".into(),
                _ => "fabric".into(),
            };
        }
        self.explore_gen = self.explore_gen.wrapping_add(1);
        self.explore_detail_gen = self.explore_detail_gen.wrapping_add(1);
        self.explore_busy = true;
        self.explore_note = "Looking on Modrinth…".into();
        self.explore_hits.clear();
        self.explore_open = None;
        let gen = self.explore_gen;
        let kind = crate::explore::project_type(self.pack_tab).to_string();
        let version = self.explore_version.clone();
        let loader = self.explore_loader.clone();
        let query = self.explore_query.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || match crate::explore::search(&kind, &version, &loader, &query) {
            Ok(hits) => {
                let _ = tx.send(Msg::Explore(gen, hits));
            }
            Err(e) => {
                let _ = tx.send(Msg::ExploreErr(gen, e));
            }
        });
    }

    pub(crate) fn on_explore(&mut self, gen: u64, hits: Vec<crate::explore::Hit>) {
        if gen != self.explore_gen {
            return;
        }
        self.explore_busy = false;
        self.explore_note = if hits.is_empty() { "Nothing on Modrinth for this version.".into() } else { String::new() };
        let want: Vec<_> = hits
            .iter()
            .filter(|h| !h.icon_url.is_empty() && !self.explore_icons.contains_key(&h.id))
            .take(16)
            .map(|h| (h.id.clone(), h.icon_url.clone()))
            .collect();
        self.explore_hits = hits;
        if !want.is_empty() {
            self.fetch_icons(want);
        }
        if std::env::var("RUS_EXPLORE_POPUP").ok().as_deref() == Some("1") {
            if let Some(hit) = self.explore_hits.first().cloned() {
                self.open_explore_hit(&hit);
            }
        }
    }

    fn open_explore_hit(&mut self, hit: &crate::explore::Hit) {
        self.explore_detail_gen = self.explore_detail_gen.wrapping_add(1);
        self.explore_optional.clear();
        self.explore_installing = false;
        self.explore_open = Some(crate::explore::Detail {
            id: hit.id.clone(),
            title: hit.title.clone(),
            description: hit.description.clone(),
            body: String::new(),
            icon_url: hit.icon_url.clone(),
            gallery_url: String::new(),
            file_url: String::new(),
            file_name: String::new(),
            deps: Vec::new(),
            ready: false,
            missing: String::new(),
        });
        let gen = self.explore_detail_gen;
        let id = hit.id.clone();
        let version = self.explore_version.clone();
        let loader = self.explore_loader.clone();
        let kind = crate::explore::project_type(self.pack_tab).to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || match crate::explore::detail(&id, &version, &loader, &kind) {
            Ok(detail) => {
                let _ = tx.send(Msg::ExploreDetail(gen, detail));
            }
            Err(e) => {
                let _ = tx.send(Msg::ExploreDetailErr(gen, e));
            }
        });
    }

    pub(crate) fn on_explore_detail(&mut self, gen: u64, detail: crate::explore::Detail) {
        if gen != self.explore_detail_gen {
            return;
        }
        let id = detail.id.clone();
        let icon = detail.icon_url.clone();
        let gallery = detail.gallery_url.clone();
        self.explore_open = Some(detail);
        let mut want = Vec::new();
        if !icon.is_empty() && !self.explore_icons.contains_key(&id) {
            want.push((id.clone(), icon));
        }
        let shot = format!("{id}#shot");
        if !gallery.is_empty() && !self.explore_icons.contains_key(&shot) {
            want.push((shot, gallery));
        }
        if !want.is_empty() {
            self.fetch_icons(want);
        }
    }

    fn fetch_icons(&self, items: Vec<(String, String)>) {
        let tx = self.tx.clone();
        for (id, url) in items {
            if !url.starts_with("https://") {
                continue;
            }
            let tx = tx.clone();
            std::thread::spawn(move || {
                if let Ok(bytes) = crate::net::get_bytes(&url) {
                    let _ = tx.send(Msg::ExploreIcon { id, bytes });
                }
            });
        }
    }

    fn install_open_explore(&mut self) {
        let Some(detail) = self.explore_open.clone() else { return };
        if self.explore_installing || detail.file_url.is_empty() {
            return;
        }
        if self.job_state().as_deref() == Some("running") {
            self.notice = "A download is already running".into();
            return;
        }
        let optional = self.explore_optional.clone();
        let dest = self.pack_dir(self.pack_folder());
        let with_deps = self.pack_tab == 1;
        let as_pack = self.pack_tab == 0;
        let tx = self.tx.clone();
        self.download_cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        let cancel = self.download_cancel.clone();
        if as_pack {
            self.explore_open = None;
            let job = self.track_job(&detail.title);
            std::thread::spawn(move || {
                let job_prog = job.clone();
                let outcome = crate::explore::install(&detail, &optional, &dest, with_deps, as_pack, &cancel, |frac| {
                    if let Ok(mut j) = job_prog.lock() {
                        if !j.canceled() {
                            j.progress = frac as f64;
                        }
                    }
                });
                let stopped = cancel.load(std::sync::atomic::Ordering::Relaxed) || outcome.note == "Download stopped";
                if let Ok(mut j) = job.lock() {
                    if stopped || j.canceled() {
                        j.state = "canceled".into();
                        j.message = "Download stopped".into();
                    } else {
                        j.state = "done".into();
                        j.progress = 1.0;
                        j.message = "Download complete".into();
                    }
                }
                if !stopped {
                    let _ = tx.send(Msg::ExploreDone(outcome));
                }
            });
            return;
        }
        self.explore_installing = true;
        self.explore_progress = 0.0;
        std::thread::spawn(move || {
            let tx_prog = tx.clone();
            let mut last = -1.0f32;
            let outcome = crate::explore::install(&detail, &optional, &dest, with_deps, as_pack, &cancel, |frac| {
                if frac - last >= 0.01 || frac >= 1.0 {
                    last = frac;
                    let _ = tx_prog.send(Msg::ExploreProgress(frac));
                }
            });
            let _ = tx.send(Msg::ExploreDone(outcome));
        });
    }

    pub(crate) fn on_explore_done(&mut self, outcome: crate::explore::Outcome) {
        self.explore_installing = false;
        self.notice = outcome.note.clone();
        self.explore_note = outcome.note;
        if outcome.pack.is_some() {
            self.explore_open = None;
            self.pending_pack = outcome.pack;
        }
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

fn chip_w(ui: &egui::Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.to_string(), nav_font(13.0), SPEC_TEXT).size().x + 22.0
}

fn tab_chip(ui: &mut egui::Ui, rect: egui::Rect, label: &str, on: bool, id: egui::Id) -> bool {
    let hit = ui.interact(rect, id, Sense::click());
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, nav_font(13.0), if on { SPEC_YELLOW } else { SPEC_TEXT });
    if on {
        ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(rect.left() + 4.0, rect.bottom() - 3.0), egui::pos2(rect.right() - 4.0, rect.bottom())), 1.0, SPEC_YELLOW);
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    hit.clicked()
}

fn compact_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f32 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn display_version(raw: &str) -> String {
    let v = raw.trim().trim_start_matches('v');
    let dotted = v.chars().filter(|c| *c == '.').count() >= 1;
    let numeric = v.chars().any(|c| c.is_ascii_digit());
    let clean = v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-' | '_'));
    if dotted && numeric && clean && v.len() <= 22 {
        format!("v{v}")
    } else {
        String::new()
    }
}

fn pretty_tag(tag: &str) -> String {
    let mut chars = tag.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn loader_name(id: &str) -> &'static str {
    match id {
        "forge" => "Forge",
        "fabric" => "Fabric",
        "neoforge" => "NeoForge",
        _ => "Vanilla",
    }
}
