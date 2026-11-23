//! Commandes reçues sur `<base>/cmd/#` : validation stricte, limitation de débit, traduction en `PhoneMsg`.
use rustv_protocol::messages::{Key, MediaAction, PhoneMsg};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Boutons Home Assistant : (charge utile, nom de l'entité).
pub const BUTTONS: &[(&str, &str)] = &[
    ("home", "Home"),
    ("play_pause", "Play/Pause"),
    ("next", "Next"),
    ("previous", "Previous"),
    ("stop", "Stop"),
    ("volume_up", "Volume up"),
    ("volume_down", "Volume down"),
    ("key_up", "Key up"),
    ("key_down", "Key down"),
    ("key_left", "Key left"),
    ("key_right", "Key right"),
    ("key_ok", "Key OK"),
    ("key_back", "Key Back"),
    ("key_menu", "Key Menu"),
    ("key_search", "Key Search"),
];

#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Tv(bool),
    Mute(bool),
    Volume(u8),
    /// Nom affiché d'une tuile (entité `select`).
    App(String),
    /// Identifiant de tuile (scripts).
    OpenTile(String),
    Button(&'static str),
}

const MAX_PAYLOAD: usize = 200;

fn on_off(p: &str) -> Option<bool> {
    match p.to_ascii_lowercase().as_str() {
        "on" | "true" => Some(true),
        "off" | "false" => Some(false),
        _ => None,
    }
}

/// Identifiant de tuile : mêmes caractères que ceux acceptés pour les mods.
pub fn valid_tile_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || "-_:".contains(c))
}

/// `sub` = partie après `<base>/cmd/`. Topic inconnu ou charge invalide -> `Err` (ignoré).
pub fn parse(sub: &str, payload: &[u8]) -> Result<Cmd, &'static str> {
    if payload.len() > MAX_PAYLOAD {
        return Err("payload_too_long");
    }
    let p = std::str::from_utf8(payload).map_err(|_| "payload_not_utf8")?.trim();
    match sub {
        "tv" => on_off(p).map(Cmd::Tv).ok_or("bad_payload"),
        "mute" => on_off(p).map(Cmd::Mute).ok_or("bad_payload"),
        "volume" => {
            let v: f64 = p.parse().map_err(|_| "bad_payload")?;
            if v.is_finite() && (0.0..=100.0).contains(&v) {
                Ok(Cmd::Volume(v.round() as u8))
            } else {
                Err("bad_payload")
            }
        }
        "app" => {
            if p.is_empty() || p.chars().count() > 100 || p.chars().any(|c| c.is_control()) {
                Err("bad_payload")
            } else {
                Ok(Cmd::App(p.to_string()))
            }
        }
        "open_tile" => {
            if valid_tile_id(p) {
                Ok(Cmd::OpenTile(p.to_string()))
            } else {
                Err("bad_payload")
            }
        }
        "button" => BUTTONS.iter().find(|(id, _)| *id == p).map(|(id, _)| Cmd::Button(id)).ok_or("bad_payload"),
        _ => Err("unknown_topic"),
    }
}

/// Message du protocole du téléphone pour un bouton validé.
pub fn button_msg(id: &str) -> Option<PhoneMsg> {
    Some(match id {
        "home" => PhoneMsg::Home,
        "play_pause" => PhoneMsg::Media { a: MediaAction::PlayPause },
        "next" => PhoneMsg::Media { a: MediaAction::Next },
        "previous" => PhoneMsg::Media { a: MediaAction::Prev },
        "stop" => PhoneMsg::Media { a: MediaAction::Stop },
        "volume_up" => PhoneMsg::Media { a: MediaAction::VolUp },
        "volume_down" => PhoneMsg::Media { a: MediaAction::VolDown },
        "key_up" => PhoneMsg::Key { k: Key::Up },
        "key_down" => PhoneMsg::Key { k: Key::Down },
        "key_left" => PhoneMsg::Key { k: Key::Left },
        "key_right" => PhoneMsg::Key { k: Key::Right },
        "key_ok" => PhoneMsg::Key { k: Key::Enter },
        "key_back" => PhoneMsg::Key { k: Key::Back },
        "key_menu" => PhoneMsg::Key { k: Key::Menu },
        "key_search" => PhoneMsg::Key { k: Key::Search },
        _ => return None,
    })
}

/// Fenêtre glissante : au plus `max` commandes par `window`.
pub struct RateLimiter {
    max: usize,
    window: Duration,
    hits: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(max: usize, window: Duration) -> Self {
        Self { max, window, hits: VecDeque::new() }
    }

    pub fn allow(&mut self, now: Instant) -> bool {
        while let Some(&t) = self.hits.front() {
            if now.saturating_duration_since(t) >= self.window {
                self.hits.pop_front();
            } else {
                break;
            }
        }
        if self.hits.len() >= self.max {
            return false;
        }
        self.hits.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_off_commands() {
        assert_eq!(parse("tv", b"ON"), Ok(Cmd::Tv(true)));
        assert_eq!(parse("tv", b" off\n"), Ok(Cmd::Tv(false)));
        assert_eq!(parse("mute", b"true"), Ok(Cmd::Mute(true)));
        assert!(parse("tv", b"toggle").is_err());
        assert!(parse("tv", b"").is_err());
    }

    #[test]
    fn volume_bounds() {
        assert_eq!(parse("volume", b"0"), Ok(Cmd::Volume(0)));
        assert_eq!(parse("volume", b"100"), Ok(Cmd::Volume(100)));
        assert_eq!(parse("volume", b"35.0"), Ok(Cmd::Volume(35)));
        assert!(parse("volume", b"101").is_err());
        assert!(parse("volume", b"-1").is_err());
        assert!(parse("volume", b"NaN").is_err());
        assert!(parse("volume", b"loud").is_err());
    }

    #[test]
    fn buttons_are_an_enum() {
        for (id, _) in BUTTONS {
            let c = parse("button", id.as_bytes()).unwrap();
            assert_eq!(c, Cmd::Button(id));
            assert!(button_msg(id).is_some(), "{id}");
        }
        assert!(parse("button", b"reboot").is_err());
        assert!(parse("button", b"").is_err());
    }

    #[test]
    fn tile_ids() {
        assert_eq!(parse("open_tile", b"mod-x-y"), Ok(Cmd::OpenTile("mod-x-y".into())));
        assert!(parse("open_tile", b"../x").is_err());
        assert!(parse("open_tile", b"a b").is_err());
        assert!(parse("open_tile", "\u{e9}".as_bytes()).is_err());
        assert!(parse("open_tile", &[b'a'; 81]).is_err());
    }

    #[test]
    fn app_name_and_misc() {
        assert_eq!(parse("app", "Plex".as_bytes()), Ok(Cmd::App("Plex".into())));
        assert!(parse("app", b"").is_err());
        assert!(parse("app", b"a\x07b").is_err());
        assert!(parse("app", &[0xff, 0xfe]).is_err());
        assert!(parse("app", &[b'a'; 300]).is_err());
        assert_eq!(parse("shutdown", b"ON"), Err("unknown_topic"));
    }

    #[test]
    fn rate_limit() {
        let mut r = RateLimiter::new(20, Duration::from_secs(1));
        let t0 = Instant::now();
        for _ in 0..20 {
            assert!(r.allow(t0));
        }
        assert!(!r.allow(t0));
        assert!(!r.allow(t0 + Duration::from_millis(999)));
        assert!(r.allow(t0 + Duration::from_millis(1000)));
    }
}
