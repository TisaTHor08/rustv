//! Linux : volume principal du système.
//!
//! `wpctl` (PipeWire / WirePlumber) en premier, `pactl` (PulseAudio ou pipewire-pulse) en repli. Sans l'un ni
//! l'autre : `get()` renvoie `None` et `set()` ne fait rien (le téléphone n'affiche alors pas de barre de volume).
//! Les changements faits sur le PC (touches, mixeur) sont repris par un sondage de 500 ms avec détection de
//! changement, uniquement tant qu'un téléphone est connecté.
use crate::core::Core;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

const WPCTL_SINK: &str = "@DEFAULT_AUDIO_SINK@";
const PACTL_SINK: &str = "@DEFAULT_SINK@";

/// Exécute un outil et renvoie sa sortie standard (None si absent ou en échec).
pub fn run_tool(prog: &str, args: &[&str]) -> Option<String> {
    let path = crate::platform::which(prog)?;
    let out = Command::new(path).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Un outil de volume est-il installé ? (`wpctl` ou `pactl`)
pub fn available() -> bool {
    crate::platform::which("wpctl").is_some() || crate::platform::which("pactl").is_some()
}

/// `Volume: 0.45` ou `Volume: 0.45 [MUTED]` (sortie de `wpctl get-volume`).
pub fn parse_wpctl_volume(out: &str) -> Option<(u8, bool)> {
    let line = out.lines().find(|l| l.trim_start().starts_with("Volume:"))?;
    let v: f64 = line.trim_start().strip_prefix("Volume:")?.split_whitespace().next()?.parse().ok()?;
    let muted = line.contains("[MUTED]");
    Some(((v * 100.0).round().clamp(0.0, 100.0) as u8, muted))
}

/// Premier pourcentage d'une ligne `Volume: front-left: 29490 /  45% / -20.00 dB, front-right: ...`.
pub fn parse_pactl_percent(line: &str) -> Option<u8> {
    let i = line.find('%')?;
    let digits: String = line[..i].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    digits.parse::<u32>().ok().map(|p| p.min(100) as u8)
}

/// `Mute: yes` / `Mute: no`.
pub fn parse_pactl_mute(out: &str) -> Option<bool> {
    let l = out.lines().find(|l| l.trim_start().starts_with("Mute:"))?;
    Some(l.contains("yes"))
}

fn get_wpctl() -> Option<(u8, bool)> {
    parse_wpctl_volume(&run_tool("wpctl", &["get-volume", WPCTL_SINK])?)
}

fn get_pactl() -> Option<(u8, bool)> {
    let vol = run_tool("pactl", &["get-sink-volume", PACTL_SINK])?;
    let level = parse_pactl_percent(vol.lines().find(|l| l.trim_start().starts_with("Volume:"))?)?;
    let muted = parse_pactl_mute(&run_tool("pactl", &["get-sink-mute", PACTL_SINK])?).unwrap_or(false);
    Some((level, muted))
}

/// Volume (0 à 100) et muet du périphérique de sortie par défaut.
pub fn get() -> Option<(u8, bool)> {
    get_wpctl().or_else(get_pactl)
}

/// Règle le volume (0 à 100) ; lève le muet si on monte le son.
pub fn set(level: u8) {
    let level = level.min(100);
    let frac = format!("{:.2}", f64::from(level) / 100.0);
    if run_tool("wpctl", &["set-volume", WPCTL_SINK, &frac]).is_some() {
        if level > 0 {
            let _ = run_tool("wpctl", &["set-mute", WPCTL_SINK, "0"]);
        }
        return;
    }
    let pct = format!("{level}%");
    let _ = run_tool("pactl", &["set-sink-volume", PACTL_SINK, &pct]);
    if level > 0 {
        let _ = run_tool("pactl", &["set-sink-mute", PACTL_SINK, "0"]);
    }
}

/// Diffuse le volume aux téléphones quand il change (sondage de 500 ms tant qu'un téléphone est connecté).
pub fn run(core: Arc<Core>) {
    let mut last: Option<(u8, bool)> = None;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if core.conn_count() == 0 {
            last = None;
            continue;
        }
        if let Some(cur) = get() {
            if last != Some(cur) {
                core.set_volume(cur.0, cur.1);
                last = Some(cur);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manuel (PulseAudio/PipeWire réel, touche au volume) : `cargo test -p rustv-core audio -- --ignored`.
    #[test]
    #[ignore]
    fn live_set_and_get() {
        let (orig, _) = get().expect("pas de serveur audio");
        set(37);
        std::thread::sleep(Duration::from_millis(300));
        let (now, muted) = get().unwrap();
        set(orig);
        assert!((i16::from(now) - 37).abs() <= 1 && !muted, "volume lu: {now}");
    }

    #[test]
    fn wpctl_samples() {
        assert_eq!(parse_wpctl_volume("Volume: 0.45\n"), Some((45, false)));
        assert_eq!(parse_wpctl_volume("Volume: 0.00 [MUTED]\n"), Some((0, true)));
        assert_eq!(parse_wpctl_volume("Volume: 1.20\n"), Some((100, false)));
        assert_eq!(parse_wpctl_volume("Volume: 0.333\n"), Some((33, false)));
        assert_eq!(parse_wpctl_volume("Could not connect"), None);
    }

    #[test]
    fn pactl_samples() {
        let l = "Volume: front-left: 29490 /  45% / -20.00 dB,   front-right: 29490 /  45% / -20.00 dB";
        assert_eq!(parse_pactl_percent(l), Some(45));
        assert_eq!(parse_pactl_percent("Volume: mono: 65536 / 100% / 0.00 dB"), Some(100));
        assert_eq!(parse_pactl_percent("Volume: mono: 0 /   0% / -inf dB"), Some(0));
        assert_eq!(parse_pactl_percent("Volume: no percent"), None);
        assert_eq!(parse_pactl_mute("Mute: yes\n"), Some(true));
        assert_eq!(parse_pactl_mute("Mute: no\n"), Some(false));
        assert_eq!(parse_pactl_mute("x"), None);
    }
}
