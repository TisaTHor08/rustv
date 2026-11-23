//! Intégration Bitwarden via la CLI officielle `bw` (jamais embarquée ni téléchargée automatiquement).
//!
//! Principes (voir docs/BITWARDEN.md) :
//! - le mot de passe maître n'est utilisé que pour `bw unlock` / `bw login`, transmis par variable
//!   d'environnement (jamais en ligne de commande), jamais stocké, mis à zéro après usage ;
//! - la clé de session `bw` reste uniquement en mémoire (variable d'environnement `BW_SESSION` du seul
//!   processus enfant), effacée au verrouillage, après l'inactivité configurée et quand le dernier
//!   téléphone se déconnecte ;
//! - la recherche ne renvoie que des métadonnées non secrètes ; les secrets ne sortent du processus
//!   que sous forme de frappes clavier injectées dans le champ qui a le focus ;
//! - tout passe par le trait [`Runner`] (faux dans les tests : la vraie `bw` n'est jamais appelée) et
//!   le trait [`Typer`] (faux dans les tests : rien n'est réellement saisi).
use rustv_protocol::messages::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_AUTOLOCK_MIN: u32 = 5;
pub const MIN_AUTOLOCK_MIN: u32 = 1;
pub const MAX_AUTOLOCK_MIN: u32 = 30;
const MAX_RESULTS: usize = 30;
/// Variable d'environnement portant le mot de passe maître vers `bw --passwordenv`.
const PW_ENV: &str = "RUSTV_BW_PASSWORD";

const T_STATUS: Duration = Duration::from_secs(20);
const T_AUTH: Duration = Duration::from_secs(90);
const T_QUERY: Duration = Duration::from_secs(40);
const T_QUICK: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------------------------
// Exécution de `bw`
// ---------------------------------------------------------------------------------------------

/// Sortie d'un appel à `bw`. La sortie standard peut contenir des secrets : elle est mise à zéro à la libération.
pub struct RunOut {
    pub code: i32,
    pub stdout: Secret,
    /// Jamais transmise ni journalisée ; sert uniquement à classer l'erreur.
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunError {
    NotFound,
    Timeout,
    Io(String),
}

pub trait Runner: Send + Sync {
    fn run(&self, args: &[&str], env: &[(&str, &str)], timeout: Duration) -> Result<RunOut, RunError>;
    /// Chemin de `bw` s'il est trouvé.
    fn path(&self) -> Option<String>;
}

/// Liste (pure) des emplacements usuels de `bw`, d'après l'environnement.
pub fn candidates(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if !cfg!(windows) {
        // Linux/macOS : `bw` (npm, snap, Homebrew) sur le PATH puis emplacements usuels.
        if let Some(p) = env("PATH") {
            for d in std::env::split_paths(&p) {
                v.push(d.join("bw"));
            }
        }
        for d in ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin", "/snap/bin"] {
            v.push(PathBuf::from(d).join("bw"));
        }
        if let Some(h) = env("HOME") {
            v.push(PathBuf::from(&h).join(".local").join("bin").join("bw"));
            v.push(PathBuf::from(&h).join(".npm-global").join("bin").join("bw"));
        }
        return v;
    }
    if let Some(p) = env("PATH") {
        for d in std::env::split_paths(&p) {
            v.push(d.join("bw.exe"));
            v.push(d.join("bw.cmd"));
        }
    }
    if let Some(l) = env("LOCALAPPDATA") {
        v.push(PathBuf::from(&l).join("Microsoft").join("WinGet").join("Links").join("bw.exe"));
    }
    if let Some(u) = env("USERPROFILE") {
        v.push(PathBuf::from(&u).join("scoop").join("shims").join("bw.exe"));
    }
    if let Some(p) = env("ProgramData") {
        v.push(PathBuf::from(&p).join("chocolatey").join("bin").join("bw.exe"));
    }
    if let Some(a) = env("APPDATA") {
        v.push(PathBuf::from(&a).join("npm").join("bw.cmd"));
    }
    if let Some(p) = env("ProgramFiles") {
        v.push(PathBuf::from(&p).join("Bitwarden CLI").join("bw.exe"));
    }
    v
}

fn find_bw() -> Option<PathBuf> {
    let env = |k: &str| std::env::var(k).ok();
    if let Some(p) = candidates(&env).into_iter().find(|p| p.is_file()) {
        return Some(p);
    }
    // winget : %LOCALAPPDATA%\Microsoft\WinGet\Packages\Bitwarden.CLI_*\bw.exe
    let base = PathBuf::from(env("LOCALAPPDATA")?).join("Microsoft").join("WinGet").join("Packages");
    for e in std::fs::read_dir(base).ok()?.flatten() {
        if e.file_name().to_string_lossy().starts_with("Bitwarden.CLI") {
            let p = e.path().join("bw.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Vraie exécution de `bw` : sans fenêtre, sans stdin, avec délai maximal.
#[derive(Default)]
pub struct BwRunner {
    path: Mutex<Option<PathBuf>>,
}

impl BwRunner {
    fn resolve(&self) -> Option<PathBuf> {
        let mut p = self.path.lock().unwrap();
        if let Some(cur) = p.as_ref() {
            if cur.is_file() {
                return Some(cur.clone());
            }
        }
        *p = find_bw();
        p.clone()
    }
}

impl Runner for BwRunner {
    fn path(&self) -> Option<String> {
        self.resolve().map(|p| p.to_string_lossy().into_owned())
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)], timeout: Duration) -> Result<RunOut, RunError> {
        use std::io::Read;
        let path = self.resolve().ok_or(RunError::NotFound)?;
        let mut cmd = Command::new(&path);
        cmd.args(args)
            .env("BW_NOINTERACTION", "true")
            .env_remove("BW_SESSION")
            .env_remove("BW_PASSWORD")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = cmd.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { RunError::NotFound } else { RunError::Io(e.kind().to_string()) })?;
        let mut out = child.stdout.take().ok_or(RunError::Io("stdout".into()))?;
        let mut err = child.stderr.take().ok_or(RunError::Io("stderr".into()))?;
        let t_out = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = out.by_ref().take(16 << 20).read_to_end(&mut b);
            b
        });
        let t_err = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = err.by_ref().take(1 << 20).read_to_end(&mut b);
            b
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(RunError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(RunError::Io(e.kind().to_string())),
            }
        };
        let stdout = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).into_owned();
        let stderr = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).into_owned();
        Ok(RunOut { code: status.code().unwrap_or(-1), stdout: Secret::new(stdout), stderr })
    }
}

