//! Enregistreur (DVR) : programmation d'enregistrements de la TV en direct.
//!
//! Deux méthodes, choisies selon l'URL (`choose_method`) :
//! - `Http` : flux HTTP MPEG-TS brut (HDHomeRun, TVHeadend, la plupart des IPTV) copié tel quel dans un
//!   fichier `.ts` par un thread `ureq`. Aucun processus externe, reconnexion automatique si le flux coupe
//!   (les `.ts` se concatènent sans problème). C'est le chemin le plus robuste.
//! - `Mpv` : tout le reste (HLS `.m3u8`, rtmp, rtsp, udp, srt…) : mpv en mode « dump » sans fenêtre,
//!   `--o=<fichier>.mkv --ovc=copy --oac=copy --of=matroska` (pas de décodage, pas de rendu, aucune fenêtre).
//!   On a préféré `--o` à `--stream-record` : ce dernier exige une lecture complète (fenêtre / sortie audio à
//!   neutraliser) alors que `--o` copie les paquets seulement. L'arrêt est demandé par l'IPC (`quit`), ce qui
//!   laisse mpv finaliser le Matroska ; on ne tue le processus (celui que l'on a lancé, jamais un autre)
//!   qu'après 8 s sans réponse.
//!
//! L'état (réglages, enregistrements, séries) est dans `%APPDATA%\RusTV\dvr.json` via `crate::store`.
//! Un enregistrement interrompu par un arrêt brutal est marqué `failed` (le fichier est conservé) ; s'il
//! reste du temps dans sa plage, la suite est reprogrammée dans un nouveau fichier.
//! Limite connue : les URL de flux (donc d'éventuels identifiants `user:pass@`) sont stockées dans dvr.json
//! et visibles dans la ligne de commande de mpv.
use crate::{live, player, store, Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const MAX_RECORDINGS: usize = 500;
const MAX_SERIES: usize = 100;
const MAX_DURATION_S: i64 = 12 * 3600;
const STOP_USER: u8 = 1;
const STOP_END: u8 = 2;

// ---------- données ----------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Scheduled,
    Recording,
    Done,
    Failed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Recording {
    pub id: String,
    pub channel_id: String,
    pub channel_name: String,
    pub url: String,
    pub title: String,
    /// Début et fin du programme (secondes Unix), hors marges.
    pub start: i64,
    pub stop: i64,
    /// Marges en minutes.
    pub pad_before: u32,
    pub pad_after: u32,
    pub status: Status,
    pub file: String,
    pub size: u64,
    pub error: String,
    pub created: i64,
}

impl Recording {
    pub fn eff_start(&self) -> i64 {
        self.start - self.pad_before as i64 * 60
    }
    pub fn eff_end(&self) -> i64 {
        self.stop + self.pad_after as i64 * 60
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub folder: String,
    pub pad_before: u32,
    pub pad_after: u32,
    pub max_parallel: u32,
    /// 0 = désactivé.
    pub retention_gb: u32,
    pub retention_days: u32,
    /// Espace disque minimal à conserver (Go).
    pub min_free_gb: u32,
    /// Décalage du fuseau local en minutes à l'est d'UTC (renseigné par l'interface, pour les noms de fichiers).
    pub tz: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self { folder: default_folder(), pad_before: 1, pad_after: 3, max_parallel: 2, retention_gb: 0, retention_days: 0, min_free_gb: 2, tz: 0 }
    }
}

pub fn default_folder() -> String {
    // `~/Videos` (Windows, Linux ; XDG_VIDEOS_DIR prioritaire), `~/Movies` (macOS).
    let base = rustv_core::platform::video_dir().unwrap_or_else(|| std::env::temp_dir().join("Videos"));
    base.join("RusTV Recordings").to_string_lossy().into_owned()
}

/// Valide et borne les réglages.
pub fn sanitize_config(mut c: Config) -> Result<Config, String> {
    c.folder = c.folder.trim().to_string();
    if c.folder.is_empty() {
        c.folder = default_folder();
    }
    if c.folder.contains('\0') || !Path::new(&c.folder).is_absolute() {
        return Err("folder_invalid".into());
    }
    c.pad_before = c.pad_before.min(120);
    c.pad_after = c.pad_after.min(120);
    c.max_parallel = c.max_parallel.clamp(1, 8);
    c.retention_gb = c.retention_gb.min(100_000);
    c.retention_days = c.retention_days.min(3650);
    c.min_free_gb = c.min_free_gb.min(1000);
    c.tz = c.tz.clamp(-14 * 60, 14 * 60);
    Ok(c)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SeriesRule {
    pub title: String,
    pub channel_id: String,
    pub channel_name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct DvrFile {
    config: Config,
    recordings: Vec<Recording>,
    series: Vec<SeriesRule>,
}

// ---------- logique pure ----------

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    Start(String),
    Stop(String),
    Missed(String),
}

/// Que faire à l'instant `now` ? Démarrer les enregistrements dont la fenêtre (avec marges) est ouverte
/// tant qu'il reste de la place (`max_parallel`), arrêter ceux dont la fenêtre est close, déclarer
/// manqués ceux qui n'ont jamais pu démarrer.
pub fn decide(recs: &[Recording], now: i64, max_parallel: usize) -> Vec<Action> {
    let mut out = Vec::new();
    let mut active = 0usize;
    for r in recs.iter().filter(|r| r.status == Status::Recording) {
        if now >= r.eff_end() {
            out.push(Action::Stop(r.id.clone()));
        } else {
            active += 1;
        }
    }
    let mut cands: Vec<&Recording> = recs.iter().filter(|r| r.status == Status::Scheduled).collect();
    cands.sort_by(|a, b| a.eff_start().cmp(&b.eff_start()).then(a.id.cmp(&b.id)));
    for r in cands {
        if now >= r.eff_end() {
            out.push(Action::Missed(r.id.clone()));
        } else if now >= r.eff_start() && active < max_parallel {
            out.push(Action::Start(r.id.clone()));
            active += 1;
        }
    }
    out
}

/// Ajouter cet enregistrement dépasserait-il `max_parallel` enregistrements simultanés (tuners) à un moment ?
pub fn overlap_exceeds(recs: &[Recording], start: i64, stop: i64, pad_before: u32, pad_after: u32, max_parallel: usize) -> bool {
    let (ns, ne) = (start - pad_before as i64 * 60, stop + pad_after as i64 * 60);
    let ivs: Vec<(i64, i64)> = recs
        .iter()
        .filter(|r| matches!(r.status, Status::Scheduled | Status::Recording))
        .map(|r| (r.eff_start(), r.eff_end()))
        .filter(|(s, e)| *s < ne && *e > ns)
        .collect();
    let mut points = vec![ns];
    points.extend(ivs.iter().map(|x| x.0).filter(|s| *s > ns));
    points.iter().any(|p| 1 + ivs.iter().filter(|(s, e)| s <= p && p < e).count() > max_parallel)
}

/// Reprise après un arrêt brutal : les enregistrements « en cours » deviennent `failed` (fichier conservé) ;
/// s'il reste plus de 30 s dans leur plage, une suite est reprogrammée (nouvel id, nouveau fichier).
pub fn recover(recs: &mut Vec<Recording>, now: i64, mut mkid: impl FnMut() -> String) -> usize {
    let mut cont = Vec::new();
    for r in recs.iter_mut().filter(|r| r.status == Status::Recording) {
        r.status = Status::Failed;
        r.error = "interrupted".into();
        if now + 30 < r.eff_end() {
            let mut n = r.clone();
            n.id = mkid();
            n.status = Status::Scheduled;
            n.file.clear();
            n.size = 0;
            n.error.clear();
            n.created = now;
            // La suite démarre tout de suite : plus de marge avant.
            n.pad_before = 0;
            n.start = n.start.min(now);
            n.title = format!("{} (2)", r.title.trim_end_matches(" (2)"));
            cont.push(n);
        }
    }
    let n = cont.len();
    recs.extend(cont);
    n
}

/// Enregistrements terminés à supprimer : plus vieux que `max_days`, puis les plus anciens jusqu'à ce que le
/// total repasse sous `max_bytes`. 0 = pas de limite.
pub fn retention_victims(recs: &[Recording], now: i64, max_bytes: u64, max_days: u32) -> Vec<String> {
    let mut done: Vec<&Recording> = recs.iter().filter(|r| r.status == Status::Done).collect();
    done.sort_by_key(|r| r.stop);
    let mut out: Vec<String> = Vec::new();
    if max_days > 0 {
        let cutoff = now - max_days as i64 * 86400;
        out.extend(done.iter().filter(|r| r.stop < cutoff).map(|r| r.id.clone()));
    }
    if max_bytes > 0 {
        let mut total: u64 = done.iter().filter(|r| !out.contains(&r.id)).map(|r| r.size).sum();
        for r in &done {
            if total <= max_bytes {
                break;
            }
            if !out.contains(&r.id) {
                total = total.saturating_sub(r.size);
                out.push(r.id.clone());
            }
        }
    }
    out
}

/// Programmes à venir portant ce titre (insensible à la casse), pas encore programmés.
pub fn series_pick(progs: &[live::Prog], title: &str, now: i64, horizon: i64, existing_starts: &[i64]) -> Vec<live::Prog> {
    let key = title.trim().to_lowercase();
    if key.is_empty() {
        return Vec::new();
    }
    progs.iter().filter(|p| p.title.trim().to_lowercase() == key && p.start > now && p.start < now + horizon && !existing_starts.contains(&p.start)).cloned().collect()
}

/// Remplace les caractères interdits dans un nom de fichier Windows, évite les noms réservés, borne la longueur.
pub fn sanitize_filename(s: &str, max: usize) -> String {
    let mapped: String = s.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let mut o: String = mapped.trim().chars().take(max).collect();
    o = o.trim_end_matches(['.', ' ']).to_string();
    let stem = o.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit() && stem.as_bytes()[3] != b'0');
    if reserved {
        o = format!("_{o}");
    }
    if o.is_empty() {
        "recording".into()
    } else {
        o
    }
}

/// « 2026-11-23 20-00 » à partir d'un horodatage Unix et d'un décalage de fuseau (minutes à l'est d'UTC).
pub fn fmt_local(ts: i64, tz_min: i32) -> String {
    let t = ts + tz_min as i64 * 60;
    let days = t.div_euclid(86400);
    let rem = t.rem_euclid(86400);
    // civil_from_days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}-{:02}", rem / 3600, rem % 3600 / 60)
}

/// Chemin du fichier : `<dossier>/<chaîne> - <titre> - <date>.<ext>` ; « (2) », « (3) »… si le nom existe.
pub fn recording_path(folder: &Path, r: &Recording, ext: &str, tz: i32, exists: &dyn Fn(&Path) -> bool) -> PathBuf {
    let base = format!("{} - {} - {}", sanitize_filename(&r.channel_name, 40), sanitize_filename(&r.title, 80), fmt_local(r.start, tz));
    let mut p = folder.join(format!("{base}.{ext}"));
    let mut n = 2;
    while exists(&p) && n < 100 {
        p = folder.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    p
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Method {
    Http,
    Mpv,
}

/// HTTP(S) sans extension de playlist/conteneur connue -> copie directe ; le reste -> mpv.
pub fn choose_method(url: &str) -> Method {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Method::Mpv;
    }
    let path = lower.split(['?', '#']).next().unwrap_or("");
    let ext = path.rsplit('/').next().unwrap_or("").rsplit_once('.').map(|x| x.1).unwrap_or("");
    if matches!(ext, "m3u8" | "m3u" | "mpd" | "mp4" | "mkv" | "flv" | "webm" | "asf" | "php" | "pls") {
        Method::Mpv
    } else {
        Method::Http
    }
}

/// Arguments de mpv pour un enregistrement sans fenêtre (voir l'en-tête du module).
pub fn mpv_args(url: &str, out: &Path, pipe: &str) -> Vec<String> {
    vec![
        "--no-config".into(),
        "--really-quiet".into(),
        "--no-terminal".into(),
        "--ytdl=no".into(),
        "--idle=no".into(),
        "--network-timeout=20".into(),
        format!("--input-ipc-server={pipe}"),
        format!("--o={}", out.display()),
        "--ovc=copy".into(),
        "--oac=copy".into(),
        "--of=matroska".into(),
        "--".into(),
        url.to_string(),
    ]
}

pub fn enough_space(free: Option<u64>, min_free: u64) -> bool {
    free.map(|f| f >= min_free).unwrap_or(true)
}

#[cfg(windows)]
pub fn free_space(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }
    let w: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let (mut a, mut t, mut f) = (0u64, 0u64, 0u64);
    // SAFETY : `w` est terminé par un NUL et les trois pointeurs désignent des u64 valides.
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut a, &mut t, &mut f) };
    (ok != 0).then_some(a)
}
#[cfg(not(windows))]
pub fn free_space(_dir: &Path) -> Option<u64> {
    None
}

