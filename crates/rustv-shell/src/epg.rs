//! Guide TV (EPG) automatique, facultatif (désactivé par défaut) : un fichier XMLTV public est téléchargé,
//! gardé sur disque 24 h, lu en flux et borné. Il fonctionne sans aucune source M3U ; si l'utilisateur a des
//! chaînes en direct, les chaînes du guide sont rapprochées des siennes (tvg-id puis nom normalisé) pour
//! pouvoir les regarder ou les enregistrer.
//!
//! Vie privée : seul un GET simple vers l'adresse choisie est émis (pas de cookie, pas d'identifiant,
//! User-Agent « RusTV »), au démarrage si le cache a plus de 24 h, au plus une fois toutes les 6 h
//! (y compris après un échec). Les données sont publiées par un tiers sans licence explicite.
use crate::live::{self, Channel};
use crate::usearch::text;
use crate::Shell;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufReader, Read};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const URL_TNT: &str = "https://xmltvfr.fr/xmltv/xmltv_tnt.xml.gz";
pub const URL_FR: &str = "https://xmltvfr.fr/xmltv/xmltv_fr.xml.gz";

const MAX_DOWNLOAD: u64 = 40 * 1024 * 1024;
const MAX_UNPACKED: u64 = 200 * 1024 * 1024;
const MAX_CHANNELS: usize = 200;
const MAX_PROGS: usize = 100_000;
/// Fenêtre gardée : de maintenant - 3 h à maintenant + 10 jours.
const KEEP_PAST: i64 = 3 * 3600;
const KEEP_FUTURE: i64 = 10 * 86400;
const MAX_TITLE: usize = 200;
const MAX_DESC: usize = 600;
const MAX_NAME: usize = 100;
const MAX_URL: usize = 2000;
/// Durée de validité du cache, délai minimal entre deux essais automatiques après échec / succès,
/// et délai minimal entre deux actualisations manuelles.
const TTL: i64 = 24 * 3600;
const BACKOFF: i64 = 6 * 3600;
const MIN_FORCE: i64 = 120;
/// Plage maximale renvoyée par `epg_guide`.
const MAX_WINDOW: i64 = 48 * 3600;
/// Au plus deux passages d'un même titre sur une même chaîne dans la recherche.
const MAX_PER_TITLE: usize = 2;

// ---------- configuration ----------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    /// "tnt" | "fr" | "custom"
    pub preset: String,
    /// Adresse personnalisée (utilisée si preset = "custom").
    pub url: String,
    /// Charger les images (logos, affiches) depuis les hôtes cités par le guide.
    pub icons: bool,
    pub last_fetch: i64,
    pub last_ok: i64,
    pub last_error: String,
    pub etag: String,
    pub modified: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { enabled: false, preset: "tnt".into(), url: String::new(), icons: true, last_fetch: 0, last_ok: 0, last_error: String::new(), etag: String::new(), modified: String::new() }
    }
}

impl Config {
    pub fn effective_url(&self) -> String {
        match self.preset.as_str() {
            "fr" => URL_FR.into(),
            "custom" => self.url.clone(),
            _ => URL_TNT.into(),
        }
    }
}

static CFG_LOCK: Mutex<()> = Mutex::new(());

fn load_config() -> Config {
    crate::store::load("epg")
}

fn update_config(f: impl FnOnce(&mut Config)) -> Result<Config, String> {
    let _g = CFG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut c = load_config();
    f(&mut c);
    crate::store::save("epg", &c)?;
    Ok(c)
}

fn cache_path() -> std::path::PathBuf {
    crate::settings::dir().join("epg").join("guide.cache")
}

/// Hôte (et port) d'une adresse http(s), sans identifiants ; `None` si elle en contient ou est mal formée.
fn host_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let auth = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    if auth.is_empty() || auth.contains('@') {
        return None;
    }
    Some(auth.to_ascii_lowercase())
}

fn host_only(hp: &str) -> &str {
    if let Some(r) = hp.strip_prefix('[') {
        return r.split(']').next().unwrap_or("");
    }
    hp.rsplit_once(':').map(|x| x.0).unwrap_or(hp)
}

/// Hôte d'un réseau local : adresses privées, boucle locale, noms sans point, `.local`, `.lan`, `.home.arpa`.
fn is_lan_host(hp: &str) -> bool {
    let h = host_only(hp);
    if h.is_empty() {
        return false;
    }
    if let Ok(ip) = h.parse::<std::net::Ipv4Addr>() {
        return ip.is_private() || ip.is_loopback() || ip.is_link_local();
    }
    if let Ok(ip) = h.parse::<std::net::Ipv6Addr>() {
        let s = ip.segments()[0];
        return ip.is_loopback() || (s & 0xfe00) == 0xfc00 || (s & 0xffc0) == 0xfe80;
    }
    h == "localhost" || !h.contains('.') || h.ends_with(".local") || h.ends_with(".lan") || h.ends_with(".home.arpa")
}

/// Valide une adresse de guide : https partout, http seulement pour un hôte du réseau local, sans identifiants.
pub fn validate_url(u: &str) -> Result<String, String> {
    let u = u.trim();
    if u.len() > MAX_URL {
        return Err("url_too_long".into());
    }
    if u.is_empty() || u.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("bad_url".into());
    }
    let hp = host_of(u).ok_or("bad_url")?;
    if u.starts_with("http://") && !is_lan_host(&hp) {
        return Err("http_not_lan".into());
    }
    if u.parse::<tauri::Url>().is_err() {
        return Err("bad_url".into());
    }
    Ok(u.to_string())
}

// ---------- données ----------

#[derive(Clone, Debug, Default)]
pub struct EpgProg {
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: String,
    pub category: String,
    pub icon: String,
    pub rating: String,
    /// Titre passé dans `text::fold` (recherche).
    pub key: String,
}

#[derive(Clone, Debug, Default)]
pub struct EpgChan {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub names: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Data {
    pub channels: Vec<EpgChan>,
    pub progs: HashMap<String, Vec<EpgProg>>,
    pub from: i64,
    pub to: i64,
    pub truncated: bool,
    pub gen: u64,
}

impl Data {
    pub fn programmes(&self) -> usize {
        self.progs.values().map(|v| v.len()).sum()
    }
}

pub struct Limits {
    pub max_unpacked: u64,
    pub max_channels: usize,
    pub max_progs: usize,
    pub keep_past: i64,
    pub keep_future: i64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_unpacked: MAX_UNPACKED, max_channels: MAX_CHANNELS, max_progs: MAX_PROGS, keep_past: KEEP_PAST, keep_future: KEEP_FUTURE }
    }
}

#[derive(Default, Clone)]
pub struct ChanRaw {
    pub id: String,
    pub names: Vec<String>,
    pub icon: String,
}

/// Lecteur qui échoue (et le signale) dès que plus de `left` octets sont demandés : borne la taille décompressée.
struct Cap<R> {
    r: R,
    left: u64,
    hit: Rc<Cell<bool>>,
}

impl<R: Read> Read for Cap<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            let mut b = [0u8; 1];
            if self.r.read(&mut b)? > 0 {
                self.hit.set(true);
                return Err(io::Error::new(io::ErrorKind::Other, "too_large"));
            }
            return Ok(0);
        }
        let want = buf.len().min(self.left.min(usize::MAX as u64) as usize);
        let n = self.r.read(&mut buf[..want])?;
        self.left -= n as u64;
        Ok(n)
    }
}

fn clip(s: &str, n: usize) -> String {
    s.trim().chars().take(n).collect()
}

/// Seules les images https sont gardées (jamais http, data:, file:…).
fn safe_icon(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 500 && s.starts_with("https://") && host_of(s).is_some() && !s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        s.to_string()
    } else {
        String::new()
    }
}

fn attr_of(e: &quick_xml::events::BytesStart, key: &[u8]) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.as_ref() == key).and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

