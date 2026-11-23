//! Configuration persistante : `<config_dir>/config.json` (`%APPDATA%\RusTV` sous Windows, voir `platform`).
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// Clé de 32 octets en base64.
    pub key: String,
    /// Faux tant que le téléphone ne s'est jamais authentifié (QR non scanné).
    #[serde(default)]
    pub used: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CustomApp {
    pub name: String,
    /// URL (https://…) ou chemin d'un exécutable.
    pub target: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub pc_id: String,
    pub pc_name: String,
    pub devices: Vec<Device>,
    #[serde(default)]
    pub custom_apps: Vec<CustomApp>,
}

fn dir() -> PathBuf {
    crate::platform::config_dir()
}

pub fn random_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Config {
    pub fn load() -> Self {
        let path = dir().join("config.json");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(c) = serde_json::from_str(s.trim_start_matches('\u{feff}')) {
                return c;
            }
            // Illisible : on le garde de côté plutôt que de perdre les appairages.
            let _ = std::fs::rename(&path, dir().join("config.json.bad"));
        }
        let name = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "PC".into());
        let c = Config { pc_id: random_hex(8), pc_name: name, devices: vec![], custom_apps: vec![
            CustomApp { name: "YouTube".into(), target: "https://www.youtube.com/tv".into() },
            CustomApp { name: "Netflix".into(), target: "https://www.netflix.com".into() },
        ] };
        c.save();
        c
    }

    pub fn save(&self) {
        let d = dir();
        let _ = std::fs::create_dir_all(&d);
        if let Ok(s) = serde_json::to_string_pretty(self) {
            // écriture atomique : fichier temporaire puis renommage
            let tmp = d.join("config.json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, d.join("config.json"));
            }
        }
    }

    pub fn key_of(&self, device_id: &str) -> Option<[u8; 32]> {
        let d = self.devices.iter().find(|d| d.id == device_id)?;
        STANDARD.decode(&d.key).ok()?.try_into().ok()
    }

    /// Crée un nouvel appairage (clé fraîche) et renvoie l'appareil.
    pub fn new_device(&mut self) -> Device {
        self.devices.retain(|d| d.used); // les QR jamais scannés sont périmés
        let mut k = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut k);
        let n = (1..).find(|n| !self.devices.iter().any(|d| d.name == format!("Téléphone {n}"))).unwrap();
        let d = Device { id: random_hex(8), name: format!("Téléphone {n}"), key: STANDARD.encode(k), used: false };
        self.devices.push(d.clone());
        self.save();
        d
    }

    /// Supprime les appairages jamais utilisés (QR affiché puis abandonné).
    pub fn prune_unused(&mut self) {
        let before = self.devices.len();
        self.devices.retain(|d| d.used);
        if self.devices.len() != before {
            self.save();
        }
    }

    pub fn mark_used(&mut self, id: &str) {
        if let Some(d) = self.devices.iter_mut().find(|d| d.id == id) {
            if !d.used {
                d.used = true;
                self.save();
            }
        }
    }

    pub fn revoke(&mut self, id: &str) {
        self.devices.retain(|d| d.id != id);
        self.save();
    }
}
