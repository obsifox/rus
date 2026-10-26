//! Small CPU rasterizer for the skin preview. Nearest sampling, depth buffer.

use crate::skinmodel::{self, BoxPart};
use image::{Rgba, RgbaImage};

pub struct View {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub walk: f32,
    pub slim: bool,
    pub cape: bool,
    pub overlay: bool,
    pub width: u32,
    pub height: u32,
}

struct Tri {
    p: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
    light: f32,
}

pub fn render(skin: &RgbaImage, cape: Option<&RgbaImage>, view: &View) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(view.width.max(1), view.height.max(1), Rgba([11, 13, 10, 255]));
    paint_shadow(&mut img);
    let mut depth = vec![f32::INFINITY; (img.width() * img.height()) as usize];
    let parts = skinmodel::player_parts(view.slim, view.walk, view.cape && cape.is_some());
    let (eye, right, up, forward) = camera(view);
    for part in &parts {
        let tex = if part.uv == [0, 0] && part.size[0] > 9.0 {
            cape.unwrap_or(skin)
        } else {
            skin
        };
        draw_part(&mut img, &mut depth, tex, part, false, view, eye, right, up, forward);
        if view.overlay {
            if let Some(uv) = part.overlay {
                let mut over = *part;
                over.uv = uv;
                over.size = [
                    part.size[0] + part.inflate * 2.0,
                    part.size[1] + part.inflate * 2.0,
                    part.size[2] + part.inflate * 2.0,
                ];
                draw_part(&mut img, &mut depth, skin, &over, true, view, eye, right, up, forward);
            }
        }
    }
    img
}

fn paint_shadow(img: &mut RgbaImage) {
    let w = img.width() as f32;
    let h = img.height() as f32;
    let cx = w * 0.5;
    let cy = h * 0.80;
    let rx = w * 0.18;
    let ry = h * 0.035;
    let x0 = (cx - rx) as i32;
    let x1 = (cx + rx) as i32;
    let y0 = (cy - ry) as i32;
    let y1 = (cy + ry) as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
                continue;
            }
            let nx = (x as f32 - cx) / rx;
            let ny = (y as f32 - cy) / ry;
            let d = nx * nx + ny * ny;
            if d <= 1.0 {
                let a = ((1.0 - d) * 90.0) as u8;
                let p = img.get_pixel_mut(x as u32, y as u32);
                p[0] = p[0].saturating_sub(a / 3);
                p[1] = p[1].saturating_sub(a / 3);
                p[2] = p[2].saturating_sub(a / 3);
            }
        }
    }
}

fn draw_part(
    img: &mut RgbaImage,
    depth: &mut [f32],
    tex: &RgbaImage,
    part: &BoxPart,
    overlay: bool,
    view: &View,
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
) {
    let faces = faces(part);
    for face in faces {
        let mut tri_a = [0, 1, 2];
        let mut tri_b = [0, 2, 3];
        let _ = (&mut tri_a, &mut tri_b);
        for idx in [[0, 1, 2], [0, 2, 3]] {
            let tri = Tri {
                p: [face.p[idx[0]], face.p[idx[1]], face.p[idx[2]]],
                uv: [face.uv[idx[0]], face.uv[idx[1]], face.uv[idx[2]]],
                light: face.light,
            };
            raster(img, depth, tex, &tri, overlay, view, eye, right, up, forward);
        }
    }
}

struct Face {
    p: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    light: f32,
}

