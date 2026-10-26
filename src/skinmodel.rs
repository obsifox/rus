//! Minecraft skin and cape boxes, in the same layout `skin.rs` ships to the game.

use image::RgbaImage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArmModel {
    Classic,
    Slim,
    Auto,
}

#[derive(Clone, Copy)]
pub struct BoxPart {
    pub size: [f32; 3],
    pub center: [f32; 3],
    pub uv: [u32; 2],
    pub overlay: Option<[u32; 2]>,
    pub inflate: f32,
    /// Rotate around the top edge (arms, legs, cape).
    pub pivot_top: bool,
    pub rot_x: f32,
}

pub fn prepare_skin(src: &RgbaImage) -> Result<RgbaImage, String> {
    match (src.width(), src.height()) {
        (64, 64) => Ok(src.clone()),
        (64, 32) => Ok(expand_legacy(src)),
        (w, h) => Err(format!("Skin must be 64×64 or 64×32, not {w}×{h}")),
    }
}

pub fn prepare_cape(src: &RgbaImage) -> Result<RgbaImage, String> {
    if src.width() == 64 && src.height() == 32 {
        Ok(src.clone())
    } else {
        Err(format!(
            "Cape must be 64×32, not {}×{}",
            src.width(),
            src.height()
        ))
    }
}

/// True when the outer column of a classic right arm is empty, which means slim.
pub fn detect_slim(skin64: &RgbaImage) -> bool {
    if skin64.width() < 64 || skin64.height() < 32 {
        return false;
    }
    (20..32).all(|y| skin64.get_pixel(47, y)[3] == 0)
}

pub fn face_avatar(skin64: &RgbaImage, px: u32) -> RgbaImage {
    let mut out = RgbaImage::new(px, px);
    for y in 0..px {
        for x in 0..px {
            let sx = 8 + (x * 8) / px;
            let sy = 8 + (y * 8) / px;
            let mut c = *skin64.get_pixel(sx, sy);
            let hx = 40 + (x * 8) / px;
            let hy = 8 + (y * 8) / px;
            if (hx as u32) < skin64.width() && (hy as u32) < skin64.height() {
                let hat = *skin64.get_pixel(hx, hy);
                if hat[3] > 16 {
                    c = hat;
                }
            }
            out.put_pixel(x, y, c);
        }
    }
    out
}

fn expand_legacy(src: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
    for y in 0..32 {
        for x in 0..64 {
            out.put_pixel(x, y, *src.get_pixel(x, y));
        }
    }
    mirror_block(src, &mut out, 0, 16, 16, 48);
    mirror_block(src, &mut out, 40, 16, 32, 48);
    out
}

fn mirror_block(src: &RgbaImage, dst: &mut RgbaImage, sx: u32, sy: u32, dx: u32, dy: u32) {
    for y in 0..16 {
        for x in 0..16 {
            if sx + x >= src.width() || sy + y >= src.height() {
                continue;
            }
            let px = *src.get_pixel(sx + x, sy + y);
            dst.put_pixel(dx + 15 - x, dy + y, px);
        }
    }
}

pub fn player_parts(slim: bool, walk: f32, cape: bool) -> Vec<BoxPart> {
    let arm_w = if slim { 3.0 } else { 4.0 };
    let arm_x = if slim { 5.5 } else { 6.0 };
    let swing = walk.sin() * 0.55;
    let mut parts = vec![
        BoxPart {
            size: [8.0, 8.0, 8.0],
            center: [0.0, 28.0, 0.0],
            uv: [0, 0],
            overlay: Some([32, 0]),
            inflate: 0.5,
            pivot_top: false,
            rot_x: 0.0,
        },
        BoxPart {
            size: [8.0, 12.0, 4.0],
            center: [0.0, 18.0, 0.0],
            uv: [16, 16],
            overlay: Some([16, 32]),
            inflate: 0.25,
            pivot_top: false,
            rot_x: 0.0,
        },
        BoxPart {
            size: [arm_w, 12.0, 4.0],
            center: [-arm_x, 18.0, 0.0],
            uv: [40, 16],
            overlay: Some([40, 32]),
            inflate: 0.25,
            pivot_top: true,
            rot_x: swing,
        },
        BoxPart {
            size: [arm_w, 12.0, 4.0],
            center: [arm_x, 18.0, 0.0],
            uv: [32, 48],
            overlay: Some([48, 48]),
            inflate: 0.25,
            pivot_top: true,
            rot_x: -swing,
        },
        BoxPart {
            size: [4.0, 12.0, 4.0],
            center: [-2.0, 6.0, 0.0],
            uv: [0, 16],
            overlay: Some([0, 32]),
            inflate: 0.25,
            pivot_top: true,
            rot_x: -swing,
        },
        BoxPart {
            size: [4.0, 12.0, 4.0],
            center: [2.0, 6.0, 0.0],
            uv: [16, 48],
            overlay: Some([0, 48]),
            inflate: 0.25,
            pivot_top: true,
            rot_x: swing,
        },
    ];
    if cape {
        parts.push(BoxPart {
            size: [10.0, 16.0, 1.0],
            center: [0.0, 16.0, -3.1],
            uv: [0, 0],
            overlay: None,
            inflate: 0.0,
            pivot_top: true,
            rot_x: 0.18 + swing * 0.25,
        });
    }
    parts
}
