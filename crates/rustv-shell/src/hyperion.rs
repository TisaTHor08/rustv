//! Ambilight natif via Hyperion (hyperion.ng) : https://github.com/hyperion-project/hyperion.ng
//!
//! Hyperion capture l'écran (grabber DXGI) et pilote une bande de LED (USB/série, WLED, Philips Hue…).
//! Le système de mods ne peut ni lancer de processus ni ouvrir de socket brut : c'est donc un module intégré,
//! optionnel (désactivé par défaut). Il :
//!  - détecte / installe (sur demande, SHA-256 vérifié) / démarre / arrête un `hyperiond.exe` géré par RusTV
//!    (jamais un Hyperion lancé par l'utilisateur) ;
//!  - pilote Hyperion par son API JSON TCP (une commande JSON par ligne, port 19444 par défaut) ;
//!  - suit le mode TV, la lecture vidéo (pause -> baisse de luminosité) et la tuile ciblée sur l'accueil.
use crate::{settings, store, Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const RELEASES_API: &str = "https://api.github.com/repos/hyperion-project/hyperion.ng/releases/latest";
const MAX_ARCHIVE: u64 = 250 * 1024 * 1024;
const MAX_UNPACKED: u64 = 900 * 1024 * 1024;
const MAX_LINE: u64 = 4 * 1024 * 1024;
const ORIGIN: &str = "RusTV";

// ---------- configuration ----------

/// Réglages persistés dans %APPDATA%\RusTV\hyperion.json. Le jeton d'API n'y est stocké que chiffré (DPAPI).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Interrupteur général : tant qu'il est faux, le module ne fait rien.
    pub enabled: bool,
    /// RusTV démarre/arrête lui-même hyperiond.exe (impossible avec un Hyperion distant).
    pub manage: bool,
    /// Chemin de hyperiond.exe choisi par l'utilisateur (vide = détection).
    pub path: String,
    /// Arguments supplémentaires de hyperiond.exe (voir docs/AMBILIGHT.md : non vérifiés).
    pub extra_args: Vec<String>,
    pub host: String,
    pub port: u16,
    pub web_port: u16,
    /// Jeton d'API chiffré avec DPAPI (hexadécimal). Jamais renvoyé à l'interface, jamais journalisé.
    pub token_enc: String,
    /// "tile" | "grabber" | "off"
    pub ambient: String,
    pub leds_on_tv_start: bool,
    pub leds_off_tv_exit: bool,
    pub stop_on_tv_exit: bool,
    /// Garde la capture d'écran active pendant une vidéo / une app.
    pub capture_on_video: bool,
    pub dim_on_pause: bool,
    pub dim_after_secs: u64,
    /// Luminosité (%) pendant la pause.
    pub dim_level: u8,
    pub ambient_priority: i32,
    pub effect_priority: i32,
    pub favourites: Vec<String>,
    pub effect: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            manage: true,
            path: String::new(),
            extra_args: Vec::new(),
            host: "127.0.0.1".into(),
            port: 19444,
            web_port: 8090,
            token_enc: String::new(),
            ambient: "tile".into(),
            leds_on_tv_start: true,
            leds_off_tv_exit: true,
            stop_on_tv_exit: true,
            capture_on_video: true,
            dim_on_pause: true,
            dim_after_secs: 30,
            dim_level: 20,
            ambient_priority: 200,
            effect_priority: 150,
            favourites: vec!["Rainbow swirl".into(), "Warm mood blobs".into(), "Cold mood blobs".into()],
            effect: String::new(),
        }
    }
}

fn host_ok(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '[' | ']'))
}

