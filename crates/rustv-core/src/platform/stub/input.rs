//! STUB non-Windows : injection d'entrées (clavier, souris, fenêtres).
//!
//! Contrat pour le vrai backend (Linux : uinput ou XTest/libei ; macOS : CGEvent) :
//! - `key_down/key_up/tap/chord` : événements clavier vers la fenêtre au premier plan ;
//! - `type_text` : saisit du texte Unicode (dispositions de clavier incluses) ;
//! - `mouse_*` : mouvement relatif, clic gauche/droit, molette (delta > 0 vers le haut) ;
//! - `foreground`, `foreground_title`, `set_topmost`, `force_foreground`, `is_window` : gestion de fenêtres
//!   (non applicable au début sous Linux/Wayland : laisser les valeurs neutres ci-dessous).
//!
//! Les codes de touches ci-dessous sont des identifiants logiques ; le backend les traduit en codes natifs.
//! Tant que ce n'est pas fait, `check()` renvoie `Err("unsupported_platform")` pour que l'appelant l'affiche ;
//! les fonctions ne font rien (l'API historique ne renvoie pas de résultat).
use std::sync::atomic::{AtomicBool, Ordering};

/// Identifiant logique de touche (mêmes valeurs que les codes virtuels Windows).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(non_camel_case_types)]
pub struct VIRTUAL_KEY(pub u16);

/// Poignée de fenêtre : même forme que `windows::Win32::Foundation::HWND` (`.0` est un pointeur).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HWND(pub *mut std::ffi::c_void);

#[allow(dead_code)]
pub mod keys {
    use super::VIRTUAL_KEY;
    pub const VK_BACK: VIRTUAL_KEY = VIRTUAL_KEY(0x08);
    pub const VK_TAB: VIRTUAL_KEY = VIRTUAL_KEY(0x09);
    pub const VK_RETURN: VIRTUAL_KEY = VIRTUAL_KEY(0x0D);
    pub const VK_MENU: VIRTUAL_KEY = VIRTUAL_KEY(0x12);
    pub const VK_ESCAPE: VIRTUAL_KEY = VIRTUAL_KEY(0x1B);
    pub const VK_LEFT: VIRTUAL_KEY = VIRTUAL_KEY(0x25);
    pub const VK_UP: VIRTUAL_KEY = VIRTUAL_KEY(0x26);
    pub const VK_RIGHT: VIRTUAL_KEY = VIRTUAL_KEY(0x27);
    pub const VK_DOWN: VIRTUAL_KEY = VIRTUAL_KEY(0x28);
    pub const VK_APPS: VIRTUAL_KEY = VIRTUAL_KEY(0x5D);
    pub const VK_F3: VIRTUAL_KEY = VIRTUAL_KEY(0x72);
    pub const VK_VOLUME_MUTE: VIRTUAL_KEY = VIRTUAL_KEY(0xAD);
    pub const VK_VOLUME_DOWN: VIRTUAL_KEY = VIRTUAL_KEY(0xAE);
    pub const VK_VOLUME_UP: VIRTUAL_KEY = VIRTUAL_KEY(0xAF);
    pub const VK_MEDIA_NEXT_TRACK: VIRTUAL_KEY = VIRTUAL_KEY(0xB0);
    pub const VK_MEDIA_PREV_TRACK: VIRTUAL_KEY = VIRTUAL_KEY(0xB1);
    pub const VK_MEDIA_STOP: VIRTUAL_KEY = VIRTUAL_KEY(0xB2);
    pub const VK_MEDIA_PLAY_PAUSE: VIRTUAL_KEY = VIRTUAL_KEY(0xB3);
}

/// Toujours `Err("unsupported_platform")` tant qu'aucun backend n'est implémenté.
pub fn check() -> Result<(), &'static str> {
    Err(crate::platform::UNSUPPORTED)
}

/// Signale une seule fois (sans donnée sensible) qu'une entrée a été ignorée.
fn unsupported() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("rustv-core: input {}", crate::platform::UNSUPPORTED);
    }
}

pub fn key_down(_vk: VIRTUAL_KEY) {
    unsupported();
}
pub fn key_up(_vk: VIRTUAL_KEY) {
    unsupported();
}
pub fn tap(_vk: VIRTUAL_KEY) {
    unsupported();
}
pub fn chord(_mods: &[VIRTUAL_KEY], _vk: VIRTUAL_KEY) {
    unsupported();
}
pub fn type_text(_text: &str) {
    unsupported();
}
pub fn mouse_move(_dx: i32, _dy: i32) {
    unsupported();
}
pub fn mouse_click(_right: bool) {
    unsupported();
}
pub fn mouse_scroll(_delta: i32) {
    unsupported();
}

// ---- Fenêtres (non applicable : valeurs neutres) ----

pub fn foreground() -> HWND {
    HWND(std::ptr::null_mut())
}
pub fn foreground_title() -> String {
    String::new()
}
pub fn set_topmost(_hwnd: HWND, _on: bool) {}
pub fn force_foreground(_hwnd: isize) {}
pub fn is_window(_hwnd: HWND) -> bool {
    false
}
