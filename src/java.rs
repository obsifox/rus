//! Java discovery + provisioning.
//!
//! Resolution order: explicit path in settings → the launcher's own runtime →
//! a system JVM that is new enough.  When nothing fits, a Temurin (Adoptium)
//! JRE matching the version's `javaVersion.majorVersion` is downloaded and
//! unpacked into `<data>/runtime/<component>/`.
use crate::net;
use crate::state::App;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const ADOPTIUM_ASSETS: &str = "https://api.adoptium.net/v3/assets/latest";

pub fn java_exe(root: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        root.join("bin/java.exe")
    } else {
        root.join("bin/java")
    }
}

/// Windowed Java. `java.exe` opens a console; `javaw.exe` does not.
pub fn gui_exe(java: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        let javaw = java.with_file_name("javaw.exe");
        if javaw.is_file() {
            return javaw;
        }
    }
    java.to_path_buf()
}

pub fn system_java() -> Option<PathBuf> {
    let exe = if cfg!(target_os = "windows") { "java.exe" } else { "java" };
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        let cand = PathBuf::from(java_home).join("bin").join(exe);
        if cand.is_file() {
            return Some(cand);
        }
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(exe);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// `java -version` → major version (17, 21, 25 …), handling the 1.8 legacy form.
pub fn major_of(exe: &Path) -> Option<u32> {
    let out = Command::new(exe).arg("-version").output().ok()?;
    let txt = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let idx = txt.find("version \"")? + "version \"".len();
    let ver: String = txt[idx..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '_' || *c == '-').collect();
    let mut it = ver.split(|c| c == '.' || c == '_' || c == '-');
    let first: u32 = it.next()?.parse().ok()?;
    if first == 1 {
        it.next()?.parse().ok()
    } else {
        Some(first)
    }
}

pub fn runtime_dir(app: &App, component: &str) -> PathBuf {
    app.p(&format!(
        "runtime/{component}/{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ))
}

pub fn bundled(app: &App, component: &str) -> Option<PathBuf> {
    let exe = java_exe(&runtime_dir(app, component));
    exe.is_file().then_some(exe)
}

/// Pick the best available JVM for a version that wants `major`.
pub fn resolve(app: &App, component: &str, major: u32) -> Option<PathBuf> {
    let forced = app.settings().java_path;
    if !forced.trim().is_empty() {
        let p = PathBuf::from(forced.trim());
        if p.is_file() {
            return Some(p);
        }
        let cand = p.join("bin").join(if cfg!(target_os = "windows") { "java.exe" } else { "java" });
        if cand.is_file() {
            return Some(cand);
        }
    }
    if let Some(exe) = bundled(app, component) {
        return Some(exe);
    }
    match system_java() {
        Some(sys) => {
            let have = major_of(&sys).unwrap_or(0);
            if major == 0 || have >= major {
                Some(sys)
            } else {
                None
            }
        }
        None => None,
    }
}

fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        "x86" => "x86",
        "arm" => "arm",
        other => other,
    }
}

/// Rewrite the unpacked tree so `<dest>/bin/java` is the entry point.
fn normalize_root(dest: &Path) -> Result<PathBuf, String> {
    if java_exe(dest).is_file() {
        return Ok(dest.to_path_buf());
    }
    let rd = fs::read_dir(dest).map_err(|e| e.to_string())?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && java_exe(&p).is_file() {
            return Ok(p);
        }
        if p.is_dir() {
            if let Ok(inner) = normalize_root(&p) {
                return Ok(inner);
            }
        }
    }
    Err(format!("no bin/java under {}", dest.display()))
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let name = archive.to_string_lossy().to_lowercase();
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if name.ends_with(".zip") {
        let f = fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        z.extract(dest).map_err(|e| e.to_string())?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let f = fs::File::open(archive).map_err(|e| e.to_string())?;
        let gz = flate2::read::GzDecoder::new(f);
        let mut ar = tar::Archive::new(gz);
        ar.unpack(dest).map_err(|e| e.to_string())?;
    } else {
        return Err(format!("unsupported archive {name}"));
    }
    Ok(())
}

