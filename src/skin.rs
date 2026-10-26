//! Local skin & cape injection: builds a tiny resource pack keyed by the
//! offline player UUID so the game renders *your* skin without any account.
use crate::state::{offline_uuid, App};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};


/// Bundled vanilla skins (embedded so presets work with no network / no UI).
const PRESET_STEVE: &[u8] = include_bytes!("../assets/steve.png");
const PRESET_ALEX: &[u8] = include_bytes!("../assets/alex.png");

pub const CAPE_PRESETS: [(&str, [u8; 3]); 4] = [
    ("crimson", [0xa8, 0x3a, 0x30]),
    ("emerald", [0x3a, 0xa0, 0x5c]),
    ("midnight", [0x33, 0x45, 0x6e]),
    ("sand", [0xc3, 0x96, 0x53]),
];

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        table[i as usize] = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc = table[((crc ^ *b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Minimal RGBA PNG encoder (stored deflate blocks — no extra crates).
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * h * 4 + h) as usize);
    for y in 0..h {
        raw.push(0u8); // filter: none
        let row = (y * w * 4) as usize;
        raw.extend_from_slice(&rgba[row..row + (w * 4) as usize]);
    }
    let mut z = vec![0x78u8, 0x01];
    let mut chunks = raw.chunks(60000).peekable();
    while let Some(c) = chunks.next() {
        let last = chunks.peek().is_none();
        z.push(if last { 1 } else { 0 });
        let len = c.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |kind: &[u8; 4], body: &[u8]| {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        let mut crcbuf = Vec::from(*kind);
        crcbuf.extend_from_slice(body);
        out.extend_from_slice(&crcbuf);
        out.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

/// Official 64×32 cape layout (box UV, 10×16×1 at 0,0):
/// top (1,0), bottom (11,0), left (0,1), front (1,1), right (11,1), back (12,1).
/// The back face is what you see in game. This is Rus's own cape, not a Mojang texture.
fn rus_cape_png() -> Vec<u8> {
    let (w, h) = (64u32, 32u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: [u8; 3]| {
        if x >= w || y >= h { return; }
        let i = ((y * w + x) * 4) as usize;
        px[i] = c[0]; px[i+1] = c[1]; px[i+2] = c[2]; px[i+3] = 255;
    };
    let cloth = [0x2a, 0x38, 0x1c];
    let edge = [0xc6, 0xf2, 0x5c];
    let fold = [0x1c, 0x28, 0x12];
    let mark = [0xc6, 0xf2, 0x5c];
    let ink = [0x16, 0x20, 0x0e];
    // fill the used UV region so no face is transparent
    for y in 0..17 {
        for x in 0..22 {
            put(x, y, cloth);
        }
    }
    // back face 10×16 at (12, 1) — the visible side
    let glyph = [
        "          ",
        " ######## ",
        " #      # ",
        " #      # ",
        " ######   ",
        " #    #   ",
        " #     #  ",
        " #      # ",
        "          ",
        "  ##  ##  ",
        "  ##  ##  ",
        "          ",
        " ######## ",
        "          ",
        "          ",
        "          ",
    ];
    for (dy, row) in glyph.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let c = match ch {
                '#' => mark,
                _ => cloth,
            };
            put(12 + dx as u32, 1 + dy as u32, c);
        }
    }
    // lime binding so the cape reads at a glance
    for x in 12..22 {
        put(x, 1, edge);
        put(x, 16, edge);
        put(x, 15, fold);
    }
    for y in 1..17 {
        put(12, y, edge);
        put(21, y, edge);
    }
    // front face, darker, so the inside of the cape isn't a hole
    for y in 1..17 {
        for x in 1..11 {
            put(x, y, ink);
        }
    }
    // edges
    for y in 1..17 {
        put(0, y, edge);
        put(11, y, edge);
    }
    for x in 1..11 { put(x, 0, fold); }
    for x in 11..21 { put(x, 0, edge); }
    encode_png(w, h, &px)
}

/// 64×32 cape texture matching the presets offered by the UI.
fn cape_png(rgb: [u8; 3]) -> Vec<u8> {
    let (w, h) = (64u32, 32u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let shade = |f: f32| {
        [
            (rgb[0] as f32 * f).clamp(0.0, 255.0) as u8,
            (rgb[1] as f32 * f).clamp(0.0, 255.0) as u8,
            (rgb[2] as f32 * f).clamp(0.0, 255.0) as u8,
        ]
    };
    let mut put = |x: u32, y: u32, c: [u8; 3], a: u8| {
        if x >= w || y >= h {
            return;
        }
        let i = ((y * w + x) * 4) as usize;
        px[i] = c[0];
        px[i + 1] = c[1];
        px[i + 2] = c[2];
        px[i + 3] = a;
    };
    // back face (1..11, 1..17)
    for y in 0..17 {
        for x in 0..12 {
            put(x, y, shade(0.86), 255);
        }
    }
    // darker outline
    for x in 0..12 {
        put(x, 0, shade(0.6), 255);
        put(x, 17, shade(0.6), 255);
    }
    for y in 0..17 {
        put(0, y, shade(0.6), 255);
        put(11, y, shade(0.6), 255);
    }
    // emblem
    for x in 4..8 {
        for y in 4..6 {
            put(x, y, shade(1.35), 255);
        }
    }
    for x in 5..7 {
        for y in 6..12 {
            put(x, y, shade(1.35), 255);
        }
    }
    for x in 3..9 {
        for y in 12..14 {
            put(x, y, shade(1.35), 255);
        }
    }
    // side strips (12..32) so 3D previews look solid
    for y in 0..17 {
        for x in 12..22 {
            put(x, y, shade(0.72), 255);
        }
        for x in 22..32 {
            put(x, y, shade(0.55), 255);
        }
    }
    encode_png(w, h, &px)
}

/// Materialise a `preset:*` id into a real file inside the launcher folder.
pub fn materialize_preset(app: &App, kind: &str, key: &str) -> Option<PathBuf> {
    let (dir, name, bytes) = match (kind, key) {
        ("skin", "steve") => (skins_dir(app), "preset_steve.png".to_string(), PRESET_STEVE.to_vec()),
        ("skin", "alex") => (skins_dir(app), "preset_alex.png".to_string(), PRESET_ALEX.to_vec()),
        ("cape", "rus") => (capes_dir(app), "preset_rus.png".to_string(), rus_cape_png()),
        ("cape", k) => {
            let rgb = CAPE_PRESETS.iter().find(|(n, _)| *n == k)?.1;
            (capes_dir(app), format!("preset_{k}.png"), cape_png(rgb))
        }
        _ => return None,
    };
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(&name);
    // Bundled presets are rewritten so an updated cape/skin ships with the binary.
    let _ = fs::write(&path, &bytes);
    Some(path)
}

/// Vanilla's nine bundled default skins (both model folders).
pub const DEFAULT_SKINS: [&str; 9] = [
    "alex", "ari", "efe", "kai", "makena", "noor", "steve", "sunny", "zuri",
];

pub fn pack_name() -> &'static str {
    "Rus.zip"
}

/// Where the launcher keeps uploaded textures.
pub fn skins_dir(app: &App) -> PathBuf {
    app.p("skins")
}
pub fn capes_dir(app: &App) -> PathBuf {
    app.p("capes")
}

pub fn resolve_texture(app: &App, id: &str) -> Option<PathBuf> {
    if id.trim().is_empty() || id == "preset:none" {
        return None;
    }
    if let Some(key) = id.strip_prefix("preset:") {
        let kind = if key == "rus" || CAPE_PRESETS.iter().any(|(n, _)| *n == key) {
            "cape"
        } else {
            "skin"
        };
        return materialize_preset(app, kind, key);
    }
    let name = id.trim_start_matches("file:").trim_start_matches("skins/");
    let cand = app.p(&format!("skins/{name}"));
    if cand.is_file() {
        return Some(cand);
    }
    let cand = app.p(&format!("capes/{name}"));
    if cand.is_file() {
        return Some(cand);
    }
    None
}

/// Add `entry` to the resourcePacks list in options.txt (idempotent).
fn enable_pack(game_dir: &Path, file_name: &str) {
    let path = game_dir.join("options.txt");
    let entry = format!("file/{file_name}");
    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map(|t| t.lines().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let mut found = false;
    let mut changed = false;
    for line in lines.iter_mut() {
        if let Some(rest) = line.strip_prefix("resourcePacks:") {
            found = true;
            if !rest.contains(&entry) {
                let mut list: Vec<String> = serde_json::from_str(rest).unwrap_or_default();
                list.retain(|x| x != &entry);
                list.push(entry.clone());
                *line = format!("resourcePacks:{}", serde_json::to_string(&list).unwrap_or_default());
                changed = true;
            }
        }
    }
    if !found {
        lines.push(format!("resourcePacks:[{}]", serde_json::to_string(&entry).unwrap_or_default()));
        changed = true;
    }
    if changed {
        let _ = fs::write(&path, lines.join("\n") + "\n");
    }
}

fn zip_pack(pack_dir: &Path, out: &Path) -> Result<(), String> {
    let file = fs::File::create(out).map_err(|e| e.to_string())?;
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    fn walk(base: &Path, dir: &Path, zw: &mut zip::ZipWriter<fs::File>, opts: zip::write::SimpleFileOptions) -> std::io::Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, zw, opts)?;
            } else {
                let rel = p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                zw.start_file(rel, opts)?;
                let data = fs::read(&p)?;
                zw.write_all(&data)?;
            }
        }
        Ok(())
    }
    walk(pack_dir, pack_dir, &mut zw, opts).map_err(|e| e.to_string())?;
    zw.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// Build (and enable) the skin pack for an instance. Returns a human note.
