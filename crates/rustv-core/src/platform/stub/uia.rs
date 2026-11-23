//! STUB non-Windows : détection des champs de saisie (focus clavier sur un champ texte).
//!
//! Contrat : `run(core)` boucle tant qu'un téléphone est connecté (`core.conn_count() > 0`) et appelle
//! `core.set_text_focus(true/false)` quand le focus entre ou sort d'un champ éditable.
//! Linux : AT-SPI (D-Bus). macOS : API Accessibility (AXUIElement).
use crate::core::Core;
use std::sync::Arc;

/// Sans backend : rend la main immédiatement ; le focus texte reste faux.
pub fn run(_core: Arc<Core>) {}
