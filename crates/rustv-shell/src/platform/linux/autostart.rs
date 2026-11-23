//! Linux : démarrage automatique par `~/.config/autostart/rustv.desktop` (spécification XDG Autostart,
//! sans droits administrateur). Sous AppImage, l'exécutable à relancer est le fichier `.AppImage` (variable
//! `APPIMAGE`), pas le point de montage temporaire.
use std::path::{Path, PathBuf};

const FILE: &str = "rustv.desktop";

/// Dossier `autostart` : `$XDG_CONFIG_HOME/autostart` ou `~/.config/autostart`.
pub fn dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let nonempty = |k: &str| env(k).filter(|v| !v.is_empty());
    nonempty("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| nonempty("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|d| d.join("autostart"))
}

/// Contenu du fichier `.desktop` pour `mode` ("tray" ou "tv").
pub fn desktop_entry(exe: &str, mode: &str) -> String {
    // `%` est un code de champ dans Exec : on le double.
    let exec = crate::autostart::run_value(&exe.replace('%', "%%"), mode);
    format!(
        "[Desktop Entry]\nType=Application\nName=RusTV\nComment=TV mode for your PC, controlled by your phone\nExec={exec}\nIcon=rustv\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

/// Mode déduit d'un fichier existant : "tv" s'il contient `--tv`, sinon "tray".
pub fn mode_of(content: &str) -> &'static str {
    let exec = content.lines().find_map(|l| l.strip_prefix("Exec=")).unwrap_or("");
    if content.lines().any(|l| l.trim() == "X-GNOME-Autostart-enabled=false") || content.lines().any(|l| l.trim() == "Hidden=true") {
        "off"
    } else if exec.contains("--tv") {
        "tv"
    } else {
        "tray"
    }
}

fn real_env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

fn path() -> Option<PathBuf> {
    dir(&real_env).map(|d| d.join(FILE))
}

pub fn get() -> &'static str {
    let Some(p) = path() else { return "off" };
    std::fs::read_to_string(p).map(|c| mode_of(&c)).unwrap_or("off")
}

fn exe_path() -> Option<String> {
    if let Some(a) = std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(a.to_string_lossy().into_owned());
    }
    std::env::current_exe().ok().map(|e| e.to_string_lossy().into_owned())
}

pub fn set(mode: &str) {
    let Some(p) = path() else { return };
    if mode == "off" {
        let _ = std::fs::remove_file(&p);
        return;
    }
    let Some(exe) = exe_path() else { return };
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&p, desktop_entry(&exe, mode));
}

#[allow(dead_code)]
fn _unused(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_content() {
        let t = desktop_entry("/opt/RusTV/rustv", "tv");
        assert!(t.starts_with("[Desktop Entry]\n"));
        assert!(t.contains("\nExec=\"/opt/RusTV/rustv\" --tv --boot\n"));
        assert!(t.contains("\nX-GNOME-Autostart-enabled=true\n") && t.contains("\nType=Application\n"));
        let n = desktop_entry("/home/u/My Apps/RusTV.AppImage", "tray");
        assert!(n.contains("Exec=\"/home/u/My Apps/RusTV.AppImage\" --boot\n") && !n.contains("--tv"));
        assert!(desktop_entry("/a%b/x", "tray").contains("/a%%b/x"));
    }

    #[test]
    fn mode_detection_round_trip() {
        assert_eq!(mode_of(&desktop_entry("/x/rustv", "tv")), "tv");
        assert_eq!(mode_of(&desktop_entry("/x/rustv", "tray")), "tray");
        assert_eq!(mode_of("[Desktop Entry]\nExec=rustv\nX-GNOME-Autostart-enabled=false\n"), "off");
        assert_eq!(mode_of("[Desktop Entry]\nExec=rustv\nHidden=true\n"), "off");
    }

    #[test]
    fn autostart_dir() {
        let env = |k: &str| (k == "HOME").then(|| "/home/u".to_string());
        assert_eq!(dir(&env), Some(PathBuf::from("/home/u/.config/autostart")));
        let env2 = |k: &str| match k {
            "XDG_CONFIG_HOME" => Some("/cfg".to_string()),
            "HOME" => Some("/home/u".to_string()),
            _ => None,
        };
        assert_eq!(dir(&env2), Some(PathBuf::from("/cfg/autostart")));
        assert_eq!(dir(&|_| None), None);
    }
}