pub fn is_local(host: &str) -> bool {
    matches!(host.trim().to_ascii_lowercase().as_str(), "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

/// Rend la configuration cohérente (bornes, valeurs connues). Ne touche pas au jeton.
pub fn sanitize(mut c: Config) -> Result<Config, String> {
    c.host = c.host.trim().to_string();
    if !host_ok(&c.host) {
        return Err("invalid_host".into());
    }
    if c.port == 0 || c.web_port == 0 {
        return Err("invalid_port".into());
    }
    if !matches!(c.ambient.as_str(), "tile" | "grabber" | "off") {
        c.ambient = "tile".into();
    }
    c.dim_after_secs = c.dim_after_secs.clamp(5, 3600);
    c.dim_level = c.dim_level.min(100);
    c.ambient_priority = c.ambient_priority.clamp(2, 253);
    c.effect_priority = c.effect_priority.clamp(2, 253);
    c.extra_args.truncate(16);
    c.extra_args.retain(|a| !a.is_empty() && a.len() <= 200 && !a.contains('\0'));
    c.favourites.truncate(32);
    c.favourites.retain(|f| !f.is_empty() && f.len() <= 100);
    c.effect.truncate(100);
    c.path = c.path.trim().to_string();
    if !c.path.is_empty() {
        let name = Path::new(&c.path).file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if name != "hyperiond.exe" && name != "hyperiond" {
            return Err("invalid_path".into());
        }
    }
    Ok(c)
}

fn load_cfg() -> Config {
    store::load::<Config>("hyperion")
}

// ---------- DPAPI (jeton d'API) ----------

#[cfg(windows)]
mod dpapi {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        unsafe {
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
            v
        }
    }
    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let inb = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(&inb, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).ok()?;
            Some(take(out))
        }
    }
    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let inb = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(&inb, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).ok()?;
            Some(take(out))
        }
    }
}
#[cfg(not(windows))]
mod dpapi {
    pub fn protect(_: &[u8]) -> Option<Vec<u8>> {
        None
    }
    pub fn unprotect(_: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

pub fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

fn token_of(c: &Config) -> String {
    if c.token_enc.is_empty() {
        return String::new();
    }
    hex_decode(&c.token_enc).and_then(|b| dpapi::unprotect(&b)).and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
}

// ---------- protocole JSON (pur, testé) ----------

#[allow(dead_code)] // SourceSelect / Instance : API prête pour le multi-instance, pas encore reliée à l'interface
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    ServerInfo,
    Component(String, bool),
    Brightness(u8),
    Effect { name: String, priority: i32 },
    Color { rgb: [u8; 3], priority: i32, duration_ms: Option<u64> },
    Clear(i32),
    SourceSelect(i32),
    Instance(u32),
    Authorize(String),
}

#[allow(dead_code)]
pub const COMPONENTS: [&str; 9] = ["ALL", "SMOOTHING", "BLACKBORDER", "FORWARDER", "BOBLIGHTSERVER", "GRABBER", "V4L", "AUDIO", "LEDDEVICE"];

impl Cmd {
    /// Construit la ligne JSON de la commande avec son `tan` (identifiant de corrélation).
    pub fn build(&self, tan: i64) -> Value {
        match self {
            Cmd::ServerInfo => json!({ "command": "serverinfo", "tan": tan }),
            Cmd::Component(c, on) => json!({ "command": "componentstate", "componentstate": { "component": c, "state": on }, "tan": tan }),
            Cmd::Brightness(b) => json!({ "command": "adjustment", "adjustment": { "brightness": (*b).min(100) }, "tan": tan }),
            Cmd::Effect { name, priority } => json!({ "command": "effect", "effect": { "name": name }, "priority": priority, "origin": ORIGIN, "tan": tan }),
            Cmd::Color { rgb, priority, duration_ms } => {
                let mut v = json!({ "command": "color", "color": rgb, "priority": priority, "origin": ORIGIN, "tan": tan });
                if let Some(d) = duration_ms {
                    v["duration"] = json!(d);
                }
                v
            }
            Cmd::Clear(p) => json!({ "command": "clear", "priority": p, "tan": tan }),
            Cmd::SourceSelect(p) => json!({ "command": "sourceselect", "priority": p, "tan": tan }),
            Cmd::Instance(i) => json!({ "command": "instance", "subcommand": "switchTo", "instance": i, "tan": tan }),
            Cmd::Authorize(t) => json!({ "command": "authorize", "subcommand": "login", "token": t, "tan": tan }),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Resp {
    pub command: String,
    pub tan: Option<i64>,
    pub success: bool,
    pub error: Option<String>,
    pub raw: Value,
}

/// Analyse une ligne de réponse. Erreur si ce n'est pas un objet JSON.
pub fn parse_response(line: &str) -> Result<Resp, String> {
    let v: Value = serde_json::from_str(line.trim()).map_err(|_| "malformed".to_string())?;
    if !v.is_object() {
        return Err("malformed".into());
    }
    let error = v["error"].as_str().map(|s| s.chars().take(200).collect::<String>());
    let success = v["success"].as_bool().unwrap_or(error.is_none());
    Ok(Resp { command: v["command"].as_str().unwrap_or("").to_string(), tan: v["tan"].as_i64(), success, error, raw: v })
}

/// Lit des lignes jusqu'à la réponse portant le `tan` attendu ; ignore les lignes illisibles ou d'autres `tan`
/// (diffusions de Hyperion). `Err("closed")` si la connexion se ferme, `Err("timeout")` sinon.
pub fn read_matching<R: BufRead>(r: &mut R, tan: i64, max_lines: usize) -> Result<Resp, String> {
    for _ in 0..max_lines {
        let mut line = String::new();
        let n = (&mut *r).take(MAX_LINE).read_line(&mut line).map_err(|e| if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) { "timeout".to_string() } else { "io".to_string() })?;
        if n == 0 {
            return Err("closed".into());
        }
        if !line.ends_with('\n') && n as u64 >= MAX_LINE {
            return Err("line_too_long".into());
        }
        if let Ok(resp) = parse_response(&line) {
            if resp.tan == Some(tan) {
                return Ok(resp);
            }
        }
    }
    Err("timeout".into())
}

// ---------- état du serveur (serverinfo) ----------

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    pub priority: i64,
    pub component: String,
    pub owner: String,
    pub active: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Instance {
    pub id: u32,
    pub name: String,
    pub running: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Info {
    pub components: HashMap<String, bool>,
    pub brightness: Option<u8>,
    pub effects: Vec<String>,
    pub priorities: Vec<Priority>,
    pub instances: Vec<Instance>,
    pub version: Option<String>,
}

impl Info {
    pub fn led_on(&self) -> bool {
        self.components.get("ALL").copied().unwrap_or(false) && self.components.get("LEDDEVICE").copied().unwrap_or(true)
    }
}

/// Extrait ce qui nous intéresse de la réponse `serverinfo` (champ `info`). Tolérant : les champs absents restent vides.
pub fn parse_serverinfo(resp: &Value) -> Info {
    let info = &resp["info"];
    let mut out = Info::default();
    for c in info["components"].as_array().into_iter().flatten() {
        if let (Some(n), Some(e)) = (c["name"].as_str(), c["enabled"].as_bool()) {
            out.components.insert(n.to_string(), e);
        }
    }
    out.brightness = info["adjustment"].as_array().and_then(|a| a.first()).and_then(|a| a["brightness"].as_u64()).map(|b| b.min(100) as u8);
    out.effects = info["effects"].as_array().into_iter().flatten().filter_map(|e| e["name"].as_str()).map(str::to_string).take(500).collect();
    out.priorities = info["priorities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some(Priority {
                priority: p["priority"].as_i64()?,
                component: p["componentId"].as_str().unwrap_or("").to_string(),
                owner: p["owner"].as_str().unwrap_or("").chars().take(60).collect(),
                active: p["active"].as_bool().unwrap_or(false),
                visible: p["visible"].as_bool().unwrap_or(false),
            })
        })
        .take(50)
        .collect();
    out.instances = info["instance"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| Some(Instance { id: i["instance"].as_u64()? as u32, name: i["friendly_name"].as_str().unwrap_or("").to_string(), running: i["running"].as_bool().unwrap_or(false) }))
        .collect();
    let b = &info["hyperion_build"];
    let b = if b.is_array() { &b[0] } else { b };
    out.version = b["version"].as_str().map(|s| s.chars().take(40).collect());
    out
}

// ---------- client TCP ----------

#[derive(Clone, PartialEq, Debug)]
struct Endpoint {
    host: String,
    port: u16,
    token: String,
}

struct Conn {
    key: Endpoint,
    r: BufReader<TcpStream>,
    w: TcpStream,
}

#[derive(Default)]
struct Client {
    conn: Option<Conn>,
    tan: i64,
}

fn endpoint(c: &Config) -> Endpoint {
    Endpoint { host: c.host.clone(), port: c.port, token: token_of(c) }
}

fn dial(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let addr = (host.trim_matches(|c| c == '[' || c == ']'), port).to_socket_addrs().map_err(|_| "unreachable".to_string())?.next().ok_or("unreachable")?;
    TcpStream::connect_timeout(&addr, timeout).map_err(|_| "unreachable".to_string())
}

impl Client {
    fn open(&mut self, ep: &Endpoint) -> Result<(), String> {
        let s = dial(&ep.host, ep.port, Duration::from_millis(1500))?;
        let _ = s.set_read_timeout(Some(Duration::from_secs(4)));
        let _ = s.set_write_timeout(Some(Duration::from_secs(3)));
        let _ = s.set_nodelay(true);
        let w = s.try_clone().map_err(|e| e.to_string())?;
        self.conn = Some(Conn { key: ep.clone(), r: BufReader::new(s), w });
        if !ep.token.is_empty() {
            let r = self.send(&Cmd::Authorize(ep.token.clone()), false)?;
            if !r.success {
                self.conn = None;
                return Err("auth_failed".into());
            }
        }
        Ok(())
    }