// ---------------------------------------------------------------------------------------------
// Saisie clavier
// ---------------------------------------------------------------------------------------------

/// Saisie dans le PC : abstraite pour les tests.
pub trait Typer: Send + Sync {
    /// Un champ de saisie a-t-il le focus (UI Automation) ?
    fn has_focus(&self) -> bool;
    /// Identifiant de la fenêtre au premier plan (pour détecter un changement de fenêtre pendant l'opération).
    fn foreground(&self) -> isize;
    fn foreground_title(&self) -> String;
    fn type_text(&self, text: &str);
    fn tab(&self);
    fn enter(&self);
}

/// Saisie réelle via `SendInput` (module `input`).
pub struct WinTyper {
    pub focus: Arc<AtomicBool>,
}

impl Typer for WinTyper {
    fn has_focus(&self) -> bool {
        self.focus.load(Ordering::Relaxed)
    }
    fn foreground(&self) -> isize {
        crate::input::foreground().0 as isize
    }
    fn foreground_title(&self) -> String {
        crate::input::foreground_title()
    }
    fn type_text(&self, text: &str) {
        crate::input::type_text(text);
    }
    fn tab(&self) {
        crate::input::tap(crate::input::keys::VK_TAB);
    }
    fn enter(&self) {
        crate::input::tap(crate::input::keys::VK_RETURN);
    }
}

// ---------------------------------------------------------------------------------------------
// Erreurs et fonctions pures
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultErr {
    NotInstalled,
    Locked,
    LoggedOut,
    BadPassword,
    Need2fa,
    Timeout,
    NoFocus,
    FocusChanged,
    Invalid,
    Empty,
    LogoutRequired,
    Failed,
}

impl VaultErr {
    /// Code stable transmis au téléphone (traduit là-bas) : jamais le message brut de `bw`.
    pub fn code(self) -> &'static str {
        match self {
            VaultErr::NotInstalled => "notInstalled",
            VaultErr::Locked => "locked",
            VaultErr::LoggedOut => "loggedOut",
            VaultErr::BadPassword => "badPassword",
            VaultErr::Need2fa => "need2fa",
            VaultErr::Timeout => "timeout",
            VaultErr::NoFocus => "noFocus",
            VaultErr::FocusChanged => "focusChanged",
            VaultErr::Invalid => "invalid",
            VaultErr::Empty => "empty",
            VaultErr::LogoutRequired => "logoutRequired",
            VaultErr::Failed => "failed",
        }
    }
}

impl From<RunError> for VaultErr {
    fn from(e: RunError) -> Self {
        match e {
            RunError::NotFound => VaultErr::NotInstalled,
            RunError::Timeout => VaultErr::Timeout,
            RunError::Io(_) => VaultErr::Failed,
        }
    }
}

/// Classe la sortie d'erreur de `bw` en code stable.
pub fn classify_error(stderr: &str) -> VaultErr {
    let s = stderr.to_lowercase();
    if s.contains("invalid master password") || s.contains("username or password is incorrect") || s.contains("invalid password") {
        VaultErr::BadPassword
    } else if s.contains("--method") || s.contains("code is required") || s.contains("two-step") || s.contains("two step") || s.contains("provider") || s.contains("invalid code") {
        VaultErr::Need2fa
    } else if s.contains("not logged in") || s.contains("you are not logged in") {
        VaultErr::LoggedOut
    } else if s.contains("locked") || s.contains("session key is invalid") || s.contains("invalid session") {
        VaultErr::Locked
    } else if s.contains("logout") && s.contains("required") || s.contains("log out") {
        VaultErr::LogoutRequired
    } else {
        VaultErr::Failed
    }
}

/// Hôte d'une URI de coffre (sans schéma, identifiants, port, chemin ni `www.`), en minuscules.
pub fn uri_host(uri: &str) -> String {
    let u = uri.trim();
    let rest = u.split_once("://").map(|(_, r)| r).unwrap_or(u);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or("");
    let rest = rest.rsplit('@').next().unwrap_or("");
    let rest = if rest.starts_with('[') { rest } else { rest.split(':').next().unwrap_or("") };
    let h = rest.to_lowercase();
    h.strip_prefix("www.").map(str::to_string).unwrap_or(h)
}

#[derive(serde::Deserialize, Default)]
struct RawUri {
    #[serde(default)]
    uri: Option<String>,
}
/// Volontairement sans champ `password` : le mot de passe n'est même pas désérialisé pour la recherche.
#[derive(serde::Deserialize, Default)]
struct RawLogin {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    uris: Option<Vec<RawUri>>,
    #[serde(default)]
    totp: Option<String>,
}
#[derive(serde::Deserialize)]
struct RawItem {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "type", default)]
    kind: u8,
    #[serde(default)]
    login: Option<RawLogin>,
}

/// Analyse la sortie de `bw list items` en métadonnées non secrètes (identifiants de type 1 uniquement).
pub fn parse_items(json: &str) -> Result<Vec<VaultItem>, VaultErr> {
    let raw: Vec<RawItem> = serde_json::from_str(json.trim()).map_err(|_| VaultErr::Failed)?;
    Ok(raw
        .into_iter()
        .filter(|i| i.kind == 1)
        .map(|i| {
            let l = i.login.unwrap_or_default();
            VaultItem {
                id: i.id,
                name: i.name,
                username: l.username.unwrap_or_default(),
                host: l.uris.and_then(|u| u.into_iter().find_map(|x| x.uri)).map(|u| uri_host(&u)).unwrap_or_default(),
                has_totp: l.totp.map(|t| !t.is_empty()).unwrap_or(false),
            }
        })
        .collect())
}

