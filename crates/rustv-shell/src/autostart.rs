//! Démarrage automatique à l'ouverture de session (HKCU\...\Run, sans droits administrateur).
use std::os::windows::process::CommandExt;
use std::process::Command;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "RusTV";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn reg(args: &[&str]) -> Option<String> {
    let out = Command::new("reg").args(args).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// "off", "tray" (RusTV démarre dans la zone de notification) ou "tv" (démarre directement en mode TV).
pub fn get() -> &'static str {
    match reg(&["query", RUN_KEY, "/v", NAME]) {
        None => "off",
        Some(s) if s.contains("--tv") => "tv",
        Some(_) => "tray",
    }
}

pub fn set(mode: &str) {
    if mode == "off" {
        let _ = reg(&["delete", RUN_KEY, "/v", NAME, "/f"]);
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let value = format!("\"{}\"{}", exe.display(), if mode == "tv" { " --tv" } else { "" });
    let _ = reg(&["add", RUN_KEY, "/v", NAME, "/t", "REG_SZ", "/d", &value, "/f"]);
}
