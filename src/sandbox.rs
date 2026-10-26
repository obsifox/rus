//! The game does not open in a browser. It starts inside Rus.
//!
//! Linux: bubblewrap, when it works, gives the client its own mount, pid and
//! hostname namespaces. The home directory is not mounted. The process dies
//! with Rus.
//! Windows: a job object plus a private profile under the Rus data folder.
//! That is a real confinement, not a kernel jail — the UI says so.

use crate::state::App;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
#[cfg(windows)]
use std::sync::Mutex;

#[derive(Clone)]
pub struct Seal {
    pub kind: String,
    pub sealed: bool,
    pub label: String,
    pub detail: String,
}

struct Mode {
    kind: &'static str,
    sealed: bool,
    bwrap: Option<PathBuf>,
    detail: String,
}

fn mode() -> &'static Mode {
    static M: OnceLock<Mode> = OnceLock::new();
    M.get_or_init(detect)
}

fn detect() -> Mode {
    #[cfg(windows)]
    {
        return Mode {
            kind: "job",
            sealed: true,
            bwrap: None,
            detail: "Windows job. The client gets a private profile under Rus and is stopped when Rus exits. This is not a kernel jail.".into(),
        };
    }
    #[cfg(not(windows))]
    {
        if let Some(bin) = find_bwrap() {
            match probe(&bin) {
                Ok(()) => Mode {
                    kind: "bubblewrap",
                    sealed: true,
                    bwrap: Some(bin),
                    detail: "Kernel sandbox. The client has its own mount, pid and hostname namespaces. Your home directory is not mounted. It dies when Rus exits.".into(),
                },
                Err(e) => Mode {
                    kind: "profile",
                    sealed: false,
                    bwrap: None,
                    detail: format!("Kernel jail is unavailable ({e}). The client still gets a private home under the Rus data folder."),
                },
            }
        } else {
            Mode {
                kind: "profile",
                sealed: false,
                bwrap: None,
                detail: "bubblewrap is not installed, so there is no kernel jail. The client still gets a private home under the Rus data folder.".into(),
            }
        }
    }
}