// ---------- état global ----------

struct Inner {
    file: DvrFile,
    /// Enregistrements en cours : drapeau d'arrêt (0 = continuer, STOP_USER, STOP_END).
    ctl: HashMap<String, Arc<AtomicU8>>,
}

static ST: OnceLock<Mutex<Inner>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

fn st() -> &'static Mutex<Inner> {
    ST.get_or_init(|| Mutex::new(Inner { file: store::load("dvr"), ctl: HashMap::new() }))
}

fn lock() -> std::sync::MutexGuard<'static, Inner> {
    st().lock().unwrap_or_else(|e| e.into_inner())
}

fn save(g: &Inner) {
    let _ = store::save("dvr", &g.file);
    if let Some(app) = APP.get() {
        let _ = app.emit("dvr-changed", ());
    }
}

fn new_id() -> String {
    format!("r{:x}{:04x}", live::now_ts(), rand::random::<u16>())
}

/// Garde au plus MAX_RECORDINGS entrées en supprimant les plus anciennes terminées.
fn prune(recs: &mut Vec<Recording>) {
    while recs.len() > MAX_RECORDINGS {
        let Some(i) = recs.iter().enumerate().filter(|(_, r)| !matches!(r.status, Status::Scheduled | Status::Recording)).min_by_key(|(_, r)| r.stop).map(|x| x.0) else { break };
        recs.remove(i);
    }
}

