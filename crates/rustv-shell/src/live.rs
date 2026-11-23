//! TV en direct : listes M3U et guides XMLTV fournis par l'utilisateur (URL ou fichier local, .gz accepté).
//! RusTV n'embarque aucune liste de chaînes.
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::time::Duration;

const MAX_M3U: u64 = 40 * 1024 * 1024;
const MAX_XMLTV: u64 = 200 * 1024 * 1024;
const MAX_CHANNELS: usize = 30_000;
/// Fenêtre du guide conservée en mémoire (secondes) : 6 h passées, 72 h à venir.
const KEEP_PAST: i64 = 6 * 3600;
const KEEP_FUTURE: i64 = 72 * 3600;
/// Plage maximale renvoyée par `live_epg`.
const MAX_WINDOW: i64 = 48 * 3600;
/// Schémas autorisés pour les flux (pas de file:, edl:, av:… qui pourraient lire des fichiers locaux).
pub const SCHEMES: &[&str] = &["http", "https", "rtmp", "rtmps", "rtsp", "udp", "rtp", "srt", "mms", "mmsh"];

#[derive(Serialize, Clone)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub logo: String,
    pub group: String,
    pub url: String,
    /// Identifiant XMLTV (tvg-id) s'il existe.
    pub tvg: String,
}

#[derive(Serialize, Clone)]
pub struct Prog {
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: String,
}

#[derive(Default)]
pub struct Cache {
    pub channels: Vec<Channel>,
    /// Programmes par identifiant de chaîne XMLTV, triés par heure de début.
    pub epg: HashMap<String, Vec<Prog>>,
    /// Nom d'affichage (minuscule) → identifiant XMLTV, pour les chaînes sans tvg-id.
    pub names: HashMap<String, String>,
}

/// Décode les séquences %XX d'un morceau d'URL.
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 1) && s.is_char_boundary(i + 3) {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Encode tout ce qui n'est pas alphanumérique ou `-._~` (identifiant / mot de passe dans une URL).
pub fn percent_encode(s: &str) -> String {
    let mut o = String::new();
    for c in s.bytes() {
        if c.is_ascii_alphanumeric() || b"-._~".contains(&c) {
            o.push(c as char);
        } else {
            o.push_str(&format!("%{c:02X}"));
        }
    }
    o
}

/// `http://user:pass@hôte/chemin` -> (`http://hôte/chemin`, Some((user, pass))). Les identifiants ne sont
/// jamais conservés dans l'URL envoyée à `ureq` (donc jamais dans ses messages d'erreur).
pub fn split_auth(src: &str) -> (String, Option<(String, String)>) {
    let Some((scheme, rest)) = src.split_once("://") else { return (src.to_string(), None) };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    match authority.rsplit_once('@') {
        Some((cred, host)) => {
            let (u, p) = cred.split_once(':').unwrap_or((cred, ""));
            (format!("{scheme}://{host}{tail}"), Some((percent_decode(u), percent_decode(p))))
        }
        None => (src.to_string(), None),
    }
}

/// « hôte:port » d'une URL (sans identifiants), en minuscules.
fn host_port(url: &str) -> String {
    let (clean, _) = split_auth(url);
    let rest = clean.split_once("://").map(|x| x.1).unwrap_or("");
    rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())].to_ascii_lowercase()
}

/// Ajoute `user:pass@` à une URL de flux si elle vise le même hôte:port que la source authentifiée
/// (les identifiants ne sont jamais envoyés à un autre serveur).
pub fn inject_auth(url: &str, auth_for: &str, cred: &(String, String)) -> String {
    let (clean, existing) = split_auth(url);
    if existing.is_some() || !(clean.starts_with("http://") || clean.starts_with("https://")) || host_port(&clean) != host_port(auth_for) {
        return url.to_string();
    }
    let (scheme, rest) = clean.split_once("://").unwrap_or(("http", ""));
    format!("{scheme}://{}:{}@{rest}", percent_encode(&cred.0), percent_encode(&cred.1))
}

