//! Contenu du QR code d'appairage.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairingInfo {
    pub v: u32,
    /// ID du PC (hex).
    pub id: String,
    pub name: String,
    /// Identifiant attribué à ce téléphone (un appairage = une clé distincte, révocable).
    pub device: String,
    /// Clé secrète de 32 octets, base64.
    pub key: String,
    pub ips: Vec<String>,
    pub port: u16,
    /// Adresse Bluetooth (optionnelle).
    #[serde(default)]
    pub bt: Option<String>,
}

impl PairingInfo {
    pub fn to_qr_string(&self) -> String {
        serde_json::to_string(self).expect("sérialisation")
    }
    pub fn from_qr_string(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}
