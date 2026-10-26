//! Official Minecraft collection wallpapers from minecraft.net.
//!
//! The page's 414px pictures are only previews. Each drop's Download button
//! is a zip. Rus fetches that pack and keeps the desktop image (1920x1080
//! when the pack has one). The art stays in the data folder, not in the binary.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub struct Paper {
    pub title: &'static str,
    pub preview: &'static str,
    /// Version id prefixes this art belongs to. Empty means gallery only.
    pub prefixes: &'static [&'static str],
}

const JAVA: Paper = Paper {
    title: "Java Edition",
    preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Vanilla_bundle_1.png",
    prefixes: &[],
};

const PAPERS: &[Paper] = &[
    Paper {
        title: "Wilderness Bound",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_SGD-26_DotNet_Wallpaper_414x414.png",
        prefixes: &["26.3"],
    },
    Paper {
        title: "Chaos Cubed",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_SGD-26_DotNet_Wallpaper_414x414.jpg",
        prefixes: &["26.2"],
    },
    Paper {
        title: "Tiny Takeover",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_SPR26Drop_TT_DotNet_Wallpaper_414x414.png",
        prefixes: &["26.1"],
    },
    Paper {
        title: "Mounts of Mayhem",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_HOL25Drop_MoM_DotNet_Wallpaper_414x414.png",
        prefixes: &["1.21.11"],
    },
    Paper {
        title: "The Copper Age",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Minecraft_Fall_Drop_Campaign_Key_Art_DotNet_Downloadable_Wallpaper_414x414.png",
        prefixes: &["1.21.9", "1.21.10"],
    },
    Paper {
        title: "Chase the Skies",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_SummerDrop_Hero_DotNet_Downloadable_Wallpaper_r414x414.png",
        prefixes: &["1.21.6", "1.21.7", "1.21.8"],
    },
    Paper {
        title: "Spring to Life",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/MCV_SpringDrop_DotNet_Downloadable_Wallpaper_414x414.png",
        prefixes: &["1.21.5"],
    },
    Paper {
        title: "The Garden Awakens",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Collectables_Carousel-H-0_TheGardenAwakens_Secondary_414x414.jpg",
        prefixes: &["1.21.4"],
    },
    Paper {
        title: "Bundles of Bravery",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Collectables_Carousel-H-0_FallDrop2024_414x414%20small.png",
        prefixes: &["1.21.2", "1.21.3"],
    },
    Paper {
        title: "Tricky Trials",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_TrickyTrials_414x414_01.jpg",
        prefixes: &["1.21", "1.21.1"],
    },
    Paper {
        title: "Trails & Tales",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_TrailsAndTales_414x414_01.jpg",
        prefixes: &["1.20"],
    },
    Paper {
        title: "Wild Update",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_WildUpdate_414x414_01.jpg",
        prefixes: &["1.19"],
    },
    Paper {
        title: "Caves & Cliffs part II",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_CavesAndCliffs2_414x414_01.jpg",
        prefixes: &["1.18"],
    },
    Paper {
        title: "Caves & Cliffs part I",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_CavesAndCliffs1_414x414_01.jpg",
        prefixes: &["1.17"],
    },
    Paper {
        title: "Nether Update",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_NetherUpdate_414x414_01.jpg",
        prefixes: &["1.16"],
    },
    Paper {
        title: "Buzzy Bees Update",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_BuzzyBeesUpdate_414x414_01.jpg",
        prefixes: &["1.15"],
    },
    Paper {
        title: "Village and Pillage",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_VillageandPillage_414x414_01.jpg",
        prefixes: &["1.14"],
    },
    Paper {
        title: "Update Aquatic",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_UpdateAquatic_414x414_01.jpg",
        prefixes: &["1.13"],
    },
    Paper {
        title: "World of Color",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_WorldofColor_414x414_01.jpg",
        prefixes: &["1.12"],
    },
    Paper {
        title: "Boat Trip",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_BoatTrip_414x414_01.jpg",
        prefixes: &[],
    },
    Paper {
        title: "Beach Cabin",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_BeachCabin_414x414_01.jpg",
        prefixes: &[],
    },
    Paper {
        title: "Mining",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_Mining_414x414_01.jpg",
        prefixes: &[],
    },
    Paper {
        title: "Winter Celebration",
        preview: "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/key-art/Wallpapers_MinecraftGame-Carousel-H-0_WinterCelebration_414x414_01.jpg",
        prefixes: &[],
    },
];