fn host_label(host: &str) -> &str {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2]
    } else {
        host
    }
}

/// Pertinence d'un identifiant pour le titre de la fenêtre au premier plan : 2 = domaine, 1 = nom, 0 = aucun.
pub fn score(item: &VaultItem, title: &str) -> u8 {
    let t = title.to_lowercase();
    let label = host_label(&item.host);
    if label.chars().count() >= 3 && t.contains(label) {
        2
    } else if item.name.chars().count() >= 3 && t.contains(&item.name.to_lowercase()) {
        1
    } else {
        0
    }
}

/// Trie par pertinence (stable) ; avec `q` vide on ne garde que les suggestions pertinentes.
pub fn rank(mut items: Vec<VaultItem>, title: &str, only_relevant: bool) -> Vec<VaultItem> {
    if only_relevant {
        items.retain(|i| score(i, title) > 0);
    }
    items.sort_by(|a, b| score(b, title).cmp(&score(a, title)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    items.truncate(MAX_RESULTS);
    items
}

fn valid_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn valid_email(e: &str) -> bool {
    e.len() <= 254 && e.contains('@') && !e.starts_with('-') && !e.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn valid_code(c: &str) -> bool {
    !c.is_empty() && c.len() <= 32 && c.chars().all(|c| c.is_ascii_alphanumeric())
}

/// URL de serveur auto-hébergé acceptée par `bw config server`.
pub fn valid_server(u: &str) -> bool {
    let u = u.trim();
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() <= 200 && !u.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"')
}

/// Délai d'inactivité écoulé ? (logique pure de l'auto-verrouillage)
pub fn expired(last: Instant, now: Instant, minutes: u32) -> bool {
    now.saturating_duration_since(last) >= Duration::from_secs(minutes.clamp(MIN_AUTOLOCK_MIN, MAX_AUTOLOCK_MIN) as u64 * 60)
}

fn clean_query(q: &str) -> String {
    q.chars().filter(|c| !c.is_control()).take(100).collect::<String>().trim().to_string()
}

// ---------------------------------------------------------------------------------------------
// Le coffre
// ---------------------------------------------------------------------------------------------

struct Inner {
    status: VaultStatus,
    email: String,
    server: String,
    session: Option<Secret>,
    last: Instant,
    autolock_min: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultInfo {
    pub installed: bool,
    pub path: Option<String>,
    pub status: VaultStatus,
    pub email: String,
    pub server: String,
    pub autolock_min: u32,
}

pub struct Vault {
    runner: Arc<dyn Runner>,
    inner: Mutex<Inner>,
    /// Sérialise les opérations lentes (un seul `bw` à la fois).
    op: Mutex<()>,
    version: AtomicU64,
    settings_path: Option<PathBuf>,
    /// Pause entre identifiant / Tab / mot de passe.
    pub pace: Duration,
}

fn load_autolock(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()?;
    Some((v.get("autolockMin")?.as_u64()? as u32).clamp(MIN_AUTOLOCK_MIN, MAX_AUTOLOCK_MIN))
}

impl Vault {
    pub fn new(runner: Arc<dyn Runner>, settings_path: Option<PathBuf>) -> Self {
        let autolock_min = settings_path.as_deref().and_then(load_autolock).unwrap_or(DEFAULT_AUTOLOCK_MIN);
        Vault {
            runner,
            inner: Mutex::new(Inner { status: VaultStatus::Unknown, email: String::new(), server: String::new(), session: None, last: Instant::now(), autolock_min }),
            op: Mutex::new(()),
            version: AtomicU64::new(0),
            settings_path,
            pace: Duration::from_millis(80),
        }
    }

    /// Vault branché sur la vraie CLI ; réglages dans `%APPDATA%\RusTV\vault.json`.
    pub fn system() -> Self {
        let base = crate::platform::config_dir().join("vault.json");
        Vault::new(Arc::new(BwRunner::default()), Some(base))
    }

    /// Compteur incrémenté à chaque changement d'état visible (le cœur diffuse alors l'état aux téléphones).
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    fn set_status(&self, g: &mut Inner, s: VaultStatus) {
        if g.status != s {
            g.status = s;
            self.version.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn state_msg(&self) -> PcMsg {
        let g = self.inner.lock().unwrap();
        PcMsg::VaultState { status: g.status, email: g.email.clone(), autolock_min: g.autolock_min }
    }

    pub fn info(&self) -> VaultInfo {
        let g = self.inner.lock().unwrap();
        let path = self.runner.path();
        VaultInfo { installed: path.is_some(), path, status: g.status, email: g.email.clone(), server: g.server.clone(), autolock_min: g.autolock_min }
    }

    pub fn autolock_min(&self) -> u32 {
        self.inner.lock().unwrap().autolock_min
    }

    pub fn set_autolock(&self, min: u32) {
        let m = min.clamp(MIN_AUTOLOCK_MIN, MAX_AUTOLOCK_MIN);
        self.inner.lock().unwrap().autolock_min = m;
        self.version.fetch_add(1, Ordering::Relaxed);
        if let Some(p) = &self.settings_path {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(p, format!("{{\"autolockMin\":{m}}}"));
        }
    }

    fn touch(&self) {
        self.inner.lock().unwrap().last = Instant::now();
    }

    fn session_env(&self) -> Option<Secret> {
        self.inner.lock().unwrap().session.clone()
    }

    /// Exécute `bw` avec la session courante ; une session refusée verrouille le coffre.
    fn run_session(&self, args: &[&str], timeout: Duration) -> Result<RunOut, VaultErr> {
        let session = self.session_env().ok_or(VaultErr::Locked)?;
        let out = self.runner.run(args, &[("BW_SESSION", session.expose())], timeout)?;
        if out.code != 0 {
            let e = classify_error(&out.stderr);
            if matches!(e, VaultErr::Locked | VaultErr::LoggedOut) {
                self.clear_session(if e == VaultErr::Locked { VaultStatus::Locked } else { VaultStatus::LoggedOut });
            }
            return Err(if e == VaultErr::Need2fa || e == VaultErr::BadPassword { VaultErr::Failed } else { e });
        }
        Ok(out)
    }

    /// Interroge `bw status` (sans secret).
    pub fn refresh(&self) -> VaultStatus {
        let _op = self.op.lock().unwrap();
        self.refresh_locked()
    }

    fn refresh_locked(&self) -> VaultStatus {
        let session = self.session_env();
        let env: Vec<(&str, &str)> = session.as_ref().map(|s| vec![("BW_SESSION", s.expose())]).unwrap_or_default();
        let res = self.runner.run(&["status", "--nointeraction"], &env, T_STATUS);
        let mut g = self.inner.lock().unwrap();
        match res {
            Err(RunError::NotFound) => {
                g.session = None;
                self.set_status(&mut g, VaultStatus::NotInstalled);
            }
            Err(_) => {}
            Ok(o) if o.code == 0 => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(o.stdout.expose().trim()) {
                    g.email = v.get("userEmail").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    g.server = v.get("serverUrl").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let s = match v.get("status").and_then(|x| x.as_str()) {
                        Some("unlocked") if g.session.is_some() => VaultStatus::Unlocked,
                        Some("unauthenticated") => VaultStatus::LoggedOut,
                        Some("locked") | Some("unlocked") => VaultStatus::Locked,
                        _ => g.status,
                    };
                    if s != VaultStatus::Unlocked {
                        g.session = None;
                    }
                    self.set_status(&mut g, s);
                }
            }
            Ok(_) => {}
        }
        g.status
    }

    fn require_installed(&self) -> Result<(), VaultErr> {
        if self.runner.path().is_none() {
            let mut g = self.inner.lock().unwrap();
            self.set_status(&mut g, VaultStatus::NotInstalled);
            return Err(VaultErr::NotInstalled);
        }
        Ok(())
    }

    fn store_session(&self, out: RunOut) -> Result<(), VaultErr> {
        let key = out.stdout.expose().trim();
        if key.is_empty() {
            return Err(VaultErr::Failed);
        }
        let mut g = self.inner.lock().unwrap();
        g.session = Some(Secret::new(key.to_string()));
        g.last = Instant::now();
        self.set_status(&mut g, VaultStatus::Unlocked);
        Ok(())
    }

    pub fn unlock(&self, password: &Secret) -> Result<(), VaultErr> {
        let _op = self.op.lock().unwrap();
        self.require_installed()?;
        if password.is_empty() {
            return Err(VaultErr::Invalid);
        }
        let out = self.runner.run(&["unlock", "--passwordenv", PW_ENV, "--raw", "--nointeraction"], &[(PW_ENV, password.expose())], T_AUTH)?;
        if out.code != 0 {
            let e = classify_error(&out.stderr);
            if e == VaultErr::LoggedOut {
                let mut g = self.inner.lock().unwrap();
                self.set_status(&mut g, VaultStatus::LoggedOut);
            }
            return Err(if e == VaultErr::Need2fa { VaultErr::Failed } else { e });
        }
        self.store_session(out)?;
        self.refresh_locked();
        Ok(())
    }

    pub fn login(&self, email: &str, password: &Secret, code: Option<&str>, method: Option<u8>) -> Result<(), VaultErr> {
        let _op = self.op.lock().unwrap();
        self.require_installed()?;
        let email = email.trim();
        if !valid_email(email) || password.is_empty() {
            return Err(VaultErr::Invalid);
        }
        let code = code.map(str::trim).filter(|c| !c.is_empty());
        if let Some(c) = code {
            if !valid_code(c) {
                return Err(VaultErr::Invalid);
            }
        }
        let method = method.unwrap_or(0);
        if !matches!(method, 0 | 1 | 3) {
            return Err(VaultErr::Invalid);
        }
        let m = method.to_string();
        let mut args = vec!["login", email, "--passwordenv", PW_ENV, "--raw", "--nointeraction"];
        if let Some(c) = code {
            args.extend(["--method", &m, "--code", c]);
        }
        let out = self.runner.run(&args, &[(PW_ENV, password.expose())], T_AUTH)?;
        if out.code != 0 {
            return Err(classify_error(&out.stderr));
        }
        self.store_session(out)?;
        self.refresh_locked();
        Ok(())
    }

    /// Efface la clé de session (mémoire) sans appeler `bw` ; renvoie vrai si une session existait.
    pub fn clear_session(&self, to: VaultStatus) -> bool {
        let mut g = self.inner.lock().unwrap();
        let had = g.session.take().is_some();
        if g.status == VaultStatus::Unlocked || had {
            self.set_status(&mut g, to);
        }
        had
    }

    /// Verrouillage immédiat : session effacée, puis `bw lock` (qui supprime aussi la clé locale de `bw`).
    pub fn lock(&self) {
        let had = self.clear_session(VaultStatus::Locked);
        if had {
            self.bw_lock();
        }
    }

    pub fn bw_lock(&self) {
        let _ = self.runner.run(&["lock", "--nointeraction"], &[], T_QUICK);
    }

    /// Verrouille sans bloquer l'appelant (`bw lock` part dans un thread).
    pub fn lock_async(self: &Arc<Self>) {
        if self.clear_session(VaultStatus::Locked) {
            let v = self.clone();
            std::thread::spawn(move || v.bw_lock());
        }
    }

    /// Auto-verrouillage : efface la session si l'inactivité dépasse le délai. Renvoie vrai si on vient de verrouiller.
    pub fn expire(&self, now: Instant) -> bool {
        let due = {
            let g = self.inner.lock().unwrap();
            g.session.is_some() && expired(g.last, now, g.autolock_min)
        };
        due && self.clear_session(VaultStatus::Locked)
    }

    pub fn logout(&self) -> Result<(), VaultErr> {
        let _op = self.op.lock().unwrap();
        self.require_installed()?;
        self.clear_session(VaultStatus::LoggedOut);
        let out = self.runner.run(&["logout", "--nointeraction"], &[], T_AUTH)?;
        if out.code != 0 && classify_error(&out.stderr) != VaultErr::LoggedOut {
            return Err(VaultErr::Failed);
        }
        let mut g = self.inner.lock().unwrap();
        g.email.clear();
        self.set_status(&mut g, VaultStatus::LoggedOut);
        Ok(())
    }

    /// `bw config server` : possible uniquement hors connexion (limite de la CLI).
    pub fn set_server(&self, url: &str) -> Result<(), VaultErr> {
        let _op = self.op.lock().unwrap();
        self.require_installed()?;
        let url = url.trim();
        // Vide = serveur officiel.
        let url = if url.is_empty() { "https://bitwarden.com" } else { url };
        if !valid_server(url) {
            return Err(VaultErr::Invalid);
        }
        if self.refresh_locked() != VaultStatus::LoggedOut {
            return Err(VaultErr::LogoutRequired);
        }
        let out = self.runner.run(&["config", "server", url], &[], T_QUICK)?;
        if out.code != 0 {
            return Err(VaultErr::Failed);
        }
        self.refresh_locked();
        Ok(())
    }

    /// Recherche : métadonnées uniquement. `title` = titre de la fenêtre au premier plan (suggestions).
    pub fn search(&self, q: &str, title: &str) -> Result<Vec<VaultItem>, VaultErr> {
        let _op = self.op.lock().unwrap();
        self.touch();
        let q = clean_query(q);
        let arg = format!("--search={q}");
        let mut args = vec!["list", "items", "--nointeraction"];
        if !q.is_empty() {
            args.push(&arg);
        }
        let out = self.run_session(&args, T_QUERY)?;
        let items = parse_items(out.stdout.expose())?;
        self.touch();
        Ok(rank(items, title, q.is_empty()))
    }

    /// Saisit un secret dans le champ qui a le focus. Les secrets ne quittent jamais ce processus autrement.
    pub fn fill(&self, id: &str, what: VaultFill, enter: bool, typer: &dyn Typer) -> Result<(), VaultErr> {
        let _op = self.op.lock().unwrap();
        self.touch();
        if !valid_id(id) {
            return Err(VaultErr::Invalid);
        }
        if self.session_env().is_none() {
            return Err(VaultErr::Locked);
        }
        if !typer.has_focus() {
            return Err(VaultErr::NoFocus);
        }
        let fg = typer.foreground();
        let (user, secret): (Secret, Secret) = if what == VaultFill::Totp {
            let out = self.run_session(&["get", "totp", id, "--nointeraction"], T_QUERY)?;
            (Secret::default(), Secret::new(out.stdout.expose().trim().to_string()))
        } else {
            let out = self.run_session(&["get", "item", id, "--nointeraction"], T_QUERY)?;
            let v: serde_json::Value = serde_json::from_str(out.stdout.expose().trim()).map_err(|_| VaultErr::Failed)?;
            let l = v.get("login");
            let get = |k: &str| Secret::new(l.and_then(|l| l.get(k)).and_then(|x| x.as_str()).unwrap_or("").to_string());
            (get("username"), get("password"))
        };
        // La fenêtre ou le champ ont pu changer pendant l'appel (plusieurs secondes) : on ne saisit jamais ailleurs.
        if !typer.has_focus() || typer.foreground() != fg {
            return Err(VaultErr::FocusChanged);
        }
        match what {
            VaultFill::Totp | VaultFill::Password => {
                if secret.is_empty() {
                    return Err(VaultErr::Empty);
                }
                typer.type_text(secret.expose());
            }
            VaultFill::Username => {
                if user.is_empty() {
                    return Err(VaultErr::Empty);
                }
                typer.type_text(user.expose());
            }
            VaultFill::Both => {
                if user.is_empty() && secret.is_empty() {
                    return Err(VaultErr::Empty);
                }
                if !user.is_empty() {
                    typer.type_text(user.expose());
                    if !secret.is_empty() {
                        std::thread::sleep(self.pace);
                        typer.tab();
                        std::thread::sleep(self.pace);
                    }
                }
                if !secret.is_empty() {
                    typer.type_text(secret.expose());
                }
            }
        }
        if enter {
            std::thread::sleep(self.pace);
            typer.enter();
        }
        self.touch();
        Ok(())
    }
}

/// Traite un message du téléphone (appelé dans un thread : `bw` est lent). Seuls des résultats
/// non secrets sont renvoyés.
pub fn dispatch(vault: &Vault, typer: &dyn Typer, msg: PhoneMsg, reply: &Sender<PcMsg>) {
    let result = |op: &str, r: Result<(), VaultErr>| {
        let _ = reply.send(PcMsg::VaultResult { op: op.into(), ok: r.is_ok(), code: r.err().map(|e| e.code().to_string()).unwrap_or_default() });
    };
    match msg {
        PhoneMsg::VaultStatus => {
            vault.refresh();
            let _ = reply.send(vault.state_msg());
        }
        PhoneMsg::VaultUnlock { password } => {
            result("unlock", vault.unlock(&password));
            let _ = reply.send(vault.state_msg());
        }
        PhoneMsg::VaultLogin { email, password, code, method } => {
            result("login", vault.login(&email, &password, code.as_deref(), method));
            let _ = reply.send(vault.state_msg());
        }
        PhoneMsg::VaultLock => {
            vault.lock();
            result("lock", Ok(()));
            let _ = reply.send(vault.state_msg());
        }
        PhoneMsg::VaultSearch { q } => match vault.search(&q, &typer.foreground_title()) {
            Ok(items) => {
                let _ = reply.send(PcMsg::VaultItems { items });
            }
            Err(e) => {
                result("search", Err(e));
                let _ = reply.send(vault.state_msg());
            }
        },
        PhoneMsg::VaultFill { id, what, enter } => {
            result("fill", vault.fill(&id, what, enter, typer));
            let _ = reply.send(vault.state_msg());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::mpsc::channel;

    type Call = (Vec<String>, Vec<(String, String)>);

    /// Faux `bw` : réponses scriptées, appels enregistrés. La vraie CLI n'est jamais lancée.
    struct Fake {
        calls: Mutex<Vec<Call>>,
        replies: Mutex<VecDeque<Result<(i32, String, String), RunError>>>,
        installed: bool,
    }
    impl Fake {
        fn new(replies: Vec<Result<(i32, String, String), RunError>>) -> Arc<Self> {
            Arc::new(Fake { calls: Mutex::new(vec![]), replies: Mutex::new(replies.into()), installed: true })
        }
    }
    impl Runner for Fake {
        fn run(&self, args: &[&str], env: &[(&str, &str)], _t: Duration) -> Result<RunOut, RunError> {
            self.calls.lock().unwrap().push((args.iter().map(|s| s.to_string()).collect(), env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()));
            match self.replies.lock().unwrap().pop_front() {
                Some(Ok((code, out, err))) => Ok(RunOut { code, stdout: Secret::new(out), stderr: err }),
                Some(Err(e)) => Err(e),
                None => Err(RunError::Io("no reply".into())),
            }
        }
        fn path(&self) -> Option<String> {
            self.installed.then(|| "bw".to_string())
        }
    }
    fn ok(s: &str) -> Result<(i32, String, String), RunError> {
        Ok((0, s.into(), String::new()))
    }
    fn fail(e: &str) -> Result<(i32, String, String), RunError> {
        Ok((1, String::new(), e.into()))
    }
    fn vault(f: &Arc<Fake>) -> Vault {
        let mut v = Vault::new(f.clone(), None);
        v.pace = Duration::ZERO;
        v
    }
    const STATUS_LOCKED: &str = r#"{"serverUrl":"https://vault.example.org","userEmail":"me@example.org","status":"locked"}"#;
    const STATUS_UNLOCKED: &str = r#"{"userEmail":"me@example.org","status":"unlocked"}"#;
    const ID: &str = "0b9c1e2a-1111-2222-3333-444455556666";

    #[derive(Default)]
    struct FakeTyper {
        log: Mutex<Vec<String>>,
        focus: AtomicBool,
        fg: AtomicU64,
        /// Si vrai, la fenêtre change dès le premier appel de `type_text`-précédent (simulé via `foreground`).
        switch_after: Mutex<Option<usize>>,
        fg_calls: Mutex<usize>,
    }
    impl FakeTyper {
        fn focused() -> Self {
            let t = FakeTyper::default();
            t.focus.store(true, Ordering::Relaxed);
            t.fg.store(7, Ordering::Relaxed);
            t
        }
        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }
    impl Typer for FakeTyper {
        fn has_focus(&self) -> bool {
            self.focus.load(Ordering::Relaxed)
        }
        fn foreground(&self) -> isize {
            let mut n = self.fg_calls.lock().unwrap();
            *n += 1;
            let sw = *self.switch_after.lock().unwrap();
            if sw.map(|s| *n > s).unwrap_or(false) {
                99
            } else {
                self.fg.load(Ordering::Relaxed) as isize
            }
        }
        fn foreground_title(&self) -> String {
            "Sign in - GitHub - Browser".into()
        }
        fn type_text(&self, t: &str) {
            self.log.lock().unwrap().push(format!("type:{t}"));
        }
        fn tab(&self) {
            self.log.lock().unwrap().push("tab".into());
        }
        fn enter(&self) {
            self.log.lock().unwrap().push("enter".into());
        }
    }

    fn unlocked(f: &Arc<Fake>) -> Vault {
        f.replies.lock().unwrap().push_front(ok(STATUS_UNLOCKED));
        f.replies.lock().unwrap().push_front(ok("SESSIONKEY\n"));
        let v = vault(f);
        v.unlock(&"pw".to_string().into()).unwrap();
        f.calls.lock().unwrap().clear();
        v
    }

    #[test]
    fn unlock_passes_password_by_env_never_args() {
        let f = Fake::new(vec![ok("SESSIONKEY\n"), ok(STATUS_UNLOCKED)]);
        let v = vault(&f);
        v.unlock(&"s3cret pw".to_string().into()).unwrap();
        let calls = f.calls.lock().unwrap();
        assert_eq!(calls[0].0, vec!["unlock", "--passwordenv", PW_ENV, "--raw", "--nointeraction"]);
        assert_eq!(calls[0].1, vec![(PW_ENV.to_string(), "s3cret pw".to_string())]);
        assert!(calls.iter().all(|c| c.0.iter().all(|a| !a.contains("s3cret"))));
        // la session suit dans l'environnement du second appel, pas en argument
        assert_eq!(calls[1].1, vec![("BW_SESSION".to_string(), "SESSIONKEY".to_string())]);
        assert_eq!(v.state_msg(), PcMsg::VaultState { status: VaultStatus::Unlocked, email: "me@example.org".into(), autolock_min: 5 });
    }

    #[test]
    fn unlock_errors_are_classified() {
        let f = Fake::new(vec![fail("Invalid master password.")]);
        assert_eq!(vault(&f).unlock(&"x".to_string().into()), Err(VaultErr::BadPassword));
        let f = Fake::new(vec![Err(RunError::Timeout)]);
        assert_eq!(vault(&f).unlock(&"x".to_string().into()), Err(VaultErr::Timeout));
        let f = Fake::new(vec![]);
        assert_eq!(vault(&f).unlock(&Secret::default()), Err(VaultErr::Invalid));
    }

    #[test]
    fn not_installed() {
        let f = Arc::new(Fake { calls: Mutex::new(vec![]), replies: Mutex::new(VecDeque::new()), installed: false });
        let v = vault(&f);
        assert_eq!(v.unlock(&"x".to_string().into()), Err(VaultErr::NotInstalled));
        assert_eq!(v.state_msg(), PcMsg::VaultState { status: VaultStatus::NotInstalled, email: String::new(), autolock_min: 5 });
        assert!(f.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn login_builds_command_with_2fa() {
        let f = Fake::new(vec![ok("KEY"), ok(STATUS_UNLOCKED)]);
        let v = vault(&f);
        v.login("me@example.org", &"pw".to_string().into(), Some("123456"), Some(0)).unwrap();
        let c = f.calls.lock().unwrap();
        assert_eq!(c[0].0, vec!["login", "me@example.org", "--passwordenv", PW_ENV, "--raw", "--nointeraction", "--method", "0", "--code", "123456"]);
        assert_eq!(c[0].1, vec![(PW_ENV.to_string(), "pw".to_string())]);
    }

    #[test]
    fn login_validates_inputs_and_maps_2fa() {
        let f = Fake::new(vec![fail("Code is required.")]);
        let v = vault(&f);
        assert_eq!(v.login("-evil@x", &"pw".to_string().into(), None, None), Err(VaultErr::Invalid));
        assert_eq!(v.login("a@b.c", &"pw".to_string().into(), Some("12 34"), None), Err(VaultErr::Invalid));
        assert_eq!(v.login("a@b.c", &"pw".to_string().into(), Some("123456"), Some(9)), Err(VaultErr::Invalid));
        assert_eq!(v.login("a@b.c", &"pw".to_string().into(), None, None), Err(VaultErr::Need2fa));
    }

    #[test]
    fn classify() {
        assert_eq!(classify_error("Username or password is incorrect. Try again."), VaultErr::BadPassword);
        assert_eq!(classify_error("Vault is locked."), VaultErr::Locked);
        assert_eq!(classify_error("You are not logged in."), VaultErr::LoggedOut);
        assert_eq!(classify_error("Login failed. No provider selected. Pass the --method option"), VaultErr::Need2fa);
        assert_eq!(classify_error("boom"), VaultErr::Failed);
    }

    #[test]
    fn parses_items_to_metadata_only() {
        let json = r#"[
          {"id":"a1","name":"GitHub","type":1,"login":{"username":"me","password":"TOPSECRET","totp":"otpauth://x","uris":[{"uri":"https://www.GitHub.com:443/login?x=1"}]}},
          {"id":"a2","name":"Note","type":2},
          {"id":"a3","name":"Bare","type":1,"login":{"username":null,"uris":[]}}
        ]"#;
        let items = parse_items(json).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], VaultItem { id: "a1".into(), name: "GitHub".into(), username: "me".into(), host: "github.com".into(), has_totp: true });
        assert_eq!(items[1], VaultItem { id: "a3".into(), name: "Bare".into(), username: String::new(), host: String::new(), has_totp: false });
        assert!(!serde_json::to_string(&items).unwrap().contains("TOPSECRET"));
        assert_eq!(parse_items("not json"), Err(VaultErr::Failed));
    }

    #[test]
    fn hosts() {
        assert_eq!(uri_host("https://user:pw@Sub.Example.com:8080/a?b#c"), "sub.example.com");
        assert_eq!(uri_host("example.org/x"), "example.org");
        assert_eq!(uri_host("androidapp://com.foo"), "com.foo");
        assert_eq!(uri_host(""), "");
    }

    #[test]
    fn ranking_by_window_title() {
        let mk = |n: &str, h: &str| VaultItem { id: n.into(), name: n.into(), username: String::new(), host: h.into(), has_totp: false };
        let items = vec![mk("Zeta", "zeta.io"), mk("Mail", "gmail.com"), mk("Code", "github.com"), mk("Git stuff", "")];
        let r = rank(items.clone(), "Sign in to GitHub - Firefox", true);
        assert_eq!(r.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["Code"]);
        let all = rank(items, "Sign in to GitHub", false);
        assert_eq!(all[0].name, "Code");
        assert_eq!(all[1].name, "Git stuff");
    }

    #[test]
    fn search_uses_session_env_and_safe_arg() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        f.replies.lock().unwrap().push_back(ok(r#"[{"id":"a1","name":"GitHub","type":1,"login":{"username":"me","uris":[{"uri":"https://github.com"}]}}]"#));
        let items = v.search("  --evil\n", "").unwrap();
        assert_eq!(items.len(), 1);
        let c = f.calls.lock().unwrap();
        assert_eq!(c[0].0, vec!["list", "items", "--nointeraction", "--search=--evil"]);
        assert_eq!(c[0].1, vec![("BW_SESSION".to_string(), "SESSIONKEY".to_string())]);
    }

    #[test]
    fn search_when_locked() {
        let f = Fake::new(vec![]);
        assert_eq!(vault(&f).search("x", ""), Err(VaultErr::Locked));
        assert!(f.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn session_rejected_locks() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        f.replies.lock().unwrap().push_back(fail("Session key is invalid."));
        assert_eq!(v.search("x", ""), Err(VaultErr::Locked));
        assert_eq!(v.search("x", ""), Err(VaultErr::Locked));
        assert_eq!(f.calls.lock().unwrap().len(), 1); // plus d'appel sans session
    }

    #[test]
    fn fill_both_types_user_tab_password_enter() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"me","password":"p@ss"}}"#));
        let t = FakeTyper::focused();
        v.fill(ID, VaultFill::Both, true, &t).unwrap();
        assert_eq!(t.log(), vec!["type:me", "tab", "type:p@ss", "enter"]);
        let c = f.calls.lock().unwrap();
        assert_eq!(c[0].0, vec!["get", "item", ID, "--nointeraction"]);
        assert!(c[0].0.iter().all(|a| !a.contains("p@ss")));
    }

    #[test]
    fn fill_single_kinds() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        let t = FakeTyper::focused();
        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"me","password":"pw1"}}"#));
        v.fill(ID, VaultFill::Username, false, &t).unwrap();
        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"me","password":"pw1"}}"#));
        v.fill(ID, VaultFill::Password, false, &t).unwrap();
        f.replies.lock().unwrap().push_back(ok("654321\n"));
        v.fill(ID, VaultFill::Totp, false, &t).unwrap();
        assert_eq!(t.log(), vec!["type:me", "type:pw1", "type:654321"]);
        assert_eq!(f.calls.lock().unwrap()[2].0, vec!["get", "totp", ID, "--nointeraction"]);
    }

    #[test]
    fn fill_refuses_without_focus_or_when_window_changes() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        let t = FakeTyper::default(); // pas de focus
        assert_eq!(v.fill(ID, VaultFill::Both, false, &t), Err(VaultErr::NoFocus));
        assert!(f.calls.lock().unwrap().is_empty());

        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"me","password":"pw"}}"#));
        let t = FakeTyper::focused();
        *t.switch_after.lock().unwrap() = Some(1); // la fenêtre change pendant l'appel à bw
        assert_eq!(v.fill(ID, VaultFill::Both, false, &t), Err(VaultErr::FocusChanged));
        assert!(t.log().is_empty());
    }

    #[test]
    fn fill_rejects_bad_ids_and_empty_secrets() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        let t = FakeTyper::focused();
        assert_eq!(v.fill("--session x", VaultFill::Both, false, &t), Err(VaultErr::Invalid));
        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"","password":""}}"#));
        assert_eq!(v.fill(ID, VaultFill::Both, false, &t), Err(VaultErr::Empty));
        assert!(t.log().is_empty());
    }

    #[test]
    fn autolock_logic() {
        let t0 = Instant::now();
        assert!(!expired(t0, t0 + Duration::from_secs(299), 5));
        assert!(expired(t0, t0 + Duration::from_secs(300), 5));
        // bornes 1..=30 min
        assert!(expired(t0, t0 + Duration::from_secs(60), 0));
        assert!(!expired(t0, t0 + Duration::from_secs(60 * 29), 99));
        assert!(expired(t0, t0 + Duration::from_secs(60 * 30), 99));
    }

    #[test]
    fn expire_clears_session_and_state() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        v.set_autolock(1);
        let v0 = v.version();
        assert!(!v.expire(Instant::now()));
        assert!(v.expire(Instant::now() + Duration::from_secs(61)));
        assert!(v.session_env().is_none());
        assert!(v.version() > v0);
        assert_eq!(v.search("x", ""), Err(VaultErr::Locked));
        assert!(!v.expire(Instant::now() + Duration::from_secs(600))); // rien à verrouiller
    }

    #[test]
    fn lock_clears_session_and_calls_bw_lock() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        f.replies.lock().unwrap().push_back(ok(""));
        v.lock();
        assert!(v.session_env().is_none());
        assert_eq!(f.calls.lock().unwrap()[0].0, vec!["lock", "--nointeraction"]);
        v.lock(); // déjà verrouillé : aucun appel de plus
        assert_eq!(f.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn set_server_requires_logged_out() {
        let f = Fake::new(vec![ok(r#"{"status":"unauthenticated"}"#), ok(""), ok(r#"{"status":"unauthenticated","serverUrl":"https://vw.lan"}"#)]);
        let v = vault(&f);
        v.set_server("https://vw.lan").unwrap();
        assert_eq!(f.calls.lock().unwrap()[1].0, vec!["config", "server", "https://vw.lan"]);
        assert_eq!(v.info().server, "https://vw.lan");
        assert_eq!(v.set_server("ftp://x"), Err(VaultErr::Invalid));
        f.replies.lock().unwrap().push_back(ok(STATUS_LOCKED));
        assert_eq!(v.set_server("https://other.example"), Err(VaultErr::LogoutRequired));
    }

    #[test]
    fn autolock_setting_is_clamped_and_persisted() {
        let dir = std::env::temp_dir().join(format!("rustv-vault-test-{}", std::process::id()));
        let path = dir.join("vault.json");
        let f = Fake::new(vec![]);
        let v = Vault::new(f.clone(), Some(path.clone()));
        assert_eq!(v.autolock_min(), 5);
        v.set_autolock(99);
        assert_eq!(v.autolock_min(), 30);
        v.set_autolock(0);
        assert_eq!(v.autolock_min(), 1);
        v.set_autolock(12);
        assert_eq!(Vault::new(f, Some(path)).autolock_min(), 12);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn candidate_paths_unix() {
        let env = |k: &str| match k {
            "PATH" => Some("/a:/b".to_string()),
            "HOME" => Some("/home/u".to_string()),
            _ => None,
        };
        let c = candidates(&env);
        assert!(c.contains(&PathBuf::from("/a").join("bw")));
        assert!(c.contains(&PathBuf::from("/opt/homebrew/bin").join("bw")));
        assert!(c.contains(&PathBuf::from("/home/u").join(".local").join("bin").join("bw")));
        assert!(!c.iter().any(|p| p.to_string_lossy().ends_with(".exe")));
    }

    #[cfg(windows)]
    #[test]
    fn candidate_paths() {
        let env = |k: &str| match k {
            "PATH" => Some("C:\\a;C:\\b".to_string()),
            "LOCALAPPDATA" => Some("C:\\L".to_string()),
            "USERPROFILE" => Some("C:\\U".to_string()),
            _ => None,
        };
        let c: Vec<String> = candidates(&env).iter().map(|p| p.to_string_lossy().replace('/', "\\")).collect();
        assert!(c.contains(&"C:\\a\\bw.exe".to_string()));
        assert!(c.contains(&"C:\\L\\Microsoft\\WinGet\\Links\\bw.exe".to_string()));
        assert!(c.contains(&"C:\\U\\scoop\\shims\\bw.exe".to_string()));
    }

    #[test]
    fn dispatch_replies_without_secrets() {
        let f = Fake::new(vec![]);
        let v = unlocked(&f);
        let t = FakeTyper::focused();
        let (tx, rx) = channel();
        f.replies.lock().unwrap().push_back(ok(r#"{"login":{"username":"me","password":"p@ss"}}"#));
        dispatch(&v, &t, PhoneMsg::VaultFill { id: ID.into(), what: VaultFill::Both, enter: false }, &tx);
        let msgs: Vec<PcMsg> = rx.try_iter().collect();
        assert_eq!(msgs[0], PcMsg::VaultResult { op: "fill".into(), ok: true, code: String::new() });
        assert!(!format!("{msgs:?}").contains("p@ss"));
        // sans focus
        let t2 = FakeTyper::default();
        dispatch(&v, &t2, PhoneMsg::VaultFill { id: ID.into(), what: VaultFill::Both, enter: false }, &tx);
        assert_eq!(rx.try_recv().unwrap(), PcMsg::VaultResult { op: "fill".into(), ok: false, code: "noFocus".into() });
    }
}