/// Démarre le planificateur : reprise après arrêt brutal puis boucle de décision toutes les 2 s.
pub fn start(app: &AppHandle) {
    let _ = APP.set(app.clone());
    {
        let mut g = lock();
        let now = live::now_ts();
        let had_running = g.file.recordings.iter().any(|r| r.status == Status::Recording);
        recover(&mut g.file.recordings, now, new_id);
        if had_running {
            save(&g);
        }
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last_series = 0i64;
        let mut last_ret = 0i64;
        loop {
            tick();
            let now = live::now_ts();
            if now - last_series >= 60 {
                last_series = now;
                series_tick(&app);
            }
            if now - last_ret >= 3600 {
                last_ret = now;
                apply_retention();
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

fn fail(r: &mut Recording, why: &str) {
    r.status = Status::Failed;
    r.error = why.to_string();
}

fn tick() {
    let now = live::now_ts();
    let mut to_run: Vec<(Recording, Config, Arc<AtomicU8>)> = Vec::new();
    {
        let mut g = lock();
        let cfg = g.file.config.clone();
        let actions = decide(&g.file.recordings, now, cfg.max_parallel.max(1) as usize);
        if actions.is_empty() {
            return;
        }
        for a in actions {
            match a {
                Action::Stop(id) => {
                    if let Some(c) = g.ctl.get(&id) {
                        let _ = c.compare_exchange(0, STOP_END, Ordering::SeqCst, Ordering::SeqCst);
                    }
                }
                Action::Missed(id) => {
                    if let Some(r) = g.file.recordings.iter_mut().find(|r| r.id == id) {
                        fail(r, "missed");
                    }
                }
                Action::Start(id) => {
                    let Some(idx) = g.file.recordings.iter().position(|r| r.id == id) else { continue };
                    let method = choose_method(&g.file.recordings[idx].url);
                    let folder = PathBuf::from(&cfg.folder);
                    let problem = if method == Method::Mpv && player::find_mpv().is_none() {
                        Some("mpv_missing")
                    } else if std::fs::create_dir_all(&folder).is_err() {
                        Some("folder")
                    } else if !enough_space(free_space(&folder), cfg.min_free_gb as u64 * (1 << 30)) {
                        Some("disk_full")
                    } else {
                        None
                    };
                    if let Some(p) = problem {
                        fail(&mut g.file.recordings[idx], p);
                        continue;
                    }
                    let ext = if method == Method::Http { "ts" } else { "mkv" };
                    let path = recording_path(&folder, &g.file.recordings[idx], ext, cfg.tz, &|p| p.exists());
                    let r = &mut g.file.recordings[idx];
                    r.status = Status::Recording;
                    r.file = path.to_string_lossy().into_owned();
                    r.size = 0;
                    r.error.clear();
                    let ctl = Arc::new(AtomicU8::new(0));
                    to_run.push((r.clone(), cfg.clone(), ctl.clone()));
                    g.ctl.insert(id, ctl);
                }
            }
        }
        save(&g);
    }
    for (r, cfg, ctl) in to_run {
        std::thread::spawn(move || {
            let out = run(&r, &cfg, &ctl);
            finish(&r.id, out);
        });
    }
}

// ---------- exécution ----------

#[derive(Debug)]
pub struct Outcome {
    pub status: Status,
    pub error: String,
    pub path: PathBuf,
}

fn stop_state(ctl: &AtomicU8, end: i64) -> Option<u8> {
    match ctl.load(Ordering::SeqCst) {
        0 if live::now_ts() >= end => Some(STOP_END),
        0 => None,
        v => Some(v),
    }
}

fn file_size(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

fn run(r: &Recording, cfg: &Config, ctl: &AtomicU8) -> Outcome {
    let path = PathBuf::from(&r.file);
    let end = r.eff_end();
    let min_free = cfg.min_free_gb as u64 * (1 << 30);
    if choose_method(&r.url) == Method::Http {
        match run_http(r, &path, end, ctl, min_free) {
            HttpEnd::NeedMpv => {
                let _ = std::fs::remove_file(&path);
                let p2 = path.with_extension("mkv");
                return run_mpv(r, &p2, end, ctl, min_free);
            }
            HttpEnd::Ended(o) => return o,
        }
    }
    run_mpv(r, &path, end, ctl, min_free)
}

enum HttpEnd {
    Ended(Outcome),
    NeedMpv,
}

fn outcome_from_stop(s: u8, path: &Path) -> Outcome {
    Outcome { status: if s == STOP_USER { Status::Cancelled } else { Status::Done }, error: String::new(), path: path.to_path_buf() }
}

fn run_http(r: &Recording, path: &Path, end: i64, ctl: &AtomicU8, min_free: u64) -> HttpEnd {
    let failed = |why: &str| HttpEnd::Ended(Outcome { status: Status::Failed, error: why.into(), path: path.to_path_buf() });
    let (clean, cred) = live::split_auth(&r.url);
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(10)).timeout_read(Duration::from_secs(20)).user_agent("RusTV").build();
    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(_) => return failed("file"),
    };
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut buf = vec![0u8; 64 * 1024];
    let (mut bytes, mut since_check, mut fails) = (0u64, 0u64, 0u32);
    let mut last_err;
    loop {
        if let Some(s) = stop_state(ctl, end) {
            return HttpEnd::Ended(outcome_from_stop(s, path));
        }
        let mut req = agent.get(&clean);
        if let Some((u, p)) = &cred {
            req = req.set("Authorization", &live::basic_auth_header(u, p));
        }
        match req.call() {
            Ok(resp) => {
                let ct = resp.content_type().to_ascii_lowercase();
                if bytes == 0 && ct.contains("mpegurl") {
                    return HttpEnd::NeedMpv;
                }
                let mut rd = resp.into_reader();
                let before = bytes;
                loop {
                    if let Some(s) = stop_state(ctl, end) {
                        return HttpEnd::Ended(outcome_from_stop(s, path));
                    }
                    match rd.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if file.write_all(&buf[..n]).is_err() {
                                return failed("file");
                            }
                            bytes += n as u64;
                            since_check += n as u64;
                            if since_check >= 32 << 20 {
                                since_check = 0;
                                if !enough_space(free_space(&dir), min_free) {
                                    return failed("disk_full");
                                }
                            }
                        }
                        Err(_) => break, // coupure ou silence de 20 s : on se reconnecte
                    }
                }
                fails = if bytes > before { 0 } else { fails + 1 };
                last_err = "stream_ended";
            }
            Err(ureq::Error::Status(401 | 403, _)) => return failed("denied"),
            Err(ureq::Error::Status(404, _)) => return failed("not_found"),
            Err(ureq::Error::Status(503, _)) => {
                fails += 1;
                last_err = "tuner_busy"; // HDHomeRun : plus de tuner libre
            }
            Err(_) => {
                fails += 1;
                last_err = "unreachable";
            }
        }
        if fails >= 6 {
            return failed(last_err);
        }
        for _ in 0..10 {
            if stop_state(ctl, end).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

#[cfg(windows)]
fn quit_mpv(pipe: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(pipe) {
        let _ = f.write_all(b"{\"command\":[\"quit\"]}\n");
    }
}
#[cfg(not(windows))]
fn quit_mpv(pipe: &str) {
    if let Ok(mut f) = rustv_core::platform::ipc::connect(pipe) {
        let _ = f.write_all(b"{\"command\":[\"quit\"]}\n");
    }
}

fn run_mpv(r: &Recording, path: &Path, end: i64, ctl: &AtomicU8, min_free: u64) -> Outcome {
    let out = |status, error: &str| Outcome { status, error: error.into(), path: path.to_path_buf() };
    let Some(mpv) = player::find_mpv() else { return out(Status::Failed, "mpv_missing") };
    let pipe = rustv_core::platform::ipc::path(&format!("rustv-dvr-{}", r.id));
    let mut cmd = Command::new(mpv);
    cmd.args(mpv_args(&r.url, path, &pipe)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return out(Status::Failed, "mpv_start"),
    };
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut last_check = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if let Some(s) = stop_state(ctl, end) {
            quit_mpv(&pipe);
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(8) && matches!(child.try_wait(), Ok(None)) {
                std::thread::sleep(Duration::from_millis(200));
            }
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let _ = child.wait();
            }
            return outcome_from_stop(s, path);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let finished = live::now_ts() >= end - 10;
                return if status.success() && finished { out(Status::Done, "") } else { out(Status::Failed, if file_size(path) == 0 { "no_data" } else { "stream_ended" }) };
            }
            Ok(None) => {}
            Err(_) => return out(Status::Failed, "mpv_wait"),
        }
        if last_check.elapsed() > Duration::from_secs(10) {
            last_check = Instant::now();
            if !enough_space(free_space(&dir), min_free) {
                quit_mpv(&pipe);
                std::thread::sleep(Duration::from_secs(2));
                let _ = child.kill();
                let _ = child.wait();
                return out(Status::Failed, "disk_full");
            }
        }
    }
}

fn finish(id: &str, o: Outcome) {
    {
        let mut g = lock();
        g.ctl.remove(id);
        if let Some(r) = g.file.recordings.iter_mut().find(|r| r.id == id) {
            r.size = file_size(&o.path);
            r.file = if r.size == 0 { String::new() } else { o.path.to_string_lossy().into_owned() };
            if r.size == 0 {
                let _ = std::fs::remove_file(&o.path);
            }
            r.status = if o.status == Status::Done && r.size == 0 { Status::Failed } else { o.status };
            r.error = if r.status == Status::Failed && o.error.is_empty() { "no_data".into() } else { o.error };
        }
        save(&g);
    }
    apply_retention();
}

fn deletable(p: &str) -> bool {
    let pb = Path::new(p);
    !p.is_empty() && pb.is_file() && matches!(pb.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("ts" | "mkv"))
}

fn apply_retention() {
    let mut g = lock();
    let cfg = g.file.config.clone();
    let victims = retention_victims(&g.file.recordings, live::now_ts(), cfg.retention_gb as u64 * (1 << 30), cfg.retention_days);
    if victims.is_empty() {
        return;
    }
    for id in &victims {
        if let Some(r) = g.file.recordings.iter().find(|r| &r.id == id) {
            if deletable(&r.file) {
                let _ = std::fs::remove_file(&r.file);
            }
        }
    }
    g.file.recordings.retain(|r| !victims.contains(&r.id));
    save(&g);
}

fn series_tick(app: &AppHandle) {
    let rules = lock().file.series.clone();
    if rules.is_empty() {
        return;
    }
    let now = live::now_ts();
    let mut found: Vec<(SeriesRule, Vec<live::Prog>)> = Vec::new();
    {
        let shell = app.state::<Shell>();
        let cache = shell.live.lock().unwrap_or_else(|e| e.into_inner());
        for rule in &rules {
            let Some(ch) = cache.channels.iter().find(|c| c.name == rule.channel_name) else { continue };
            let existing: Vec<i64> = lock().file.recordings.iter().filter(|r| r.channel_name == rule.channel_name && r.title == rule.title).map(|r| r.start).collect();
            let picks = series_pick(live::programs_for(&cache, ch), &rule.title, now, 48 * 3600, &existing);
            if !picks.is_empty() {
                found.push((SeriesRule { url: ch.url.clone(), channel_id: ch.id.clone(), ..rule.clone() }, picks));
            }
        }
    }
    for (rule, picks) in found {
        for p in picks {
            let _ = add_recording(Spec { channel_id: rule.channel_id.clone(), channel_name: rule.channel_name.clone(), url: rule.url.clone(), title: p.title, start: p.start, stop: p.stop, pad_before: None, pad_after: None });
        }
    }
}

// ---------- commandes ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub channel_id: String,
    pub channel_name: String,
    pub url: String,
    pub title: String,
    pub start: i64,
    pub stop: i64,
    pub pad_before: Option<u32>,
    pub pad_after: Option<u32>,
}

