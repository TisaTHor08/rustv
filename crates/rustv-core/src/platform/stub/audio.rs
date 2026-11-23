//! STUB non-Windows : volume principal du système.
//!
//! Contrat pour le vrai backend : `get()` renvoie (volume 0..=100, muet) du périphérique de sortie par défaut ;
//! `set(level)` règle le volume (0..=100) et lève le muet si `level > 0` ; `run(core)` boucle en appelant
//! `core.set_volume(level, muted)` quand le volume change (sondage ~300 ms ou événements).
//! Linux : `wpctl get-volume/set-volume @DEFAULT_AUDIO_SINK@` (PipeWire) ou `pactl`. macOS : CoreAudio.
use crate::core::Core;
use std::sync::Arc;

pub fn get() -> Option<(u8, bool)> {
    None
}

pub fn set(_level: u8) {}

/// Sans backend : rend la main immédiatement (le thread se termine).
pub fn run(_core: Arc<Core>) {}