/// Indices (ordre du fichier) des chaînes gardées : d'abord celles que `prefer` désigne, puis les autres dans
/// l'ordre du fichier, au plus `max`.
pub fn pick_channels(chs: &[ChanRaw], max: usize, prefer: &dyn Fn(&ChanRaw) -> bool) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..chs.len()).filter(|&i| prefer(&chs[i])).collect();
    idx.truncate(max);
    for i in 0..chs.len() {
        if idx.len() >= max {
            break;
        }
        if !idx.contains(&i) {
            idx.push(i);
        }
    }
    idx.sort_unstable();
    idx
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    None,
    Name,
    Title,
    Desc,
    Category,
    Rating,
}

/// Lit un guide XMLTV (brut ou .gz) en flux, ne garde que la fenêtre utile et applique les plafonds de `lim`.
pub fn parse_guide(bytes: &[u8], now: i64, lim: &Limits, prefer: &dyn Fn(&ChanRaw) -> bool) -> Result<Data, String> {
    let gz = bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b;
    let hit = Rc::new(Cell::new(false));
    let inner: Box<dyn Read + '_> = if gz { Box::new(GzDecoder::new(bytes)) } else { Box::new(bytes) };
    let capped = Cap { r: inner, left: lim.max_unpacked, hit: hit.clone() };
    let mut reader = Reader::from_reader(BufReader::with_capacity(64 * 1024, capped));
    reader.config_mut().trim_text(true);
    let (lo, hi) = (now - lim.keep_past, now + lim.keep_future);

    let mut raws: Vec<ChanRaw> = Vec::new();
    let mut cur_chan: Option<ChanRaw> = None;
    let mut kept: HashSet<String> = HashSet::new();
    let mut decided = false;
    let mut progs: HashMap<String, Vec<EpgProg>> = HashMap::new();
    let mut n_progs = 0usize;
    let mut truncated = false;
    let mut cur: Option<(String, EpgProg)> = None;
    let mut field = Field::None;
    let mut txt = String::new();
    let mut buf = Vec::new();
    let mut seen_any = false;

    loop {
        let ev = reader.read_event_into(&mut buf);
        match ev {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let is_start = matches!(ev, Ok(Event::Start(_)));
                match e.name().as_ref() {
                    b"channel" if is_start => {
                        cur_chan = Some(ChanRaw { id: clip(&attr_of(e, b"id").unwrap_or_default(), MAX_NAME), ..Default::default() });
                    }
                    b"display-name" if is_start && cur_chan.is_some() => {
                        field = Field::Name;
                        txt.clear();
                    }
                    b"icon" => {
                        let src = safe_icon(&attr_of(e, b"src").unwrap_or_default());
                        if let Some((_, p)) = cur.as_mut() {
                            if p.icon.is_empty() {
                                p.icon = src;
                            }
                        } else if let Some(c) = cur_chan.as_mut() {
                            if c.icon.is_empty() {
                                c.icon = src;
                            }
                        }
                    }
                    b"programme" => {
                        seen_any = true;
                        if !decided {
                            let idx = pick_channels(&raws, lim.max_channels, prefer);
                            kept = idx.iter().map(|&i| raws[i].id.clone()).collect();
                            decided = true;
                        }
                        if is_start {
                            let (mut st, mut sp, mut ch) = (None, None, String::new());
                            for a in e.attributes().flatten() {
                                let v = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
                                match a.key.as_ref() {
                                    b"start" => st = live::parse_time(&v),
                                    b"stop" => sp = live::parse_time(&v),
                                    b"channel" => ch = v,
                                    _ => {}
                                }
                            }
                            cur = match (st, sp) {
                                (Some(a), Some(b)) if b > a && b > lo && a < hi && kept.contains(&ch) => {
                                    if n_progs >= lim.max_progs {
                                        truncated = true;
                                        None
                                    } else {
                                        Some((ch, EpgProg { start: a, stop: b, ..Default::default() }))
                                    }
                                }
                                _ => None,
                            };
                        }
                    }
                    b"title" if is_start && cur.is_some() => {
                        field = Field::Title;
                        txt.clear();
                    }
                    b"desc" if is_start && cur.is_some() => {
                        field = Field::Desc;
                        txt.clear();
                    }
                    b"category" if is_start && cur.is_some() => {
                        field = Field::Category;
                        txt.clear();
                    }
                    b"value" if is_start && cur.is_some() => {
                        field = Field::Rating;
                        txt.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) if field != Field::None => {
                if let Ok(s) = t.unescape() {
                    if txt.len() < 4 * MAX_DESC {
                        txt.push_str(&s);
                    }
                }
            }
            Ok(Event::CData(ref c)) if field != Field::None => {
                if txt.len() < 4 * MAX_DESC {
                    txt.push_str(&String::from_utf8_lossy(c));
                }
            }
            Ok(Event::End(ref e)) => {
                match (e.name().as_ref(), field) {
                    (b"display-name", Field::Name) => {
                        if let Some(c) = cur_chan.as_mut() {
                            let n = clip(&txt, MAX_NAME);
                            if !n.is_empty() {
                                c.names.push(n);
                            }
                        }
                    }
                    (b"title", Field::Title) => {
                        if let Some((_, p)) = cur.as_mut() {
                            if p.title.is_empty() {
                                p.title = clip(&txt, MAX_TITLE);
                            }
                        }
                    }
                    (b"desc", Field::Desc) => {
                        if let Some((_, p)) = cur.as_mut() {
                            if p.desc.is_empty() {
                                p.desc = clip(&txt, MAX_DESC);
                            }
                        }
                    }
                    (b"category", Field::Category) => {
                        if let Some((_, p)) = cur.as_mut() {
                            if p.category.is_empty() {
                                p.category = clip(&txt, MAX_NAME);
                            }
                        }
                    }
                    (b"value", Field::Rating) => {
                        if let Some((_, p)) = cur.as_mut() {
                            if p.rating.is_empty() {
                                p.rating = clip(&txt, 20);
                            }
                        }
                    }
                    _ => {}
                }
                field = Field::None;
                match e.name().as_ref() {
                    b"channel" => {
                        if let Some(c) = cur_chan.take() {
                            // Une chaîne déclarée après le premier programme est ignorée (fichier non conforme).
                            if !decided && !c.id.is_empty() && raws.len() < 5000 {
                                raws.push(c);
                            }
                        }
                    }
                    b"programme" => {
                        if let Some((ch, mut p)) = cur.take() {
                            if !p.title.is_empty() && n_progs < lim.max_progs {
                                p.key = text::fold(&p.title);
                                progs.entry(ch).or_default().push(p);
                                n_progs += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                if hit.get() {
                    return Err("too_large".into());
                }
                if n_progs == 0 {
                    let _ = (&e, seen_any);
                    return Err("bad_xml".into());
                }
                truncated = true;
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    if hit.get() {
        return Err("too_large".into());
    }
    for v in progs.values_mut() {
        normalize(v);
    }
    progs.retain(|_, v| !v.is_empty());
    if progs.is_empty() {
        return Err("empty".into());
    }
    let channels: Vec<EpgChan> = raws
        .into_iter()
        .filter(|c| progs.contains_key(&c.id))
        .map(|c| EpgChan { name: c.names.first().cloned().unwrap_or_else(|| c.id.clone()), id: c.id, icon: c.icon, names: c.names })
        .collect();
    let from = progs.values().filter_map(|v| v.first()).map(|p| p.start).min().unwrap_or(0);
    let to = progs.values().filter_map(|v| v.last()).map(|p| p.stop).max().unwrap_or(0);
    Ok(Data { channels, progs, from, to, truncated, gen: 0 })
}

/// Trie, supprime les doublons (même début), écarte les durées nulles et rogne les chevauchements.
pub fn normalize(v: &mut Vec<EpgProg>) {
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

/// Programmes de remplissage (« Programme indéterminé », « Programmes de la nuit ») : gardés dans la grille,
/// jamais dans les résultats de recherche.
pub fn is_placeholder(p: &EpgProg) -> bool {
    let c = text::fold(&p.category);
    c == "programme indetermine" || p.key.starts_with("programmes de la nuit") || p.key == "programme indetermine"
}

/// Programmes d'une chaîne qui recouvrent [from, to) (liste triée, sans chevauchement : recherche dichotomique).
pub fn window(progs: &[EpgProg], from: i64, to: i64) -> &[EpgProg] {
    let first = progs.partition_point(|p| p.stop <= from);
    let last = progs.partition_point(|p| p.start < to);
    if first >= last {
        &[]
    } else {
        &progs[first..last]
    }
}

/// Fenêtre demandée ramenée à 48 h au plus ; `None` si elle est vide ou inversée.
pub fn clamp_window(from: i64, to: i64) -> Option<(i64, i64)> {
    if to <= from {
        return None;
    }
    Some((from, to.min(from.saturating_add(MAX_WINDOW))))
}

/// Programme en cours et suivant d'une liste triée.
pub fn now_next(progs: &[EpgProg], now: i64) -> (Option<&EpgProg>, Option<&EpgProg>) {
    match progs.iter().position(|p| p.start <= now && now < p.stop) {
        Some(i) => (progs.get(i), progs.get(i + 1)),
        None => (None, progs.iter().find(|p| p.start > now)),
    }
}

/// « Ce soir » : 20 h 50 locale aujourd'hui, ou demain si l'heure locale a dépassé 22 h 30. `tz` : décalage en secondes.
pub fn tonight(now: i64, tz: i64) -> i64 {
    let local = now + tz;
    let day = local.div_euclid(86400) * 86400;
    let sod = local - day;
    let target = day + 20 * 3600 + 50 * 60;
    (if sod > 22 * 3600 + 30 * 60 { target + 86400 } else { target }) - tz
}

// ---------- rapprochement avec les chaînes de l'utilisateur ----------

const ALIASES: &[(&str, &str)] = &[
    ("canalplus", "canal"),
    ("lachaineparlementaire", "lcp"),
    ("lcpan", "lcp"),
    ("lequipe21", "lequipe"),
    ("equipe21", "lequipe"),
    ("franceinfotv", "franceinfo"),
];
const NOISE: &[&str] = &["hd", "fhd", "uhd", "sd", "4k", "fr", "hevc", "h265"];

/// Nom de chaîne normalisé : sans casse, accents, ponctuation, suffixe « .fr », mots « HD / FHD… » ni espaces.
/// « France 2 HD », « France2.fr » et « france 2 » donnent « france2 ».
pub fn norm_name(s: &str) -> String {
    let s = s.trim();
    let s = if s.len() > 3 && s.is_char_boundary(s.len() - 3) && s[s.len() - 3..].eq_ignore_ascii_case(".fr") { &s[..s.len() - 3] } else { s };
    let folded = text::fold(s);
    let joined: String = folded.split(' ').filter(|w| !w.is_empty() && !NOISE.contains(w)).collect();
    let joined = if joined.is_empty() { folded.replace(' ', "") } else { joined };
    ALIASES.iter().find(|(a, _)| *a == joined).map(|(_, b)| b.to_string()).unwrap_or(joined)
}

/// Associe les chaînes du guide aux chaînes en direct de l'utilisateur : tvg-id d'abord, puis nom normalisé.
/// Résultat : identifiant du guide → indice dans `live` (la première chaîne gagne).
pub fn match_channels(epg: &[EpgChan], live: &[Channel]) -> HashMap<String, usize> {
    let mut by_id: HashMap<String, &str> = HashMap::new();
    let mut by_norm: HashMap<String, &str> = HashMap::new();
    for c in epg {
        by_id.entry(c.id.to_lowercase()).or_insert(&c.id);
        by_norm.entry(norm_name(&c.id)).or_insert(&c.id);
        for n in &c.names {
            by_norm.entry(norm_name(n)).or_insert(&c.id);
        }
    }
    let mut out: HashMap<String, usize> = HashMap::new();
    for (i, l) in live.iter().enumerate() {
        let hit = if !l.tvg.is_empty() { by_id.get(&l.tvg.to_lowercase()).or_else(|| by_norm.get(&norm_name(&l.tvg))) } else { None };
        let hit = hit.or_else(|| by_norm.get(&norm_name(&l.name)));
        if let Some(id) = hit {
            out.entry((*id).to_string()).or_insert(i);
        }
    }
    out
}

fn prefer_keys(live: &[Channel]) -> HashSet<String> {
    let mut s = HashSet::new();
    for c in live {
        if !c.tvg.is_empty() {
            s.insert(norm_name(&c.tvg));
        }
        s.insert(norm_name(&c.name));
    }
    s.remove("");
    s
}

// ---------- recherche ----------

pub struct RawHit<'a> {
    pub chan: &'a EpgChan,
    pub prog: &'a EpgProg,
    pub score: u32,
    pub now: bool,
    pub upcoming: usize,
}

/// Cherche dans les titres du guide (insensible à la casse et aux accents). Tri : début de titre / mot entier
/// avant simple sous-chaîne, puis programme en cours d'abord, puis le plus proche. Les programmes de
/// remplissage sont écartés, les passages consécutifs identiques fusionnés, deux passages au plus par titre et chaîne.
pub fn search<'a>(data: &'a Data, q: &str, limit: usize, now: i64) -> Vec<RawHit<'a>> {
    let fq = text::fold(q);
    if fq.chars().count() < 2 || limit == 0 {
        return vec![];
    }
    let mut hits: Vec<RawHit<'a>> = vec![];
    for c in &data.channels {
        let Some(v) = data.progs.get(&c.id) else { continue };
        let mut counts: HashMap<&str, usize> = HashMap::new();
        let mut last: Option<&EpgProg> = None;
        for p in v.iter().filter(|p| p.stop > now) {
            let repeat = last.map(|l| l.key == p.key && l.stop == p.start).unwrap_or(false);
            last = Some(p);
            if repeat || is_placeholder(p) {
                continue;
            }
            let Some(score) = text::score(&fq, &p.key) else { continue };
            let n = counts.entry(p.key.as_str()).or_insert(0);
            if *n >= MAX_PER_TITLE {
                continue;
            }
            hits.push(RawHit { chan: c, prog: p, score, now: p.start <= now, upcoming: *n });
            *n += 1;
        }
    }
    hits.sort_by(|a, b| {
        let tier = |h: &RawHit| if h.score >= 450 { 0 } else { 1 };
        tier(a)
            .cmp(&tier(b))
            .then(b.now.cmp(&a.now))
            .then(a.prog.start.cmp(&b.prog.start))
            .then(b.score.cmp(&a.score))
            .then(a.chan.name.cmp(&b.chan.name))
    });
    hits.truncate(limit);
    hits
}

// ---------- cache : quand télécharger ----------

/// Faut-il aller sur le réseau ? `Ok(())` oui ; `Err(raison)` : "disabled", "too_soon" (actualisation manuelle
/// trop rapprochée), "fresh" (cache de moins de 24 h), "backoff" (dernier essai trop récent : 6 h après un
/// échec ou un succès, 2 min tant que rien n'a jamais réussi).
pub fn should_refresh(cfg: &Config, now: i64, force: bool, has_cache: bool) -> Result<(), &'static str> {
    if !cfg.enabled {
        return Err("disabled");
    }
    // Horloge reculée : le dernier essai est dans le futur, on ne bloque pas.
    let gap = if cfg.last_fetch > now { i64::MAX } else { now - cfg.last_fetch };
    if force {
        return if gap < MIN_FORCE { Err("too_soon") } else { Ok(()) };
    }
    let ok_before = cfg.last_ok > 0 && has_cache;
    if ok_before && cfg.last_error.is_empty() && now - cfg.last_ok < TTL && cfg.last_ok <= now {
        return Err("fresh");
    }
    let min_gap = if !cfg.last_error.is_empty() || ok_before { BACKOFF } else { MIN_FORCE };
    if gap < min_gap {
        return Err("backoff");
    }
    Ok(())
}

/// Lit au plus `max` octets ; `Err("too_large")` si la source en contient davantage.
pub fn read_capped(r: &mut dyn Read, max: u64) -> Result<Vec<u8>, String> {
    let mut v = Vec::new();
    r.take(max.saturating_add(1)).read_to_end(&mut v).map_err(|_| "network".to_string())?;
    if v.len() as u64 > max {
        return Err("too_large".into());
    }
    Ok(v)
}

enum Fetched {
    NotModified,
    Body { bytes: Vec<u8>, etag: String, modified: String },
}

fn header_val(v: Option<&str>) -> String {
    v.filter(|s| s.len() <= 200 && s.chars().all(|c| c.is_ascii_graphic() || c == ' ')).unwrap_or("").to_string()
}

/// GET simple : pas de cookie, User-Agent « RusTV ». `ureq` annonce `Accept-Encoding: gzip` et décompresse
/// un éventuel Content-Encoding ; les fichiers .gz sont reconnus à leur signature.
fn download(url: &str, etag: &str, modified: &str) -> Result<Fetched, String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).timeout(Duration::from_secs(300)).redirects(3).user_agent("RusTV").build();
    let mut req = agent.get(url);
    if !etag.is_empty() {
        req = req.set("If-None-Match", etag);
    }
    if !modified.is_empty() {
        req = req.set("If-Modified-Since", modified);
    }
    let r = req.call().map_err(|e| match e {
        ureq::Error::Status(c, _) => format!("http_{c}"),
        _ => "network".to_string(),
    })?;
    if r.status() == 304 {
        return Ok(Fetched::NotModified);
    }
    if r.header("Content-Length").and_then(|v| v.trim().parse::<u64>().ok()).map(|n| n > MAX_DOWNLOAD).unwrap_or(false) {
        return Err("too_large".into());
    }
    let (e, m) = (header_val(r.header("ETag")), header_val(r.header("Last-Modified")));
    let bytes = read_capped(&mut r.into_reader(), MAX_DOWNLOAD)?;
    Ok(Fetched::Body { bytes, etag: e, modified: m })
}

// ---------- état en mémoire ----------

static DATA: Mutex<Option<Arc<Data>>> = Mutex::new(None);
static GEN: AtomicU64 = AtomicU64::new(1);
static BUSY: AtomicBool = AtomicBool::new(false);
static CACHE_TRIED: AtomicBool = AtomicBool::new(false);

type MatchMap = HashMap<String, (String, String, String)>; // id du guide → (id, nom, url) de la chaîne en direct
static MATCH: Mutex<Option<(u64, Arc<MatchMap>)>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn current() -> Option<Arc<Data>> {
    lock(&DATA).clone()
}

fn install(mut d: Data) -> Arc<Data> {
    d.gen = GEN.fetch_add(1, Ordering::SeqCst);
    let a = Arc::new(d);
    *lock(&DATA) = Some(a.clone());
    *lock(&MATCH) = None;
    a
}

fn clear_memory() {
    *lock(&DATA) = None;
    *lock(&MATCH) = None;
    CACHE_TRIED.store(false, Ordering::SeqCst);
}

fn live_snapshot_keys(app: &AppHandle) -> HashSet<String> {
    let sh = app.state::<Shell>();
    let g = lock(&sh.live);
    prefer_keys(&g.channels)
}

fn parse_with(bytes: &[u8], now: i64, keys: &HashSet<String>) -> Result<Data, String> {
    let prefer = |c: &ChanRaw| keys.contains(&norm_name(&c.id)) || c.names.iter().any(|n| keys.contains(&norm_name(n)));
    parse_guide(bytes, now, &Limits::default(), &prefer)
}

/// Charge le cache disque en mémoire s'il y en a un (une seule tentative tant que la config ne change pas).
fn ensure_loaded(app: &AppHandle) -> Option<Arc<Data>> {
    if let Some(d) = current() {
        return Some(d);
    }
    if !load_config().enabled || CACHE_TRIED.swap(true, Ordering::SeqCst) {
        return None;
    }
    let bytes = std::fs::read(cache_path()).ok()?;
    let keys = live_snapshot_keys(app);
    parse_with(&bytes, live::now_ts(), &keys).ok().map(install)
}

fn write_cache(bytes: &[u8]) -> Result<(), String> {
    let p = cache_path();
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|_| "write_failed".to_string())?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|_| "write_failed".to_string())?;
    std::fs::rename(&tmp, &p).map_err(|_| "write_failed".to_string())
}

/// Actualise le guide si les règles de cache le permettent. `Ok(true)` si le réseau a été interrogé.
fn refresh(app: &AppHandle, force: bool) -> Result<bool, String> {
    let now = live::now_ts();
    let cfg = load_config();
    let has_cache = cache_path().exists();
    match should_refresh(&cfg, now, force, has_cache) {
        Ok(()) => {}
        Err("disabled") => return Err("disabled".into()),
        Err("too_soon") => return Err("too_soon".into()),
        Err(_) => return Ok(false),
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    let res = (|| -> Result<(), String> {
        let url = validate_url(&cfg.effective_url())?;
        update_config(|c| c.last_fetch = now)?;
        let keys = live_snapshot_keys(app);
        let (etag, modified) = if has_cache { (cfg.etag.as_str(), cfg.modified.as_str()) } else { ("", "") };
        match download(&url, etag, modified)? {
            Fetched::NotModified => {
                if current().is_none() {
                    if let Ok(b) = std::fs::read(cache_path()) {
                        install(parse_with(&b, now, &keys)?);
                    }
                }
                update_config(|c| {
                    c.last_ok = now;
                    c.last_error.clear();
                })?;
            }
            Fetched::Body { bytes, etag, modified } => {
                let d = parse_with(&bytes, now, &keys)?;
                write_cache(&bytes)?;
                install(d);
                update_config(|c| {
                    c.last_ok = now;
                    c.last_error.clear();
                    c.etag = etag;
                    c.modified = modified;
                })?;
            }
        }
        Ok(())
    })();
    BUSY.store(false, Ordering::SeqCst);
    if let Err(e) = &res {
        let code = e.clone();
        let _ = update_config(|c| c.last_error = code);
    }
    let _ = app.emit("epg-changed", ());
    res.map(|_| true)
}

fn matches_for(app: &AppHandle, d: &Data) -> Arc<MatchMap> {
    let sh = app.state::<Shell>();
    let g = lock(&sh.live);
    let ch = &g.channels;
    let sample = |i: usize| ch.get(i).map(|c| format!("{}|{}", c.url, c.name)).unwrap_or_default();
    let fp = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (ch.len(), sample(0), sample(ch.len() / 2), sample(ch.len().saturating_sub(1))).hash(&mut h);
        h.finish() ^ d.gen.rotate_left(32)
    };
    if let Some((k, m)) = lock(&MATCH).as_ref() {
        if *k == fp {
            return m.clone();
        }
    }
    let map: MatchMap = match_channels(&d.channels, ch).into_iter().map(|(k, i)| (k, (ch[i].id.clone(), ch[i].name.clone(), ch[i].url.clone()))).collect();
    let a = Arc::new(map);
    *lock(&MATCH) = Some((fp, a.clone()));
    a
}

fn play_fields(m: &MatchMap, id: &str) -> (bool, Value, Value) {
    match m.get(id) {
        Some((lid, _, url)) => (true, json!(lid), json!(url)),
        None => (false, Value::Null, Value::Null),
    }
}

fn icon_of(cfg_icons: bool, s: &str) -> Value {
    if cfg_icons && !s.is_empty() {
        json!(s)
    } else {
        Value::Null
    }
}

fn status_json(cfg: &Config, d: Option<&Data>, now: i64) -> Value {
    let mut s = json!({
        "loaded": d.is_some(),
        "channels": d.map(|d| d.channels.len()).unwrap_or(0),
        "programmes": d.map(|d| d.programmes()).unwrap_or(0),
        "from": d.map(|d| d.from).unwrap_or(0),
        "to": d.map(|d| d.to).unwrap_or(0),
        "ageSecs": if cfg.last_ok > 0 { json!((now - cfg.last_ok).max(0)) } else { Value::Null },
        "truncated": d.map(|d| d.truncated).unwrap_or(false),
        "refreshing": BUSY.load(Ordering::SeqCst),
    });
    if !cfg.last_error.is_empty() {
        s["error"] = json!(cfg.last_error);
    }
    s
}

fn config_json(cfg: &Config) -> Value {
    let url = cfg.effective_url();
    json!({
        "enabled": cfg.enabled,
        "preset": cfg.preset,
        "url": cfg.url,
        "effectiveUrl": url,
        "host": host_of(&url).unwrap_or_default(),
        "icons": cfg.icons,
        "lastFetch": cfg.last_fetch,
        "lastOk": cfg.last_ok,
        "lastError": cfg.last_error,
        "presets": { "tnt": URL_TNT, "fr": URL_FR },
    })
}

fn get_json(app: &AppHandle) -> Value {
    let cfg = load_config();
    let d = if cfg.enabled { ensure_loaded(app) } else { None };
    json!({ "config": config_json(&cfg), "status": status_json(&cfg, d.as_deref(), live::now_ts()) })
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|_| "failed".to_string())?
}

