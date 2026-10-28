fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/rus.ico");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target != "windows" {
        return;
    }
    let ico = std::path::Path::new("assets/rus.ico")
        .canonicalize()
        .expect("assets/rus.ico");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let rc_path = std::path::Path::new(&out_dir).join("rus.rc");
    let obj_path = std::path::Path::new(&out_dir).join("rus-icon.o");
    let ico_s = ico.to_string_lossy().replace('\\', "/");
    std::fs::write(&rc_path, format!("1 ICON \"{ico_s}\"\n")).expect("write rc");
    let status = std::process::Command::new("x86_64-w64-mingw32-windres")
        .arg("-O")
        .arg("coff")
        .arg("-i")
        .arg(&rc_path)
        .arg("-o")
        .arg(&obj_path)
        .status()
        .expect("windres");
    if !status.success() {
        panic!("windres failed to embed the square icon");
    }
    println!("cargo:rustc-link-arg-bins={}", obj_path.display());
}