    fn send(&mut self, cmd: &Cmd, _retry: bool) -> Result<Resp, String> {
        self.tan = self.tan % 1_000_000 + 1;
        let tan = self.tan;
        let line = format!("{}\n", cmd.build(tan));
        let c = self.conn.as_mut().ok_or("not_connected")?;
        c.w.write_all(line.as_bytes()).and_then(|_| c.w.flush()).map_err(|_| "io".to_string())?;
        read_matching(&mut c.r, tan, 50)
    }

    /// Envoie une commande ; reconnecte une fois si la connexion est tombée.
    fn request(&mut self, ep: &Endpoint, cmd: &Cmd) -> Result<Resp, String> {
        if self.conn.as_ref().map(|c| &c.key) != Some(ep) {
            self.conn = None;
        }
        for attempt in 0..2 {
            if self.conn.is_none() {
                self.open(ep)?;
            }
            match self.send(cmd, false) {
                Ok(r) if r.success => return Ok(r),
                Ok(r) => return Err(r.error.unwrap_or_else(|| "failed".into())),
                Err(e) => {
                    self.conn = None;
                    if attempt == 1 || e == "timeout" {
                        return Err(e);
                    }
                }
            }
        }
        Err("io".into())
    }
}

static CLIENT: Mutex<Client> = Mutex::new(Client { conn: None, tan: 0 });

fn call(cfg: &Config, cmd: &Cmd) -> Result<Resp, String> {
    let ep = endpoint(cfg);
    CLIENT.lock().unwrap_or_else(|e| e.into_inner()).request(&ep, cmd)
}

fn query(cfg: &Config) -> Result<Info, String> {
    call(cfg, &Cmd::ServerInfo).map(|r| parse_serverinfo(&r.raw))
}

fn drop_conn() {
    CLIENT.lock().unwrap_or_else(|e| e.into_inner()).conn = None;
}

// ---------- couleur d'ambiance (pur, testé) ----------

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().strip_prefix('#')?;
    let v: Vec<u8> = match h.len() {
        3 => h.chars().map(|c| c.to_digit(16).map(|d| (d * 17) as u8)).collect::<Option<_>>()?,
        6 => (0..6).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok()).collect::<Option<_>>()?,
        _ => return None,
    };
    Some([v[0], v[1], v[2]])
}

/// Couleur envoyée aux LED pour une tuile : moyenne pondérée du dégradé (70 % clair / 30 % sombre), puis
/// renforcement de la luminosité (les teintes du dégradé sont sombres, les LED les rendent ternes) jusqu'à un
/// pic de 220 sur le canal dominant, facteur borné à 2,4, résultat limité à 0..255.
pub fn ambient_color(c1: &str, c2: Option<&str>) -> Option<[u8; 3]> {
    let a = parse_hex(c1)?;
    let b = c2.and_then(parse_hex).unwrap_or(a);
    let mut f = [0f32; 3];
    for i in 0..3 {
        f[i] = a[i] as f32 * 0.7 + b[i] as f32 * 0.3;
    }
    let peak = f.iter().cloned().fold(0f32, f32::max);
    if peak < 1.0 {
        return Some([0, 0, 0]);
    }
    let k = (220.0 / peak).clamp(1.0, 2.4);
    let mut out = [0u8; 3];
    for i in 0..3 {
        out[i] = (f[i] * k).round().clamp(0.0, 255.0) as u8;
    }
    Some(out)
}

// ---------- machines à états (pures, testées) ----------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    Start,
    LedsOn,
    ClearOurs,
    LedsOff,
    Stop,
}

/// Actions à enchaîner lors d'un changement du mode TV.
pub fn tv_plan(on: bool, c: &Config) -> Vec<Step> {
    if !c.enabled {
        return vec![];
    }
    let managed = c.manage && is_local(&c.host);
    let mut v = vec![];
    if on {
        if managed {
            v.push(Step::Start);
        }
        if c.leds_on_tv_start {
            v.push(Step::LedsOn);
        }
    } else {
        v.push(Step::ClearOurs);
        if c.leds_off_tv_exit {
            v.push(Step::LedsOff);
        }
        if managed && c.stop_on_tv_exit {
            v.push(Step::Stop);
        }
    }
    v
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimAct {
    None,
    Dim,
    Restore,
}

#[derive(Default, Debug)]
pub struct Dim {
    since: Option<u64>,
    dimmed: bool,
}

impl Dim {
    /// `paused` : une vidéo est en pause. `now` en secondes (horloge monotone).
    pub fn step(&mut self, now: u64, paused: bool, enabled: bool, after: u64) -> DimAct {
        if !enabled || !paused {
            self.since = None;
            if self.dimmed {
                self.dimmed = false;
                return DimAct::Restore;
            }
            return DimAct::None;
        }
        let since = *self.since.get_or_insert(now);
        if !self.dimmed && now.saturating_sub(since) >= after {
            self.dimmed = true;
            return DimAct::Dim;
        }
        DimAct::None
    }
}

// ---------- processus géré ----------

struct Managed {
    pid: u32,
    gen: u64,
}
static MANAGED: Mutex<Option<Managed>> = Mutex::new(None);
static GEN: Mutex<u64> = Mutex::new(0);

#[derive(Default)]
struct Rt {
    /// Dernière couleur d'ambiance demandée par l'accueil.
    last_rgb: Option<[u8; 3]>,
    ambient_set: bool,
    ambient_gen: u64,
    capture: bool,
    dim: Dim,
    saved_brightness: Option<u8>,
}
static RT: Mutex<Option<Rt>> = Mutex::new(None);

fn rt<R>(f: impl FnOnce(&mut Rt) -> R) -> R {
    let mut g = RT.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Rt::default))
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}
#[cfg(not(windows))]
fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}

fn hyperion_dir() -> PathBuf {
    settings::dir().join("hyperion")
}
fn data_dir() -> PathBuf {
    hyperion_dir().join("data")
}
fn app_dir() -> PathBuf {
    hyperion_dir().join("app")
}

/// Cherche `hyperiond.exe` sous `dir` (profondeur limitée : le zip peut contenir un dossier racine).
fn find_in(dir: &Path, depth: u32) -> Option<PathBuf> {
    let direct = dir.join("hyperiond.exe");
    if direct.is_file() {
        return Some(direct);
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(dir).ok()?.flatten().filter(|e| e.path().is_dir()).find_map(|e| find_in(&e.path(), depth - 1))
}

/// Emplacements candidats (hors dossier d'installation RusTV), pur pour les tests.
pub fn candidate_paths(cfg_path: &str, env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut v = vec![];
    if !cfg_path.is_empty() {
        v.push(PathBuf::from(cfg_path));
    }
    for k in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(b) = env(k).map(PathBuf::from) {
            let b = if k == "LOCALAPPDATA" { b.join("Programs") } else { b };
            v.push(b.join("Hyperion").join("bin").join("hyperiond.exe"));
            v.push(b.join("Hyperion").join("hyperiond.exe"));
        }
    }
    if let Some(p) = env("PATH") {
        v.extend(std::env::split_paths(&p).map(|d| d.join("hyperiond.exe")));
    }
    v
}

