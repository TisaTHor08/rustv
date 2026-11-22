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
/// Schémas autorisés pour les flux (pas de file:, edl:, av:… qui pourraient lire des fichiers locaux).
const SCHEMES: &[&str] = &["http", "https", "rtmp", "rtmps", "rtsp", "udp", "rtp", "srt", "mms", "mmsh"];

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

fn open(src: &str, max: u64) -> Result<Box<dyn Read + Send>, String> {
    let raw: Box<dyn Read + Send> = if src.starts_with("http://") || src.starts_with("https://") {
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).user_agent("RusTV").build();
        let r = agent.get(src).call().map_err(|e| format!("téléchargement impossible : {e}"))?;
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
                    c.url = l;
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

/// « 20261123200000 +0100 » → secondes Unix (UTC).
fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 12 {
        return None;
    }
    let n = |a: usize, b: usize| digits.get(a..b).and_then(|x| x.parse::<i64>().ok());
    let (y, mo, d, h, mi) = (n(0, 4)?, n(4, 6)?, n(6, 8)?, n(8, 10)?, n(10, 12)?);
    let sec = n(12, 14).unwrap_or(0);
    // jours depuis 1970 (algorithme de Howard Hinnant)
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let mut t = days * 86400 + h * 3600 + mi * 60 + sec;
    let tz = s[digits.len()..].trim();
    if tz.len() >= 5 && (tz.starts_with('+') || tz.starts_with('-')) {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let (th, tm) = (tz[1..3].parse::<i64>().ok()?, tz[3..5].parse::<i64>().ok()?);
        t -= sign * (th * 3600 + tm * 60);
    }
    Some(t)
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Lit un guide XMLTV en flux et ne garde que les programmes des prochaines 36 h (et des 3 dernières).
pub fn parse_xmltv(src: &str) -> Result<(HashMap<String, Vec<Prog>>, HashMap<String, String>), String> {
    let mut reader = Reader::from_reader(BufReader::new(open(src, MAX_XMLTV)?));
    reader.config_mut().trim_text(true);
    let now = now_ts();
    let (lo, hi) = (now - 3 * 3600, now + 36 * 3600);
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
        v.sort_by_key(|p| p.start);
    }
    Ok((epg, names))
}

#[derive(Serialize)]
pub struct NowNext {
    pub now: Option<Prog>,
    pub next: Option<Prog>,
}

pub fn now_next(cache: &Cache) -> HashMap<String, NowNext> {
    let now = now_ts();
    let mut out = HashMap::new();
    for c in &cache.channels {
        let key = if !c.tvg.is_empty() && cache.epg.contains_key(&c.tvg) { Some(c.tvg.clone()) } else { cache.names.get(&c.name.to_lowercase()).cloned() };
        let Some(progs) = key.and_then(|k| cache.epg.get(&k)) else { continue };
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
}