// ---------- commandes ----------

/// `epg_get() -> { config:{enabled,preset,url,effectiveUrl,host,icons,lastFetch,lastOk,lastError,presets:{tnt,fr}},
/// status:{loaded,channels,programmes,from,to,ageSecs,truncated,refreshing,error?} }`.
#[tauri::command]
pub async fn epg_get(app: AppHandle) -> Result<Value, String> {
    blocking(move || Ok(get_json(&app))).await
}

/// `epg_set({ enabled?, preset?: "tnt"|"fr"|"custom", url?, icons? })` -> même forme que `epg_get`. Erreurs :
/// bad_url, url_too_long, http_not_lan, bad_preset. Changer d'adresse efface le cache. Ne télécharge rien :
/// l'interface appelle ensuite `epg_refresh`.
#[tauri::command]
pub async fn epg_set(app: AppHandle, cfg: Value) -> Result<Value, String> {
    blocking(move || {
        let old = load_config();
        let mut c = old.clone();
        if let Some(b) = cfg["enabled"].as_bool() {
            c.enabled = b;
        }
        if let Some(b) = cfg["icons"].as_bool() {
            c.icons = b;
        }
        if let Some(p) = cfg["preset"].as_str() {
            if !["tnt", "fr", "custom"].contains(&p) {
                return Err("bad_preset".to_string());
            }
            c.preset = p.into();
        }
        if let Some(u) = cfg["url"].as_str() {
            c.url = if u.trim().is_empty() { String::new() } else { validate_url(u)? };
        }
        if c.preset == "custom" {
            validate_url(&c.url)?;
        }
        if c.effective_url() != old.effective_url() {
            c.last_ok = 0;
            c.last_error.clear();
            c.etag.clear();
            c.modified.clear();
            let _ = std::fs::remove_file(cache_path());
            clear_memory();
        }
        if !c.enabled {
            clear_memory();
        }
        update_config(|x| *x = c)?;
        let _ = app.emit("epg-changed", ());
        Ok(get_json(&app))
    })
    .await
}

