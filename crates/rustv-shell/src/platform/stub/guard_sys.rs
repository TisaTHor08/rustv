//! STUB non-Windows : enveloppes système de la garde TV (`guard.rs`).
//!
//! Contrat pour le vrai backend (mêmes signatures que `guard::sys` côté Windows) :
//! - `toasts_read/toasts_set/toasts_restore` : notifications du système coupées pendant le mode TV
//!   (Windows : registre HKCU ; Linux : `gsettings org.gnome.desktop.notifications show-banners` ou dunstctl ;
//!   macOS : non prévu) ;
//! - `keep_awake_start` : empêche la mise en veille jusqu'à l'envoi sur le `Sender` renvoyé
//!   (Linux : `org.freedesktop.ScreenSaver.Inhibit` / `systemd-inhibit` ; macOS : `IOPMAssertion`) ;
//! - `set_taskbar_visible_all` : barre des tâches (sans objet au départ sous Linux/macOS) ;
//! - `set_event_sender/hook_thread/quit_hook_thread/worker` : surveillance des fenêtres du mode TV
//!   (Windows : WinEventHook ; Linux : non applicable sous Wayland, EWMH sous X11 ; macOS : non prévu).
//!
//! Sans backend, tout est un no-op inoffensif : la garde ne fait rien et ne modifie rien.
use super::*;
use std::sync::mpsc::{Receiver, SyncSender};

pub fn toasts_read() -> Option<u32> {
    None
}

pub fn toasts_set(_v: u32) {}

pub fn toasts_restore(_t: &ToastSaved) {}

/// Le récepteur est abandonné tout de suite : `send` échoue sans conséquence (l'appelant ignore l'erreur).
pub fn keep_awake_start() -> std::sync::mpsc::Sender<()> {
    std::sync::mpsc::channel::<()>().0
}

pub fn set_taskbar_visible_all() {}

pub fn set_event_sender(_tx: Option<SyncSender<isize>>) {}

pub fn hook_thread(_tid: Arc<AtomicU32>) {}

pub fn quit_hook_thread(_tid: u32) {}

pub fn worker(_app: AppHandle, _cfg: GuardCfg, _stop: Arc<AtomicBool>, _rx: Receiver<isize>) {}
