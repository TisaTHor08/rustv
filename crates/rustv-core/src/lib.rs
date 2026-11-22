//! RusTV core: remote-control server, input injection, pairing, Bluetooth, text-field detection.
//! Embedded by the TV shell (`rustv-shell`); contains no UI.
pub mod apps;
pub mod bluetooth;
pub mod config;
pub mod core;
pub mod icons;
pub mod input;
pub mod server;
pub mod smtc;
pub mod uia;

pub use crate::core::{Core, DeviceInfo, Event, NowPlaying, PlayerInfo};

use std::sync::Arc;
use std::time::Duration;

/// Démarre tous les services d'arrière-plan. Renvoie une erreur si le port est déjà pris
/// (une autre instance de RusTV tourne).
pub fn start(core: Arc<Core>) -> std::io::Result<()> {
    let listener = server::bind()?;
    {
        let c = core.clone();
        std::thread::spawn(move || server::run_tcp(c, listener));
    }
    {
        let c = core.clone();
        std::thread::spawn(move || server::run_beacon(c));
    }
    {
        let c = core.clone();
        std::thread::spawn(move || loop {
            c.tick();
            std::thread::sleep(Duration::from_millis(200));
        });
    }
    {
        let c = core.clone();
        std::thread::spawn(move || uia::run(c));
    }
    {
        let c = core.clone();
        std::thread::spawn(move || smtc::run(c));
    }
    {
        // Bluetooth : facultatif (PC sans adaptateur ou Bluetooth désactivé → on ignore).
        let c = core.clone();
        std::thread::spawn(move || {
            if let Some(addr) = bluetooth::adapter_address() {
                *c.bt_addr.lock().unwrap() = Some(addr);
                let _ = bluetooth::run(c);
            }
        });
    }
    Ok(())
}