pub fn find_exe(cfg: &Config) -> Option<PathBuf> {
    if !cfg.path.is_empty() {
        let p = PathBuf::from(&cfg.path);
        return p.is_file().then_some(p);
    }
    find_in(&app_dir(), 3).or_else(|| candidate_paths("", &|k| std::env::var(k).ok()).into_iter().find(|p| p.is_file()))
}

fn pid_alive() -> bool {
    MANAGED.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

fn can_connect(cfg: &Config) -> bool {
    dial(&cfg.host, cfg.port, Duration::from_millis(400)).is_ok()
}

/// Windows : un objet Job « kill on close » garantit que Hyperion meurt avec RusTV, même après un plantage.
#[cfg(windows)]
fn bind_to_job(child: &std::process::Child) {
    use std::os::windows::io::AsRawHandle;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE};
    static JOB: OnceLock<isize> = OnceLock::new();
    let job = *JOB.get_or_init(|| unsafe {
        let Ok(h) = CreateJobObjectW(None, windows::core::PCWSTR::null()) else { return 0 };
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(h, JobObjectExtendedLimitInformation, &info as *const _ as *const _, std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32).is_err() {
            return 0;
        }
        h.0 as isize
    });
    if job != 0 {
        unsafe {
            let _ = AssignProcessToJobObject(HANDLE(job as *mut _), HANDLE(child.as_raw_handle()));
        }
    }
}
#[cfg(not(windows))]
fn bind_to_job(_: &std::process::Child) {}

/// Démarre hyperiond.exe (uniquement si rien n'écoute déjà sur le port : on ne touche jamais à un Hyperion
/// lancé par l'utilisateur).
pub fn start_process(cfg: &Config) -> Result<(), String> {
    if !is_local(&cfg.host) {
        return Err("remote_host".into());
    }
    if pid_alive() {
        return Ok(());
    }
    if can_connect(cfg) {
        return Ok(()); // un Hyperion tourne déjà (le nôtre ou celui de l'utilisateur) : on s'y connecte
    }
    let exe = find_exe(cfg).ok_or("not_installed")?;
    let data = data_dir();
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&exe);
    cmd.arg("--userdata").arg(&data).args(&cfg.extra_args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(d) = exe.parent() {
        cmd.current_dir(d);
    }
    let mut child = no_window(&mut cmd).spawn().map_err(|_| "spawn_failed".to_string())?;
    bind_to_job(&child);
    let pid = child.id();
    let gen = {
        let mut g = GEN.lock().unwrap();
        *g += 1;
        *g
    };
    *MANAGED.lock().unwrap() = Some(Managed { pid, gen });
    std::thread::spawn(move || {
        let _ = child.wait();
        let mut m = MANAGED.lock().unwrap_or_else(|e| e.into_inner());
        if m.as_ref().map(|x| x.gen) == Some(gen) {
            *m = None;
        }
    });
    std::thread::sleep(Duration::from_millis(900));
    if !pid_alive() {
        return Err("exited_early".into());
    }
    Ok(())
}

/// Arrête uniquement le processus lancé par RusTV (arbre complet).
pub fn stop_process() {
    drop_conn();
    let m = MANAGED.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(m) = m {
        let mut c = Command::new("taskkill");
        c.args(["/PID", &m.pid.to_string(), "/T", "/F"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        let _ = no_window(&mut c).status();
    }
}

/// Attend que l'API réponde (démarrage de Hyperion).
fn wait_ready(cfg: &Config, secs: u64) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if query(cfg).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(700));
    }
    false
}

// ---------- installation ----------

#[derive(Debug, Clone, PartialEq)]
pub enum AssetKind {
    Zip,
    Installer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub digest: String,
    pub kind: AssetKind,
    pub tag: String,
}

/// Choisit l'archive Windows x64 de la dernière version : zip portable de préférence, sinon l'installeur .exe.
/// Exige l'empreinte SHA-256 publiée par GitHub (champ `digest`) et une taille plausible.
pub fn pick_asset(rel: &Value) -> Result<Picked, String> {
    let tag = rel["tag_name"].as_str().unwrap_or("").chars().take(40).collect::<String>();
    let mut best: Option<(u8, &Value)> = None;
    for a in rel["assets"].as_array().into_iter().flatten() {
        let n = a["name"].as_str().unwrap_or("").to_ascii_lowercase();
        let win = n.contains("windows") || n.contains("win64") || n.contains("win_x64");
        let x64 = n.contains("amd64") || n.contains("x64") || n.contains("x86_64") || n.contains("win64");
        let bad = ["debug", "arm", "x86.", "win32", "sha", ".sig", ".asc", "symbols", ".pdb"].iter().any(|k| n.contains(k));
        if !win || !x64 || bad {
            continue;
        }
        let rank = if n.ends_with(".zip") {
            0
        } else if n.ends_with(".exe") {
            1
        } else {
            continue;
        };
        if best.map(|(r, _)| rank < r).unwrap_or(true) {
            best = Some((rank, a));
        }
    }
    let (rank, a) = best.ok_or("no_windows_asset")?;
    let size = a["size"].as_u64().unwrap_or(0);
    if size == 0 || size > MAX_ARCHIVE {
        return Err("bad_size".into());
    }
    let digest = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(|d| d.to_ascii_lowercase()).filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit())).ok_or("no_digest")?;
    let url = a["browser_download_url"].as_str().filter(|u| u.starts_with("https://github.com/hyperion-project/hyperion.ng/")).ok_or("bad_url")?.to_string();
    Ok(Picked { name: a["name"].as_str().unwrap_or("hyperion").to_string(), url, size, digest, kind: if rank == 0 { AssetKind::Zip } else { AssetKind::Installer }, tag })
}

fn emit_progress(app: &AppHandle, pct: u64, stage: Option<&str>) {
    let mut v = json!({ "pct": pct });
    if let Some(s) = stage {
        v["stage"] = json!(s);
    }
    let _ = app.emit("hyperion-install", v);
}

fn safe_file_name(n: &str) -> String {
    n.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect()
}