pub fn build_pack(
    app: &App,
    game_dir: &Path,
    username: &str,
    skin_id: &str,
    cape_id: &str,
) -> Result<String, String> {
    let uuid = offline_uuid(username);
    let pack_root = app.p("cache/skinpack");
    let _ = fs::remove_dir_all(&pack_root);
    let tex_root = pack_root.join("assets/minecraft/textures/entity");
    fs::create_dir_all(&tex_root).map_err(|e| e.to_string())?;
    fs::write(
        pack_root.join("pack.mcmeta"),
        serde_json::json!({
            "pack": {
                "pack_format": 34,
                "supported_formats": [15, 99],
                "description": format!("Rus · {username}")
            }
        })
        .to_string(),
    )
    .map_err(|e| e.to_string())?;

    let mut parts: Vec<String> = Vec::new();
    if let Some(src) = resolve_texture(app, skin_id) {
        let data = fs::read(&src).map_err(|e| e.to_string())?;
        let model = app.settings().skin_model;
        // (a) vanilla default skins: an offline profile resolves to one of these
        //     nine, so overriding them makes the local skin appear in game.
        for folder in ["wide", "slim"] {
            let dir = tex_root.join("player").join(folder);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            for name in DEFAULT_SKINS {
                fs::write(dir.join(format!("{name}.png")), &data).map_err(|e| e.to_string())?;
            }
        }
        // (b) modern layout: per-player texture keyed by the offline UUID
        let target = tex_root.join("player").join(if model == "slim" { "slim" } else { "wide" });
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        fs::write(target.join(format!("{uuid}.png")), &data).map_err(|e| e.to_string())?;
        parts.push(if model == "slim" { "skin (slim)" } else { "skin (classic)" }.into());
    }
    if let Some(src) = resolve_texture(app, cape_id) {
        let data = fs::read(&src).map_err(|e| e.to_string())?;
        let capes = tex_root.join("capes");
        fs::create_dir_all(&capes).map_err(|e| e.to_string())?;
        fs::write(capes.join(format!("{uuid}.png")), &data).map_err(|e| e.to_string())?;
        // classic cape slots used by older versions / skin-aware servers
        for name in ["cape.png", "cape_1.png", "minecon_2016.png"] {
            fs::write(capes.join(name), &data).map_err(|e| e.to_string())?;
        }
        parts.push("cape".into());
    }

    if parts.is_empty() {
        return Ok("No local texture — default skin".into());
    }

    let rp = game_dir.join("resourcepacks");
    fs::create_dir_all(&rp).map_err(|e| e.to_string())?;
    let out = rp.join(pack_name());
    zip_pack(&pack_root, &out)?;
    enable_pack(game_dir, pack_name());
    Ok(format!("{} · {}", parts.join(" + "), uuid))
}

