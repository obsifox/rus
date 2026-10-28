//! Interactive skin and cape preview. Renders on the CPU only while something changes.

use crate::raster::{self, View};
use crate::skinmodel::ArmModel;
use eframe::egui::{self, Sense};
use image::RgbaImage;

pub struct SkinPreview {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub auto_rotate: bool,
    pub show_cape: bool,
    pub show_overlay: bool,
    pub walk: bool,
    pub arms: ArmModel,
    walk_phase: f32,
    texture: Option<egui::TextureHandle>,
    dirty: bool,
    drag: Option<egui::Pos2>,
}

impl Default for SkinPreview {
    fn default() -> Self {
        Self {
            yaw: 0.35,
            pitch: 0.18,
            zoom: 1.0,
            auto_rotate: false,
            show_cape: true,
            show_overlay: true,
            walk: false,
            arms: ArmModel::Auto,
            walk_phase: 0.0,
            texture: None,
            dirty: true,
            drag: None,
        }
    }
}

impl SkinPreview {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub fn reset_view(&mut self) {
        self.yaw = 0.35;
        self.pitch = 0.18;
        self.zoom = 1.0;
        self.dirty = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, skin: &RgbaImage, cape: Option<&RgbaImage>, detected_slim: bool) {
        let slim = match self.arms {
            ArmModel::Classic => false,
            ArmModel::Slim => true,
            ArmModel::Auto => detected_slim,
        };
        ui.vertical(|ui| {
            ui.set_width(148.0);
            ui.set_max_width(148.0);
            let size = egui::vec2(148.0, 132.0);
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
            paint_preview_template(ui.painter(), rect);
            let anim = self.auto_rotate || self.walk;
            if anim {
                let dt = ui.input(|i| i.stable_dt).min(0.05);
                if self.auto_rotate {
                    self.yaw += dt * 0.7;
                }
                if self.walk {
                    self.walk_phase += dt * 3.2;
                }
                self.dirty = true;
                ui.ctx().request_repaint();
            }
            if resp.drag_started() {
                self.drag = resp.interact_pointer_pos();
            }
            if resp.dragged() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    if let Some(prev) = self.drag {
                        self.yaw += (pos.x - prev.x) * 0.01;
                        self.pitch = (self.pitch - (pos.y - prev.y) * 0.01).clamp(-0.8, 0.9);
                        self.drag = Some(pos);
                        self.dirty = true;
                    }
                }
            } else {
                self.drag = None;
            }
            if resp.double_clicked() {
                self.reset_view();
            }
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if resp.hovered() && scroll.abs() > 0.0 {
                self.zoom = (self.zoom + scroll * 0.002).clamp(0.55, 1.8);
                self.dirty = true;
            }
            if resp.hovered() {
                let keys = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowLeft),
                        i.key_pressed(egui::Key::ArrowRight),
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                        i.key_pressed(egui::Key::Minus),
                    )
                });
                if keys.0 { self.yaw -= 0.18; self.dirty = true; }
                if keys.1 { self.yaw += 0.18; self.dirty = true; }
                if keys.2 { self.pitch = (self.pitch + 0.08).min(0.9); self.dirty = true; }
                if keys.3 { self.pitch = (self.pitch - 0.08).max(-0.8); self.dirty = true; }
                if keys.4 { self.zoom = (self.zoom + 0.08).min(1.8); self.dirty = true; }
                if keys.5 { self.zoom = (self.zoom - 0.08).max(0.55); self.dirty = true; }
            }
            if self.dirty || self.texture.is_none() {
                let ppp = ui.ctx().pixels_per_point().clamp(1.0, 2.0);
                let frame = raster::render(
                    skin,
                    cape,
                    &View {
                        yaw: self.yaw,
                        pitch: self.pitch,
                        zoom: self.zoom,
                        walk: if self.walk { self.walk_phase } else { 0.0 },
                        slim,
                        cape: self.show_cape,
                        overlay: self.show_overlay,
                        width: (size.x * ppp) as u32,
                        height: (size.y * ppp) as u32,
                    },
                );
                let color = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width() as usize, frame.height() as usize],
                    frame.as_raw(),
                );
                self.texture = Some(ui.ctx().load_texture("skin-preview", color, egui::TextureOptions::NEAREST));
                self.dirty = false;
            }
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if view_button(ui, "Front") { self.yaw = 0.0; self.pitch = 0.12; self.dirty = true; }
                gold_tick(ui);
                if view_button(ui, "Back") { self.yaw = std::f32::consts::PI; self.pitch = 0.12; self.dirty = true; }
                gold_tick(ui);
                if view_button(ui, "Side") { self.yaw = std::f32::consts::FRAC_PI_2; self.pitch = 0.12; self.dirty = true; }
            });
            ui.add_space(2.0);
            toggle(ui, &mut self.show_cape, "Cape", &mut self.dirty);
            toggle(ui, &mut self.show_overlay, "Overlay layers", &mut self.dirty);
            toggle(ui, &mut self.auto_rotate, "Auto-rotate", &mut self.dirty);
            toggle(ui, &mut self.walk, "Walk", &mut self.dirty);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (i, (model, label)) in [
                    (ArmModel::Auto, "Auto"),
                    (ArmModel::Classic, "Classic"),
                    (ArmModel::Slim, "Slim"),
                ].into_iter().enumerate() {
                    if i > 0 {
                        gold_tick(ui);
                    }
                    let on = self.arms == model;
                    if model_button(ui, label, on) {
                        self.arms = model;
                        self.dirty = true;
                    }
                }
            });
        });
    }
}