fn faces(part: &BoxPart) -> Vec<Face> {
    let (w, h, d) = (part.size[0], part.size[1], part.size[2]);
    let (u, v) = (part.uv[0] as f32, part.uv[1] as f32);
    // Official box unfold. Front is +Z (toward the camera at yaw 0).
    // Back UV is what the game shows on a cape from behind, at (12, 1) for a 10×16×1 cape.
    let specs = [
        ([0.0, 1.0, 0.0], [u + d, v], [w, d], 1.0),             // top
        ([0.0, -1.0, 0.0], [u + d + w, v], [w, d], 0.5),         // bottom
        ([-1.0, 0.0, 0.0], [u, v + d], [d, h], 0.6),             // character right
        ([1.0, 0.0, 0.0], [u + d + w, v + d], [d, h], 0.6),      // character left
        ([0.0, 0.0, 1.0], [u + d, v + d], [w, h], 0.8),          // front
        ([0.0, 0.0, -1.0], [u + d + w + d, v + d], [w, h], 0.8), // back
    ];
    let mut out = Vec::new();
    for (normal, origin, size, light) in specs {
        let (nw, nh) = (size[0], size[1]);
        let corners = face_corners(normal, w, h, d);
        let mut p = [[0.0; 3]; 4];
        let mut uv = [[0.0; 2]; 4];
        for i in 0..4 {
            p[i] = place(part, corners[i]);
            uv[i] = uv_of(i, origin, nw, nh);
        }
        out.push(Face { p, uv, light });
    }
    out
}

fn uv_of(i: usize, origin: [f32; 2], w: f32, h: f32) -> [f32; 2] {
    // 0 top-left, 1 top-right, 2 bottom-right, 3 bottom-left. Image v grows down.
    match i {
        0 => origin,
        1 => [origin[0] + w, origin[1]],
        2 => [origin[0] + w, origin[1] + h],
        _ => [origin[0], origin[1] + h],
    }
}

fn face_corners(normal: [f32; 3], w: f32, h: f32, d: f32) -> [[f32; 3]; 4] {
    let (hw, hh, hd) = (w * 0.5, h * 0.5, d * 0.5);
    // Order must match uv_of: top-left, top-right, bottom-right, bottom-left
    // as seen from outside the box. Texture left is character right on the front.
    if normal[1] > 0.0 {
        return [
            [-hw, hh, hd],
            [hw, hh, hd],
            [hw, hh, -hd],
            [-hw, hh, -hd],
        ];
    }
    if normal[1] < 0.0 {
        return [
            [-hw, -hh, -hd],
            [hw, -hh, -hd],
            [hw, -hh, hd],
            [-hw, -hh, hd],
        ];
    }
    if normal[0] < 0.0 {
        return [
            [-hw, hh, -hd],
            [-hw, hh, hd],
            [-hw, -hh, hd],
            [-hw, -hh, -hd],
        ];
    }
    if normal[0] > 0.0 {
        return [
            [hw, hh, hd],
            [hw, hh, -hd],
            [hw, -hh, -hd],
            [hw, -hh, hd],
        ];
    }
    if normal[2] > 0.0 {
        return [
            [-hw, hh, hd],
            [hw, hh, hd],
            [hw, -hh, hd],
            [-hw, -hh, hd],
        ];
    }
    [
        [hw, hh, -hd],
        [-hw, hh, -hd],
        [-hw, -hh, -hd],
        [hw, -hh, -hd],
    ]
}

fn place(part: &BoxPart, local: [f32; 3]) -> [f32; 3] {
    let mut p = local;
    if part.pivot_top {
        p[1] -= part.size[1] * 0.5;
        p = rot_x(p, part.rot_x);
        p[1] += part.size[1] * 0.5;
    }
    [p[0] + part.center[0], p[1] + part.center[1], p[2] + part.center[2]]
}

fn rot_x(p: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c]
}

fn camera(view: &View) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let dist = 96.0 / view.zoom.clamp(0.55, 1.8);
    let target = [0.0, 15.0, 0.0];
    let eye = [
        target[0] + dist * view.yaw.sin() * view.pitch.cos(),
        target[1] + dist * view.pitch.sin(),
        target[2] + dist * view.yaw.cos() * view.pitch.cos(),
    ];
    let forward = norm(sub(target, eye));
    let right = norm(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    (eye, right, up, forward)
}

