//! Serveur de test de la télécommande web : sert un dossier (ex. `remote/dist`) avec un appareil jumelé fictif,
//! sans toucher à la configuration réelle. Affiche l'adresse, l'identifiant et la clé (base64) puis attend.
//!
//! cargo run -p rustv-core --example webremote_dev -- <dossier> [port] [ip]   (ip : 127.0.0.1 par défaut)
use base64::{engine::general_purpose::STANDARD, Engine};
use rustv_core::config::{Config, Device};
use rustv_core::webremote::{Asset, Assets, WebRemote};
use rustv_core::Core;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;

struct Dir(PathBuf);
impl Assets for Dir {
    fn get(&self, p: &str) -> Option<Asset> {
        let body = std::fs::read(self.0.join(p)).ok()?;
        let mime = match p.rsplit('.').next().unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" => "text/javascript; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "webmanifest" => "application/manifest+json",
            _ => "application/octet-stream",
        };
        Some(Asset { body, mime: mime.into() })
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = PathBuf::from(a.next().expect("dossier à servir"));
    let port: u16 = a.next().and_then(|p| p.parse().ok()).unwrap_or(47822);
    let ip: Ipv4Addr = a.next().and_then(|p| p.parse().ok()).unwrap_or(Ipv4Addr::LOCALHOST);
    let key: [u8; 32] = rand::random();
    let dev = Device { id: "devtest".into(), name: "Navigateur de test".into(), key: STANDARD.encode(key), used: true };
    let cfg = Config { pc_id: "dev".into(), pc_name: "Dev".into(), devices: vec![dev], custom_apps: vec![] };
    let core = Arc::new(Core::new(cfg));
    let w = WebRemote::start_on(core, Arc::new(Dir(dir)), vec![ip], port).expect("port libre");
    println!("{}", serde_json::json!({ "addr": w.addrs()[0].to_string(), "device": "devtest", "key": STANDARD.encode(key) }));
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
