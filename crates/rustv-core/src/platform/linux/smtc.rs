//! Linux : média en cours de lecture et commandes de lecture, via MPRIS (D-Bus `org.mpris.MediaPlayer2.*`).
//!
//! Fonctionne avec tout lecteur qui s'y déclare (Firefox, Chromium, Spotify, VLC, mpv avec le script
//! mpv-mpris…). Le lecteur RusTV (mpv lancé par le shell) a déjà la priorité dans `Core::np_msg` : cette
//! source ne sert que pour les autres lecteurs. Sans bus de session D-Bus, rien n'est détecté (pas d'erreur).
//! Les événements envoyés au téléphone sont identiques à ceux du module Windows (`Core::set_smtc`).
use crate::core::{Core, NowPlaying};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use zbus::blocking::{fdo::DBusProxy, Connection, Proxy};
use zbus::zvariant::OwnedValue;

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const IFACE: &str = "org.mpris.MediaPlayer2.Player";

/// Métadonnées MPRIS utiles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub title: String,
    pub artists: Vec<String>,
    pub length_us: Option<i64>,
}

/// État d'un lecteur à un instant donné.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub bus: String,
    pub status: String,
    pub meta: Meta,
    pub position_us: i64,
}

/// `org.mpris.MediaPlayer2.firefox.instance123` → `firefox`.
pub fn source_name(bus: &str) -> String {
    let n = bus.strip_prefix(PREFIX).unwrap_or(bus);
    n.split('.').next().unwrap_or(n).to_string()
}

/// Lecteur à suivre : celui qui joue, sinon un lecteur en pause ; les lecteurs sans titre sont ignorés.
pub fn pick(snaps: &[Snapshot]) -> Option<usize> {
    let ok = |s: &Snapshot| !s.meta.title.is_empty();
    snaps
        .iter()
        .position(|s| s.status == "Playing" && ok(s))
        .or_else(|| snaps.iter().position(|s| s.status == "Paused" && ok(s)))
}

/// Correspondance MPRIS → `NowPlaying` (secondes).
pub fn to_now_playing(s: &Snapshot) -> NowPlaying {
    NowPlaying {
        title: s.meta.title.clone(),
        artist: s.meta.artists.join(", "),
        playing: s.status == "Playing",
        position: (s.position_us.max(0) as f64) / 1e6,
        duration: (s.meta.length_us.unwrap_or(0).max(0) as f64) / 1e6,
        source: source_name(&s.bus),
    }
}

fn meta_from(map: &HashMap<String, OwnedValue>) -> Meta {
    let text = |k: &str| map.get(k).and_then(|v| v.try_clone().ok()).and_then(|v| String::try_from(v).ok());
    let artists = map
        .get("xesam:artist")
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| Vec::<String>::try_from(v).ok())
        .or_else(|| text("xesam:artist").map(|a| vec![a]))
        .unwrap_or_default();
    // `mpris:length` est un entier 64 bits signé ou non selon les lecteurs.
    let length_us = map.get("mpris:length").and_then(|v| {
        v.try_clone().ok().and_then(|v| i64::try_from(v).ok()).or_else(|| v.try_clone().ok().and_then(|v| u64::try_from(v).ok()).map(|u| u as i64))
    });
    Meta { title: text("xesam:title").unwrap_or_default(), artists, length_us }
}

fn proxy<'a>(conn: &'a Connection, bus: &str) -> Option<Proxy<'a>> {
    Proxy::new(conn, bus.to_string(), PATH, IFACE).ok()
}

fn players(conn: &Connection) -> Vec<String> {
    let Ok(dbus) = DBusProxy::new(conn) else { return Vec::new() };
    let mut v: Vec<String> = dbus.list_names().map(|n| n.iter().map(|x| x.to_string()).filter(|n| n.starts_with(PREFIX)).collect()).unwrap_or_default();
    v.sort();
    v
}