pub fn basic_auth_header(user: &str, pass: &str) -> String {
    format!("Basic {}", base64(format!("{user}:{pass}").as_bytes()))
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for ch in data.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}

/// Ouvre une source (URL http(s) avec identifiants facultatifs `user:pass@`, ou fichier local), .gz accepté.
pub fn open(src: &str, max: u64) -> Result<Box<dyn Read + Send>, String> {
    let raw: Box<dyn Read + Send> = if src.starts_with("http://") || src.starts_with("https://") {
        let (url, cred) = split_auth(src);
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).user_agent("RusTV").build();
        let mut req = agent.get(&url);
        if let Some((u, p)) = &cred {
            req = req.set("Authorization", &basic_auth_header(u, p));
        }
        let r = req.call().map_err(|e| match e {
            ureq::Error::Status(401 | 403, _) => "accès refusé (identifiants ?)".to_string(),
            ureq::Error::Status(c, _) => format!("le serveur répond {c}"),
            _ => "téléchargement impossible".to_string(),
        })?;
        Box::new(r.into_reader().take(max))
    } else {
        Box::new(std::fs::File::open(src).map_err(|e| format!("fichier illisible : {e}"))?.take(max))
    };
    let mut br = BufReader::new(raw);
    let gz = br.fill_buf().map(|b| b.len() >= 2 && b[0] == 0x1f && b[1] == 0x8b).unwrap_or(false);
    Ok(if gz { Box::new(GzDecoder::new(br)) } else { Box::new(br) })
}

fn attr(line: &str, key: &str) -> String {
    let pat = format!("{key}=\"");
    match line.find(&pat) {
        Some(i) => {
            let rest = &line[i + pat.len()..];
            rest[..rest.find('"').unwrap_or(rest.len())].to_string()
        }
        None => String::new(),
    }
}

pub fn parse_m3u(src: &str) -> Result<Vec<Channel>, String> {
    let mut reader = BufReader::new(open(src, MAX_M3U)?);
    let cred = split_auth(src).1;
    let mut out = Vec::new();
    let mut pending: Option<Channel> = None;
    let mut line = String::new();
    loop {
        line.clear();
        let mut bytes = Vec::new();
        if reader.read_until(b'\n', &mut bytes).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let l = String::from_utf8_lossy(&bytes).trim().to_string();
        if let Some(info) = l.strip_prefix("#EXTINF:") {
            let name = info.rsplit_once(',').map(|x| x.1.trim().to_string()).unwrap_or_default();
            let tvg_name = attr(info, "tvg-name");
            pending = Some(Channel {
                id: String::new(),
                name: if name.is_empty() { tvg_name } else { name },
                logo: attr(info, "tvg-logo"),
                group: attr(info, "group-title"),
                url: String::new(),
                tvg: attr(info, "tvg-id"),
            });
        } else if !l.is_empty() && !l.starts_with('#') {
            if let Some(mut c) = pending.take() {
                let scheme = l.split("://").next().unwrap_or("").to_ascii_lowercase();
                if SCHEMES.contains(&scheme.as_str()) && l.contains("://") {
                    c.url = match &cred {
                        Some(cr) => inject_auth(&l, src, cr),
                        None => l,
                    };
                    c.id = format!("ch{}", out.len());
                    if c.name.is_empty() {
                        c.name = c.url.clone();
                    }
                    if !(c.logo.starts_with("http://") || c.logo.starts_with("https://")) {
                        c.logo.clear();
                    }
                    out.push(c);
                    if out.len() >= MAX_CHANNELS {
                        break;
                    }
                }
            }
        }
    }
    if out.is_empty() {
        return Err("aucune chaîne trouvée dans cette liste".into());
    }
    Ok(out)
}

