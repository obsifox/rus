//! Rus — a Minecraft: Java Edition launcher in Rust.
//!
//! The binary owns all real work (Mojang metadata, installs, assets, java
//! runtime, skin injection, launching, mods) and exposes it over a local HTTP
//! JSON API used by the bundled UI. Same API is reachable from the browser
//! preview while the binary runs.
mod horizon;
mod install;
mod java;
mod mods;
mod mojang;
mod net;
mod play;
mod sandbox;
mod server;
mod skin;
mod state;


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Rus {}\n\n  --port <n>     listen port (default 8899, 0 = random)\n  --dir <path>   data directory (default ~/.rus, or %APPDATA%\\Rus on Windows)\n\nRus is the environment. It does not open a browser.\nPlay starts the client inside the sandbox.\n", env!("CARGO_PKG_VERSION"));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--dir") {
        if let Some(v) = args.get(i + 1) {
            std::env::set_var("LAUNCHER_DIR", v);
        }
    }
    let port: u16 = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8899);

    let app = state::App::load();
    let _ = skin::materialize_preset(&app, "skin", "steve");
    let _ = skin::materialize_preset(&app, "skin", "alex");
    let _ = skin::materialize_preset(&app, "cape", "rus");
    println!("data dir → {}", app.dir.display());
    if let Err(e) = server::start(app, port) {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
