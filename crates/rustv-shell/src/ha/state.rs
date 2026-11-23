//! État publié vers Home Assistant : instantané du shell -> couples (topic, charge), et diff pour ne publier que les changements.
use super::topics::Topics;
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Np {
    pub title: String,
    pub artist: String,
    pub source: String,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub tv_on: bool,
    pub now: Option<Np>,
    /// (niveau 0-100, muet) ; `None` si inconnu.
    pub volume: Option<(u8, bool)>,
    pub phones: usize,
    /// Nom de la dernière tuile ouverte (ou en avant-plan si connu).
    pub current_app: Option<String>,
    pub uptime_min: u64,
    pub version: String,
    /// Nom de la tuile courante dans la liste du `select`, si elle y figure.
    pub select_option: Option<String>,
}

/// off / home / playing / paused / app
pub fn ha_state(s: &Snapshot) -> &'static str {
    if !s.tv_on {
        return "off";
    }
    match &s.now {
        Some(n) if n.playing => "playing",
        Some(_) => "paused",
        None if s.current_app.is_some() => "app",
        None => "home",
    }
}

fn onoff(b: bool) -> &'static str {
    if b {
        "ON"
    } else {
        "OFF"
    }
}

/// La position avance chaque seconde : on l'arrondit à 5 s pour ne pas publier en continu.
fn coarse(sec: f64) -> u64 {
    ((sec.max(0.0) / 5.0).floor() as u64) * 5
}

pub fn topics(t: &Topics, s: &Snapshot) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    let mut put = |k: &str, val: String| v.push((t.state(k), val));
    let (np_title, np_artist, np_source) = s.now.as_ref().map(|n| (n.title.clone(), n.artist.clone(), n.source.clone())).unwrap_or_default();
    put("state", ha_state(s).into());
    put(
        "state_attrs",
        json!({
            "tv_mode": s.tv_on,
            "current_app": s.current_app,
            "title": np_title,
            "volume": s.volume.map(|x| x.0),
            "muted": s.volume.map(|x| x.1),
            "phones": s.phones,
        })
        .to_string(),
    );
    put("now_playing", if np_title.is_empty() { "idle".into() } else { np_title.chars().take(250).collect() });
    put(
        "now_playing_attrs",
        json!({
            "artist": np_artist,
            "source": np_source,
            "playing": s.now.as_ref().map(|n| n.playing).unwrap_or(false),
            "position": s.now.as_ref().map(|n| coarse(n.position)).unwrap_or(0),
            "duration": s.now.as_ref().map(|n| n.duration.max(0.0).floor() as u64).unwrap_or(0),
        })
        .to_string(),
    );
    put("current_app", s.current_app.clone().unwrap_or_else(|| "none".into()));
    put("app_launcher", s.select_option.clone().unwrap_or_else(|| "None".into()));
    if let Some((level, muted)) = s.volume {
        put("volume", level.to_string());
        put("muted", onoff(muted).into());
    }
    put("tv_mode", onoff(s.tv_on).into());
    put("phone_connected", onoff(s.phones > 0).into());
    put("phone_attrs", json!({ "count": s.phones }).to_string());
    put("uptime", s.uptime_min.to_string());
    put("version", s.version.clone());
    v
}

/// Dernière charge publiée par topic.
#[derive(Default)]
pub struct StateCache {
    last: HashMap<String, String>,
}

impl StateCache {
    /// Ne garde que ce qui a changé depuis la dernière publication (et mémorise le nouvel état).
    pub fn diff(&mut self, new: Vec<(String, String)>) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (k, v) in new {
            if self.last.get(&k) != Some(&v) {
                self.last.insert(k.clone(), v.clone());
                out.push((k, v));
            }
        }
        out
    }

    /// Après une reconnexion : tout est republié.
    pub fn reset(&mut self) {
        self.last.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot { tv_on: true, version: "1.2.3".into(), volume: Some((30, false)), ..Default::default() }
    }

    fn get(v: &[(String, String)], key: &str) -> String {
        v.iter().find(|(k, _)| k.ends_with(&format!("/state/{key}"))).map(|(_, p)| p.clone()).unwrap_or_default()
    }

    #[test]
    fn states() {
        let mut s = snap();
        assert_eq!(ha_state(&s), "home");
        s.current_app = Some("Plex".into());
        assert_eq!(ha_state(&s), "app");
        s.now = Some(Np { title: "T".into(), playing: false, ..Default::default() });
        assert_eq!(ha_state(&s), "paused");
        s.now.as_mut().unwrap().playing = true;
        assert_eq!(ha_state(&s), "playing");
        s.tv_on = false;
        assert_eq!(ha_state(&s), "off");
    }

    #[test]
    fn topics_content_is_valid() {
        let t = Topics::new("rustv/x");
        let mut s = snap();
        s.now = Some(Np { title: "Song".into(), artist: "Me".into(), source: "Spotify".into(), playing: true, position: 63.4, duration: 200.9 });
        let v = topics(&t, &s);
        assert_eq!(get(&v, "state"), "playing");
        assert_eq!(get(&v, "volume"), "30");
        assert_eq!(get(&v, "muted"), "OFF");
        assert_eq!(get(&v, "tv_mode"), "ON");
        assert_eq!(get(&v, "phone_connected"), "OFF");
        for k in ["state_attrs", "now_playing_attrs", "phone_attrs"] {
            let j: serde_json::Value = serde_json::from_str(&get(&v, k)).unwrap();
            assert!(j.is_object());
        }
        let a: serde_json::Value = serde_json::from_str(&get(&v, "now_playing_attrs")).unwrap();
        assert_eq!(a["position"], 60);
        assert_eq!(a["duration"], 200);
        assert_eq!(a["source"], "Spotify");
        assert!(v.iter().all(|(k, _)| k.starts_with("rustv/x/state/")));
    }

    #[test]
    fn unknown_volume_is_not_published() {
        let t = Topics::new("b");
        let mut s = snap();
        s.volume = None;
        let v = topics(&t, &s);
        assert!(get(&v, "volume").is_empty() && get(&v, "muted").is_empty());
    }

    #[test]
    fn diff_publishes_only_changes() {
        let t = Topics::new("b");
        let mut c = StateCache::default();
        let mut s = snap();
        let first = c.diff(topics(&t, &s));
        assert!(first.len() > 5);
        assert!(c.diff(topics(&t, &s)).is_empty());
        s.volume = Some((31, false));
        let d = c.diff(topics(&t, &s));
        assert!(d.contains(&("b/state/volume".to_string(), "31".to_string())));
        assert_eq!(d.len(), 2); // volume + attributs JSON
        // la position ne change le diff que par pas de 5 s
        s.now = Some(Np { title: "x".into(), playing: true, position: 10.0, duration: 100.0, ..Default::default() });
        c.diff(topics(&t, &s));
        s.now.as_mut().unwrap().position = 13.0;
        assert!(c.diff(topics(&t, &s)).is_empty());
        s.now.as_mut().unwrap().position = 15.2;
        assert_eq!(c.diff(topics(&t, &s)).len(), 1);
        c.reset();
        assert_eq!(c.diff(topics(&t, &s)).len(), first.len());
    }
}
