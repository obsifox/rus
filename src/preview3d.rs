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
            let size = egui::vec2(220.0, 240.0);
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
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
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0x3E, 0x6C, 0xA4)),
                egui::StrokeKind::Inside,
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if view_button(ui, "Front") { self.yaw = 0.0; self.pitch = 0.12; self.dirty = true; }
                if view_button(ui, "Back") { self.yaw = std::f32::consts::PI; self.pitch = 0.12; self.dirty = true; }
                if view_button(ui, "Side") { self.yaw = std::f32::consts::FRAC_PI_2; self.pitch = 0.12; self.dirty = true; }
            });
            ui.add_space(4.0);
            toggle(ui, &mut self.show_cape, "Cape", &mut self.dirty);
            toggle(ui, &mut self.show_overlay, "Overlay layers", &mut self.dirty);
            toggle(ui, &mut self.auto_rotate, "Auto-rotate", &mut self.dirty);
            toggle(ui, &mut self.walk, "Walk", &mut self.dirty);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (model, label) in [
                    (ArmModel::Auto, "Auto"),
                    (ArmModel::Classic, "Classic"),
                    (ArmModel::Slim, "Slim"),
                ] {
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
    ui.add(egui::Button::new(label).min_size(egui::vec2(64.0, 28.0))).clicked()
}

fn model_button(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let mut b = egui::Button::new(label).min_size(egui::vec2(64.0, 28.0));
    if on {
        b = b.fill(egui::Color32::from_rgb(0x3B, 0x82, 0xF6));
    }
    ui.add(b).clicked()
}

fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str, dirty: &mut bool) {
    if ui.checkbox(value, label).changed() {
        *dirty = true;
    }
}
