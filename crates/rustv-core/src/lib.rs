//! RusTV core: remote-control server, input injection, pairing, Bluetooth, text-field detection.
//! Embedded by the TV shell (`rustv-shell`); contains no UI.
#[cfg_attr(target_os = "linux", path = "platform/linux/apps.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/apps.rs")]
pub mod apps;
#[cfg_attr(target_os = "linux", path = "platform/linux/audio.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/audio.rs")]
pub mod audio;
#[cfg_attr(not(windows), path = "platform/stub/bluetooth.rs")]
pub mod bluetooth;
pub mod config;
pub mod core;
#[cfg_attr(target_os = "linux", path = "platform/linux/icons.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/icons.rs")]
pub mod icons;
#[cfg_attr(target_os = "linux", path = "platform/linux/input.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/input.rs")]
pub mod input;
pub mod platform;
pub mod server;
#[cfg_attr(target_os = "linux", path = "platform/linux/smtc.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/smtc.rs")]
pub mod smtc;
#[cfg_attr(target_os = "linux", path = "platform/linux/uia.rs")]
#[cfg_attr(all(not(windows), not(target_os = "linux")), path = "platform/stub/uia.rs")]
pub mod uia;
pub mod textfocus;
pub mod vault;
pub mod webremote;

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
        let c = core.clone();
        std::thread::spawn(move || audio::run(c));
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