fn add_recording(spec: Spec) -> Result<Value, String> {
    let url = spec.url.trim().to_string();
    let scheme = url.split("://").next().unwrap_or("").to_ascii_lowercase();
    if !url.contains("://") || !live::SCHEMES.contains(&scheme.as_str()) {
        return Err("url_invalid".into());
    }
    let now = live::now_ts();
    if spec.stop <= spec.start || spec.stop - spec.start > MAX_DURATION_S {
        return Err("range_invalid".into());
    }
    let mut g = lock();
    let (pb, pa) = (spec.pad_before.unwrap_or(g.file.config.pad_before).min(120), spec.pad_after.unwrap_or(g.file.config.pad_after).min(120));
    if spec.stop + pa as i64 * 60 <= now {
        return Err("in_the_past".into());
    }
    let title: String = spec.title.trim().chars().take(200).collect();
    let title = if title.is_empty() { spec.channel_name.clone() } else { title };
    if g.file.recordings.iter().any(|r| matches!(r.status, Status::Scheduled | Status::Recording) && r.channel_name == spec.channel_name && r.start == spec.start && r.title == title) {
        return Err("already_scheduled".into());
    }
    let warning = overlap_exceeds(&g.file.recordings, spec.start, spec.stop, pb, pa, g.file.config.max_parallel.max(1) as usize);
    let id = new_id();
    g.file.recordings.push(Recording {
        id: id.clone(),
        channel_id: spec.channel_id,
        channel_name: spec.channel_name.chars().take(120).collect(),
        url,
        title,
        start: spec.start,
        stop: spec.stop,
        pad_before: pb,
        pad_after: pa,
        status: Status::Scheduled,
        created: now,
        ..Default::default()
    });
    prune(&mut g.file.recordings);
    save(&g);
    Ok(json!({ "id": id, "warning": if warning { Some("overlap") } else { None } }))
}

