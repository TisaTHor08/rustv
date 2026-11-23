//! Démarrage automatique à l'ouverture de session (HKCU\...\Run, sans droits administrateur).
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const NAME: &str = "RusTV";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn reg(args: &[&str]) -> Option<String> {
    let out = Command::new("reg").args(args).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// "off", "tray" (RusTV démarre dans la zone de notification) ou "tv" (démarre directement en mode TV).
#[cfg(windows)]
pub fn get() -> &'static str {
    match reg(&["query", RUN_KEY, "/v", NAME]) {
        None => "off",
        Some(s) if s.contains("--tv") => "tv",
        Some(_) => "tray",
    }
}

#[cfg(windows)]
pub fn set(mode: &str) {
    if mode == "off" {
        let _ = reg(&["delete", RUN_KEY, "/v", NAME, "/f"]);
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let value = run_value(&exe.display().to_string(), mode);
    let _ = reg(&["add", RUN_KEY, "/v", NAME, "/t", "REG_SZ", "/d", &value, "/f"]);
}

/// STUB non-Windows : démarrage automatique non pris en charge (toujours "off", `set` ne fait rien).
/// Backend prévu : `~/.config/autostart/rustv.desktop` (Linux), LaunchAgent plist (macOS), voir docs/PORTING.md.
#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn get() -> &'static str {
    "off"
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn set(_mode: &str) {}

/// Linux : `~/.config/autostart/rustv.desktop` (voir `platform/linux/autostart.rs`).
#[cfg(target_os = "linux")]
pub fn get() -> &'static str {
    crate::linux::autostart::get()
}

#[cfg(target_os = "linux")]
pub fn set(mode: &str) {
    crate::linux::autostart::set(mode)
}

/// Valeur de la clé Run. `--boot` = démarrage silencieux (pas de fenêtre d'appairage, attente de l'écran de la
/// TV, reprise du mode TV). Les anciennes entrées (`--tv` seul, ou sans argument) restent comprises par `get`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn run_value(exe: &str, mode: &str) -> String {
    match mode {
        "tv" => format!("\"{exe}\" --tv --boot"),
        _ => format!("\"{exe}\" --boot"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_values() {
        assert_eq!(run_value(r"C:\a b\rustv.exe", "tv"), "\"C:\\a b\\rustv.exe\" --tv --boot");
        assert_eq!(run_value(r"C:\r.exe", "tray"), "\"C:\\r.exe\" --boot");
        // `get` reconnaît « tv » à la présence de --tv (ancien et nouveau format)
        assert!(run_value("x", "tv").contains("--tv") && !run_value("x", "tray").contains("--tv"));
    }
}