pub fn all() -> &'static [Paper] {
    PAPERS
}

pub fn for_version(id: &str) -> &'static Paper {
    for paper in PAPERS {
        for prefix in paper.prefixes {
            if version_is(id, prefix) {
                return paper;
            }
        }
    }
    &JAVA
}

fn version_is(id: &str, prefix: &str) -> bool {
    id == prefix
        || id.starts_with(&format!("{prefix}."))
        || id.starts_with(&format!("{prefix}-"))
}

pub fn slug(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

pub fn cache_path(dir: &Path, paper: &Paper) -> PathBuf {
    dir.join("wallpapers").join(format!("{}-desktop.png", slug(paper.title)))
}

pub fn load_cached(dir: &Path, paper: &Paper) -> Result<Vec<u8>, String> {
    let path = cache_path(dir, paper);
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() > 80_000 {
            return Ok(bytes);
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Some(url) = pack_url(paper.title) {
        if let Ok(zip) = crate::net::get_bytes(url) {
            if let Ok(image) = desktop_from_pack(&zip) {
                let _ = std::fs::write(&path, &image);
                return Ok(image);
            }
        }
    }
    let bytes = crate::net::get_bytes(paper.preview)?;
    let _ = std::fs::write(&path, &bytes);
    Ok(bytes)
}

fn pack_url(title: &str) -> Option<&'static str> {
    let file = match title {
        "Java Edition" => "wallpapers_minecraft_pc_bundle.zip",
        "Wilderness Bound" => "wallpapers_wilderness_bound_drop.zip",
        "Chaos Cubed" => "wallpapers_chaos_cubed_drop.zip",
        "Tiny Takeover" => "wallpapers_tiny_takeover_drop.zip",
        "Mounts of Mayhem" => "wallpapers_mounts_of_mayhem_drop.zip",
        "The Copper Age" => "wallpapers_the_copper_age_drop-1.zip",
        "Chase the Skies" => "wallpapers_chase_the_skies_update.zip",
        "Spring to Life" => "wallpapers_spring_to_life_update.zip",
        "The Garden Awakens" => "wallpapers_the_garden_awakens_update.zip",
        "Bundles of Bravery" => "wallpapers_bundles_of_bravery.zip",
        "Tricky Trials" => "wallpapers_tricky_trials_update2.zip",
        "Trails & Tales" => "Minecraft_Trails_and_Tales_.Net.zip",
        "Wild Update" => "wallpaper_minecraft_wild_update.zip",
        "Caves & Cliffs part II" => "wallpapers_minecraft_caves_cliffs%28part2%29.zip",
        "Caves & Cliffs part I" => "wallpapers_minecraft_caves_cliffs%28part1%29.zip",
        "Nether Update" => "wallpapers_minecraft_nether_update.zip",
        "Buzzy Bees Update" => "wallpapers_minecraft_buzzybees.zip",
        "Village and Pillage" => "wallpapers_minecraft_village_pillage.zip",
        "Update Aquatic" => "wallpapers_minecraft_update_aquatic.zip",
        "World of Color" => "wallpapers_minecraft_world_color.zip",
        "Boat Trip" => "wallpapers_minecraft_ocean_monument_.zip",
        "Beach Cabin" => "wallpapers_minecraft_island.zip",
        "Mining" => "wallpapers_minecraft_mineshaft.zip",
        "Winter Celebration" => "wallpapers_minecraft_winter_celebration.zip",
        _ => return None,
    };
    match file {
        "wallpapers_wilderness_bound_drop.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_wilderness_bound_drop.zip"),
        "wallpapers_chaos_cubed_drop.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_chaos_cubed_drop.zip"),
        "wallpapers_tiny_takeover_drop.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_tiny_takeover_drop.zip"),
        "wallpapers_mounts_of_mayhem_drop.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_mounts_of_mayhem_drop.zip"),
        "wallpapers_the_copper_age_drop-1.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_the_copper_age_drop-1.zip"),
        "wallpapers_chase_the_skies_update.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_chase_the_skies_update.zip"),
        "wallpapers_spring_to_life_update.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_spring_to_life_update.zip"),
        "wallpapers_the_garden_awakens_update.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_the_garden_awakens_update.zip"),
        "wallpapers_bundles_of_bravery.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_bundles_of_bravery.zip"),
        "wallpapers_tricky_trials_update2.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_tricky_trials_update2.zip"),
        "Minecraft_Trails_and_Tales_.Net.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/Minecraft_Trails_and_Tales_.Net.zip"),
        "wallpaper_minecraft_wild_update.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpaper_minecraft_wild_update.zip"),
        "wallpapers_minecraft_caves_cliffs%28part2%29.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_caves_cliffs%28part2%29.zip"),
        "wallpapers_minecraft_caves_cliffs%28part1%29.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_caves_cliffs%28part1%29.zip"),
        "wallpapers_minecraft_nether_update.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_nether_update.zip"),
        "wallpapers_minecraft_buzzybees.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_buzzybees.zip"),
        "wallpapers_minecraft_village_pillage.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_village_pillage.zip"),
        "wallpapers_minecraft_update_aquatic.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_update_aquatic.zip"),
        "wallpapers_minecraft_world_color.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_world_color.zip"),
        "wallpapers_minecraft_ocean_monument_.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_ocean_monument_.zip"),
        "wallpapers_minecraft_island.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_island.zip"),
        "wallpapers_minecraft_mineshaft.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_mineshaft.zip"),
        "wallpapers_minecraft_winter_celebration.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_winter_celebration.zip"),
        "wallpapers_minecraft_pc_bundle.zip" => Some("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/wallpapers_minecraft_pc_bundle.zip"),
        _ => None,
    }
}