/// `epg_refresh(force?)` -> même forme que `epg_get`. Sans `force`, ne contacte le serveur que si le cache a plus de
/// 24 h et que le dernier essai date de plus de 6 h. Avec `force` (bouton « Actualiser »), au plus une fois toutes
/// les 2 minutes. Erreurs : disabled, too_soon, busy, network, http_<code>, too_large, bad_xml, empty, write_failed, bad_url.
#[tauri::command]
pub async fn epg_refresh(app: AppHandle, force: Option<bool>) -> Result<Value, String> {
    blocking(move || {
        refresh(&app, force.unwrap_or(false))?;
        Ok(get_json(&app))
    })
    .await
}

/// `epg_channels() -> [{ id, name, icon, playable, liveChannelId }]`.
#[tauri::command]
pub async fn epg_channels(app: AppHandle) -> Result<Value, String> {
    blocking(move || {
        let cfg = load_config();
        let Some(d) = (if cfg.enabled { ensure_loaded(&app) } else { None }) else { return Ok(json!([])) };
        let m = matches_for(&app, &d);
        let v: Vec<Value> = d
            .channels
            .iter()
            .map(|c| {
                let (playable, lid, _) = play_fields(&m, &c.id);
                json!({ "id": c.id, "name": c.name, "icon": icon_of(cfg.icons, &c.icon), "playable": playable, "liveChannelId": lid })
            })
            .collect();
        Ok(Value::Array(v))
    })
    .await
}

