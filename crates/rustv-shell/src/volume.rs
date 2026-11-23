//! Affichage du volume à l'écran : émet l'événement Tauri `volume-changed { level, muted }` quand le volume ou le muet
//! du PC change (touches du clavier, mixeur Windows, téléphone, tuile Domotique...).
//!
//! Le noyau (`rustv_core::audio::run`) interroge déjà Core Audio toutes les 300 ms et mémorise le dernier état dans
//! `Core::volume()` : on se contente de comparer cet état à celui déjà annoncé, sans second accès à Core Audio.
//! La première valeur lue au démarrage n'est jamais annoncée (pas d'affichage parasite au lancement).
use crate::Shell;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct VolumeEvent {
    pub level: u8,
    pub muted: bool,
}

/// Détecteur de changement (pur, testé) : mémorise la dernière valeur vue et ne rend `Some` que sur un vrai changement.
#[derive(Default)]
pub struct Watch {
    last: Option<(u8, bool)>,
}

impl Watch {
    /// `cur` : dernier volume connu du noyau. Le premier relevé sert de référence et ne déclenche rien.
    pub fn update(&mut self, cur: Option<(u8, bool)>) -> Option<VolumeEvent> {
        let cur = cur?;
        match self.last.replace(cur) {
            Some(prev) if prev != cur => Some(VolumeEvent { level: cur.0.min(100), muted: cur.1 }),
            _ => None,
        }
    }
}

/// Démarre le fil de surveillance (léger : un verrou et une comparaison tous les 250 ms).
pub fn start(app: &AppHandle) {
    let h = app.clone();
    std::thread::spawn(move || {
        let mut w = Watch::default();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let cur = h.try_state::<Shell>().and_then(|s| s.core.volume());
            if let Some(ev) = w.update(cur) {
                let _ = h.emit("volume-changed", ev);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_reading_is_silent() {
        let mut w = Watch::default();
        assert_eq!(w.update(None), None);
        assert_eq!(w.update(Some((30, false))), None);
    }

    #[test]
    fn reports_level_and_mute_changes_once() {
        let mut w = Watch::default();
        w.update(Some((30, false)));
        assert_eq!(w.update(Some((30, false))), None);
        assert_eq!(w.update(Some((35, false))), Some(VolumeEvent { level: 35, muted: false }));
        assert_eq!(w.update(Some((35, false))), None);
        assert_eq!(w.update(Some((35, true))), Some(VolumeEvent { level: 35, muted: true }));
        assert_eq!(w.update(Some((0, true))), Some(VolumeEvent { level: 0, muted: true }));
    }

    #[test]
    fn unknown_reading_keeps_reference() {
        let mut w = Watch::default();
        w.update(Some((10, false)));
        assert_eq!(w.update(None), None);
        assert_eq!(w.update(Some((10, false))), None);
    }

    #[test]
    fn serializes_for_the_ui() {
        let v = serde_json::to_value(VolumeEvent { level: 42, muted: true }).unwrap();
        assert_eq!(v, serde_json::json!({"level":42,"muted":true}));
    }
}