#[tauri::command]
pub async fn dvr_list(tz: Option<i32>) -> Result<Value, String> {
    let mut g = lock();
    if let Some(tz) = tz.filter(|t| (-840..=840).contains(t)) {
        if g.file.config.tz != tz {
            g.file.config.tz = tz;
            let _ = store::save("dvr", &g.file);
        }
    }
    let free = free_space(Path::new(&g.file.config.folder).ancestors().find(|p| p.exists()).unwrap_or(Path::new(".")));
    Ok(json!({ "recordings": g.file.recordings, "series": g.file.series, "config": g.file.config, "running": g.ctl.len(), "freeBytes": free }))
}

#[tauri::command]
pub async fn dvr_schedule(spec: Spec) -> Result<Value, String> {
    let r = add_recording(spec)?;
    tauri::async_runtime::spawn_blocking(tick);
    Ok(r)
}

#[tauri::command]
pub async fn dvr_cancel(id: String) -> Result<(), String> {
    let mut g = lock();
    let Some(i) = g.file.recordings.iter().position(|r| r.id == id) else { return Err("not_found".into()) };
    match g.file.recordings[i].status {
        Status::Scheduled => g.file.recordings[i].status = Status::Cancelled,
        Status::Recording => {
            if let Some(c) = g.ctl.get(&id) {
                c.store(STOP_USER, Ordering::SeqCst);
            }
        }
        _ => return Err("not_active".into()),
    }
    save(&g);
    Ok(())
}

