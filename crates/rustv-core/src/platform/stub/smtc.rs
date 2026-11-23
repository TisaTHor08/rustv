//! STUB non-Windows : média en cours de lecture.
//!
//! Contrat : `run(core)` sonde toutes les ~2 s tant qu'un téléphone est connecté et appelle
//! `core.set_smtc(Some(NowPlaying{..}))` (ou `None`) quand titre, artiste, état ou position (>= 5 s) changent.
//! Linux : MPRIS (D-Bus `org.mpris.MediaPlayer2.*`). macOS : MediaRemote (framework privé) ou AppleScript.
use crate::core::Core;
use std::sync::Arc;

/// Sans backend : rend la main immédiatement (aucun média détecté).
pub fn run(_core: Arc<Core>) {}
