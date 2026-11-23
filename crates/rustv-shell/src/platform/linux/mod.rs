//! Backends Linux du shell (voir docs/PORTING.md et docs/LINUX.md). Inclus par `main.rs` sous
//! `#[cfg(target_os = "linux")]`.
pub mod autostart;
pub mod disk;
pub mod mounts;
pub mod pkg;
pub mod secrets;
pub mod streaming;
pub mod tray;

/// Arguments Chromium propres à la session : Wayland (`--ozone-platform=wayland`). Sous X11 la position et la
/// taille de la fenêtre sont déjà données par `--window-position` / `--window-size`.
pub fn browser_extra_args() -> Vec<String> {
    wayland_args(session_is_wayland())
}

pub fn wayland_args(wayland: bool) -> Vec<String> {
    if wayland {
        vec!["--ozone-platform=wayland".into(), "--enable-features=UseOzonePlatform".into()]
    } else {
        Vec::new()
    }
}

pub fn session_is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false) || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn wayland_flags() {
        assert!(super::wayland_args(true).contains(&"--ozone-platform=wayland".to_string()));
        assert!(super::wayland_args(false).is_empty());
    }
}