#[tauri::command]
pub async fn dvr_delete(id: String, delete_file: bool) -> Result<(), String> {
    let mut g = lock();
    let Some(i) = g.file.recordings.iter().position(|r| r.id == id) else { return Err("not_found".into()) };
    if g.file.recordings[i].status == Status::Recording {
        return Err("recording".into());
    }
    let r = g.file.recordings.remove(i);
    if delete_file && deletable(&r.file) {
        let _ = std::fs::remove_file(&r.file);
    }
    save(&g);
    Ok(())
}

#[tauri::command]
pub async fn dvr_get_config() -> Result<Config, String> {
    Ok(lock().file.config.clone())
}

#[tauri::command]
pub async fn dvr_set_config(cfg: Config) -> Result<Config, String> {
    let mut g = lock();
    let mut c = sanitize_config(cfg)?;
    c.tz = g.file.config.tz; // le fuseau vient de l'interface, via dvr_list
    g.file.config = c.clone();
    save(&g);
    Ok(c)
}

#[derive(Deserialize)]
pub struct NowChannel {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// « Enregistrer maintenant » : de maintenant à maintenant + `minutes` (1..=720), sans marges.
#[tauri::command]
pub async fn dvr_start_now(channel: NowChannel, minutes: u32) -> Result<Value, String> {
    if !(1..=720).contains(&minutes) {
        return Err("range_invalid".into());
    }
    let now = live::now_ts();
    let r = add_recording(Spec { channel_id: channel.id, channel_name: channel.name.clone(), url: channel.url, title: channel.name, start: now, stop: now + minutes as i64 * 60, pad_before: Some(0), pad_after: Some(0) })?;
    tauri::async_runtime::spawn_blocking(tick);
    Ok(r)
}

#[tauri::command]
pub async fn dvr_series_add(rule: SeriesRule) -> Result<(), String> {
    let mut g = lock();
    if rule.title.trim().is_empty() || rule.channel_name.is_empty() {
        return Err("rule_invalid".into());
    }
    if g.file.series.iter().any(|s| s.title.eq_ignore_ascii_case(&rule.title) && s.channel_name == rule.channel_name) {
        return Ok(());
    }
    if g.file.series.len() >= MAX_SERIES {
        return Err("too_many".into());
    }
    g.file.series.push(rule);
    save(&g);
    Ok(())
}

#[tauri::command]
pub async fn dvr_series_remove(title: String, channel_name: String) -> Result<(), String> {
    let mut g = lock();
    g.file.series.retain(|s| !(s.title.eq_ignore_ascii_case(&title) && s.channel_name == channel_name));
    save(&g);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, start: i64, stop: i64, status: Status) -> Recording {
        Recording { id: id.into(), channel_name: "Chaîne".into(), title: "Titre".into(), url: "http://h/auto/v1".into(), start, stop, pad_before: 1, pad_after: 2, status, ..Default::default() }
    }

