//! Tuners réseau : HDHomeRun (découverte UDP + discover.json / lineup.json) et TVHeadend (playlist M3U et
//! XMLTV derrière une authentification HTTP basique). Tout est converti en `{ m3u, xmltv }`, le format que
//! `live_load` sait déjà lire : le reste de l'interface n'a rien à savoir des tuners.
//!
//! URL supposées (à vérifier sur votre installation, voir docs/LIVE-TV.md) :
//! - HDHomeRun : `http://<ip>/discover.json`, `http://<ip>/lineup.json`, flux MPEG-TS `http://<ip>:5004/auto/v<canal>`.
//! - TVHeadend : `http://<hôte>:9981/playlist/channels?profile=pass`, `http://<hôte>:9981/xmltv/channels`.
//!
//! Les identifiants TVHeadend sont des identifiants que l'utilisateur configure lui-même ; ils sont placés
//! dans l'URL (`user:pass@`) pour que `live::open` et mpv les utilisent. Ils ne sont jamais journalisés.
use crate::{live, settings};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const HDHR_PORT: u16 = 65001;
const TYPE_DISCOVER_REQ: u16 = 0x0002;
const TYPE_DISCOVER_RPY: u16 = 0x0003;
const TAG_DEVICE_TYPE: u8 = 0x01;
const TAG_DEVICE_ID: u8 = 0x02;
const TAG_TUNER_COUNT: u8 = 0x10;
const TAG_LINEUP_URL: u8 = 0x27;
const TAG_BASE_URL: u8 = 0x2a;
const WILDCARD: u32 = 0xFFFF_FFFF;
const MAX_JSON: u64 = 8 * 1024 * 1024;

// ---------- protocole de découverte HDHomeRun ----------