/// Jours depuis 1970-01-01 pour une date civile (algorithme de Howard Hinnant).
fn days_from_civil(y: i64, mo: i64, d: i64) -> i64 {
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// « 20261123200000 +0100 » → secondes Unix (UTC). Accepte aussi « +01:00 », « +01 », « Z », « UTC »,
/// l'absence de décalage (= UTC, comme le prévoit la DTD XMLTV) et l'absence de secondes.
pub(crate) fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 12 {
        return None;
    }
    let n = |a: usize, b: usize| digits.get(a..b).and_then(|x| x.parse::<i64>().ok());
    let (y, mo, d, h, mi) = (n(0, 4)?, n(4, 6)?, n(6, 8)?, n(8, 10)?, n(10, 12)?);
    let sec = n(12, 14).unwrap_or(0);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 24 || mi > 59 || sec > 60 || y < 1970 {
        return None;
    }
    let mut t = days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + sec;
    let tz = s[digits.len()..].trim();
    let sign = match tz.chars().next() {
        Some('+') => 1,
        Some('-') => -1,
        _ => 0,
    };
    if sign != 0 {
        let dg: String = tz[1..].chars().filter(|c| c.is_ascii_digit()).collect();
        let (th, tm) = match dg.len() {
            2 => (dg.parse::<i64>().ok()?, 0),
            4 => (dg[..2].parse::<i64>().ok()?, dg[2..].parse::<i64>().ok()?),
            _ => return None,
        };
        if th > 14 || tm > 59 {
            return None;
        }
        t -= sign * (th * 3600 + tm * 60);
    }
    Some(t)
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Lit un guide XMLTV en flux et ne garde que les programmes des prochaines 72 h (et des 6 dernières).
pub fn parse_xmltv(src: &str) -> Result<(HashMap<String, Vec<Prog>>, HashMap<String, String>), String> {
    let mut reader = Reader::from_reader(BufReader::new(open(src, MAX_XMLTV)?));
    reader.config_mut().trim_text(true);
    let now = now_ts();
    let (lo, hi) = (now - KEEP_PAST, now + KEEP_FUTURE);
    let mut epg: HashMap<String, Vec<Prog>> = HashMap::new();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut buf = Vec::new();
    let (mut cur_chan, mut cur_prog): (Option<String>, Option<(String, Prog)>) = (None, None);
    let mut field = 0u8; // 1 = display-name, 2 = title, 3 = desc
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"channel" => {
                    cur_chan = e.attributes().flatten().find(|a| a.key.as_ref() == b"id").and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()));
                }
                b"programme" => {
                    let (mut start, mut stop, mut chan) = (None, None, String::new());
                    for a in e.attributes().flatten() {
                        let v = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
                        match a.key.as_ref() {
                            b"start" => start = parse_time(&v),
                            b"stop" => stop = parse_time(&v),
                            b"channel" => chan = v,
                            _ => {}
                        }
                    }
                    cur_prog = match (start, stop) {
                        (Some(a), Some(b)) if b > lo && a < hi => Some((chan, Prog { start: a, stop: b, title: String::new(), desc: String::new() })),
                        _ => None,
                    };
                }
                b"display-name" => field = 1,
                b"title" if cur_prog.is_some() => field = 2,
                b"desc" if cur_prog.is_some() => field = 3,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                let text = t.unescape().map(|v| v.into_owned()).unwrap_or_default();
                match field {
                    1 => {
                        if let Some(id) = &cur_chan {
                            names.entry(text.to_lowercase()).or_insert_with(|| id.clone());
                        }
                    }
                    2 => {
                        if let Some((_, p)) = cur_prog.as_mut() {
                            if p.title.is_empty() {
                                p.title = text;
                            }
                        }
                    }
                    3 => {
                        if let Some((_, p)) = cur_prog.as_mut() {
                            if p.desc.is_empty() {
                                p.desc = text.chars().take(400).collect();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"channel" => cur_chan = None,
                    b"programme" => {
                        if let Some((chan, p)) = cur_prog.take() {
                            epg.entry(chan).or_default().push(p);
                        }
                    }
                    _ => {}
                }
                field = 0;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                if epg.is_empty() {
                    return Err(format!("guide XMLTV illisible : {e}"));
                }
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    for v in epg.values_mut() {
        normalize(v);
    }
    Ok((epg, names))
}

#[derive(Serialize)]
pub struct NowNext {
    pub now: Option<Prog>,
    pub next: Option<Prog>,
}

/// Trie, supprime les doublons (même début), écarte les programmes sans durée et rogne les chevauchements :
/// un programme qui dépasse le début du suivant est raccourci.
pub fn normalize(v: &mut Vec<Prog>) {
    v.retain(|p| p.stop > p.start);
    v.sort_by(|a, b| a.start.cmp(&b.start).then(b.stop.cmp(&a.stop)));
    v.dedup_by(|b, a| a.start == b.start);
    for i in 0..v.len().saturating_sub(1) {
        let next = v[i + 1].start;
        if v[i].stop > next {
            v[i].stop = next;
        }
    }
    v.retain(|p| p.stop > p.start);
}

/// Programmes (triés) d'une chaîne, vides si elle n'a pas de guide.
pub fn programs_for<'a>(cache: &'a Cache, c: &Channel) -> &'a [Prog] {
    epg_key(cache, c).and_then(|k| cache.epg.get(&k)).map(|v| v.as_slice()).unwrap_or(&[])
}

