#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod bluetooth;
mod config;
mod core;
mod icons;
mod input;
mod server;
mod ui;
mod uia;

use std::sync::Arc;
use std::time::Duration;

fn main() {
    let core = Arc::new(core::Core::new(config::Config::load()));

    let listener = match server::bind() {
        Ok(l) => l,
        Err(_) => return, // déjà lancé (port occupé)
    };

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
        // Bluetooth : facultatif (PC sans adaptateur ou Bluetooth désactivé → on ignore).
        let c = core.clone();
        std::thread::spawn(move || {
            if let Some(addr) = bluetooth::adapter_address() {
                *c.bt_addr.lock().unwrap() = Some(addr);
                let _ = bluetooth::run(c);
            }
        });
    }
    ui::run(core);
}