/// Télécharge et vérifie le SHA-256 (même approche que l'installation de mpv dans player.rs).
fn download_verified(app: &AppHandle, agent: &ureq::Agent, p: &Picked, dest: &Path) -> Result<(), String> {
    let resp = agent.get(&p.url).call().map_err(|_| "download_failed".to_string())?;
    let mut rd = resp.into_reader().take(MAX_ARCHIVE + 1);
    let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let (mut done, mut last) = (0u64, 0u64);
    loop {
        let n = rd.read(&mut buf).map_err(|_| "download_failed".to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if done > MAX_ARCHIVE {
            let _ = std::fs::remove_file(dest);
            return Err("bad_size".into());
        }
        if done - last > 1024 * 1024 {
            last = done;
            emit_progress(app, (done * 100 / p.size.max(1)).min(95), None);
        }
    }
    let got = hex_encode(&hasher.finalize());
    if got != p.digest {
        let _ = std::fs::remove_file(dest);
        return Err("digest_mismatch".into());
    }
    Ok(())
}

fn unzip(archive: &Path, to: &Path) -> Result<(), String> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(archive).map_err(|e| e.to_string())?).map_err(|_| "bad_archive".to_string())?;
    if z.len() > 20_000 {
        return Err("bad_archive".into());
    }
    let mut total = 0u64;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|_| "bad_archive".to_string())?;
        let Some(rel) = f.enclosed_name() else { continue }; // refuse les chemins sortant du dossier
        let dest = to.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            continue;
        }
        total += f.size();
        if total > MAX_UNPACKED {
            return Err("bad_archive".into());
        }
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut (&mut f).take(MAX_UNPACKED), &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Résultat de l'installation : "ready" (portable installé) ou "installer" (installeur téléchargé et vérifié,
/// à ouvrir par l'utilisateur : on ne l'exécute jamais en silence).
pub fn install(app: &AppHandle) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).user_agent("RusTV").build();
    let mut body = String::new();
    agent.get(RELEASES_API).set("Accept", "application/vnd.github+json").call().map_err(|_| "github_unreachable".to_string())?.into_reader().take(4 * 1024 * 1024).read_to_string(&mut body).map_err(|e| e.to_string())?;
    let rel: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let pick = pick_asset(&rel)?;
    let dir = hyperion_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    emit_progress(app, 0, None);
    match pick.kind {
        AssetKind::Installer => {
            let d = dir.join("installer");
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
            let dest = d.join(safe_file_name(&pick.name));
            download_verified(app, &agent, &pick, &dest)?;
            emit_progress(app, 100, None);
            Ok(json!({ "kind": "installer", "path": dest.to_string_lossy(), "version": pick.tag }))
        }
        AssetKind::Zip => {
            let archive = dir.join("download.zip");
            download_verified(app, &agent, &pick, &archive)?;
            emit_progress(app, 96, Some("extract"));
            let stage = dir.join("app-new");
            let _ = std::fs::remove_dir_all(&stage);
            std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
            let r = unzip(&archive, &stage);
            let _ = std::fs::remove_file(&archive);
            r?;
            if find_in(&stage, 3).is_none() {
                let _ = std::fs::remove_dir_all(&stage);
                return Err("exe_missing".into());
            }
            // Remplace l'ancienne installation (jamais pendant que notre Hyperion tourne).
            stop_process();
            let _ = std::fs::remove_dir_all(app_dir());
            std::fs::rename(&stage, app_dir()).map_err(|_| "replace_failed".to_string())?;
            let _ = std::fs::write(dir.join("version.txt"), &pick.tag);
            emit_progress(app, 100, None);
            Ok(json!({ "kind": "ready", "version": pick.tag }))
        }
    }
}

fn installed_version() -> Option<String> {
    std::fs::read_to_string(hyperion_dir().join("version.txt")).ok().map(|s| s.trim().chars().take(40).collect()).filter(|s: &String| !s.is_empty())
}

fn installer_path() -> Option<String> {
    std::fs::read_dir(hyperion_dir().join("installer")).ok()?.flatten().map(|e| e.path()).find(|p| p.is_file()).map(|p| p.to_string_lossy().into_owned())
}

// ---------- ambiance et comportement ----------

fn capture_active(app: &AppHandle) -> bool {
    let shell = app.state::<Shell>();
    let video = shell.player.state.lock().map(|s| s.active).unwrap_or(false);
    video || crate::browser::running() || app.webview_windows().keys().any(|l| l.starts_with("app-"))
}

fn send_ambient(cfg: &Config, rgb: [u8; 3]) -> bool {
    call(cfg, &Cmd::Color { rgb, priority: cfg.ambient_priority, duration_ms: None }).is_ok()
}

fn clear_ours(cfg: &Config) {
    let was = rt(|r| std::mem::replace(&mut r.ambient_set, false));
    let _ = was;
    let _ = call(cfg, &Cmd::Clear(cfg.ambient_priority));
    let _ = call(cfg, &Cmd::Clear(cfg.effect_priority));
}

fn apply_step(cfg: &Config, step: Step) {
    match step {
        Step::Start => {
            if start_process(cfg).is_ok() {
                wait_ready(cfg, 25);
            }
        }
        Step::LedsOn => {
            let _ = call(cfg, &Cmd::Component("ALL".into(), true));
            let _ = call(cfg, &Cmd::Component("LEDDEVICE".into(), true));
        }
        Step::ClearOurs => {
            restore_brightness(cfg);
            clear_ours(cfg);
        }
        Step::LedsOff => {
            let _ = call(cfg, &Cmd::Component("ALL".into(), false));
        }
        Step::Stop => {
            std::thread::sleep(Duration::from_millis(300));
            stop_process();
        }
    }
}

fn restore_brightness(cfg: &Config) {
    if let Some(b) = rt(|r| r.saved_brightness.take()) {
        let _ = call(cfg, &Cmd::Brightness(b));
    }
}

fn tv_changed(app: &AppHandle, on: bool) {
    let cfg = load_cfg();
    let plan = tv_plan(on, &cfg);
    if plan.is_empty() {
        return;
    }
    let _ = app.emit("hyperion-changed", ());
    std::thread::spawn(move || {
        for s in plan {
            apply_step(&cfg, s);
        }
    });
}

/// Une passe du moniteur (1 s) : capture vidéo/app <-> accueil, atténuation à la pause.
fn tick(app: &AppHandle, t0: Instant) {
    let cfg = load_cfg();
    let shell = app.state::<Shell>();
    if !cfg.enabled || !shell.tv.lock().map(|t| t.active).unwrap_or(false) {
        return;
    }
    let capture = capture_active(app);
    let prev = rt(|r| std::mem::replace(&mut r.capture, capture));
    if capture && !prev {
        // Une vidéo ou une app démarre : on efface l'ambiance (le grabber reprend la main) et on garde la capture active.
        clear_ours(&cfg);
        if cfg.capture_on_video && cfg.ambient != "off" {
            let _ = call(&cfg, &Cmd::Component("GRABBER".into(), true));
        }
    } else if !capture && prev {
        if cfg.ambient == "tile" {
            if let Some(rgb) = rt(|r| r.last_rgb) {
                if send_ambient(&cfg, rgb) {
                    rt(|r| r.ambient_set = true);
                }
            }
        }
    }
    let paused = shell.player.state.lock().map(|s| s.active && s.paused).unwrap_or(false);
    let act = rt(|r| r.dim.step(t0.elapsed().as_secs(), paused, cfg.dim_on_pause, cfg.dim_after_secs));
    match act {
        DimAct::Dim => {
            if let Ok(info) = query(&cfg) {
                let cur = info.brightness.unwrap_or(100);
                if cur > cfg.dim_level {
                    rt(|r| r.saved_brightness = Some(cur));
                    let _ = call(&cfg, &Cmd::Brightness(cfg.dim_level));
                }
            }
        }
        DimAct::Restore => restore_brightness(&cfg),
        DimAct::None => {}
    }
}