/// Clé du guide d'une chaîne : tvg-id s'il existe dans le guide, sinon correspondance par nom.
fn epg_key(cache: &Cache, c: &Channel) -> Option<String> {
    if !c.tvg.is_empty() && cache.epg.contains_key(&c.tvg) {
        Some(c.tvg.clone())
    } else {
        cache.names.get(&c.name.to_lowercase()).cloned()
    }
}

pub fn now_next(cache: &Cache) -> HashMap<String, NowNext> {
    let now = now_ts();
    let mut out = HashMap::new();
    for c in &cache.channels {
        let Some(progs) = epg_key(cache, c).and_then(|k| cache.epg.get(&k)) else { continue };
        let cur = progs.iter().position(|p| p.start <= now && now < p.stop);
        let (now_p, next_p) = match cur {
            Some(i) => (progs.get(i).cloned(), progs.get(i + 1).cloned()),
            None => (None, progs.iter().find(|p| p.start > now).cloned()),
        };
        if now_p.is_some() || next_p.is_some() {
            out.insert(c.id.clone(), NowNext { now: now_p, next: next_p });
        }
    }
    out
}

/// Position d'un programme dans la fenêtre [from, to) : (gauche, largeur) en fractions 0..1, et si le
/// programme déborde à gauche / à droite. `None` s'il est hors fenêtre.
pub fn layout(start: i64, stop: i64, from: i64, to: i64) -> Option<(f64, f64, bool, bool)> {
    if to <= from || stop <= from || start >= to || stop <= start {
        return None;
    }
    let span = (to - from) as f64;
    let (a, b) = (start.max(from), stop.min(to));
    Some(((a - from) as f64 / span, (b - a) as f64 / span, start < from, stop > to))
}

/// Programmes qui recouvrent [from, to), par recherche dichotomique (liste triée, sans chevauchement).
pub fn window(progs: &[Prog], from: i64, to: i64) -> &[Prog] {
    let first = progs.partition_point(|p| p.stop <= from);
    let last = progs.partition_point(|p| p.start < to);
    if first >= last {
        &[]
    } else {
        &progs[first..last]
    }
}

#[derive(Serialize)]
pub struct ProgBlock {
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: String,
    /// Position et largeur dans la fenêtre demandée (fractions 0..1).
    pub left: f64,
    pub width: f64,
    #[serde(rename = "clipL")]
    pub clip_l: bool,
    #[serde(rename = "clipR")]
    pub clip_r: bool,
}