    #[test]
    fn decide_start_and_stop_with_padding() {
        let recs = vec![rec("a", 1000, 2000, Status::Scheduled)];
        // marge avant = 60 s : rien avant 940, démarrage à 940
        assert!(decide(&recs, 939, 2).is_empty());
        assert_eq!(decide(&recs, 940, 2), vec![Action::Start("a".into())]);
        // enregistrement en cours : arrêt à stop + 120 s = 2120
        let run = vec![rec("a", 1000, 2000, Status::Recording)];
        assert!(decide(&run, 2119, 2).is_empty());
        assert_eq!(decide(&run, 2120, 2), vec![Action::Stop("a".into())]);
    }

    #[test]
    fn decide_parallel_limit_and_missed() {
        let recs = vec![rec("a", 1000, 2000, Status::Scheduled), rec("b", 1010, 2000, Status::Scheduled), rec("c", 1020, 2000, Status::Scheduled)];
        // 2 tuners : a et b démarrent, c attend
        assert_eq!(decide(&recs, 1100, 2), vec![Action::Start("a".into()), Action::Start("b".into())]);
        // un enregistrement déjà en cours occupe une place
        let mut r2 = recs.clone();
        r2[0].status = Status::Recording;
        assert_eq!(decide(&r2, 1100, 2), vec![Action::Start("b".into())]);
        // c n'a jamais pu démarrer et sa fenêtre est close : manqué
        assert!(decide(&recs, 2200, 2).contains(&Action::Missed("c".into())));
        // une place se libère quand a s'arrête dans le même tick (stop ne compte plus), mais pas avant
        let r3 = vec![rec("a", 1000, 1100, Status::Recording), rec("c", 1000, 5000, Status::Scheduled)];
        assert_eq!(decide(&r3, 1300, 1), vec![Action::Stop("a".into()), Action::Start("c".into())]);
    }

    #[test]
    fn overlap_detection() {
        let recs = vec![rec("a", 1000, 2000, Status::Scheduled), rec("b", 1500, 2500, Status::Scheduled)];
        assert!(overlap_exceeds(&recs, 1600, 1800, 0, 0, 2)); // 3 simultanés
        assert!(!overlap_exceeds(&recs, 1600, 1800, 0, 0, 3));
        assert!(!overlap_exceeds(&recs, 3000, 3500, 0, 0, 2)); // plus tard
        // les marges comptent : 2000+120 chevauche un nouvel enregistrement qui commence à 2100
        assert!(overlap_exceeds(&[rec("a", 1000, 2000, Status::Scheduled), rec("b", 1000, 2000, Status::Scheduled)], 2100, 2200, 0, 0, 2));
        // terminés et annulés ne comptent pas
        let done = vec![rec("a", 1000, 2000, Status::Done), rec("b", 1000, 2000, Status::Cancelled)];
        assert!(!overlap_exceeds(&done, 1000, 2000, 0, 0, 1));
    }

    #[test]
    fn recover_after_crash() {
        let mut v = vec![rec("a", 1000, 2000, Status::Recording), rec("b", 100, 200, Status::Recording), rec("c", 5000, 6000, Status::Scheduled)];
        v[0].file = "C:\\x\\a.ts".into();
        let mut n = 0;
        let created = recover(&mut v, 1500, || {
            n += 1;
            format!("new{n}")
        });
        assert_eq!(created, 1); // seul a a encore du temps devant lui
        assert_eq!(v[0].status, Status::Failed);
        assert_eq!(v[0].error, "interrupted");
        assert_eq!(v[0].file, "C:\\x\\a.ts"); // le fichier est conservé
        assert_eq!(v[1].status, Status::Failed);
        assert_eq!(v[2].status, Status::Scheduled);
        let cont = v.last().unwrap();
        assert_eq!((cont.id.as_str(), cont.status, cont.title.as_str(), cont.file.as_str()), ("new1", Status::Scheduled, "Titre (2)", ""));
        // la suite démarre immédiatement
        assert_eq!(decide(&[cont.clone()], 1500, 2), vec![Action::Start("new1".into())]);
    }

    #[test]
    fn retention() {
        let mut a = rec("a", 100, 1000, Status::Done);
        a.size = 4 << 30;
        let mut b = rec("b", 100, 2000, Status::Done);
        b.size = 4 << 30;
        let mut c = rec("c", 100, 3000, Status::Done);
        c.size = 4 << 30;
        let mut d = rec("d", 100, 500, Status::Scheduled);
        d.size = 99 << 30;
        let v = vec![c.clone(), a.clone(), b.clone(), d];
        assert!(retention_victims(&v, 10_000, 0, 0).is_empty());
        // 5 Go max : on supprime les plus anciens jusqu'à passer sous la limite : a puis b
        assert_eq!(retention_victims(&v, 10_000, 5 << 30, 0), vec!["a", "b"]);
        // 1 jour : tout ce qui est plus vieux que (now - 86400)
        let now = 1000 + 86400 + 1;
        assert_eq!(retention_victims(&v, now, 0, 1), vec!["a"]);
        // jours puis taille se cumulent sans doublon
        assert_eq!(retention_victims(&v, now, 5 << 30, 1), vec!["a", "b"]);
    }

    #[test]
    fn series_matching() {
        let p = |s: i64, t: &str| live::Prog { start: s, stop: s + 10, title: t.into(), desc: String::new() };
        let progs = vec![p(100, "Journal"), p(200, "journal "), p(300, "Film"), p(400, "Journal"), p(100_000, "Journal")];
        let got = series_pick(&progs, "JOURNAL", 150, 1000, &[400]);
        assert_eq!(got.iter().map(|x| x.start).collect::<Vec<_>>(), vec![200]); // 100 passé, 400 déjà pris, 100000 hors horizon
        assert!(series_pick(&progs, "  ", 0, 1000, &[]).is_empty());
    }

