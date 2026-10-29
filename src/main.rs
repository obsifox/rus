//! Rus — a native Minecraft: Java Edition launcher.
//!
//! The window is the program. It does not open a browser and it does not
//! serve a web page. Release builds do not open a console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "product.rs"]
mod product;
mod desktop;
mod dialog;
mod explore;
mod horizon;
mod instances;
mod install;
mod java;
mod loaders;
mod mods;
mod mojang;
mod net;
mod play;
mod preview3d;
mod raster;
mod sandbox;
mod sfx;
mod skin;
mod skinmodel;
mod state;
mod wallpapers;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Rus {}\n\n  --dir <path>     data directory (default ~/.rus, or %APPDATA%\\Rus on Windows)\n  --shot <file>    save a picture of the window, then exit\n  --tab <name>     home, versions, mods, or settings\n\nRus is a native window. It does not open a browser.\n",
            crate::product::VERSION
        );
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--dir") {
        if let Some(v) = args.get(i + 1) {
            std::env::set_var("LAUNCHER_DIR", v);
        }
    }
    let shot = args
        .iter()
        .position(|a| a == "--shot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let tab = args
        .iter()
        .position(|a| a == "--tab")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "home".into());

    if let Err(e) = desktop::run(shot, tab) {
        eprintln!("Rus could not open its window: {e}");
        eprintln!("Rus does not open a browser.");
        std::process::exit(1);
    }
}
