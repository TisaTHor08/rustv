//! Réglages du mode TV, persistés dans %APPDATA%\RusTV\tv.json.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Noircir les écrans autres que celui de la TV.
    pub blackout: bool,
    /// URL saisies par l'utilisateur pour les tuiles qui en exigent une (Jellyfin, NAS…).
    #[serde(default)]
    pub urls: HashMap<String, String>,
    /// Dernier écran choisi (par son index dans l'ordre gauche→droite).
    #[serde(default)]
    pub monitor: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { blackout: true, urls: HashMap::new(), monitor: None }
    }
}

pub fn dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("RusTV")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(dir().join("tv.json"))
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let d = dir();
        let _ = std::fs::create_dir_all(&d);
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = d.join("tv.json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, d.join("tv.json"));
            }
        }
    }
}