/// Point d'entrée : n'active rien tant que `enabled` est faux.
pub fn start(app: &AppHandle) {
    crate::hooks::on_tv_mode(|app, on| tv_changed(app, on));
    let app2 = app.clone();
    std::thread::spawn(move || {
        let t0 = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            tick(&app2, t0);
        }
    });
    // Reprise après un lancement avec `--tv` : le mode TV est peut-être déjà actif.
    let on = app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(false);
    if on {
        tv_changed(app, true);
    }
}

// ---------- commandes Tauri ----------

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

fn cfg_json(c: &Config) -> Value {
    let mut v = serde_json::to_value(c).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("tokenEnc");
        o.insert("hasToken".into(), json!(!c.token_enc.is_empty()));
    }
    v
}

fn status_json(cfg: &Config) -> Value {
    let exe = find_exe(cfg);
    let running = pid_alive();
    let info = if cfg.enabled || running { query(cfg).ok() } else { None };
    let connected = info.is_some();
    let i = info.unwrap_or_default();
    json!({
        "installed": exe.is_some(),
        "path": exe.map(|p| p.to_string_lossy().into_owned()),
        "version": i.version.clone().or_else(installed_version),
        "running": running,
        "connected": connected,
        "external": connected && !running,
        "ledOn": i.led_on(),
        "brightness": i.brightness,
        "effects": i.effects,
        "components": i.components,
        "instances": i.instances,
        "activePriorities": i.priorities,
        "installerPath": installer_path(),
    })
}

fn snapshot() -> Value {
    let cfg = load_cfg();
    json!({ "config": cfg_json(&cfg), "status": status_json(&cfg) })
}

#[tauri::command]
pub async fn hyperion_get() -> Result<Value, String> {
    blocking(|| Ok(snapshot())).await
}

