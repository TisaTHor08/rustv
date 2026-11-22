//! Préférences de l'interface (thème, fond d'écran, tuiles…) : un document JSON détenu par l'interface,
//! persisté ici dans %APPDATA%\RusTV\prefs.json (écriture atomique, taille bornée).
use crate::settings;
use serde_json::Value;

const MAX_BYTES: usize = 1_000_000;

pub fn get() -> Value {
    std::fs::read_to_string(settings::dir().join("prefs.json"))
        .ok()
        .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or(Value::Null)
}

pub fn set(v: &Value) -> Result<(), String> {
    let s = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    if s.len() > MAX_BYTES {
        return Err("Préférences trop volumineuses".into());
    }
    let d = settings::dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let tmp = d.join("prefs.json.tmp");
    std::fs::write(&tmp, s).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, d.join("prefs.json")).map_err(|e| e.to_string())
}