fn snapshot(conn: &Connection, bus: &str) -> Option<Snapshot> {
    let p = proxy(conn, bus)?;
    let status: String = p.get_property("PlaybackStatus").ok()?;
    let md: HashMap<String, OwnedValue> = p.get_property("Metadata").unwrap_or_default();
    let position_us: i64 = p.get_property("Position").unwrap_or(0);
    Some(Snapshot { bus: bus.to_string(), status, meta: meta_from(&md), position_us })
}

fn snapshots(conn: &Connection) -> Vec<Snapshot> {
    players(conn).iter().filter_map(|b| snapshot(conn, b)).collect()
}

fn session() -> Option<Connection> {
    Connection::session().ok()
}

/// Envoie `PlayPause`, `Next`, `Previous` ou `Stop` au lecteur actif. Faux s'il n'y en a pas (l'appelant envoie
/// alors la vraie touche multimédia).
pub fn control(method: &str) -> bool {
    if !matches!(method, "PlayPause" | "Next" | "Previous" | "Stop") {
        return false;
    }
    let Some(conn) = session() else { return false };
    let snaps = snapshots(&conn);
    // Sans titre (lecteur inactif), un lecteur existant reste une cible valable pour PlayPause.
    let idx = pick(&snaps).or_else(|| (!snaps.is_empty()).then_some(0));
    let Some(s) = idx.map(|i| &snaps[i]) else { return false };
    proxy(&conn, &s.bus).map(|p| p.call_method(method, &()).is_ok()).unwrap_or(false)
}

/// Sonde toutes les 2 s tant qu'un téléphone est connecté.
pub fn run(core: Arc<Core>) {
    let mut conn: Option<Connection> = None;
    let mut last: Option<NowPlaying> = None;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        if core.conn_count() == 0 {
            continue;
        }
        if conn.is_none() {
            conn = session();
        }
        let Some(c) = conn.as_ref() else { continue };
        let snaps = snapshots(c);
        let cur = pick(&snaps).map(|i| to_now_playing(&snaps[i]));
        let changed = match (&last, &cur) {
            (Some(a), Some(b)) => a.title != b.title || a.artist != b.artist || a.playing != b.playing || (a.position - b.position).abs() >= 5.0,
            (None, None) => false,
            _ => true,
        };
        if changed {
            core.set_smtc(cur.clone());
            last = cur;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(bus: &str, status: &str, title: &str) -> Snapshot {
        Snapshot { bus: bus.into(), status: status.into(), meta: Meta { title: title.into(), artists: vec!["A".into(), "B".into()], length_us: Some(215_000_000) }, position_us: 61_500_000 }
    }

    #[test]
    fn mapping_to_now_playing() {
        let n = to_now_playing(&snap("org.mpris.MediaPlayer2.spotify", "Playing", "Song"));
        assert_eq!(n.title, "Song");
        assert_eq!(n.artist, "A, B");
        assert!(n.playing);
        assert!((n.position - 61.5).abs() < 1e-9 && (n.duration - 215.0).abs() < 1e-9);
        assert_eq!(n.source, "spotify");
        let p = to_now_playing(&snap("org.mpris.MediaPlayer2.firefox.instance_1_2", "Paused", "T"));
        assert!(!p.playing);
        assert_eq!(p.source, "firefox");
        let neg = Snapshot { position_us: -5, ..Snapshot::default() };
        assert_eq!(to_now_playing(&neg).position, 0.0);
    }

    #[test]
    fn picks_playing_then_paused() {
        let s = vec![snap("a", "Paused", "x"), snap("b", "Playing", "y"), snap("c", "Stopped", "z")];
        assert_eq!(pick(&s), Some(1));
        let s2 = vec![snap("a", "Stopped", "x"), snap("b", "Paused", "y")];
        assert_eq!(pick(&s2), Some(1));
        assert_eq!(pick(&[snap("a", "Playing", "")]), None);
        assert_eq!(pick(&[]), None);
    }

    #[test]
    fn source_names() {
        assert_eq!(source_name("org.mpris.MediaPlayer2.vlc"), "vlc");
        assert_eq!(source_name("org.mpris.MediaPlayer2.chromium.instance42"), "chromium");
        assert_eq!(source_name("other"), "other");
    }

    #[test]
    fn rejects_unknown_methods() {
        assert!(!control("Quit"));
    }
}