/// CRC-32 standard (IEEE 802.3, polynôme réfléchi 0xEDB88320), comme dans libhdhomerun.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Trame de découverte : type 0x0002, longueur, [tag 0x01 type d'appareil = joker][tag 0x02 id = joker], CRC32 (little endian).
pub fn build_discover_packet() -> Vec<u8> {
    let mut payload = Vec::new();
    for tag in [TAG_DEVICE_TYPE, TAG_DEVICE_ID] {
        payload.push(tag);
        payload.push(4);
        payload.extend_from_slice(&WILDCARD.to_be_bytes());
    }
    let mut f = Vec::new();
    f.extend_from_slice(&TYPE_DISCOVER_REQ.to_be_bytes());
    f.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    f.extend_from_slice(&payload);
    let crc = crc32(&f);
    f.extend_from_slice(&crc.to_le_bytes());
    f
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Device {
    pub device_id: String,
    pub device_type: u32,
    pub base_url: Option<String>,
    pub lineup_url: Option<String>,
    pub tuner_count: Option<u8>,
}

/// Lit une longueur de TLV (1 octet, ou 2 si le bit 7 est levé).
fn tlv_len(b: &[u8], i: &mut usize) -> Option<usize> {
    let first = *b.get(*i)? as usize;
    *i += 1;
    if first & 0x80 == 0 {
        Some(first)
    } else {
        let second = *b.get(*i)? as usize;
        *i += 1;
        Some((first & 0x7f) | (second << 7))
    }
}

/// Analyse une réponse de découverte (type 0x0003) ; `None` si la trame est invalide (type, longueur ou CRC).
pub fn parse_discover_reply(pkt: &[u8]) -> Option<Device> {
    if pkt.len() < 8 {
        return None;
    }
    let ty = u16::from_be_bytes([pkt[0], pkt[1]]);
    let len = u16::from_be_bytes([pkt[2], pkt[3]]) as usize;
    if ty != TYPE_DISCOVER_RPY || pkt.len() != 4 + len + 4 {
        return None;
    }
    let crc = u32::from_le_bytes([pkt[4 + len], pkt[5 + len], pkt[6 + len], pkt[7 + len]]);
    if crc != crc32(&pkt[..4 + len]) {
        return None;
    }
    let body = &pkt[4..4 + len];
    let mut d = Device::default();
    let mut i = 0;
    while i < body.len() {
        let tag = body[i];
        i += 1;
        let l = tlv_len(body, &mut i)?;
        let v = body.get(i..i + l)?;
        i += l;
        match tag {
            TAG_DEVICE_TYPE if l == 4 => d.device_type = u32::from_be_bytes([v[0], v[1], v[2], v[3]]),
            TAG_DEVICE_ID if l == 4 => d.device_id = format!("{:08X}", u32::from_be_bytes([v[0], v[1], v[2], v[3]])),
            TAG_TUNER_COUNT if l == 1 => d.tuner_count = Some(v[0]),
            TAG_BASE_URL => d.base_url = Some(String::from_utf8_lossy(v).into_owned()),
            TAG_LINEUP_URL => d.lineup_url = Some(String::from_utf8_lossy(v).into_owned()),
            _ => {}
        }
    }
    Some(d)
}

/// Diffuse la requête en broadcast UDP et collecte les réponses pendant ~1,5 s : (adresse, appareil).
fn discover_udp() -> Vec<(String, Device)> {
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else { return Vec::new() };
    let _ = sock.set_broadcast(true);
    let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
    let pkt = build_discover_packet();
    for _ in 0..2 {
        let _ = sock.send_to(&pkt, SocketAddr::from(([255, 255, 255, 255], HDHR_PORT)));
    }
    let mut out: Vec<(String, Device)> = Vec::new();
    let end = Instant::now() + Duration::from_millis(1500);
    let mut buf = [0u8; 1500];
    while Instant::now() < end {
        if let Ok((n, from)) = sock.recv_from(&mut buf) {
            if let Some(d) = parse_discover_reply(&buf[..n]) {
                let ip = from.ip().to_string();
                if !out.iter().any(|(h, _)| *h == ip) && out.len() < 16 {
                    out.push((ip, d));
                }
            }
        }
    }
    out
}

// ---------- JSON HDHomeRun ----------

#[derive(Debug, Clone, PartialEq)]
pub struct LineupEntry {
    pub number: String,
    pub name: String,
    pub url: String,
}

/// `lineup.json` : `[{GuideNumber, GuideName, URL, DRM?, HD?}]`. Les chaînes chiffrées (DRM=1) sont écartées.
pub fn parse_lineup_json(text: &str) -> Result<Vec<LineupEntry>, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| "lineup illisible".to_string())?;
    let arr = v.as_array().ok_or("lineup inattendu")?;
    let mut out = Vec::new();
    for e in arr {
        if e["DRM"].as_i64().unwrap_or(0) == 1 || e["DRM"].as_bool().unwrap_or(false) {
            continue;
        }
        let url = e["URL"].as_str().unwrap_or("").trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        let number = e["GuideNumber"].as_str().map(|s| s.to_string()).or_else(|| e["GuideNumber"].as_f64().map(|n| n.to_string())).unwrap_or_default();
        let name = e["GuideName"].as_str().unwrap_or("").trim().to_string();
        out.push(LineupEntry { number, name: if name.is_empty() { url.to_string() } else { name }, url: url.to_string() });
        if out.len() >= 2000 {
            break;
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscoverJson {
    pub name: String,
    pub model: String,
    pub tuner_count: u32,
    pub lineup_url: String,
    pub base_url: String,
}

pub fn parse_discover_json(text: &str) -> Result<DiscoverJson, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| "discover.json illisible".to_string())?;
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    Ok(DiscoverJson { name: s("FriendlyName"), model: s("ModelNumber"), tuner_count: v["TuningCount"].as_u64().or(v["TunerCount"].as_u64()).unwrap_or(0) as u32, lineup_url: s("LineupURL"), base_url: s("BaseURL") })
}

/// Texte M3U (sans identifiants) d'un lineup.
pub fn lineup_to_m3u(entries: &[LineupEntry]) -> String {
    let clean = |s: &str| s.replace(['\r', '\n', '"', ','], " ");
    let mut o = String::from("#EXTM3U\n");
    for e in entries {
        let label = if e.number.is_empty() { clean(&e.name) } else { format!("{} {}", clean(&e.number), clean(&e.name)) };
        o.push_str(&format!("#EXTINF:-1 tvg-name=\"{}\" group-title=\"HDHomeRun\",{}\n{}\n", clean(&e.name), label.trim(), e.url));
    }
    o
}

// ---------- HTTP ----------

fn valid_host(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

fn get_text(url: &str, auth: Option<&(String, String)>, timeout: u64) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(timeout)).timeout_read(Duration::from_secs(timeout.max(5))).user_agent("RusTV").build();
    let mut req = agent.get(url);
    if let Some((u, p)) = auth {
        req = req.set("Authorization", &live::basic_auth_header(u, p));
    }
    let r = req.call().map_err(|e| match e {
        ureq::Error::Status(401 | 403, _) => "accès refusé (identifiants ?)".to_string(),
        ureq::Error::Status(c, _) => format!("le serveur répond {c}"),
        _ => "appareil injoignable".to_string(),
    })?;
    let mut s = String::new();
    r.into_reader().take(MAX_JSON).read_to_string(&mut s).map_err(|_| "réponse illisible".to_string())?;
    Ok(s)
}

