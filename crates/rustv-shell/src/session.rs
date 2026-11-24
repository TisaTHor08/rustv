//! Sauvegarde d'état de la TV : page ouverte, tuile sélectionnée, app en cours. Si la TV est fermée puis rouverte
//! dans les 3 heures, l'accueil retombe là où on était (voir ui/src/lib/Home.svelte). Fichier session.json,
//! écriture atomique, taille bornée ; rien n'est envoyé hors du PC.
use crate::settings;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Durée de validité d'une sauvegarde.
pub const MAX_AGE_MS: u64 = 3 * 3600 * 1000;
const MAX_BYTES: usize = 20_000;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn path() -> std::path::PathBuf {
    settings::dir().join("session.json")
}

fn read_raw() -> Option<Value> {
    let s = std::fs::read_to_string(path()).ok()?;
    serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()
}

fn write(v: &Value) -> Result<(), String> {
    let s = serde_json::to_string(v).map_err(|e| e.to_string())?;
    if s.len() > MAX_BYTES {
        return Err("Sauvegarde trop volumineuse".into());
    }
    let d = settings::dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let tmp = d.join("session.json.tmp");
    std::fs::write(&tmp, s).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path()).map_err(|e| e.to_string())
}

/// Enregistre l'état envoyé par l'interface et l'horodate.
pub fn save(state: &Value) -> Result<(), String> {
    let mut v = if state.is_object() { state.clone() } else { Value::Object(Default::default()) };
    v["ts"] = Value::from(now_ms());
    write(&v)
}

/// Renouvelle l'horodatage (à la fermeture de la TV) : la fenêtre de 3 h court à partir de la fermeture.
pub fn touch() {
    if let Some(mut v) = read_raw() {
        v["ts"] = Value::from(now_ms());
        let _ = write(&v);
    }
}

/// État sauvegardé s'il date de moins de 3 h, sinon null.
pub fn load() -> Value {
    match read_raw() {
        Some(v) if is_fresh(v["ts"].as_u64().unwrap_or(0), now_ms()) => v,
        _ => Value::Null,
    }
}

pub fn clear() {
    let _ = std::fs::remove_file(path());
}

fn is_fresh(ts: u64, now: u64) -> bool {
    ts > 0 && now >= ts && now - ts <= MAX_AGE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_window() {
        let h = 3600 * 1000;
        assert!(is_fresh(1000, 1000 + 2 * h));
        assert!(is_fresh(1000, 1000 + 3 * h));
        assert!(!is_fresh(1000, 1000 + 3 * h + 1));
        assert!(!is_fresh(0, 5));
        assert!(!is_fresh(10_000, 5)); // horloge revenue en arrière : on ignore
    }
}