fn make_executable(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for rel in ["bin/java", "lib/jspawnhelper"] {
            let p = dir.join(rel);
            if p.is_file() {
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
            }
        }
        if let Ok(rd) = fs::read_dir(dir.join("bin")) {
            for e in rd.flatten() {
                let _ = fs::set_permissions(e.path(), fs::Permissions::from_mode(0o755));
            }
        }
    }
}

/// Download + unpack a Temurin JRE for the given feature release.
pub fn install(
    app: &App,
    component: &str,
    major: u32,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let feature = if major == 0 { 21 } else { major };
    let url = format!(
        "{ADOPTIUM_ASSETS}/{feature}/hotspot?architecture={}&image_type=jre&os={}&vendor=eclipse",
        arch_name(),
        os_name()
    );
    let json = net::get_json(&url)?;
    let first = json
        .as_array()
        .and_then(|a| a.first().cloned())
        .ok_or_else(|| format!("no Temurin build for java {feature} on {}/{}", os_name(), arch_name()))?;
    let pkg = first
        .get("binary")
        .and_then(|b| b.get("package"))
        .ok_or("adoptium package missing")?;
    let link = pkg.get("link").and_then(|l| l.as_str()).ok_or("download link missing")?.to_string();
    let checksum = pkg.get("checksum").and_then(|c| c.as_str()).map(|s| s.to_string());
    let name = pkg
        .get("name")
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "jre.tar.gz".into());

    let root = runtime_dir(app, component);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let archive = app.p(&format!("cache/{name}"));
    let mut last = 0u64;
    let sum_ref = checksum.clone();
    let size = net::download_with(&link, &archive, sum_ref.as_deref(), |n| {
        last += n;
        progress(last, 0);
    })?;
    if let Some(sum) = checksum {
        let got = net::sha256_file(&archive).unwrap_or_default();
        if !got.eq_ignore_ascii_case(&sum) {
            let _ = fs::remove_file(&archive);
            return Err("java archive checksum mismatch".into());
        }
    }
    progress(size, size);
    extract(&archive, &root)?;
    let _ = fs::remove_file(&archive);
    let real = normalize_root(&root)?;
    if real != root {
        // flatten: move the single top-level dir's contents to `root`
        for e in fs::read_dir(&real).map_err(|e| e.to_string())?.flatten() {
            let to = root.join(e.file_name());
            let _ = if to.exists() { fs::remove_dir_all(&to) } else { Ok(()) };
            fs::rename(e.path(), &to).map_err(|e| e.to_string())?;
        }
        let _ = fs::remove_dir_all(&real);
    }
    make_executable(&root);
    let exe = java_exe(&root);
    if !exe.is_file() {
        return Err(format!("java runtime incomplete at {}", exe.display()));
    }
    Ok(exe)
}

/// Friendly label for a version's `javaVersion.component`.
pub fn component_label(component: &str, major: u32) -> String {
    if major > 0 {
        return format!("Java {major}");
    }
    match component {
        "jre-legacy" => "Java 8".into(),
        "java-runtime-alpha" => "Java 16".into(),
        "java-runtime-beta" | "java-runtime-gamma" => "Java 17".into(),
        "java-runtime-delta" | "java-runtime-epsilon" => "Java 21".into(),
        _ => "Java".into(),
    }
}

pub fn version_of(exe: &Path) -> String {
    match Command::new(exe).arg("-version").output() {
        Ok(o) => {
            let txt = String::from_utf8_lossy(&o.stderr).to_string();
            txt.lines().next().unwrap_or("java").trim().to_string()
        }
        Err(_) => "java (unknown)".into(),
    }
}

pub fn probe_major(exe: &Path) -> Option<u32> {
    let mut f = fs::File::open(exe).ok()?;
    let mut buf = vec![0u8; 0];
    let _ = (&mut f).read(&mut buf);
    major_of(exe)
}