/// Write an uploaded PNG (data URL or raw base64) into the launcher folder.
pub fn save_texture(app: &App, kind: &str, name: &str, data_url: &str) -> Result<String, String> {
    use base64::Engine;
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let clean = if clean.is_empty() { "custom".to_string() } else { clean };
    let b64 = data_url
        .split("base64,")
        .nth(1)
        .unwrap_or(data_url)
        .trim()
        .to_string();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| format!("bad image data: {e}"))?;
    if bytes.len() < 100 {
        return Err("image too small".into());
    }
    let dir = if kind == "cape" { capes_dir(app) } else { skins_dir(app) };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = if kind == "cape" {
        format!("{clean}.png")
    } else {
        format!("{clean}.png")
    };
    fs::write(dir.join(&file), bytes).map_err(|e| e.to_string())?;
    Ok(format!("file:{file}"))
}

pub fn import_texture(app: &App, kind: &str, bytes: &[u8]) -> Result<String, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Not a PNG: {e}"))?
        .to_rgba8();
    match kind {
        "skin" => crate::skinmodel::prepare_skin(&img)?,
        "cape" => crate::skinmodel::prepare_cape(&img)?,
        _ => return Err("unknown texture".into()),
    };
    let dir = if kind == "cape" { capes_dir(app) } else { skins_dir(app) };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("import-{}.png", crate::state::now_ms());
    fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
    Ok(format!("file:{name}"))
}

pub fn list(app: &App) -> serde_json::Value {
    let read = |dir: PathBuf| -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("png") {
                    out.push(serde_json::json!({
                        "id": format!("file:{}", p.file_name().unwrap_or_default().to_string_lossy()),
                        "name": p.file_name().unwrap_or_default().to_string_lossy(),
                        "size": p.metadata().map(|m| m.len()).unwrap_or(0),
                    }));
                }
            }
        }
        out.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        out
    };
    serde_json::json!({
        "skins": read(skins_dir(app)),
        "capes": read(capes_dir(app)),
    })
}
