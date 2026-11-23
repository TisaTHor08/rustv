//! Linux : appels système de `display.rs`. Sorties audio réelles (`pactl list sinks`, repli `wpctl status`) ;
//! modes d'affichage, liste des écrans et HDR restent ceux du stub (`Err("unsupported_platform")`, listes vides) :
//! xrandr / wlr-randr ne sont pas encore pilotés (voir docs/LINUX.md).
use super::*;
use std::process::{Command, Stdio};

#[allow(dead_code)]
#[path = "../stub/display_sys.rs"]
mod base;
pub use base::{apply, current_mode, hdr_get, hdr_set, monitors, raw_modes, target_of};

const NO_TOOL: &str = "tool_missing";

fn tool(prog: &str, args: &[&str]) -> Option<String> {
    let path = rustv_core::platform::which(prog)?;
    let out = Command::new(path).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Premier pourcentage d'une ligne `Volume: front-left: 29490 /  45% / -20.00 dB, ...`.
fn percent_of(line: &str) -> Option<u8> {
    let i = line.find('%')?;
    let d: String = line[..i].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    d.parse::<u32>().ok().map(|p| p.min(100) as u8)
}

/// Sorties de `pactl list sinks` (texte). `default_name` : résultat de `pactl get-default-sink`.
pub fn parse_pactl_sinks(text: &str, default_name: &str) -> Vec<Output> {
    let mut out = Vec::new();
    let mut cur: Option<Output> = None;
    for line in text.lines() {
        if line.starts_with("Sink #") {
            if let Some(o) = cur.take() {
                out.push(o);
            }
            cur = Some(Output { id: String::new(), name: String::new(), default: false, volume: 0, muted: false });
            continue;
        }
        let Some(o) = cur.as_mut() else { continue };
        let l = line.trim();
        if let Some(v) = l.strip_prefix("Name:") {
            o.id = v.trim().to_string();
        } else if let Some(v) = l.strip_prefix("Description:") {
            o.name = v.trim().to_string();
        } else if let Some(v) = l.strip_prefix("Mute:") {
            o.muted = v.trim() == "yes";
        } else if l.starts_with("Volume:") {
            o.volume = percent_of(l).unwrap_or(0);
        }
    }
    out.extend(cur);
    out.retain(|o| !o.id.is_empty());
    for o in out.iter_mut() {
        if o.name.is_empty() {
            o.name = o.id.clone();
        }
        o.default = o.id == default_name;
    }
    out
}

/// Sorties de `wpctl status` (section « Sinks: »). L'identifiant est le numéro WirePlumber.
pub fn parse_wpctl_status(text: &str) -> Vec<Output> {
    let mut out = Vec::new();
    let mut in_sinks = false;
    for line in text.lines() {
        let s = line.trim_start_matches(|c: char| c == '│' || c == ' ' || c == '├' || c == '─' || c == '└' || c == '\t');
        if !in_sinks {
            in_sinks = s.starts_with("Sinks:");
            continue;
        }
        if s.trim().is_empty() {
            continue;
        }
        if s.trim_end().ends_with(':') {
            break; // section suivante
        }
        let (default, rest) = match s.strip_prefix('*') {
            Some(r) => (true, r.trim_start()),
            None => (false, s),
        };
        let Some((id, rest)) = rest.split_once('.') else { continue };
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let (name, vol) = match rest.split_once("[vol:") {
            Some((n, v)) => (n.trim(), Some(v)),
            None => (rest.trim(), None),
        };
        let (volume, muted) = vol
            .map(|v| {
                let n: f64 = v.split_whitespace().next().and_then(|x| x.trim_end_matches(']').parse().ok()).unwrap_or(0.0);
                ((n * 100.0).round().clamp(0.0, 100.0) as u8, v.contains("MUTED"))
            })
            .unwrap_or((0, false));
        out.push(Output { id: id.to_string(), name: name.to_string(), default, volume, muted });
    }
    out
}

pub fn outputs() -> Result<Vec<Output>, String> {
    if let Some(t) = tool("pactl", &["list", "sinks"]) {
        let def = tool("pactl", &["get-default-sink"]).unwrap_or_default();
        return Ok(parse_pactl_sinks(&t, def.trim()));
    }
    if let Some(t) = tool("wpctl", &["status"]) {
        return Ok(parse_wpctl_status(&t));
    }
    Err(NO_TOOL.to_string())
}

fn is_wp_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

fn safe_name(id: &str) -> bool {
    !id.is_empty() && id.len() < 200 && !id.starts_with('-') && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-:+".contains(c))
}

pub fn set_default(id: &str) -> Result<(), String> {
    if is_wp_id(id) {
        return tool("wpctl", &["set-default", id]).map(|_| ()).ok_or_else(|| NO_TOOL.to_string());
    }
    if !safe_name(id) {
        return Err("invalid_output".into());
    }
    tool("pactl", &["set-default-sink", id]).map(|_| ()).ok_or_else(|| NO_TOOL.to_string())
}

pub fn set_volume(id: &str, percent: u32) -> Result<(), String> {
    let p = percent.min(100);
    if is_wp_id(id) {
        return tool("wpctl", &["set-volume", id, &format!("{:.2}", f64::from(p) / 100.0)]).map(|_| ()).ok_or_else(|| NO_TOOL.to_string());
    }
    if !safe_name(id) {
        return Err("invalid_output".into());
    }
    tool("pactl", &["set-sink-volume", id, &format!("{p}%")]).map(|_| ()).ok_or_else(|| NO_TOOL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACTL: &str = "Sink #47\n\tState: RUNNING\n\tName: alsa_output.pci-0000_00_1f.3.analog-stereo\n\tDescription: Built-in Audio Analog Stereo\n\tDriver: PipeWire\n\tMute: no\n\tVolume: front-left: 26214 /  40% / -23.88 dB,   front-right: 26214 /  40% / -23.88 dB\n\t        balance 0.00\n\nSink #52\n\tState: SUSPENDED\n\tName: alsa_output.pci-0000_01_00.1.hdmi-stereo\n\tDescription: HDMI / DisplayPort\n\tMute: yes\n\tVolume: front-left: 65536 / 100% / 0.00 dB,   front-right: 65536 / 100% / 0.00 dB\n";

    #[test]
    fn pactl_sinks() {
        let v = parse_pactl_sinks(PACTL, "alsa_output.pci-0000_01_00.1.hdmi-stereo");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], Output { id: "alsa_output.pci-0000_00_1f.3.analog-stereo".into(), name: "Built-in Audio Analog Stereo".into(), default: false, volume: 40, muted: false });
        assert!(v[1].default && v[1].muted && v[1].volume == 100 && v[1].name == "HDMI / DisplayPort");
        assert!(parse_pactl_sinks("", "x").is_empty());
    }

    const WPCTL: &str = "PipeWire 'pipewire-0' [1.0.5, user@host, cookie:123]\n └─ Clients:\n        33. WirePlumber                         [1.0.5, user, pid:1]\n\nAudio\n ├─ Devices:\n │      49. Built-in Audio                      [alsa]\n │\n ├─ Sinks:\n │  *   50. Built-in Audio Analog Stereo        [vol: 0.40]\n │      51. HDMI / DisplayPort                  [vol: 1.00 MUTED]\n │\n ├─ Sink endpoints:\n │\n └─ Streams:\n\nVideo\n";

    #[test]
    fn wpctl_sinks() {
        let v = parse_wpctl_status(WPCTL);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], Output { id: "50".into(), name: "Built-in Audio Analog Stereo".into(), default: true, volume: 40, muted: false });
        assert_eq!(v[1].id, "51");
        assert!(v[1].muted && !v[1].default && v[1].volume == 100);
        assert!(parse_wpctl_status("Audio\n ├─ Devices:\n").is_empty());
    }

    /// Manuel (PulseAudio réel) : liste les sorties, ne modifie rien.
    #[test]
    #[ignore]
    fn live_outputs() {
        let v = outputs().expect("pactl");
        assert!(!v.is_empty() && v.iter().any(|o| o.default), "{v:?}");
    }

    #[test]
    fn id_validation() {
        assert!(safe_name("alsa_output.pci-0000_00_1f.3.analog-stereo"));
        assert!(!safe_name("--help") && !safe_name("a b") && !safe_name(""));
        assert!(set_default("a;rm").is_err());
    }
}
