//! STUB non-Windows : appels système de `display.rs` (sorties audio, modes d'affichage, HDR).
//!
//! Contrat pour le vrai backend (mêmes signatures, mêmes types que `display::sys` côté Windows) :
//! - `outputs()` : sorties audio actives (id stable, nom, par défaut, volume 0..=100, muet) ;
//!   Linux : `wpctl status` / `pactl list sinks` ; macOS : CoreAudio.
//! - `set_default(id)`, `set_volume(id, percent)` : `wpctl set-default` / `pactl set-default-sink` ; CoreAudio.
//! - `monitors()` : écrans actifs triés par x puis y (`Raw`) ; Linux : `xrandr --listmonitors` (X11) ou
//!   `wlr-randr` (Wayland) ; macOS : CoreGraphics (`CGGetActiveDisplayList`).
//! - `raw_modes(device)` / `current_mode(device)` / `apply(device, mode, persist)` : liste, mode courant et
//!   changement de mode (`xrandr --output X --mode WxH --rate R`, `wlr-randr`, `CGDisplaySetDisplayMode`).
//! - `target_of` / `hdr_get` / `hdr_set` : HDR (sans équivalent simple sous Linux ; CoreGraphics sous macOS).
//!
//! Sans backend : listes vides, `None`, ou `Err("unsupported_platform")` ; jamais de faux succès.
use super::*;

const UNSUPPORTED: &str = rustv_core::platform::UNSUPPORTED;

pub fn outputs() -> Result<Vec<Output>, String> {
    Err(UNSUPPORTED.to_string())
}

pub fn set_default(_id: &str) -> Result<(), String> {
    Err(UNSUPPORTED.to_string())
}

pub fn set_volume(_id: &str, _percent: u32) -> Result<(), String> {
    Err(UNSUPPORTED.to_string())
}

/// Écran brut tel que renvoyé par le système.
pub struct Raw {
    pub device: String,
    pub name: String,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub bits: u32,
}

pub fn monitors() -> Vec<Raw> {
    Vec::new()
}

pub fn raw_modes(_device: &str) -> Vec<RawMode> {
    Vec::new()
}

pub fn current_mode(_device: &str) -> Option<Mode> {
    None
}

pub fn apply(_device: &str, _m: Mode, _persist: bool) -> Result<(), String> {
    Err(UNSUPPORTED.to_string())
}

/// Identifiant de la sortie vidéo pour le HDR (adaptateur + id sous Windows).
#[derive(Clone, Copy)]
pub struct Target;

pub fn target_of(_device: &str) -> Option<Target> {
    None
}

pub fn hdr_get(_t: Target) -> Hdr {
    Hdr::default()
}

pub fn hdr_set(_t: Target, _on: bool) -> Result<(), String> {
    Err(UNSUPPORTED.to_string())
}