#[cfg(not(windows))]
fn find_bwrap() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let beside = exe.with_file_name("bwrap");
        if beside.is_file() {
            return Some(beside);
        }
    }
    for p in ["/usr/bin/bwrap", "/bin/bwrap"] {
        if Path::new(p).is_file() {
            return Some(PathBuf::from(p));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let c = dir.join("bwrap");
            if c.is_file() {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn probe(bin: &Path) -> Result<(), String> {
    let true_bin = if Path::new("/bin/true").is_file() {
        "/bin/true"
    } else {
        "/usr/bin/true"
    };
    let mut cmd = Command::new(bin);
    cmd.args(["--unshare-user", "--unshare-pid", "--die-with-parent"]);
    for dir in ["/usr", "/lib", "/lib64", "/bin"] {
        if Path::new(dir).exists() {
            cmd.args(["--ro-bind", dir, dir]);
        }
    }
    let out = cmd.arg("--").arg(true_bin)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        let line = err.lines().next().unwrap_or("bubblewrap failed").trim();
        Err(line.chars().take(160).collect())
    }
}

pub fn profile_home(app: &App) -> PathBuf {
    app.p("sandbox/home")
}

fn ensure_profile(app: &App) -> PathBuf {
    let home = profile_home(app);
    let _ = std::fs::create_dir_all(home.join("tmp"));
    let _ = std::fs::create_dir_all(home.join("AppData/Roaming"));
    let _ = std::fs::create_dir_all(home.join("AppData/Local"));
    home
}

/// Program to exec, plus arguments that must come before the client's own args.
pub fn prepare(program: &Path, app: &App) -> (PathBuf, Vec<OsString>, Seal) {
    let m = mode();
    let seal = Seal {
        kind: m.kind.into(),
        sealed: m.sealed,
        label: label_for(m.kind, m.sealed),
        detail: m.detail.clone(),
    };
    if let Some(bwrap) = &m.bwrap {
        let mut args = bwrap_args(app, program);
        args.push(OsString::from("--"));
        args.push(program.as_os_str().to_os_string());
        (bwrap.clone(), args, seal)
    } else {
        (program.to_path_buf(), Vec::new(), seal)
    }
}

fn label_for(kind: &str, sealed: bool) -> String {
    match kind {
        "bubblewrap" if sealed => "Kernel sandbox".into(),
        "job" => "Job sandbox".into(),
        _ => "Private profile".into(),
    }
}

fn push(args: &mut Vec<OsString>, parts: &[&str]) {
    for p in parts {
        args.push(OsString::from(*p));
    }
}

fn bind_ro(args: &mut Vec<OsString>, path: &str) {
    if Path::new(path).exists() {
        push(args, &["--ro-bind", path, path]);
    }
}

fn bwrap_args(app: &App, _program: &Path) -> Vec<OsString> {
    let mut args = Vec::new();
    push(
        &mut args,
        &[
            "--unshare-user",
            "--unshare-pid",
            "--unshare-uts",
            "--unshare-ipc",
            "--hostname",
            "rus-sandbox",
            "--die-with-parent",
            "--share-net",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
        ],
    );
    for dir in ["/usr", "/lib", "/lib64", "/bin", "/sbin"] {
        bind_ro(&mut args, dir);
    }
    if Path::new("/sys").exists() {
        bind_ro(&mut args, "/sys");
    }
    push(&mut args, &["--dir", "/etc"]);
    for path in [
        "/etc/resolv.conf",
        "/etc/hosts",
        "/etc/nsswitch.conf",
        "/etc/passwd",
        "/etc/group",
        "/etc/localtime",
        "/etc/ssl",
        "/etc/ca-certificates",
        "/etc/ssl/certs",
        "/etc/pki",
        "/etc/alternatives",
        "/etc/fonts",
    ] {
        bind_ro(&mut args, path);
    }
    if Path::new("/dev/dri").exists() {
        push(&mut args, &["--dev-bind", "/dev/dri", "/dev/dri"]);
    }
    for dev in ["/dev/nvidiactl", "/dev/nvidia0", "/dev/nvidia-modeset", "/dev/nvidia-uvm"] {
        if Path::new(dev).exists() {
            push(&mut args, &["--dev-bind", dev, dev]);
        }
    }
    if Path::new("/dev/shm").exists() {
        push(&mut args, &["--tmpfs", "/dev/shm"]);
    }
    if Path::new("/tmp/.X11-unix").exists() {
        push(&mut args, &["--bind", "/tmp/.X11-unix", "/tmp/.X11-unix"]);
    }
    if let Some(rt) = std::env::var_os("XDG_RUNTIME_DIR") {
        if Path::new(&rt).exists() {
            args.push(OsString::from("--bind"));
            args.push(rt.clone());
            args.push(rt);
        }
    }
    let data = app.dir.as_os_str();
    args.push(OsString::from("--bind"));
    args.push(data.to_os_string());
    args.push(data.to_os_string());
    args.push(OsString::from("--chdir"));
    args.push(data.to_os_string());
    args
}

pub fn apply_env(cmd: &mut Command, app: &App) {
    let home = ensure_profile(app);
    cmd.current_dir(&app.dir);
    cmd.env("HOME", &home);
    cmd.env("USERPROFILE", &home);
    cmd.env("APPDATA", home.join("AppData/Roaming"));
    cmd.env("LOCALAPPDATA", home.join("AppData/Local"));
    cmd.env("TEMP", home.join("tmp"));
    cmd.env("TMP", home.join("tmp"));
    cmd.env("XDG_CONFIG_HOME", home.join(".config"));
    cmd.env("XDG_DATA_HOME", home.join(".local/share"));
    cmd.env("RUS_SANDBOX", "1");
}

pub fn describe(app: &App) -> Value {
    let m = mode();
    let home = profile_home(app);
    json!({
        "kind": m.kind,
        "sealed": m.sealed,
        "label": label_for(m.kind, m.sealed),
        "hostname": if m.kind == "bubblewrap" { "rus-sandbox" } else { "" },
        "root": home.to_string_lossy(),
        "data_dir": app.dir.to_string_lossy(),
        "hides_home": m.sealed && m.kind == "bubblewrap",
        "network": "shared",
        "display": "own-window",
        "dies_with_rus": m.kind == "bubblewrap" || m.kind == "job",
        "detail": m.detail,
    })
}

pub fn attach(child: &Child) {
    #[cfg(windows)]
    win::assign(child);
    #[cfg(not(windows))]
    {
        let _ = child;
    }
}

pub fn terminate() {
    #[cfg(windows)]
    win::terminate();
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct BasicLimit {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        _pad0: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        _pad1: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    struct ExtendedLimit {
        basic: BasicLimit,
        io: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    const _: () = assert!(std::mem::size_of::<ExtendedLimit>() == 144);

    const KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

    extern "system" {
        fn CreateJobObjectW(attrs: *mut std::ffi::c_void, name: *const u16) -> *mut std::ffi::c_void;
        fn SetInformationJobObject(
            job: *mut std::ffi::c_void,
            class: i32,
            info: *const std::ffi::c_void,
            len: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: *mut std::ffi::c_void, process: *mut std::ffi::c_void) -> i32;
        fn TerminateJobObject(job: *mut std::ffi::c_void, code: u32) -> i32;
    }

    fn job() -> *mut std::ffi::c_void {
        static H: OnceLock<Mutex<usize>> = OnceLock::new();
        let slot = H.get_or_init(|| Mutex::new(0));
        let mut g = slot.lock().unwrap();
        if *g == 0 {
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
                if !handle.is_null() {
                    let mut info = ExtendedLimit {
                        basic: BasicLimit {
                            per_process_user_time_limit: 0,
                            per_job_user_time_limit: 0,
                            limit_flags: KILL_ON_JOB_CLOSE,
                            _pad0: 0,
                            minimum_working_set_size: 0,
                            maximum_working_set_size: 0,
                            active_process_limit: 0,
                            _pad1: 0,
                            affinity: 0,
                            priority_class: 0,
                            scheduling_class: 0,
                        },
                        io: IoCounters {
                            read_operation_count: 0,
                            write_operation_count: 0,
                            other_operation_count: 0,
                            read_transfer_count: 0,
                            write_transfer_count: 0,
                            other_transfer_count: 0,
                        },
                        process_memory_limit: 0,
                        job_memory_limit: 0,
                        peak_process_memory_used: 0,
                        peak_job_memory_used: 0,
                    };
                    SetInformationJobObject(
                        handle,
                        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                        &mut info as *mut _ as *const _,
                        std::mem::size_of::<ExtendedLimit>() as u32,
                    );
                    *g = handle as usize;
                }
            }
        }
        *g as *mut std::ffi::c_void
    }

    pub fn assign(child: &Child) {
        let handle = job();
        if handle.is_null() {
            return;
        }
        unsafe {
            AssignProcessToJobObject(handle, child.as_raw_handle() as *mut _);
        }
    }

    pub fn terminate() {
        let handle = job();
        if handle.is_null() {
            return;
        }
        unsafe {
            TerminateJobObject(handle, 1);
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn kernel_jail_hides_home() {
        std::env::set_var("LAUNCHER_DIR", "/tmp/rus-sandbox-test");
        let app = crate::state::App::load();
        let (bin, args, seal) = prepare(Path::new("/bin/sh"), &app);
        assert!(seal.sealed, "{}", seal.detail);
        let out = Command::new(bin)
            .args(args)
            .arg("-c")
            .arg("hostname; test ! -e /home/user/.ssh && echo hidden")
            .output()
            .expect("spawn");
        let text = String::from_utf8_lossy(&out.stdout);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{text} {err}");
        assert!(text.contains("rus-sandbox"), "{text}");
        assert!(text.contains("hidden"), "{text}");
    }
}

