//! Official Minecraft collection wallpapers, fetched from minecraft.net.
//!
//! These files stay on Mojang's servers. Rus downloads a preview into the
//! data folder when a version is shown. They are not bundled in the binary.

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
    let ext = if paper.preview.ends_with(".png") { "png" } else { "jpg" };
    dir.join("wallpapers").join(format!("{}.{}", slug(paper.title), ext))
}

pub fn load_cached(dir: &Path, paper: &Paper) -> Result<Vec<u8>, String> {
    let path = cache_path(dir, paper);
    if path.is_file() {
        return std::fs::read(&path).map_err(|e| e.to_string());
    }
    let bytes = crate::net::get_bytes(paper.preview)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &bytes);
    Ok(bytes)
}
