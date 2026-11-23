//! Linux : ouverture des applications de streaming dans un navigateur de la famille Chromium.
//!
//! WebKitGTK (la vue web de Tauri sous Linux) n'a pas de module de déchiffrement Widevine : Netflix, Prime Video,
//! Disney+ et autres services protégés (DRM) n'y lisent pas. Google Chrome embarque Widevine ; Brave et Vivaldi
//! le proposent selon leur configuration ; Chromium des distributions en est le plus souvent dépourvu.
//! Réglage global `streaming_browser` (défaut : activé, sans effet si aucun navigateur n'est trouvé) et clé
//! facultative `drm` d'une tuile (`true` force, `false` exclut).
use crate::browser::{Found, Kind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tuiles du catalogue dont le contenu est protégé (DRM).
pub const DRM_TILES: &[&str] =
    &["netflix", "prime", "disney", "max", "appletv", "canal", "crunchyroll", "paramount", "ocs", "rakuten", "molotov", "spotify", "applemusic", "tidal"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Cfg {
    /// Ouvrir les applications protégées par DRM dans un navigateur Chromium.
    pub streaming_browser: bool,
    /// `auto`, `chrome`, `brave`, `vivaldi` ou `chromium`.
    pub streaming_browser_kind: String,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg { streaming_browser: true, streaming_browser_kind: "auto".into() }
    }
}

pub fn load() -> Cfg {
    crate::store::load("linux")
}

pub fn save(c: &Cfg) -> Result<(), String> {
    if !["auto", "chrome", "brave", "vivaldi", "chromium"].contains(&c.streaming_browser_kind.as_str()) {
        return Err("invalid_browser".into());
    }
    crate::store::save("linux", c)
}

/// Cette tuile doit-elle s'ouvrir dans un navigateur externe ?
pub fn is_drm_tile(id: &str, tile: &Value) -> bool {
    match tile["drm"].as_bool() {
        Some(b) => b,
        None => DRM_TILES.contains(&id),
    }
}

/// Navigateur à utiliser : ordre de préférence Chrome (Widevine inclus), Brave, Vivaldi, Chromium.
pub fn pick_kind(kind: &str, found: &[Found]) -> Option<Found> {
    let order: &[Kind] = match kind {
        "chrome" => &[Kind::Chrome],
        "brave" => &[Kind::Brave],
        "vivaldi" => &[Kind::Vivaldi],
        "chromium" => &[Kind::Chromium],
        _ => &[Kind::Chrome, Kind::Brave, Kind::Vivaldi, Kind::Chromium],
    };
    order.iter().find_map(|k| found.iter().find(|f| f.kind == *k)).cloned()
}

/// Sélection complète pour une tuile (None : WebKitGTK).
pub fn select(id: &str, tile: &Value, found: &[Found], cfg: &Cfg) -> Option<Found> {
    if !cfg.streaming_browser || !is_drm_tile(id, tile) {
        return None;
    }
    let kind = tile["browser"].as_str().filter(|b| *b != "auto" && *b != "builtin").unwrap_or(&cfg.streaming_browser_kind);
    if tile["browser"].as_str() == Some("builtin") {
        return None;
    }
    pick_kind(kind, found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn f(k: Kind) -> Found {
        Found { kind: k, path: PathBuf::from("/x") }
    }

    #[test]
    fn drm_tiles_and_overrides() {
        assert!(is_drm_tile("netflix", &json!({})));
        assert!(!is_drm_tile("youtube", &json!({})));
        assert!(is_drm_tile("custom", &json!({ "drm": true })));
        assert!(!is_drm_tile("netflix", &json!({ "drm": false })));
    }

    #[test]
    fn order_prefers_chrome() {
        let all = [f(Kind::Chromium), f(Kind::Brave), f(Kind::Chrome)];
        assert_eq!(pick_kind("auto", &all).unwrap().kind, Kind::Chrome);
        assert_eq!(pick_kind("auto", &all[..2]).unwrap().kind, Kind::Brave);
        assert_eq!(pick_kind("chromium", &all).unwrap().kind, Kind::Chromium);
        assert!(pick_kind("vivaldi", &all).is_none());
        assert!(pick_kind("auto", &[]).is_none());
    }

    #[test]
    fn global_switch_and_tile_choice() {
        let found = [f(Kind::Chrome), f(Kind::Brave)];
        let on = Cfg::default();
        assert_eq!(select("netflix", &json!({}), &found, &on).unwrap().kind, Kind::Chrome);
        assert_eq!(select("netflix", &json!({ "browser": "brave" }), &found, &on).unwrap().kind, Kind::Brave);
        assert!(select("netflix", &json!({ "browser": "builtin" }), &found, &on).is_none());
        assert!(select("youtube", &json!({}), &found, &on).is_none());
        let off = Cfg { streaming_browser: false, ..Cfg::default() };
        assert!(select("netflix", &json!({}), &found, &off).is_none());
    }

    #[test]
    fn default_is_on_and_save_validates() {
        assert!(Cfg::default().streaming_browser);
        assert!(save(&Cfg { streaming_browser: true, streaming_browser_kind: "firefox".into() }).is_err());
    }
}