fn guide_json(app: &AppHandle, from: i64, to: i64, ids: Option<&[String]>) -> Result<Value, String> {
    let (from, to) = clamp_window(from, to).ok_or("bad_window")?;
    let cfg = load_config();
    let Some(d) = (if cfg.enabled { ensure_loaded(app) } else { None }) else { return Ok(json!({ "channels": [] })) };
    let m = matches_for(app, &d);
    let now = live::now_ts();
    let mut out = vec![];
    for c in &d.channels {
        if ids.map(|i| !i.iter().any(|x| x == &c.id)).unwrap_or(false) {
            continue;
        }
        let progs: Vec<Value> = d
            .progs
            .get(&c.id)
            .map(|v| {
                window(v, from, to)
                    .iter()
                    .filter_map(|p| {
                        let (left, width, clip_l, clip_r) = live::layout(p.start, p.stop, from, to)?;
                        Some(json!({
                            "start": p.start, "stop": p.stop, "title": p.title, "desc": p.desc, "category": p.category,
                            "icon": icon_of(cfg.icons, &p.icon), "rating": if p.rating.is_empty() { Value::Null } else { json!(p.rating) },
                            "now": p.start <= now && now < p.stop, "placeholder": is_placeholder(p),
                            "left": left, "width": width, "clipL": clip_l, "clipR": clip_r,
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let (playable, lid, url) = play_fields(&m, &c.id);
        out.push(json!({ "id": c.id, "name": c.name, "icon": icon_of(cfg.icons, &c.icon), "playable": playable, "liveChannelId": lid, "streamUrl": url, "programs": progs }));
    }
    Ok(json!({ "channels": out }))
}

/// `epg_guide(fromTs, toTs, channelIds?) -> { channels:[{ id, name, icon, playable, liveChannelId, streamUrl,
/// programs:[{ start, stop, title, desc, category, icon, rating, now, placeholder, left, width, clipL, clipR }] }] }`.
/// `channelIds` : identifiants du guide. Fenêtre limitée à 48 h.
#[tauri::command]
pub async fn epg_guide(app: AppHandle, from_ts: i64, to_ts: i64, channel_ids: Option<Vec<String>>) -> Result<Value, String> {
    blocking(move || guide_json(&app, from_ts, to_ts, channel_ids.as_deref())).await
}

/// Résultats de recherche (calculés localement, sans réseau). Utilisé par `usearch_query` et `epg_search`.
pub fn search_json(app: &AppHandle, q: &str, limit: usize) -> Value {
    let cfg = load_config();
    let Some(d) = (if cfg.enabled { ensure_loaded(app) } else { None }) else { return json!([]) };
    let now = live::now_ts();
    let m = matches_for(app, &d);
    let v: Vec<Value> = search(&d, q, limit.min(50), now)
        .into_iter()
        .map(|h| {
            let (playable, lid, url) = play_fields(&m, &h.chan.id);
            json!({
                "channelId": h.chan.id, "channelName": h.chan.name, "channelIcon": icon_of(cfg.icons, &h.chan.icon),
                "title": h.prog.title, "desc": h.prog.desc, "category": h.prog.category, "icon": icon_of(cfg.icons, &h.prog.icon),
                "start": h.prog.start, "stop": h.prog.stop, "now": h.now, "upcomingIndex": h.upcoming,
                "playable": playable, "liveChannelId": lid, "streamUrl": url,
            })
        })
        .collect();
    Value::Array(v)
}

/// `epg_search(q, limit) -> [{ channelId, channelName, channelIcon, title, desc, category, icon, start, stop, now,
/// upcomingIndex, playable, liveChannelId, streamUrl }]`. `upcomingIndex` : 0 = prochain passage de ce titre sur
/// cette chaîne, 1 = le suivant.
#[tauri::command]
pub async fn epg_search(app: AppHandle, q: String, limit: Option<usize>) -> Result<Value, String> {
    blocking(move || Ok(search_json(&app, &q, limit.unwrap_or(8)))).await
}

/// État du guide pour la recherche : "off" (désactivé), "empty" (activé, rien de chargé), "ok".
pub fn state_label(app: &AppHandle) -> &'static str {
    let cfg = load_config();
    if !cfg.enabled {
        "off"
    } else if current().is_some() || ensure_loaded(app).is_some() {
        "ok"
    } else {
        "empty"
    }
}

/// `epg_tonight(tzMinutes)` -> secondes Unix de « ce soir » (20 h 50 locale, demain après 22 h 30).
/// `tzMinutes` : décalage de l'heure locale de la fenêtre TV (`-new Date().getTimezoneOffset()`).
#[tauri::command]
pub async fn epg_tonight(tz: Option<i64>) -> Result<i64, String> {
    Ok(tonight(live::now_ts(), tz.unwrap_or(0).clamp(-14 * 60, 14 * 60) * 60))
}

/// `epg_live_nownext() -> { <id chaîne en direct>: { now, next } }` : en cours / suivant du guide pour les chaînes
/// de l'utilisateur rapprochées du guide (liste TV quand sa source n'a pas de guide).
#[tauri::command]
pub async fn epg_live_nownext(app: AppHandle) -> Result<Value, String> {
    blocking(move || {
        let cfg = load_config();
        let Some(d) = (if cfg.enabled { ensure_loaded(&app) } else { None }) else { return Ok(json!({})) };
        let now = live::now_ts();
        let sh = app.state::<Shell>();
        let g = lock(&sh.live);
        let map = match_channels(&d.channels, &g.channels);
        let pj = |p: &EpgProg| json!({ "start": p.start, "stop": p.stop, "title": p.title, "desc": p.desc });
        let mut out = serde_json::Map::new();
        for (eid, i) in map {
            let Some(v) = d.progs.get(&eid) else { continue };
            let (a, b) = now_next(v, now);
            if a.is_some() || b.is_some() {
                out.insert(g.channels[i].id.clone(), json!({ "now": a.map(pj), "next": b.map(pj) }));
            }
        }
        Ok(Value::Object(out))
    })
    .await
}

/// Au démarrage puis toutes les heures : charge le cache disque et, s'il a plus de 24 h (et que le dernier essai
/// date de plus de 6 h), le renouvelle en arrière-plan. Ne fait rien tant que le guide est désactivé.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(30));
        loop {
            if load_config().enabled {
                ensure_loaded(&app);
                let _ = refresh(&app, false);
            }
            std::thread::sleep(Duration::from_secs(3600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> i64 {
        live::parse_time(s).unwrap()
    }
    // 2026-11-23 20:00 à Paris (UTC+1)
    fn now() -> i64 {
        t("20261123200000 +0100")
    }
    fn none(_: &ChanRaw) -> bool {
        false
    }

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv generator-info-name="test">
<channel id="TF1.fr"><display-name lang="fr">TF1</display-name><icon src="https://img.example/tf1.png"/></channel>
<channel id="France2.fr"><display-name lang="fr">France 2</display-name><icon src="http://img.example/f2.png"/></channel>
<channel id="BFMTV.fr"><display-name lang="fr">BFM TV</display-name></channel>
<channel id="Vide.fr"><display-name>Vide</display-name></channel>
<programme start="20261123180000 +0100" stop="20261123193000 +0100" channel="TF1.fr"><title lang="fr">Le Journal</title></programme>
<programme start="20261123193000 +0100" stop="20261123205500 +0100" channel="TF1.fr"><title lang="fr">Les Chiffres &amp; Lettres</title><desc lang="fr">Un jeu &quot;de mots&quot;.</desc><category lang="fr">Jeu</category><icon src="https://img.example/p1.jpg"/><rating system="CSA"><value>-10</value></rating></programme>
<programme start="20261123205500 +0100" stop="20261123223000 +0100" channel="TF1.fr"><title>Film : Amélie</title><category>Film</category></programme>
<programme start="20261123223000 +0100" stop="20261123233000 +0100" channel="TF1.fr"><title>Film : Amélie</title></programme>
<programme start="20261123233000 +0100" stop="20261124010000 +0100" channel="TF1.fr"><title>Programmes de la nuit</title><category>Programme indéterminé</category></programme>
<programme start="20261123214000 +0200" stop="20261123225000 +0200" channel="France2.fr"><title>Envoyé spécial</title><icon src="http://x/y.png"/></programme>
<programme start="20261123190000 +0100" stop="20261123200000 +0100" channel="BFMTV.fr"><title>Amélie la matinale</title></programme>
<programme start="20261123200000 +0100" stop="20261123210000 +0100" channel="BFMTV.fr"><title>Debat</title></programme>
<programme start="20261123100000 +0100" stop="20261123110000 +0100" channel="TF1.fr"><title>Trop vieux</title></programme>
<programme start="20271201100000 +0100" stop="20271201110000 +0100" channel="TF1.fr"><title>Trop loin</title></programme>
<programme start="20261123200000 +0100" stop="20261123210000 +0100" channel="Inconnue.fr"><title>Orphelin</title></programme>
</tv>"#;

    fn sample() -> Data {
        parse_guide(SAMPLE.as_bytes(), now(), &Limits::default(), &none).unwrap()
    }

    #[test]
    fn parse_sample() {
        let d = sample();
        let ids: Vec<_> = d.channels.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["TF1.fr", "France2.fr", "BFMTV.fr"], "chaîne sans programme écartée, ordre du fichier gardé");
        assert_eq!(d.channels[0].icon, "https://img.example/tf1.png");
        assert_eq!(d.channels[1].icon, "", "icône http refusée");
        let tf1 = &d.progs["TF1.fr"];
        // "Trop vieux" et "Trop loin" écartés, "Programmes de la nuit" gardé dans la grille
        let titles: Vec<_> = tf1.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, vec!["Le Journal", "Les Chiffres & Lettres", "Film : Amélie", "Film : Amélie", "Programmes de la nuit"]);
        assert_eq!(tf1[1].desc, "Un jeu \"de mots\".");
        assert_eq!(tf1[1].category, "Jeu");
        assert_eq!(tf1[1].icon, "https://img.example/p1.jpg");
        assert_eq!(tf1[1].rating, "-10");
        // +0200 : 21:40 +0200 = 20:40 +0100
        assert_eq!(d.progs["France2.fr"][0].start, t("20261123204000 +0100"));
        assert_eq!(d.progs["France2.fr"][0].icon, "", "affiche http refusée");
        assert!(!d.progs.contains_key("Inconnue.fr"));
        assert_eq!(d.from, t("20261123180000 +0100"));
        assert_eq!(d.to, t("20261124010000 +0100"));
        assert!(!d.truncated);
    }

    #[test]
    fn parse_gzip_and_errors() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(SAMPLE.as_bytes()).unwrap();
        let gz = e.finish().unwrap();
        assert_eq!(parse_guide(&gz, now(), &Limits::default(), &none).unwrap().programmes(), sample().programmes());
        assert_eq!(parse_guide(b"", now(), &Limits::default(), &none).unwrap_err(), "empty");
        assert!(parse_guide(b"<html><body>nope</body></html>", now(), &Limits::default(), &none).is_err());
        assert!(parse_guide(b"<tv><programme", now(), &Limits::default(), &none).is_err());
    }

    #[test]
    fn caps_and_oversize() {
        // taille décompressée
        let lim = Limits { max_unpacked: 300, ..Default::default() };
        assert_eq!(parse_guide(SAMPLE.as_bytes(), now(), &lim, &none).unwrap_err(), "too_large");
        // un gzip qui se décompresse en beaucoup plus que la limite
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;
        let mut e = GzEncoder::new(Vec::new(), Compression::best());
        let mut big = String::from("<tv>");
        for _ in 0..2000 {
            big.push_str("<!-- remplissage remplissage remplissage -->");
        }
        big.push_str("</tv>");
        e.write_all(big.as_bytes()).unwrap();
        let gz = e.finish().unwrap();
        assert!(gz.len() < 2000);
        let lim = Limits { max_unpacked: 5000, ..Default::default() };
        assert_eq!(parse_guide(&gz, now(), &lim, &none).unwrap_err(), "too_large");
        // nombre de programmes
        let lim = Limits { max_progs: 3, ..Default::default() };
        let d = parse_guide(SAMPLE.as_bytes(), now(), &lim, &none).unwrap();
        assert_eq!(d.programmes(), 3);
        assert!(d.truncated);
        // nombre de chaînes : on préfère celles de l'utilisateur
        let lim = Limits { max_channels: 1, ..Default::default() };
        let d = parse_guide(SAMPLE.as_bytes(), now(), &lim, &|c| c.id == "BFMTV.fr").unwrap();
        assert_eq!(d.channels.len(), 1);
        assert_eq!(d.channels[0].id, "BFMTV.fr");
        let d = parse_guide(SAMPLE.as_bytes(), now(), &lim, &none).unwrap();
        assert_eq!(d.channels[0].id, "TF1.fr", "sans préférence : ordre du fichier");
        // fenêtre de conservation
        let lim = Limits { keep_past: 0, keep_future: 3600, ..Default::default() };
        let d = parse_guide(SAMPLE.as_bytes(), now(), &lim, &none).unwrap();
        assert!(d.progs["TF1.fr"].iter().all(|p| p.stop > now() && p.start < now() + 3600));
        // téléchargement
        assert_eq!(read_capped(&mut &b"abcdef"[..], 5).unwrap_err(), "too_large");
        assert_eq!(read_capped(&mut &b"abcde"[..], 5).unwrap(), b"abcde");
        // titres et descriptions bornés
        let long = format!("<tv><channel id=\"a\"><display-name>A</display-name></channel><programme start=\"20261123200000 +0100\" stop=\"20261123210000 +0100\" channel=\"a\"><title>{}</title><desc>{}</desc></programme></tv>", "t".repeat(900), "d".repeat(5000));
        let d = parse_guide(long.as_bytes(), now(), &Limits::default(), &none).unwrap();
        assert_eq!(d.progs["a"][0].title.chars().count(), MAX_TITLE);
        assert_eq!(d.progs["a"][0].desc.chars().count(), MAX_DESC);
    }

    #[test]
    fn pick_prefers_user_channels() {
        let c = |id: &str| ChanRaw { id: id.into(), ..Default::default() };
        let v = vec![c("a"), c("b"), c("c"), c("d")];
        assert_eq!(pick_channels(&v, 2, &|c| c.id == "c" || c.id == "d"), vec![2, 3]);
        assert_eq!(pick_channels(&v, 3, &|c| c.id == "d"), vec![0, 1, 3]);
        assert_eq!(pick_channels(&v, 10, &none), vec![0, 1, 2, 3]);
    }

    #[test]
    fn placeholders() {
        let d = sample();
        let nuit = d.progs["TF1.fr"].last().unwrap();
        assert!(is_placeholder(nuit));
        assert!(!is_placeholder(&d.progs["TF1.fr"][0]));
        // recherche : jamais de programme de remplissage
        assert!(search(&d, "programmes de la nuit", 8, now()).is_empty());
        assert!(search(&d, "indetermine", 8, now()).is_empty());
    }

    #[test]
    fn search_ranking_and_dedupe() {
        let d = sample();
        let n = now();
        // « amelie » : insensible aux accents ; Film : Amélie est un mot entier, « Amélie la matinale » est un début de titre.
        let h = search(&d, "AMELIE", 8, n);
        let t: Vec<_> = h.iter().map(|x| (x.chan.id.as_str(), x.prog.title.as_str())).collect();
        // BFMTV « Amélie la matinale » est terminé (20:00) : écarté. Les deux passages consécutifs de TF1 sont fusionnés.
        assert_eq!(t, vec![("TF1.fr", "Film : Amélie")]);
        assert_eq!(h[0].upcoming, 0);
        assert!(!h[0].now);
        // en cours d'abord, puis le plus proche
        let h = search(&d, "e", 8, n);
        assert!(h.is_empty(), "une lettre : pas de recherche");
        let h = search(&d, "de", 8, n);
        assert!(!h.is_empty());
        let h = search(&d, "chiffres", 8, n);
        assert_eq!(h.len(), 1);
        assert!(h[0].now, "Chiffres & Lettres passe de 19:30 à 20:55");
        // début de titre avant simple sous-chaîne (« Bazoo », pourtant en cours), quel que soit l'horaire
        let mut d2 = Data::default();
        let mk = |s: i64, title: &str| EpgProg { start: s, stop: s + 600, title: title.into(), key: text::fold(title), ..Default::default() };
        d2.channels = vec![EpgChan { id: "x".into(), name: "X".into(), ..Default::default() }];
        d2.progs.insert("x".into(), vec![mk(n - 100, "Bazoo"), mk(n + 700, "Zoo de nuit"), mk(n + 1400, "Zoo de nuit"), mk(n + 5000, "Zoo de nuit"), mk(n + 9000, "Zoo de nuit")]);
        let h = search(&d2, "zoo", 8, n);
        let t: Vec<_> = h.iter().map(|x| (x.prog.title.as_str(), x.upcoming)).collect();
        assert_eq!(t, vec![("Zoo de nuit", 0), ("Zoo de nuit", 1), ("Bazoo", 0)]);
        assert_eq!(search(&d2, "zoo", 1, n).len(), 1);
    }

    #[test]
    fn now_next_and_windows() {
        let d = sample();
        let n = now();
        let v = &d.progs["TF1.fr"];
        let (a, b) = now_next(v, n);
        assert_eq!(a.unwrap().title, "Les Chiffres & Lettres");
        assert_eq!(b.unwrap().title, "Film : Amélie");
        let (a, b) = now_next(v, t("20261123120000 +0100"));
        assert!(a.is_none());
        assert_eq!(b.unwrap().title, "Le Journal");
        let (a, b) = now_next(v, t("20261125120000 +0100"));
        assert!(a.is_none() && b.is_none());
        assert_eq!(window(v, t("20261123194000 +0100"), t("20261123210000 +0100")).len(), 2);
        assert!(window(v, t("20261130000000 +0100"), t("20261130010000 +0100")).is_empty());
        assert_eq!(clamp_window(0, 100), Some((0, 100)));
        assert_eq!(clamp_window(0, 10 * 86400), Some((0, MAX_WINDOW)));
        assert_eq!(clamp_window(100, 100), None);
        assert_eq!(clamp_window(100, 50), None);
        assert_eq!(clamp_window(i64::MAX - 5, i64::MAX), Some((i64::MAX - 5, i64::MAX)));
    }

    #[test]
    fn tonight_logic() {
        let tz = 3600; // Paris, hiver
        let at = |s: &str| t(&format!("{s} +0100"));
        assert_eq!(tonight(at("20261123101500"), tz), at("20261123205000"));
        assert_eq!(tonight(at("20261123210000"), tz), at("20261123205000"), "déjà commencé mais avant 22 h 30 : encore ce soir");
        assert_eq!(tonight(at("20261123223000"), tz), at("20261123205000"));
        assert_eq!(tonight(at("20261123223100"), tz), at("20261124205000"));
        assert_eq!(tonight(at("20261124003000"), tz), at("20261124205000"), "après minuit : le soir du même jour local");
        assert_eq!(tonight(at("20261123120000"), 0), t("20261123205000 +0000"), "UTC");
    }

    #[test]
    fn name_normalization_and_matching() {
        for (a, b) in [("France 2", "France2.fr"), ("France 2 HD", "france2"), ("M6", "M6 HD"), ("Arte", "ARTE HD"), ("CNews", "CNEWS.fr"), ("BFM TV", "BFMTV.fr"), ("Canal+", "Canal Plus"), ("TF1 Séries-Films", "TF1SeriesFilms.fr"), ("L'Équipe", "LEquipe.fr"), ("W9 FHD", "w9")] {
            assert_eq!(norm_name(a), norm_name(b), "{a} / {b}");
        }
        assert_ne!(norm_name("France 2"), norm_name("France 3"));
        assert_ne!(norm_name("TF1"), norm_name("TF1 Séries Films"));
        assert_eq!(norm_name("HD"), "hd", "jamais de nom vide");
        assert_eq!(norm_name(""), "");
        let e = |id: &str, name: &str| EpgChan { id: id.into(), name: name.into(), names: vec![name.into()], ..Default::default() };
        let epg = vec![e("TF1.fr", "TF1"), e("France2.fr", "France 2"), e("BFMTV.fr", "BFM TV"), e("M6.fr", "M6")];
        let l = |name: &str, tvg: &str| Channel { id: format!("ch-{name}"), name: name.into(), logo: String::new(), group: String::new(), url: "http://x".into(), tvg: tvg.into() };
        let live = vec![
            l("Chaîne bizarre", "tf1.fr"),  // tvg-id sans respect de la casse
            l("FR: France 2 HD", ""),       // « FR » est du bruit : correspond aussi
            l("France 2 HD", ""),           // nom
            l("BFM TV", "inconnu"),         // tvg inconnu, repli sur le nom
            l("M6 HD", ""),
            l("M6 HD (bis)", ""),
            l("TV inconnue", ""),
        ];
        let m = match_channels(&epg, &live);
        assert_eq!(m.get("TF1.fr"), Some(&0));
        assert_eq!(m.get("France2.fr"), Some(&1), "la première chaîne gagne");
        assert_eq!(m.get("BFMTV.fr"), Some(&3));
        assert_eq!(m.get("M6.fr"), Some(&4), "la première chaîne gagne");
        assert_eq!(m.len(), 4);
        let keys = prefer_keys(&live);
        assert!(keys.contains("france2") && keys.contains("tf1") && !keys.contains(""));
    }

    #[test]
    fn url_validation() {
        assert!(validate_url(URL_TNT).is_ok() && validate_url(URL_FR).is_ok());
        assert!(validate_url("https://example.org/guide.xml").is_ok());
        assert!(validate_url("  https://example.org:8443/g.xml.gz ").is_ok());
        assert!(validate_url("http://192.168.1.10:8080/xmltv").is_ok());
        assert!(validate_url("http://10.0.0.2/x").is_ok());
        assert!(validate_url("http://nas.local/guide.xml").is_ok());
        assert!(validate_url("http://nas/guide.xml").is_ok());
        assert!(validate_url("http://localhost:9981/xmltv").is_ok());
        assert!(validate_url("http://[fd00::1]/x").is_ok());
        assert_eq!(validate_url("http://example.org/x").unwrap_err(), "http_not_lan");
        assert_eq!(validate_url("http://8.8.8.8/x").unwrap_err(), "http_not_lan");
        assert_eq!(validate_url("http://172.32.0.1/x").unwrap_err(), "http_not_lan");
        assert_eq!(validate_url("ftp://example.org/x").unwrap_err(), "bad_url");
        assert_eq!(validate_url("file:///C:/x.xml").unwrap_err(), "bad_url");
        assert_eq!(validate_url("javascript:alert(1)").unwrap_err(), "bad_url");
        assert_eq!(validate_url("https://user:pw@example.org/x").unwrap_err(), "bad_url");
        assert_eq!(validate_url("https://exa mple.org/x").unwrap_err(), "bad_url");
        assert_eq!(validate_url("https://").unwrap_err(), "bad_url");
        assert_eq!(validate_url("").unwrap_err(), "bad_url");
        assert_eq!(validate_url(&format!("https://example.org/{}", "a".repeat(2000))).unwrap_err(), "url_too_long");
        assert_eq!(host_of(URL_TNT).as_deref(), Some("xmltvfr.fr"));
    }

    #[test]
    fn refresh_state_machine() {
        let h = 3600;
        let n = 1_000_000_000;
        let mut c = Config::default();
        assert_eq!(should_refresh(&c, n, false, false), Err("disabled"));
        assert_eq!(should_refresh(&c, n, true, false), Err("disabled"));
        c.enabled = true;
        // jamais téléchargé
        assert_eq!(should_refresh(&c, n, false, false), Ok(()));
        // essai échoué à l'instant : 2 min de répit tant que rien n'a jamais réussi... et 6 h après une erreur
        c.last_fetch = n - 60;
        assert_eq!(should_refresh(&c, n, false, false), Err("backoff"));
        c.last_error = "network".into();
        c.last_fetch = n - 5 * h;
        assert_eq!(should_refresh(&c, n, false, false), Err("backoff"));
        c.last_fetch = n - 6 * h;
        assert_eq!(should_refresh(&c, n, false, false), Ok(()));
        // succès récent : cache frais
        let mut c = Config { enabled: true, last_ok: n - 3 * h, last_fetch: n - 3 * h, ..Default::default() };
        assert_eq!(should_refresh(&c, n, false, true), Err("fresh"));
        // 24 h passées : on actualise
        c.last_ok = n - 25 * h;
        c.last_fetch = n - 25 * h;
        assert_eq!(should_refresh(&c, n, false, true), Ok(()));
        // 24 h passées mais dernier essai (échoué) il y a 1 h : backoff
        c.last_fetch = n - h;
        c.last_error = "http_503".into();
        assert_eq!(should_refresh(&c, n, false, true), Err("backoff"));
        // cache disque disparu : on retélécharge (après 2 min)
        let c2 = Config { enabled: true, last_ok: n - h, last_fetch: n - h, ..Default::default() };
        assert_eq!(should_refresh(&c2, n, false, false), Ok(()));
        // manuel : au plus toutes les 2 minutes, même si le cache est frais
        let c3 = Config { enabled: true, last_ok: n - 60, last_fetch: n - 60, ..Default::default() };
        assert_eq!(should_refresh(&c3, n, true, true), Err("too_soon"));
        assert_eq!(should_refresh(&Config { last_fetch: n - 121, ..c3.clone() }, n, true, true), Ok(()));
        // horloge reculée
        let c4 = Config { enabled: true, last_ok: n + 10 * h, last_fetch: n + 10 * h, ..Default::default() };
        assert_eq!(should_refresh(&c4, n, false, true), Ok(()));
        assert_eq!(should_refresh(&c4, n, true, true), Ok(()));
    }

    #[test]
    fn config_defaults_and_urls() {
        let c = Config::default();
        assert!(!c.enabled, "désactivé par défaut");
        assert_eq!(c.effective_url(), URL_TNT);
        let c: Config = serde_json::from_str(r#"{"preset":"fr"}"#).unwrap();
        assert_eq!(c.effective_url(), URL_FR);
        let c: Config = serde_json::from_str(r#"{"preset":"custom","url":"https://a.b/c"}"#).unwrap();
        assert_eq!(c.effective_url(), "https://a.b/c");
        assert!(c.icons && !c.enabled);
    }

    #[test]
    fn normalize_dedup_clamp() {
        let p = |a: i64, b: i64, t: &str| EpgProg { start: a, stop: b, title: t.into(), ..Default::default() };
        let mut v = vec![p(200, 300, "b"), p(100, 250, "a"), p(100, 150, "dup"), p(300, 300, "vide"), p(300, 400, "c")];
        normalize(&mut v);
        let t: Vec<_> = v.iter().map(|x| (x.start, x.stop, x.title.as_str())).collect();
        assert_eq!(t, vec![(100, 200, "a"), (200, 300, "b"), (300, 400, "c")]);
    }
}
