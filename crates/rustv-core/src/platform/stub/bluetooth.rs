//! STUB non-Windows : service Bluetooth RFCOMM (secours du Wi-Fi), facultatif.
//!
//! Contrat : `adapter_address()` renvoie l'adresse du PC « AA:BB:CC:DD:EE:FF » ou `None` sans Bluetooth ;
//! `run(core)` publie le service RFCOMM `rustv_protocol::RFCOMM_UUID` et sert chaque connexion avec
//! `server::serve_session`. Linux : BlueZ (D-Bus, `ProfileManager1`). macOS : non prévu (ignoré).
use crate::core::Core;
use std::sync::Arc;

pub fn adapter_address() -> Option<String> {
    None
}

pub fn run(_core: Arc<Core>) -> Result<(), String> {
    Err(crate::platform::UNSUPPORTED.to_string())
}