fn raster(
    img: &mut RgbaImage,
    depth: &mut [f32],
    tex: &RgbaImage,
    tri: &Tri,
    overlay: bool,
    _view: &View,
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
) {
    let w = img.width() as f32;
    let h = img.height() as f32;
    let fov = 28.0_f32.to_radians();
    let f = 1.0 / (fov * 0.5).tan();
    let aspect = w / h;
    let mut s = [[0.0; 3]; 3];
    let mut inv_z = [0.0; 3];
    for i in 0..3 {
        let rel = sub(tri.p[i], eye);
        let cam = [dot(rel, right), dot(rel, up), dot(rel, forward)];
        if cam[2] < 0.2 {
            return;
        }
        inv_z[i] = 1.0 / cam[2];
        s[i] = [
            (cam[0] * inv_z[i]) * f / aspect * (w * 0.5) + w * 0.5,
            -(cam[1] * inv_z[i]) * f * (h * 0.5) + h * 0.5,
            cam[2],
        ];
    }
    let min_x = s.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min).floor().max(0.0) as i32;
    let max_x = s.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max).ceil().min(w - 1.0) as i32;
    let min_y = s.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min).floor().max(0.0) as i32;
    let max_y = s.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max).ceil().min(h - 1.0) as i32;
    let area = edge(s[0], s[1], s[2]);
    if area.abs() < 0.5 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = [x as f32 + 0.5, y as f32 + 0.5, 0.0];
            let w0 = edge(s[1], s[2], p) / area;
            let w1 = edge(s[2], s[0], p) / area;
            let w2 = edge(s[0], s[1], p) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let iz = w0 * inv_z[0] + w1 * inv_z[1] + w2 * inv_z[2];
            if iz <= 0.0 {
                continue;
            }
            let z = 1.0 / iz;
            let idx = (y as u32 * img.width() + x as u32) as usize;
            let bias = if overlay { -0.02 } else { 0.0 };
            if z + bias >= depth[idx] {
                continue;
            }
            let u = (w0 * tri.uv[0][0] * inv_z[0] + w1 * tri.uv[1][0] * inv_z[1] + w2 * tri.uv[2][0] * inv_z[2]) * z;
            let v = (w0 * tri.uv[0][1] * inv_z[0] + w1 * tri.uv[1][1] * inv_z[1] + w2 * tri.uv[2][1] * inv_z[2]) * z;
            let sample = nearest(tex, u, v);
            if sample[3] < 16 {
                continue;
            }
            depth[idx] = z + bias;
            let lit = |c: u8| ((c as f32) * tri.light).clamp(0.0, 255.0) as u8;
            img.put_pixel(x as u32, y as u32, Rgba([lit(sample[0]), lit(sample[1]), lit(sample[2]), 255]));
        }
    }
}

fn nearest(tex: &RgbaImage, u: f32, v: f32) -> Rgba<u8> {
    if tex.width() == 0 || tex.height() == 0 {
        return Rgba([0, 0, 0, 0]);
    }
    let x = u.floor() as i32;
    let y = v.floor() as i32;
    if x < 0 || y < 0 || x >= tex.width() as i32 || y >= tex.height() as i32 {
        return Rgba([0, 0, 0, 0]);
    }
    *tex.get_pixel(x as u32, y as u32)
}

fn edge(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.0001);
    [v[0] / l, v[1] / l, v[2] / l]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(yaw: f32) -> View {
        View {
            yaw,
            pitch: 0.15,
            zoom: 1.0,
            walk: 0.0,
            slim: false,
            cape: true,
            overlay: true,
            width: 260,
            height: 380,
        }
    }

    #[test]
    fn steve_views() {
        let skin = image::load_from_memory(include_bytes!("../assets/steve.png"))
            .unwrap()
            .to_rgba8();
        let skin = crate::skinmodel::prepare_skin(&skin).unwrap();
        render(&skin, None, &view(0.0)).save("/tmp/steve-front.png").unwrap();
        render(&skin, None, &view(std::f32::consts::PI)).save("/tmp/steve-back.png").unwrap();
        render(&skin, None, &view(0.6)).save("/tmp/steve-side.png").unwrap();
        let mut cape = image::RgbaImage::new(64, 32);
        for y in 1..17 {
            for x in 12..22 {
                cape.put_pixel(x, y, image::Rgba([198, 242, 92, 255]));
            }
            for x in 1..11 {
                cape.put_pixel(x, y, image::Rgba([180, 40, 40, 255]));
            }
        }
        let mut back = view(std::f32::consts::PI);
        back.cape = true;
        render(&skin, Some(&cape), &back).save("/tmp/cape-back.png").unwrap();
    }
}