/// `cfg` : la configuration (camelCase) ; `token` optionnel : absent = inchangé, "" = effacer.
#[tauri::command]
pub async fn hyperion_set(cfg: Value) -> Result<Value, String> {
    blocking(move || {
        let mut v = cfg;
        let token = v.as_object_mut().and_then(|o| {
            o.remove("tokenEnc");
            o.remove("hasToken");
            o.remove("token")
        });
        let old = load_cfg();
        let mut merged = serde_json::to_value(&old).map_err(|e| e.to_string())?;
        if let (Some(m), Some(n)) = (merged.as_object_mut(), v.as_object()) {
            for (k, x) in n {
                m.insert(k.clone(), x.clone());
            }
        }
        let mut new: Config = serde_json::from_value(merged).map_err(|_| "invalid_config".to_string())?;
        new.token_enc = old.token_enc.clone();
        if let Some(t) = token.and_then(|t| t.as_str().map(|s| s.trim().to_string())) {
            if t.len() > 256 {
                return Err("invalid_token".into());
            }
            new.token_enc = if t.is_empty() { String::new() } else { hex_encode(&dpapi::protect(t.as_bytes()).ok_or("dpapi_failed")?) };
        }
        let new = sanitize(new)?;
        store::save("hyperion", &new)?;
        drop_conn();
        Ok(snapshot())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_install(app: AppHandle) -> Result<Value, String> {
    blocking(move || install(&app)).await
}

#[tauri::command]
pub async fn hyperion_start() -> Result<Value, String> {
    blocking(|| {
        let cfg = load_cfg();
        start_process(&cfg)?;
        wait_ready(&cfg, 20);
        Ok(snapshot())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_stop() -> Result<Value, String> {
    blocking(|| {
        stop_process();
        Ok(snapshot())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_power(on: bool) -> Result<(), String> {
    blocking(move || {
        let cfg = load_cfg();
        call(&cfg, &Cmd::Component("ALL".into(), on))?;
        if on {
            call(&cfg, &Cmd::Component("LEDDEVICE".into(), true))?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_brightness(level: u8) -> Result<(), String> {
    blocking(move || {
        let cfg = load_cfg();
        rt(|r| r.saved_brightness = None);
        call(&cfg, &Cmd::Brightness(level.min(100))).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_effect(name: String) -> Result<(), String> {
    blocking(move || {
        if name.is_empty() || name.len() > 100 || name.contains('\0') {
            return Err("invalid_effect".into());
        }
        let cfg = load_cfg();
        call(&cfg, &Cmd::Effect { name, priority: cfg.effect_priority }).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn hyperion_clear() -> Result<(), String> {
    blocking(|| {
        let cfg = load_cfg();
        clear_ours(&cfg);
        Ok(())
    })
    .await
}

/// Ambiance de l'accueil : `color`/`color2` = dégradé de la tuile ciblée (#rrggbb). Anti-rebond de 400 ms.
#[tauri::command]
pub async fn hyperion_ambient(app: AppHandle, color: String, color2: Option<String>) -> Result<(), String> {
    let Some(rgb) = ambient_color(&color, color2.as_deref()) else { return Err("invalid_color".into()) };
    let gen = rt(|r| {
        r.last_rgb = Some(rgb);
        r.ambient_gen += 1;
        r.ambient_gen
    });
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        if rt(|r| r.ambient_gen) != gen {
            return;
        }
        let cfg = load_cfg();
        let tv = app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(false);
        if !cfg.enabled || !tv || capture_active(&app) {
            return;
        }
        match cfg.ambient.as_str() {
            "tile" => {
                if send_ambient(&cfg, rgb) {
                    rt(|r| r.ambient_set = true);
                }
            }
            "grabber" => {
                if rt(|r| r.ambient_set) {
                    clear_ours(&cfg);
                }
            }
            _ => {}
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn hyperion_open_web_ui() -> Result<(), String> {
    let cfg = load_cfg();
    if !host_ok(&cfg.host) {
        return Err("invalid_host".into());
    }
    let host = if cfg.host.contains(':') && !cfg.host.starts_with('[') { format!("[{}]", cfg.host) } else { cfg.host.clone() };
    rustv_core::apps::launch(&format!("http://{host}:{}", cfg.web_port));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn config_defaults() {
        let c = Config::default();
        assert!(!c.enabled && c.manage && c.leds_on_tv_start && c.leds_off_tv_exit && c.stop_on_tv_exit && c.dim_on_pause && c.capture_on_video);
        assert_eq!((c.host.as_str(), c.port, c.web_port, c.ambient.as_str()), ("127.0.0.1", 19444, 8090, "tile"));
        assert!(c.ambient_priority < 240 && c.token_enc.is_empty());
        let p: Config = serde_json::from_str(r#"{"enabled":true,"port":1234}"#).unwrap();
        assert!(p.enabled && p.port == 1234 && p.host == "127.0.0.1");
    }

    #[test]
    fn sanitize_bounds() {
        let mut c = Config::default();
        c.ambient = "x".into();
        c.dim_level = 250;
        c.dim_after_secs = 0;
        c.ambient_priority = 999;
        c.extra_args = vec!["".into(), "--a".into()];
        let s = sanitize(c).unwrap();
        assert_eq!((s.ambient.as_str(), s.dim_level, s.dim_after_secs, s.ambient_priority), ("tile", 100, 5, 253));
        assert_eq!(s.extra_args, vec!["--a".to_string()]);
        let mut bad = Config::default();
        bad.host = "a b;c".into();
        assert!(sanitize(bad).is_err());
        let mut bad = Config::default();
        bad.path = r"C:\Windows\cmd.exe".into();
        assert!(sanitize(bad).is_err());
        let mut ok = Config::default();
        ok.path = r"C:\H\hyperiond.exe".into();
        assert!(sanitize(ok).is_ok());
    }

    #[test]
    fn locality() {
        assert!(is_local("127.0.0.1") && is_local("LocalHost") && !is_local("192.168.1.5"));
    }

    #[test]
    fn command_building() {
        assert_eq!(Cmd::ServerInfo.build(3), json!({"command":"serverinfo","tan":3}));
        let c = Cmd::Component("LEDDEVICE".into(), false).build(1);
        assert_eq!(c["componentstate"], json!({"component":"LEDDEVICE","state":false}));
        assert_eq!(Cmd::Brightness(250).build(1)["adjustment"]["brightness"], 100);
        let e = Cmd::Effect { name: "Rainbow swirl".into(), priority: 150 }.build(2);
        assert_eq!((e["effect"]["name"].as_str(), e["priority"].as_i64(), e["origin"].as_str()), (Some("Rainbow swirl"), Some(150), Some("RusTV")));
        let col = Cmd::Color { rgb: [1, 2, 3], priority: 200, duration_ms: None }.build(4);
        assert_eq!(col["color"], json!([1, 2, 3]));
        assert!(col.get("duration").is_none());
        let col = Cmd::Color { rgb: [1, 2, 3], priority: 200, duration_ms: Some(5000) }.build(4);
        assert_eq!(col["duration"], 5000);
        assert_eq!(Cmd::Clear(-1).build(5)["priority"], -1);
        assert_eq!(Cmd::SourceSelect(200).build(5)["command"], "sourceselect");
        let i = Cmd::Instance(2).build(6);
        assert_eq!((i["subcommand"].as_str(), i["instance"].as_u64()), (Some("switchTo"), Some(2)));
        let a = Cmd::Authorize("tok".into()).build(7);
        assert_eq!((a["command"].as_str(), a["subcommand"].as_str(), a["token"].as_str()), (Some("authorize"), Some("login"), Some("tok")));
    }

    #[test]
    fn response_parsing() {
        let r = parse_response(r#"{"command":"color","success":true,"tan":5}"#).unwrap();
        assert!(r.success && r.tan == Some(5) && r.command == "color");
        let r = parse_response(r#"{"command":"effect","success":false,"error":"Effect not found","tan":6}"#).unwrap();
        assert!(!r.success && r.error.as_deref() == Some("Effect not found"));
        assert!(parse_response("not json").is_err());
        assert!(parse_response("[1,2]").is_err());
        assert!(parse_response("").is_err());
        let r = parse_response(r#"{"command":"x","error":"boom"}"#).unwrap();
        assert!(!r.success);
    }

    #[test]
    fn tan_matching_skips_noise() {
        let data = "garbage\n{\"command\":\"priorities-update\",\"data\":{}}\n{\"command\":\"color\",\"success\":true,\"tan\":8}\n{\"command\":\"clear\",\"success\":true,\"tan\":9}\n";
        let mut c = Cursor::new(data.as_bytes());
        let r = read_matching(&mut c, 9, 10).unwrap();
        assert_eq!(r.command, "clear");
        let mut c = Cursor::new(data.as_bytes());
        assert_eq!(read_matching(&mut c, 99, 10).unwrap_err(), "closed");
        let mut c = Cursor::new(data.as_bytes());
        assert_eq!(read_matching(&mut c, 99, 2).unwrap_err(), "timeout");
        let f = "{\"command\":\"componentstate\",\"success\":false,\"error\":\"nope\",\"tan\":4}\n";
        let r = read_matching(&mut Cursor::new(f.as_bytes()), 4, 3).unwrap();
        assert!(!r.success);
    }

    #[test]
    fn serverinfo_parsing() {
        let v = json!({"command":"serverinfo","success":true,"tan":1,"info":{
            "components":[{"name":"ALL","enabled":true},{"name":"LEDDEVICE","enabled":true},{"name":"GRABBER","enabled":false}],
            "adjustment":[{"brightness":80,"saturationGain":1}],
            "effects":[{"name":"Rainbow swirl"},{"name":"Knight rider"},{"file":"x"}],
            "priorities":[{"priority":240,"componentId":"GRABBER","owner":"X11","active":true,"visible":true},{"componentId":"x"}],
            "instance":[{"instance":0,"friendly_name":"TV","running":true}],
            "hyperion_build":[{"version":"2.0.16"}]}});
        let i = parse_serverinfo(&v);
        assert!(i.led_on());
        assert_eq!(i.brightness, Some(80));
        assert_eq!(i.effects, vec!["Rainbow swirl", "Knight rider"]);
        assert_eq!(i.priorities.len(), 1);
        assert_eq!(i.priorities[0].component, "GRABBER");
        assert_eq!(i.instances, vec![Instance { id: 0, name: "TV".into(), running: true }]);
        assert_eq!(i.version.as_deref(), Some("2.0.16"));
        assert_eq!(i.components.get("GRABBER"), Some(&false));
        let empty = parse_serverinfo(&json!({}));
        assert!(!empty.led_on() && empty.effects.is_empty() && empty.brightness.is_none());
        let off = parse_serverinfo(&json!({"info":{"components":[{"name":"ALL","enabled":false}]}}));
        assert!(!off.led_on());
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_decode(&hex_encode(&[0, 15, 255])), Some(vec![0, 15, 255]));
        assert_eq!(hex_decode("zz"), None);
        assert_eq!(hex_decode("abc"), None);
    }

    #[test]
    fn ambient_colour() {
        assert_eq!(parse_hex("#fff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#0a0B0c"), Some([10, 11, 12]));
        assert_eq!(parse_hex("red"), None);
        assert_eq!(parse_hex("#12345"), None);
        // dégradé sombre : le pic est ramené vers 220 (facteur borné à 2,4)
        let c = ambient_color("#6d4aff", Some("#1d1066")).unwrap();
        assert!(c.iter().max().copied().unwrap() >= 219);
        // pas de boost au-dessus de 255 et jamais de débordement
        let w = ambient_color("#ffffff", Some("#ffffff")).unwrap();
        assert_eq!(w, [255, 255, 255]);
        // facteur borné : un gris très sombre n'est pas amplifié au-delà de 2,4
        let d = ambient_color("#101010", None).unwrap();
        assert_eq!(d, [38, 38, 38]);
        assert_eq!(ambient_color("#000", Some("#000")), Some([0, 0, 0]));
        assert!(ambient_color("nope", None).is_none());
        // teinte conservée : le canal dominant reste dominant
        let r = ambient_color("#d62f4a", Some("#4d0e1b")).unwrap();
        assert!(r[0] > r[1] && r[0] > r[2]);
    }

    #[test]
    fn dim_state_machine() {
        let mut d = Dim::default();
        assert_eq!(d.step(0, false, true, 30), DimAct::None);
        assert_eq!(d.step(10, true, true, 30), DimAct::None);
        assert_eq!(d.step(39, true, true, 30), DimAct::None);
        assert_eq!(d.step(40, true, true, 30), DimAct::Dim);
        assert_eq!(d.step(41, true, true, 30), DimAct::None); // une seule fois
        assert_eq!(d.step(50, false, true, 30), DimAct::Restore);
        assert_eq!(d.step(51, false, true, 30), DimAct::None);
        // la pause redémarre le compteur
        assert_eq!(d.step(60, true, true, 30), DimAct::None);
        assert_eq!(d.step(80, true, true, 30), DimAct::None);
        // désactivé en cours de route : restauration
        assert_eq!(d.step(100, true, true, 30), DimAct::Dim);
        assert_eq!(d.step(101, true, false, 30), DimAct::Restore);
        let mut off = Dim::default();
        assert_eq!(off.step(100, true, false, 5), DimAct::None);
    }

    #[test]
    fn tv_transitions() {
        let mut c = Config::default();
        assert!(tv_plan(true, &c).is_empty() && tv_plan(false, &c).is_empty()); // désactivé : rien
        c.enabled = true;
        assert_eq!(tv_plan(true, &c), vec![Step::Start, Step::LedsOn]);
        assert_eq!(tv_plan(false, &c), vec![Step::ClearOurs, Step::LedsOff, Step::Stop]);
        c.manage = false;
        assert_eq!(tv_plan(true, &c), vec![Step::LedsOn]);
        assert_eq!(tv_plan(false, &c), vec![Step::ClearOurs, Step::LedsOff]);
        c.manage = true;
        c.host = "192.168.1.20".into(); // distant : jamais de processus
        assert_eq!(tv_plan(true, &c), vec![Step::LedsOn]);
        assert!(!tv_plan(false, &c).contains(&Step::Stop));
        c.host = "127.0.0.1".into();
        c.leds_off_tv_exit = false;
        c.stop_on_tv_exit = false;
        c.leds_on_tv_start = false;
        assert_eq!(tv_plan(true, &c), vec![Step::Start]);
        assert_eq!(tv_plan(false, &c), vec![Step::ClearOurs]);
    }

    fn sample_release(with_zip: bool, digest: bool) -> Value {
        let d = if digest { json!("sha256:".to_string() + &"ab".repeat(32)) } else { Value::Null };
        let mut assets = vec![
            json!({"name":"Hyperion-2.0.16-Linux-x86_64.deb","size":1000,"browser_download_url":"https://github.com/hyperion-project/hyperion.ng/releases/download/2.0.16/a.deb","digest":d}),
            json!({"name":"Hyperion-2.0.16-Windows-AMD64.exe","size":50_000_000,"browser_download_url":"https://github.com/hyperion-project/hyperion.ng/releases/download/2.0.16/Hyperion-2.0.16-Windows-AMD64.exe","digest":d}),
            json!({"name":"Hyperion-2.0.16-Windows-AMD64-debug.zip","size":50_000_000,"browser_download_url":"https://github.com/hyperion-project/hyperion.ng/releases/download/2.0.16/dbg.zip","digest":d}),
            json!({"name":"Hyperion-2.0.16-Darwin-arm64.tar.gz","size":50_000_000,"browser_download_url":"https://github.com/hyperion-project/hyperion.ng/x","digest":d}),
        ];
        if with_zip {
            assets.push(json!({"name":"Hyperion-2.0.16-Windows-AMD64.zip","size":60_000_000,"browser_download_url":"https://github.com/hyperion-project/hyperion.ng/releases/download/2.0.16/Hyperion-2.0.16-Windows-AMD64.zip","digest":d}));
        }
        json!({"tag_name":"2.0.16","assets":assets})
    }

    #[test]
    fn asset_selection() {
        let p = pick_asset(&sample_release(true, true)).unwrap();
        assert_eq!(p.kind, AssetKind::Zip);
        assert_eq!(p.name, "Hyperion-2.0.16-Windows-AMD64.zip");
        assert_eq!(p.tag, "2.0.16");
        assert_eq!(p.digest, "ab".repeat(32));
        let p = pick_asset(&sample_release(false, true)).unwrap();
        assert_eq!(p.kind, AssetKind::Installer);
        assert!(p.name.ends_with(".exe"));
        assert_eq!(pick_asset(&sample_release(true, false)).unwrap_err(), "no_digest");
        assert_eq!(pick_asset(&json!({"assets":[]})).unwrap_err(), "no_windows_asset");
        let mut r = sample_release(true, true);
        r["assets"][4]["browser_download_url"] = json!("https://evil.example/x.zip");
        assert_eq!(pick_asset(&r).unwrap_err(), "bad_url");
        let mut r = sample_release(true, true);
        r["assets"][4]["size"] = json!(u64::MAX / 2);
        assert_eq!(pick_asset(&r).unwrap_err(), "bad_size");
    }

    #[test]
    fn install_candidates() {
        let env = |k: &str| match k {
            "ProgramFiles" => Some(r"C:\Program Files".to_string()),
            "LOCALAPPDATA" => Some(r"C:\Users\u\AppData\Local".to_string()),
            _ => None,
        };
        let c = candidate_paths(r"D:\x\hyperiond.exe", &env);
        assert_eq!(c[0], PathBuf::from(r"D:\x\hyperiond.exe"));
        assert!(c.contains(&PathBuf::from(r"C:\Program Files").join("Hyperion").join("bin").join("hyperiond.exe")));
        assert!(c.iter().any(|p| p.to_string_lossy().contains("Programs")));
        assert_eq!(safe_file_name("a b/../c.exe"), "a_b_.._c.exe");
    }
}
