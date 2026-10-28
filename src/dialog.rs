//! System file dialogs and folder open. Not a browser.

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn pick_file(title: &str, pattern: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Title = '{title}'; $f.Filter = '{pattern}|{pattern}'; if ($f.ShowDialog() -eq 'OK') {{ $f.FileName }}"
        );
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", &script]);
        hide_console(&mut cmd);
        let out = cmd.output().ok()?;
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(PathBuf::from(path)) }
    }
    #[cfg(not(windows))]
    {
        if Command::new("zenity").arg("--version").output().is_ok() {
            let out = Command::new("zenity")
                .args(["--file-selection", "--title", title, "--file-filter", pattern])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(PathBuf::from(path)) }
        } else if Command::new("kdialog").arg("--version").output().is_ok() {
            let out = Command::new("kdialog")
                .args(["--getopenfilename", ".", pattern])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(PathBuf::from(path)) }
        } else {
            None
        }
    }
}

fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

pub fn open_dir(path: &Path) -> Result<(), String> {
    let _ = std::fs::create_dir_all(path);
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(path);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}