#[derive(Serialize)]
pub struct EpgChannel {
    pub id: String,
    pub programs: Vec<ProgBlock>,
}

/// Programmes de la fenêtre [from, to) pour les chaînes demandées (toutes si `ids` est `None`).
pub fn epg_window(cache: &Cache, from: i64, to: i64, ids: Option<&[String]>) -> Vec<EpgChannel> {
    let to = to.min(from.saturating_add(MAX_WINDOW));
    let mut out = Vec::new();
    for c in &cache.channels {
        if let Some(ids) = ids {
            if !ids.iter().any(|i| i == &c.id) {
                continue;
            }
        }
        let programs: Vec<ProgBlock> = epg_key(cache, c)
            .and_then(|k| cache.epg.get(&k))
            .map(|v| {
                window(v, from, to)
                    .iter()
                    .filter_map(|p| {
                        let (left, width, clip_l, clip_r) = layout(p.start, p.stop, from, to)?;
                        Some(ProgBlock { start: p.start, stop: p.stop, title: p.title.clone(), desc: p.desc.clone(), left, width, clip_l, clip_r })
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.push(EpgChannel { id: c.id.clone(), programs });
    }
    out
}

/// `live_epg(fromTs, toTs, channelIds?)` -> `{ channels: [{ id, programs: [...] }] }`.
#[tauri::command]
pub async fn live_epg(shell: tauri::State<'_, crate::Shell>, from_ts: i64, to_ts: i64, channel_ids: Option<Vec<String>>) -> Result<serde_json::Value, String> {
    if to_ts <= from_ts {
        return Err("fenêtre invalide".into());
    }
    let cache = shell.live.lock().unwrap();
    let ch = epg_window(&cache, from_ts, to_ts, channel_ids.as_deref());
    Ok(serde_json::json!({ "channels": ch }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xmltv_time() {
        assert_eq!(parse_time("19700101000000 +0000"), Some(0));
        assert_eq!(parse_time("20000101000000 +0100"), Some(946_684_800 - 3600));
        // 2024-02-29 12:00 à UTC-5 = 17:00 UTC = 1 709 226 000
        assert_eq!(parse_time("20240229120000 -0500"), Some(1_709_226_000));
        assert!(parse_time("bad").is_none());
    }

    #[test]
    fn m3u_attrs() {
        let l = r#"-1 tvg-id="a.fr" tvg-logo="http://x/l.png" group-title="News",Chaine A"#;
        assert_eq!(attr(l, "tvg-id"), "a.fr");
        assert_eq!(attr(l, "group-title"), "News");
        assert_eq!(attr(l, "missing"), "");
    }

    fn p(a: i64, b: i64, t: &str) -> Prog {
        Prog { start: a, stop: b, title: t.into(), desc: String::new() }
    }

    #[test]
    fn xmltv_time_variants() {
        // Même instant écrit avec plusieurs décalages
        let t = parse_time("20261123200000 +0100").unwrap();
        assert_eq!(parse_time("20261123190000 +0000"), Some(t));
        assert_eq!(parse_time("20261123190000"), Some(t));
        assert_eq!(parse_time("20261123140000 -0500"), Some(t));
        assert_eq!(parse_time("20261123200000 +01:00"), Some(t));
        assert_eq!(parse_time("20261123200000+01"), Some(t));
        assert_eq!(parse_time("20261123190000 Z"), Some(t));
        assert_eq!(parse_time("202611231930 +0030"), Some(t));
        assert!(parse_time("20261323200000 +0100").is_none());
        assert!(parse_time("20261123200000 +9900").is_none());
        // jamais de panique sur du texte non ASCII
        let _ = parse_time("20261123200000 +é");
        let _ = parse_time("2026112320000é +01é");
    }

    #[test]
    fn normalize_dedup_clamp() {
        let mut v = vec![p(200, 300, "b"), p(100, 250, "a"), p(100, 150, "dup"), p(300, 300, "vide"), p(300, 400, "c")];
        normalize(&mut v);
        let t: Vec<_> = v.iter().map(|x| (x.start, x.stop, x.title.as_str())).collect();
        // doublon de début 100 : on garde le plus long ; "a" est rogné à 200 ; "vide" écarté
        assert_eq!(t, vec![(100, 200, "a"), (200, 300, "b"), (300, 400, "c")]);
    }

    #[test]
    fn window_query() {
        let v = vec![p(0, 100, "a"), p(100, 200, "b"), p(200, 300, "c"), p(300, 400, "d")];
        let titles = |w: &[Prog]| w.iter().map(|x| x.title.clone()).collect::<Vec<_>>().join("");
        assert_eq!(titles(window(&v, 150, 250)), "bc");
        assert_eq!(titles(window(&v, 100, 200)), "b");
        assert_eq!(titles(window(&v, 400, 500)), "");
        assert_eq!(titles(window(&v, -50, 10)), "a");
        assert_eq!(titles(window(&v, 0, 1000)), "abcd");
    }

    #[test]
    fn grid_layout_maths() {
        // fenêtre de 3 h = 10 800 s
        let (l, w, cl, cr) = layout(1800, 5400, 0, 10_800).unwrap();
        assert!((l - 1.0 / 6.0).abs() < 1e-9 && (w - 1.0 / 3.0).abs() < 1e-9 && !cl && !cr);
        let (l, w, cl, cr) = layout(-3600, 3600, 0, 10_800).unwrap();
        assert!(l == 0.0 && (w - 1.0 / 3.0).abs() < 1e-9 && cl && !cr);
        let (l, w, _, cr) = layout(9000, 20_000, 0, 10_800).unwrap();
        assert!((l - 9000.0 / 10_800.0).abs() < 1e-9 && (l + w - 1.0).abs() < 1e-9 && cr);
        assert!(layout(10_800, 12_000, 0, 10_800).is_none());
        assert!(layout(-10, 0, 0, 10_800).is_none());
        assert!(layout(5, 5, 0, 10).is_none());
    }

    #[test]
    fn epg_window_by_channel() {
        let mut c = Cache::default();
        c.channels = vec![
            Channel { id: "ch0".into(), name: "A".into(), logo: String::new(), group: String::new(), url: String::new(), tvg: "a.fr".into() },
            Channel { id: "ch1".into(), name: "B".into(), logo: String::new(), group: String::new(), url: String::new(), tvg: String::new() },
        ];
        c.epg.insert("a.fr".into(), vec![p(0, 100, "x"), p(100, 200, "y")]);
        c.epg.insert("b.id".into(), vec![p(50, 150, "z")]);
        c.names.insert("b".into(), "b.id".into());
        let r = epg_window(&c, 90, 120, None);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].programs.len(), 2);
        assert_eq!(r[1].programs[0].title, "z");
        let r = epg_window(&c, 90, 120, Some(&["ch1".to_string()]));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "ch1");
    }

    #[test]
    fn auth_helpers() {
        let (u, c) = split_auth("http://bob:p%40ss@host:9981/playlist/channels?profile=pass");
        assert_eq!(u, "http://host:9981/playlist/channels?profile=pass");
        assert_eq!(c, Some(("bob".into(), "p@ss".into())));
        assert_eq!(split_auth("http://host/x").1, None);
        let cred = ("bob".to_string(), "p@ss".to_string());
        assert_eq!(inject_auth("http://host:9981/stream/channel/1", "http://bob:x@host:9981/pl", &cred), "http://bob:p%40ss@host:9981/stream/channel/1");
        // autre hôte : les identifiants ne sont pas divulgués
        assert_eq!(inject_auth("http://evil/stream", "http://bob:x@host:9981/pl", &cred), "http://evil/stream");
        assert_eq!(base64(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(percent_decode("a%2"), "a%2");
    }
}
