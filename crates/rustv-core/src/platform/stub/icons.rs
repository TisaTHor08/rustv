//! STUB non-Windows : icône d'une appli/raccourci en PNG base64.
//!
//! Contrat : `icon_png_base64(target)` renvoie un PNG RGBA encodé en base64 ou `None` ; `com_init()` est appelé
//! une fois par thread d'extraction (sans objet hors Windows).
//! Linux : icône du fichier `.desktop` via le thème d'icônes (hicolor). macOS : `.icns` du bundle.
pub fn com_init() {}

pub fn icon_png_base64(_path: &str) -> Option<String> {
    None
}