/// Interroge un HDHomeRun : (infos, lineup).
fn hdhr_fetch(host: &str, timeout: u64) -> Result<(DiscoverJson, Vec<LineupEntry>), String> {
    if !valid_host(host) {
        return Err("adresse invalide".into());
    }
    let info = parse_discover_json(&get_text(&format!("http://{host}/discover.json"), None, timeout)?).unwrap_or_default();
    // LineupURL fourni par l'appareil si c'est bien du http ; sinon /lineup.json.
    let lineup_url = if info.lineup_url.starts_with("http://") { info.lineup_url.clone() } else { format!("http://{host}/lineup.json") };
    let lineup = parse_lineup_json(&get_text(&lineup_url, None, timeout)?)?;
    Ok((info, lineup))
}

// ---------- commandes ----------

#[derive(Serialize)]
pub struct Found {
    pub kind: String,
    pub name: String,
    pub host: String,
    pub channels: Option<usize>,
    pub tuners: Option<u32>,
}

/// Cherche les HDHomeRun du réseau local (broadcast UDP 65001). TVHeadend n'a pas de découverte : saisie manuelle.
#[tauri::command]
pub async fn tuner_discover() -> Result<Vec<Found>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut out = Vec::new();
        for (ip, d) in discover_udp() {
            let host = ip.clone();
            let (name, channels, tuners) = match hdhr_fetch(&host, 4) {
                Ok((info, l)) => (if info.name.is_empty() { format!("HDHomeRun {}", d.device_id) } else { info.name }, Some(l.len()), Some(info.tuner_count).filter(|n| *n > 0).or(d.tuner_count.map(|n| n as u32))),
                Err(_) => (format!("HDHomeRun {}", d.device_id), None, d.tuner_count.map(|n| n as u32)),
            };
            out.push(Found { kind: "hdhomerun".into(), name, host, channels, tuners });
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct Spec {
    pub kind: String,
    pub host: String,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    /// Guide XMLTV externe facultatif (HDHomeRun n'en fournit pas).
    pub xmltv: Option<String>,
}

/// URLs M3U / XMLTV de TVHeadend (identifiants intégrés si fournis).
pub fn tvheadend_urls(host: &str, port: u16, user: &str, pass: &str) -> (String, String) {
    let auth = if user.is_empty() { String::new() } else { format!("{}:{}@", live::percent_encode(user), live::percent_encode(pass)) };
    let base = format!("http://{auth}{host}:{port}");
    (format!("{base}/playlist/channels?profile=pass"), format!("{base}/xmltv/channels"))
}

fn sanitize_id(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect()
}

/// `tuner_build_source(spec)` -> `{ m3u, xmltv }` à passer tel quel à `live_load`.
/// HDHomeRun : télécharge le lineup et écrit un M3U local (`%APPDATA%\RusTV\tuners\<hôte>.m3u`).
/// TVHeadend : construit les URL (aucun accès réseau ici ; `live_load` les lira avec l'authentification).
#[tauri::command]
pub async fn tuner_build_source(spec: Spec) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let host = spec.host.trim().to_string();
        if !valid_host(&host) {
            return Err("adresse invalide".to_string());
        }
        match spec.kind.as_str() {
            "hdhomerun" => {
                let (_, lineup) = hdhr_fetch(&host, 8)?;
                if lineup.is_empty() {
                    return Err("aucune chaîne sur cet appareil (scan des chaînes fait ?)".into());
                }
                let dir = settings::dir().join("tuners");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(format!("hdhr-{}.m3u", sanitize_id(&host)));
                std::fs::write(&path, lineup_to_m3u(&lineup)).map_err(|e| e.to_string())?;
                Ok(json!({ "m3u": path.to_string_lossy(), "xmltv": spec.xmltv.unwrap_or_default() }))
            }
            "tvheadend" => {
                let (m3u, xmltv) = tvheadend_urls(&host, spec.port.unwrap_or(9981), spec.user.as_deref().unwrap_or(""), spec.password.as_deref().unwrap_or(""));
                Ok(json!({ "m3u": m3u, "xmltv": spec.xmltv.filter(|x| !x.trim().is_empty()).unwrap_or(xmltv) }))
            }
            _ => Err("type de tuner inconnu".to_string()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ty: u16, payload: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&ty.to_be_bytes());
        f.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        f.extend_from_slice(payload);
        let c = crc32(&f);
        f.extend_from_slice(&c.to_le_bytes());
        f
    }

    #[test]
    fn crc_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn discover_packet_layout() {
        let p = build_discover_packet();
        assert_eq!(&p[..4], &[0x00, 0x02, 0x00, 0x0c]);
        assert_eq!(&p[4..16], &[0x01, 0x04, 0xff, 0xff, 0xff, 0xff, 0x02, 0x04, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(p.len(), 20);
        let crc = u32::from_le_bytes([p[16], p[17], p[18], p[19]]);
        assert_eq!(crc, crc32(&p[..16]));
    }

    #[test]
    fn discover_reply_parse() {
        let mut pl = vec![TAG_DEVICE_TYPE, 4, 0, 0, 0, 1, TAG_DEVICE_ID, 4, 0x10, 0x20, 0x30, 0x40, TAG_TUNER_COUNT, 1, 3];
        let url = b"http://192.168.1.50:80";
        pl.extend_from_slice(&[TAG_BASE_URL, url.len() as u8]);
        pl.extend_from_slice(url);
        let d = parse_discover_reply(&frame(TYPE_DISCOVER_RPY, &pl)).unwrap();
        assert_eq!(d.device_type, 1);
        assert_eq!(d.device_id, "10203040");
        assert_eq!(d.tuner_count, Some(3));
        assert_eq!(d.base_url.as_deref(), Some("http://192.168.1.50:80"));
    }

    #[test]
    fn discover_reply_rejects_bad_frames() {
        let good = frame(TYPE_DISCOVER_RPY, &[TAG_DEVICE_ID, 4, 1, 2, 3, 4]);
        assert!(parse_discover_reply(&good).is_some());
        let mut bad_crc = good.clone();
        *bad_crc.last_mut().unwrap() ^= 1;
        assert!(parse_discover_reply(&bad_crc).is_none());
        assert!(parse_discover_reply(&frame(TYPE_DISCOVER_REQ, &[])).is_none()); // mauvais type
        assert!(parse_discover_reply(&good[..good.len() - 1]).is_none()); // tronquée
        assert!(parse_discover_reply(&[]).is_none());
        // TLV qui dépasse la trame : pas de panique
        assert!(parse_discover_reply(&frame(TYPE_DISCOVER_RPY, &[TAG_BASE_URL, 50, 1])).is_none());
    }

    #[test]
    fn lineup_parse_skips_drm() {
        let j = r#"[{"GuideNumber":"2.1","GuideName":"TF1","URL":"http://10.0.0.5:5004/auto/v2.1","HD":1},
                    {"GuideNumber":"3","GuideName":"Crypte","URL":"http://10.0.0.5:5004/auto/v3","DRM":1},
                    {"GuideNumber":"4","GuideName":"","URL":"http://10.0.0.5:5004/auto/v4"},
                    {"GuideNumber":"5","GuideName":"Mauvais","URL":"file:///etc/passwd"}]"#;
        let l = parse_lineup_json(j).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0], LineupEntry { number: "2.1".into(), name: "TF1".into(), url: "http://10.0.0.5:5004/auto/v2.1".into() });
        assert_eq!(l[1].name, "http://10.0.0.5:5004/auto/v4");
        assert!(parse_lineup_json("nope").is_err());
        assert!(parse_lineup_json("{}").is_err());
    }

    #[test]
    fn discover_json_and_m3u() {
        let d = parse_discover_json(r#"{"FriendlyName":"HDHomeRun PRIME","ModelNumber":"HDHR3-CC","TunerCount":3,"LineupURL":"http://10.0.0.5/lineup.json"}"#).unwrap();
        assert_eq!((d.name.as_str(), d.tuner_count, d.lineup_url.as_str()), ("HDHomeRun PRIME", 3, "http://10.0.0.5/lineup.json"));
        let m = lineup_to_m3u(&[LineupEntry { number: "7".into(), name: "Ch,\"X\"".into(), url: "http://h/auto/v7".into() }]);
        assert!(m.starts_with("#EXTM3U\n#EXTINF:-1"));
        assert!(m.ends_with("http://h/auto/v7\n"));
        // le nom ne peut pas casser la syntaxe M3U
        assert_eq!(m.lines().count(), 3);
        assert!(!m.lines().nth(1).unwrap().contains("\"X\""));
    }

    #[test]
    fn tvheadend_urls_with_credentials() {
        let (m, x) = tvheadend_urls("nas.lan", 9981, "bob", "p@ss w");
        assert_eq!(m, "http://bob:p%40ss%20w@nas.lan:9981/playlist/channels?profile=pass");
        assert_eq!(x, "http://bob:p%40ss%20w@nas.lan:9981/xmltv/channels");
        assert_eq!(tvheadend_urls("h", 1, "", "").0, "http://h:1/playlist/channels?profile=pass");
    }

    #[test]
    fn host_validation() {
        assert!(valid_host("192.168.1.5") && valid_host("hdhr-1234.local") && valid_host("[::1]:80"));
        assert!(!valid_host("a/b") && !valid_host("u@h") && !valid_host("") && !valid_host("a b"));
    }
}