fn desktop_from_pack(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut best: Option<(i64, usize)> = None;
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(|e| e.to_string())?;
        if !file.is_file() {
            continue;
        }
        let score = desktop_score(file.name());
        if score < 0 {
            continue;
        }
        if best.as_ref().map(|(have, _)| score > *have).unwrap_or(true) {
            best = Some((score, i));
        }
    }
    let (_, index) = best.ok_or("collection pack has no desktop wallpaper")?;
    let mut file = zip.by_index(index).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).map_err(|e| e.to_string())?;
    if out.len() < 1000 {
        return Err("collection wallpaper was empty".into());
    }
    Ok(out)
}

fn desktop_score(name: &str) -> i64 {
    let lower = name.to_ascii_lowercase();
    if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
        return -1;
    }
    let Some((w, h)) = named_size(&lower) else {
        return 1;
    };
    if h >= w {
        return -1;
    }
    if w == 1920 && h == 1080 {
        return 10_000_000;
    }
    if w == 2560 && h == 1440 {
        return 8_000_000;
    }
    let aspect = w as f32 / h as f32;
    if (aspect - 16.0 / 9.0).abs() < 0.04 {
        return 5_000_000 + w as i64;
    }
    w as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_pack_prefers_1920x1080() {
        assert_eq!(named_size("Minecraft_WildernessBound_.Net_1920x1080.png"), Some((1920, 1080)));
        assert!(desktop_score("Minecraft_WildernessBound_.Net_1920x1080.png") > desktop_score("Minecraft_WildernessBound_.Net_2560x1440.png"));
        assert!(desktop_score("Minecraft_WildernessBound_.Net_1080x1920.png") < 0);
        assert!(desktop_score("Minecraft_WildernessBound_.Net_2048x2048.png") < 0);
    }
}

fn named_size(name: &str) -> Option<(u32, u32)> {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i >= bytes.len() || (bytes[i] != b'x' && bytes[i] != b'X') {
            continue;
        }
        let w: u32 = name[start..i].parse().ok()?;
        i += 1;
        let hstart = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i - hstart < 3 || i - start < 3 {
            continue;
        }
        let h: u32 = name[hstart..i].parse().ok()?;
        if w >= 100 && h >= 100 {
            return Some((w, h));
        }
    }
    None
}
