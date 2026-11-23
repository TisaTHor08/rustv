//! Historique de lecture (`history.json`) : alimente la rangée « Reprendre la lecture ».
//! La position de reprise fine reste gérée par le « watch-later » de mpv ; ici on garde de quoi afficher les cartes.
use serde::{Deserialize, Serialize};

/// Au-delà de cette part de la durée, l'élément est considéré comme terminé.
pub const FINISHED_RATIO: f64 = 0.92;
pub const MAX_ENTRIES: usize = 200;
pub const MAX_LIST: usize = 20;
/// En dessous, on considère que rien n'a vraiment été regardé.
pub const MIN_POSITION: f64 = 10.0;
/// Les contenus audio ne sont mémorisés qu'au-delà de 10 minutes (podcasts, livres audio).
pub const MIN_AUDIO_DURATION: f64 = 600.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub url: String,
    pub title: String,
    pub kind: String,
    pub position: f64,
    pub duration: f64,
    /// Secondes Unix de la dernière mise à jour.
    pub updated: u64,
    pub poster: Option<String>,
    pub finished: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    pub items: Vec<Entry>,
}

/// Cet élément doit-il être mémorisé ? (vidéo : oui ; audio : seulement s'il est long ; direct : jamais)
pub fn eligible(kind: &str, duration: f64) -> bool {
    match kind {
        "video" => true,
        "audio" => duration > MIN_AUDIO_DURATION,
        _ => false,
    }
}

pub fn is_finished(position: f64, duration: f64) -> bool {
    duration > 0.0 && position / duration >= FINISHED_RATIO
}

impl History {
    /// Enregistre/actualise un élément. `ended` : mpv a atteint la fin du fichier.
    /// Renvoie vrai si l'historique a changé.
    pub fn record(&mut self, mut e: Entry, ended: bool) -> bool {
        if e.url.is_empty() || !eligible(&e.kind, e.duration) || !e.position.is_finite() {
            return false;
        }
        e.finished = ended || is_finished(e.position, e.duration);
        let existing = self.items.iter().position(|x| x.url == e.url);
        if existing.is_none() && e.position < MIN_POSITION && !e.finished {
            return false;
        }
        if let Some(i) = existing {
            let old = self.items.remove(i);
            if e.poster.is_none() {
                e.poster = old.poster;
            }
            if e.title.is_empty() {
                e.title = old.title;
            }
        }
        self.items.insert(0, e);
        self.items.sort_by(|a, b| b.updated.cmp(&a.updated));
        self.items.truncate(MAX_ENTRIES);
        true
    }

    /// Éléments à reprendre : non terminés, déjà entamés, du plus récent au plus ancien, 20 au plus.
    pub fn continue_list(&self) -> Vec<Entry> {
        let mut v: Vec<Entry> = self.items.iter().filter(|e| !e.finished && e.position >= MIN_POSITION).cloned().collect();
        v.sort_by(|a, b| b.updated.cmp(&a.updated));
        v.truncate(MAX_LIST);
        v
    }

    pub fn remove(&mut self, url: &str) -> bool {
        let n = self.items.len();
        self.items.retain(|e| e.url != url);
        self.items.len() != n
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Position de reprise connue pour cette URL (élément non terminé).
    pub fn resume_position(&self, url: &str) -> Option<f64> {
        self.items.iter().find(|e| e.url == url && !e.finished && e.position >= MIN_POSITION).map(|e| e.position)
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(url: &str, kind: &str, pos: f64, dur: f64, updated: u64) -> Entry {
        Entry { url: url.into(), title: url.into(), kind: kind.into(), position: pos, duration: dur, updated, ..Default::default() }
    }

    #[test]
    fn finish_threshold() {
        assert!(!is_finished(91.0, 100.0));
        assert!(is_finished(92.0, 100.0));
        assert!(!is_finished(500.0, 0.0));
    }

    #[test]
    fn record_and_order() {
        let mut h = History::default();
        assert!(h.record(e("a", "video", 100.0, 1000.0, 1), false));
        assert!(h.record(e("b", "video", 50.0, 1000.0, 2), false));
        assert!(h.record(e("a", "video", 200.0, 1000.0, 3), false));
        let l = h.continue_list();
        assert_eq!(l.iter().map(|x| x.url.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(l[0].position, 200.0);
        assert_eq!(h.items.len(), 2);
    }

    #[test]
    fn finished_leaves_continue() {
        let mut h = History::default();
        h.record(e("a", "video", 100.0, 1000.0, 1), false);
        h.record(e("a", "video", 950.0, 1000.0, 2), false);
        assert!(h.continue_list().is_empty());
        let mut h2 = History::default();
        h2.record(e("b", "video", 100.0, 1000.0, 1), true);
        assert!(h2.continue_list().is_empty());
        assert_eq!(h2.resume_position("b"), None);
    }

    #[test]
    fn ignores_live_short_audio_and_barely_started() {
        let mut h = History::default();
        assert!(!h.record(e("l", "live", 100.0, 0.0, 1), false));
        assert!(!h.record(e("m", "audio", 100.0, 200.0, 1), false));
        assert!(h.record(e("p", "audio", 100.0, 3000.0, 1), false));
        assert!(!h.record(e("v", "video", 3.0, 1000.0, 1), false));
        assert_eq!(h.items.len(), 1);
    }

    #[test]
    fn caps_and_remove_clear() {
        let mut h = History::default();
        for i in 0..30u64 {
            h.record(e(&format!("u{i}"), "video", 100.0, 1000.0, i + 1), false);
        }
        let l = h.continue_list();
        assert_eq!(l.len(), MAX_LIST);
        assert_eq!(l[0].url, "u29");
        assert!(h.remove("u29"));
        assert!(!h.remove("u29"));
        assert_eq!(h.continue_list()[0].url, "u28");
        h.clear();
        assert!(h.items.is_empty());
    }

    #[test]
    fn keeps_poster() {
        let mut h = History::default();
        let mut a = e("a", "video", 100.0, 1000.0, 1);
        a.poster = Some("p.jpg".into());
        h.record(a, false);
        h.record(e("a", "video", 120.0, 1000.0, 2), false);
        assert_eq!(h.items[0].poster.as_deref(), Some("p.jpg"));
    }
}
