//! Petit stockage JSON partagé par les modules (garde TV, HA, Hyperion…) : `%APPDATA%\RusTV\<nom>.json`,
//! écriture atomique, taille bornée. Le contenu non valide est ignoré (valeurs par défaut).
use crate::settings;
use serde::{de::DeserializeOwned, Serialize};

const MAX_BYTES: usize = 512 * 1024;

fn valid(name: &str) -> bool {
    !name.is_empty() && name.len() <= 40 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Lit `<nom>.json` ; `T::default()` si absent ou illisible.
pub fn load<T: DeserializeOwned + Default>(name: &str) -> T {
    if !valid(name) {
        return T::default();
    }
    std::fs::read_to_string(settings::dir().join(format!("{name}.json")))
        .ok()
        .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

/// Écrit `<nom>.json` de façon atomique.
pub fn save<T: Serialize>(name: &str, v: &T) -> Result<(), String> {
    if !valid(name) {
        return Err("nom de stockage invalide".into());
    }
    let s = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    if s.len() > MAX_BYTES {
        return Err("données trop volumineuses".into());
    }
    let d = settings::dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let tmp = d.join(format!("{name}.json.tmp"));
    std::fs::write(&tmp, s).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, d.join(format!("{name}.json"))).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::valid;

    #[test]
    fn names() {
        assert!(valid("guard") && valid("home-assistant") && !valid("../x") && !valid("A") && !valid(""));
    }
}