fn view_button(ui: &mut egui::Ui, label: &str) -> bool {
    text_hit(ui, label, false)
}

fn model_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    text_hit(ui, label, on)
}

fn text_hit(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(52.0, 24.0), egui::Sense::click());
    paint_preview_template(ui.painter(), rect);
    let ink = if on {
        egui::Color32::from_rgb(0xF4, 0xCA, 0x54)
    } else {
        egui::Color32::WHITE
    };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), ink);
    resp.clicked()
}

fn gold_tick(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 22.0), egui::Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.top() + 4.0..=rect.bottom() - 4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(0xF4, 0xCA, 0x54)),
    );
}

fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str, dirty: &mut bool) {
    let font = egui::FontId::proportional(12.0);
    let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), egui::Color32::WHITE).size().x;
    let w = (text_w + 16.0).min(ui.available_width().max(48.0));
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 22.0), egui::Sense::click());
    paint_preview_template(ui.painter(), rect);
    let ink = if *value {
        egui::Color32::from_rgb(0xF4, 0xCA, 0x54)
    } else {
        egui::Color32::WHITE
    };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, font, ink);
    if resp.clicked() {
        *value = !*value;
        *dirty = true;
    }
}

fn paint_preview_template(painter: &egui::Painter, rect: egui::Rect) {
    let fill = egui::Color32::from_rgba_unmultiplied(36, 62, 112, 226);
    let edge = egui::Color32::from_rgb(196, 214, 232);
    painter.rect_filled(rect, 1.5, fill);
    painter.rect_stroke(rect, 1.5, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    let stroke = egui::Stroke::new(1.15, edge);
    let leg = 5.0;
    let inset = 1.5;
    let corners = [
        (rect.left() + inset, rect.top() + inset, 1.0, 1.0),
        (rect.right() - inset, rect.top() + inset, -1.0, 1.0),
        (rect.left() + inset, rect.bottom() - inset, 1.0, -1.0),
        (rect.right() - inset, rect.bottom() - inset, -1.0, -1.0),
    ];
    for (x, y, sx, sy) in corners {
        painter.line_segment([egui::pos2(x, y), egui::pos2(x + sx * leg, y)], stroke);
        painter.line_segment([egui::pos2(x, y), egui::pos2(x, y + sy * leg)], stroke);
    }
}
