//! Fonctionnalités réellement disponibles sur ce système (pour que les pages de réglages masquent ou
//! signalent ce qui est un stub), erreurs d'installation propres à chaque OS et réglages propres à Linux.
use serde::Serialize;
use serde_json::{json, Value};

/// Erreur renvoyée par `player::install` / `hyperion::install` hors Windows : sous Linux, la commande du
/// gestionnaire de paquets (`use_package_manager:sudo apt install mpv`).
pub fn install_hint(package: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        crate::linux::pkg::use_package_manager(package)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = package;
        rustv_core::platform::UNSUPPORTED.to_string()
    }
}

/// Erreur de `update::install` hors Windows : sous Linux, mise à jour manuelle (le lien de la version reste affiché).
pub fn update_hint() -> String {
    if cfg!(target_os = "linux") {
        "manual_update".to_string()
    } else {
        rustv_core::platform::UNSUPPORTED.to_string()
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Features {
    pub os: &'static str,
    /// Injection de touches et de souris depuis le téléphone.
    pub input: bool,
    /// Code d'erreur stable quand `input` est faux (`uinput_denied`, `unsupported_platform`).
    pub input_error: Option<String>,
    pub volume: bool,
    pub audio_outputs: bool,
    pub apps: bool,
    pub media_session: bool,
    pub text_focus: bool,
    pub secrets: bool,
    pub autostart: bool,
    pub guard: bool,
    pub display_modes: bool,
    pub hdr: bool,
    pub gamepad: bool,
    pub bluetooth: bool,
    /// Installation automatique de mpv / Hyperion / mises à jour (sinon : indication du gestionnaire de paquets).
    pub self_install: bool,
    /// Applications DRM ouvertes dans un navigateur Chromium (Linux).
    pub streaming_browser: bool,
}

/// Valeurs de la plateforme. Sous Windows tout est disponible (comportement historique).
pub fn current() -> Features {
    #[cfg(windows)]
    {
        Features {
            os: "windows",
            input: true,
            input_error: None,
            volume: true,
            audio_outputs: true,
            apps: true,
            media_session: true,
            text_focus: true,
            secrets: true,
            autostart: true,
            guard: true,
            display_modes: true,
            hdr: true,
            gamepad: true,
            bluetooth: true,
            self_install: true,
            streaming_browser: false,
        }
    }
    #[cfg(target_os = "linux")]
    {
        let input = rustv_core::input::check();
        let audio = rustv_core::audio::available();
        Features {
            os: "linux",
            input: input.is_ok(),
            input_error: input.err().map(str::to_string),
            volume: audio,
            audio_outputs: audio,
            apps: true,
            media_session: true,
            // Les vues web de RusTV signalent leurs champs ; les autres applications ne sont pas détectées.
            text_focus: false,
            secrets: crate::linux::secrets::available(),
            autostart: true,
            guard: false,
            display_modes: false,
            hdr: false,
            gamepad: false,
            bluetooth: false,
            self_install: false,
            streaming_browser: true,
        }
    }
    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        let input = rustv_core::input::check();
        Features {
            os: "other",
            input: input.is_ok(),
            input_error: input.err().map(str::to_string),
            volume: false,
            audio_outputs: false,
            apps: false,
            media_session: false,
            text_focus: false,
            secrets: false,
            autostart: false,
            guard: false,
            display_modes: false,
            hdr: false,
            gamepad: false,
            bluetooth: false,
            self_install: false,
            streaming_browser: false,
        }
    }
}

/// Fonctionnalités disponibles : les pages de réglages masquent ou grisent ce qui est à `false`.
#[tauri::command]
pub async fn platform_features() -> Result<Features, String> {
    tauri::async_runtime::spawn_blocking(current).await.map_err(|e| e.to_string())
}

/// Réglages propres à Linux : `{ supported, streamingBrowser, streamingBrowserKind }`.
#[tauri::command]
pub async fn linux_get() -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        let c = crate::linux::streaming::load();
        let found: Vec<String> = crate::browser::detect().iter().map(|f| format!("{:?}", f.kind).to_lowercase()).collect();
        Ok(json!({ "supported": true, "streamingBrowser": c.streaming_browser, "streamingBrowserKind": c.streaming_browser_kind, "browsersFound": found }))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(json!({ "supported": false }))
    }
}

/// Modifie un réglage Linux (champs absents = inchangés).
#[tauri::command]
pub async fn linux_set(streaming_browser: Option<bool>, streaming_browser_kind: Option<String>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let mut c = crate::linux::streaming::load();
        if let Some(b) = streaming_browser {
            c.streaming_browser = b;
        }
        if let Some(k) = streaming_browser_kind {
            c.streaming_browser_kind = k;
        }
        crate::linux::streaming::save(&c)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (streaming_browser, streaming_browser_kind);
        Err(rustv_core::platform::UNSUPPORTED.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_are_short_stable_codes() {
        assert!(!update_hint().is_empty());
        let h = install_hint("mpv");
        assert!(h.starts_with("use_package_manager:") || h == "unsupported_platform");
    }

    #[test]
    fn features_serialize_with_camel_free_names() {
        let v = serde_json::to_value(current()).unwrap();
        assert!(v["os"].is_string() && v["guard"].is_boolean() && v["input"].is_boolean());
        #[cfg(not(windows))]
        assert_eq!(v["guard"], json!(false));
    }
}