    #[test]
    fn filename_sanitisation() {
        assert_eq!(sanitize_filename("a<b>c:d\"e/f\\g|h?i*j", 80), "a_b_c_d_e_f_g_h_i_j");
        assert_eq!(sanitize_filename("  Titre...  ", 80), "Titre");
        assert_eq!(sanitize_filename("CON", 80), "_CON");
        assert_eq!(sanitize_filename("nul.txt", 80), "_nul.txt");
        assert_eq!(sanitize_filename("COM1", 80), "_COM1");
        assert_eq!(sanitize_filename("COM0", 80), "COM0");
        assert_eq!(sanitize_filename("", 80), "recording");
        assert_eq!(sanitize_filename("...", 80), "recording");
        assert_eq!(sanitize_filename("..\\..\\evil", 80), ".._.._evil");
        assert_eq!(sanitize_filename(&"x".repeat(200), 40).len(), 40);
        assert_eq!(sanitize_filename("é\u{0}\n", 10), "é__");
    }

    #[test]
    fn local_time_format() {
        assert_eq!(fmt_local(0, 0), "1970-01-01 00-00");
        // 2024-02-29 17:00 UTC à UTC+1 -> 18-00
        assert_eq!(fmt_local(1_709_226_000, 60), "2024-02-29 18-00");
        assert_eq!(fmt_local(1_709_226_000, -300), "2024-02-29 12-00");
        // passage à la veille (négatif)
        assert_eq!(fmt_local(-60, 0), "1969-12-31 23-59");
    }

    #[test]
    fn path_naming_and_collisions() {
        let r = rec("a", 1_709_226_000, 1_709_229_600, Status::Scheduled);
        let p = recording_path(Path::new("D:\\Rec"), &r, "ts", 60, &|_| false);
        assert_eq!(p, Path::new("D:\\Rec").join("Chaîne - Titre - 2024-02-29 18-00.ts"));
        let taken = p.clone();
        let p2 = recording_path(Path::new("D:\\Rec"), &r, "ts", 60, &|x| x == taken);
        assert!(p2.to_string_lossy().ends_with("18-00 (2).ts"));
        let mut hostile = r.clone();
        hostile.title = "..\\..\\Windows\\x".into();
        let p3 = recording_path(Path::new("D:\\Rec"), &hostile, "ts", 0, &|_| false);
        assert_eq!(p3.parent(), Some(Path::new("D:\\Rec"))); // le nom ne peut pas sortir du dossier
    }

    #[test]
    fn method_selection() {
        assert_eq!(choose_method("http://192.168.1.5:5004/auto/v2.1"), Method::Http);
        assert_eq!(choose_method("http://nas:9981/stream/channelid/12?profile=pass"), Method::Http);
        assert_eq!(choose_method("http://h/live/1.ts"), Method::Http);
        assert_eq!(choose_method("https://h/live/index.m3u8?token=a.b"), Method::Mpv);
        assert_eq!(choose_method("http://h/a.MP4"), Method::Mpv);
        assert_eq!(choose_method("rtmp://h/live"), Method::Mpv);
        assert_eq!(choose_method("udp://@239.1.1.1:1234"), Method::Mpv);
    }

    #[test]
    fn mpv_command_line() {
        let a = mpv_args("rtsp://h/x", Path::new("D:\\Rec\\a b.mkv"), r"\\.\pipe\p");
        assert_eq!(a.last().unwrap(), "rtsp://h/x");
        assert_eq!(a[a.len() - 2], "--"); // une URL commençant par '-' ne peut pas être prise pour une option
        for must in ["--no-config", "--really-quiet", "--no-terminal", "--ovc=copy", "--oac=copy", "--of=matroska", "--o=D:\\Rec\\a b.mkv", "--input-ipc-server=\\\\.\\pipe\\p"] {
            assert!(a.iter().any(|x| x == must), "manque {must}");
        }
        // le chemin de sortie reste un seul argument (pas de découpage sur l'espace)
        assert!(a.iter().filter(|x| x.starts_with("--o=")).count() == 1);
    }

    #[test]
    fn disk_space_and_config() {
        assert!(enough_space(Some(10), 10));
        assert!(!enough_space(Some(9), 10));
        assert!(enough_space(None, 10)); // inconnu : on n'empêche pas
        let c = sanitize_config(Config { folder: std::env::temp_dir().to_string_lossy().into_owned(), pad_before: 999, max_parallel: 0, tz: 5000, ..Default::default() }).unwrap();
        assert_eq!((c.pad_before, c.max_parallel, c.tz), (120, 1, 840));
        assert!(sanitize_config(Config { folder: "relatif\\dossier".into(), ..Default::default() }).is_err());
        assert!(!sanitize_config(Config { folder: "  ".into(), ..Default::default() }).unwrap().folder.is_empty());
    }

    #[test]
    fn status_serialises_lowercase() {
        assert_eq!(serde_json::to_string(&Status::Scheduled).unwrap(), "\"scheduled\"");
        let r: Recording = serde_json::from_str(r#"{"id":"x","status":"done","channelName":"A"}"#).unwrap();
        assert_eq!((r.status, r.channel_name.as_str()), (Status::Done, "A"));
    }
}
