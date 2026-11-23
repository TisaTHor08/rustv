//! Ouvre certaines tuiles (YouTube) dans Brave ou Vivaldi, s'ils sont installés, pour profiter de leur bloqueur de pub.
//!
//! Le navigateur est lancé en fenêtre « app » plein écran sur l'écran de la TV, avec un profil dédié à RusTV
//! (`%APPDATA%\RusTV\browser-profiles\<id>`) : le profil habituel de l'utilisateur n'est jamais touché.
//! Aucune écriture dans le registre (seules des requêtes `reg query` en lecture).
use crate::tv::{self, MonInfo};
use crate::{settings, Shell};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Brave,
    Vivaldi,
    /// Google Chrome (Linux) : Widevine inclus, pas de bloqueur de pub.
    Chrome,
    /// Chromium (Linux) : sans Widevine dans la plupart des distributions.
    Chromium,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct Found {
    pub kind: Kind,
    pub path: PathBuf,
}

/// Chemins d'installation standard (Brave d'abord : son bloqueur est actif par défaut).
pub fn candidate_paths(env: &dyn Fn(&str) -> Option<String>) -> Vec<(Kind, PathBuf)> {
    if !cfg!(windows) {
        return unix_candidate_paths(env);
    }
    let mut v = Vec::new();
    let local = env("LOCALAPPDATA").map(PathBuf::from);
    let pf: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|k| env(k)).map(PathBuf::from).collect();
    if let Some(l) = &local {
        v.push((Kind::Brave, l.join(r"BraveSoftware\Brave-Browser\Application\brave.exe")));
    }
    for p in &pf {
        v.push((Kind::Brave, p.join(r"BraveSoftware\Brave-Browser\Application\brave.exe")));
    }
    if let Some(l) = &local {
        v.push((Kind::Vivaldi, l.join(r"Vivaldi\Application\vivaldi.exe")));
    }
    for p in &pf {
        v.push((Kind::Vivaldi, p.join(r"Vivaldi\Application\vivaldi.exe")));
    }
    v
}

/// Linux/macOS : emplacements usuels (paquets, snap, flatpak, /opt, /Applications) puis `PATH`.
fn unix_candidate_paths(env: &dyn Fn(&str) -> Option<String>) -> Vec<(Kind, PathBuf)> {
    let mut v: Vec<(Kind, PathBuf)> = Vec::new();
    let (brave, vivaldi): (&[&str], &[&str]) = if cfg!(target_os = "macos") {
        (&["/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"], &["/Applications/Vivaldi.app/Contents/MacOS/Vivaldi"])
    } else {
        (
            &["/usr/bin/brave-browser", "/usr/bin/brave", "/opt/brave.com/brave/brave", "/snap/bin/brave", "/var/lib/flatpak/exports/bin/com.brave.Browser"],
            &["/usr/bin/vivaldi", "/usr/bin/vivaldi-stable", "/opt/vivaldi/vivaldi", "/snap/bin/vivaldi", "/var/lib/flatpak/exports/bin/com.vivaldi.Vivaldi"],
        )
    };
    v.extend(brave.iter().map(|p| (Kind::Brave, PathBuf::from(p))));
    v.extend(vivaldi.iter().map(|p| (Kind::Vivaldi, PathBuf::from(p))));
    if cfg!(target_os = "linux") {
        for p in ["/usr/bin/google-chrome-stable", "/usr/bin/google-chrome", "/opt/google/chrome/chrome", "/var/lib/flatpak/exports/bin/com.google.Chrome"] {
            v.push((Kind::Chrome, PathBuf::from(p)));
        }
        for p in ["/usr/bin/chromium", "/usr/bin/chromium-browser", "/snap/bin/chromium", "/var/lib/flatpak/exports/bin/org.chromium.Chromium"] {
            v.push((Kind::Chromium, PathBuf::from(p)));
        }
    }
    if let Some(path) = env("PATH") {
        for d in std::env::split_paths(&path) {
            for n in ["brave-browser", "brave"] {
                v.push((Kind::Brave, d.join(n)));
            }
            for n in ["vivaldi", "vivaldi-stable"] {
                v.push((Kind::Vivaldi, d.join(n)));
            }
            if cfg!(target_os = "linux") {
                for n in ["google-chrome-stable", "google-chrome"] {
                    v.push((Kind::Chrome, d.join(n)));
                }
                for n in ["chromium", "chromium-browser"] {
                    v.push((Kind::Chromium, d.join(n)));
                }
            }
        }
    }
    v
}

/// Navigateurs trouvés (un par type, premier chemin existant), Brave avant Vivaldi.
/// `extra` : chemins supplémentaires (registre « App Paths »), essayés après les chemins standard.
pub fn detect_with(env: &dyn Fn(&str) -> Option<String>, exists: &dyn Fn(&Path) -> bool, extra: &[(Kind, PathBuf)]) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    let mut cands = candidate_paths(env);
    cands.extend(extra.iter().cloned());
    for kind in [Kind::Brave, Kind::Vivaldi, Kind::Chrome, Kind::Chromium] {
        if let Some((_, p)) = cands.iter().find(|(k, p)| *k == kind && exists(p)) {
            out.push(Found { kind, path: p.clone() });
        }
    }
    out
}

/// Extrait le chemin d'une réponse `reg query ... /ve` (ligne « (Default) REG_SZ C:\...\x.exe »).
pub fn parse_reg_default(out: &str) -> Option<PathBuf> {
    for line in out.lines() {
        let l = line.trim();
        if l.starts_with('(') {
            if let Some(i) = l.find("REG_SZ") {
                let p = l[i + 6..].trim().trim_matches('"');
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    None
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

fn registry_paths() -> Vec<(Kind, PathBuf)> {
    if !cfg!(windows) {
        return Vec::new(); // pas de registre : les chemins usuels et le PATH suffisent
    }
    let mut v = Vec::new();
    for (kind, exe) in [(Kind::Brave, "brave.exe"), (Kind::Vivaldi, "vivaldi.exe")] {
        for hive in ["HKCU", "HKLM"] {
            let key = format!(r"{hive}\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}");
            let mut c = Command::new("reg");
            c.args(["query", &key, "/ve"]).stdin(Stdio::null()).stderr(Stdio::null());
            if let Ok(o) = no_window(&mut c).output() {
                if o.status.success() {
                    if let Some(p) = parse_reg_default(&String::from_utf8_lossy(&o.stdout)) {
                        v.push((kind, p));
                    }
                }
            }
        }
    }
    v
}

pub fn detect() -> Vec<Found> {
    detect_with(&|k| std::env::var(k).ok(), &|p| p.is_file(), &registry_paths())
}

/// Choix de l'utilisateur (`auto`, `builtin`, `brave`, `vivaldi`) → navigateur à utiliser, ou None pour WebView2.
pub fn resolve(choice: &str, found: &[Found]) -> Option<Found> {
    match choice {
        "builtin" => None,
        "brave" => found.iter().find(|f| f.kind == Kind::Brave).cloned(),
        "vivaldi" => found.iter().find(|f| f.kind == Kind::Vivaldi).cloned(),
        "chrome" => found.iter().find(|f| f.kind == Kind::Chrome).cloned(),
        "chromium" => found.iter().find(|f| f.kind == Kind::Chromium).cloned(),
        // Auto : seuls Brave et Vivaldi ont un bloqueur de pub (Chrome et Chromium ne servent qu'au choix explicite).
        _ => found.iter().find(|f| matches!(f.kind, Kind::Brave | Kind::Vivaldi)).cloned(),
    }
}

/// Navigateur externe pour une tuile : tuiles `adblock_browser` (toutes plateformes), et sous Linux les tuiles
/// protégées par DRM (WebKitGTK n'a pas Widevine, voir `platform/linux/streaming.rs`). `None` : vue web intégrée.
pub fn select(id: &str, tile: &Value) -> Option<Found> {
    if tile["adblock_browser"].as_bool().unwrap_or(false) {
        let choice = tile["browser"].as_str().unwrap_or("auto");
        return resolve(choice, &detect());
    }
    #[cfg(target_os = "linux")]
    {
        let cfg = crate::linux::streaming::load();
        if cfg.streaming_browser && crate::linux::streaming::is_drm_tile(id, tile) {
            return crate::linux::streaming::select(id, tile, &detect(), &cfg);
        }
    }
    let _ = id;
    None
}

/// Arguments Chromium : fenêtre « app » plein écran sur l'écran de la TV, profil dédié.
pub fn build_args(url: &str, ua: Option<&str>, profile: &Path, mon: Option<&MonInfo>, lang: Option<&str>) -> Vec<String> {
    let mut a = vec![
        format!("--app={url}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-features=Translate".into(),
        "--autoplay-policy=no-user-gesture-required".into(),
    ];
    if let Some(ua) = ua {
        a.push(format!("--user-agent={ua}"));
    }
    if let Some(m) = mon {
        a.push(format!("--window-position={},{}", m.x, m.y));
        a.push(format!("--window-size={},{}", m.w, m.h));
    }
    a.push("--start-fullscreen".into());
    if let Some(l) = lang.filter(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')) {
        a.push(format!("--lang={l}"));
    }
    a
}

/// Paramètre `hl` d'une URL (langue de l'interface YouTube), s'il y en a un.
fn url_hl(u: &str) -> Option<String> {
    let q = u.split_once('?')?.1;
    q.split('&').find_map(|kv| kv.strip_prefix("hl=")).map(|s| s.split('#').next().unwrap_or("").to_string())
}

// ---------- processus suivi ----------

struct Running {
    gen: u64,
    pid: u32,
}
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static GEN: Mutex<u64> = Mutex::new(0);

/// Ferme le navigateur lancé par RusTV (arbre de processus complet, uniquement le PID lancé par nous).
pub fn close() {
    let r = RUNNING.lock().unwrap().take();
    if let Some(r) = r {
        #[cfg(windows)]
        let mut c = {
            let mut c = Command::new("taskkill");
            c.args(["/PID", &r.pid.to_string(), "/T", "/F"]);
            c
        };
        // Unix : SIGTERM au processus lancé par nous (le backend Linux/macOS affinera : groupe de processus).
        #[cfg(not(windows))]
        let mut c = {
            let mut c = Command::new("kill");
            c.args(["-TERM", &r.pid.to_string()]);
            c
        };
        c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        let _ = no_window(&mut c).status();
    }
}

/// Vrai si un navigateur lancé par RusTV est en cours (utilisé par le module Hyperion).
pub fn running() -> bool {
    RUNNING.lock().map(|r| r.is_some()).unwrap_or(false)
}

/// Lance `url` dans le navigateur. Retourne Err si le lancement échoue (l'appelant retombe sur WebView2).
pub fn launch(app: &AppHandle, found: &Found, id: &str, tile: &Value, url: &str) -> Result<(), String> {
    close();
    let shell = app.state::<Shell>();
    let mons = tv::monitors(app);
    let idx = if shell.tv.lock().unwrap().active { shell.tv.lock().unwrap().monitor } else { 0 };
    let profile = settings::dir().join("browser-profiles").join(id);
    let _ = std::fs::create_dir_all(&profile);
    let lang = url_hl(url);
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut args = build_args(url, tile["ua"].as_str(), &profile, mons.get(idx), lang.as_deref());
    #[cfg(target_os = "linux")]
    args.extend(crate::linux::browser_extra_args());
    let mut cmd = Command::new(&found.path);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = no_window(&mut cmd).spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    let gen = {
        let mut g = GEN.lock().unwrap();
        *g += 1;
        *g
    };
    *RUNNING.lock().unwrap() = Some(Running { gen, pid });

    // Surveillance : si le navigateur se ferme tout seul, retour à l'accueil (ou repli sur WebView2 s'il quitte aussitôt).
    let app2 = app.clone();
    let tile2 = tile.clone();
    let url2 = url.to_string();
    let id2 = id.to_string();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = child.wait();
        {
            let mut r = RUNNING.lock().unwrap();
            match r.as_ref() {
                Some(x) if x.gen == gen => *r = None,
                _ => return, // fermé par RusTV (ou remplacé) : rien à faire
            }
        }
        if started.elapsed() < Duration::from_secs(2) {
            if let Ok(u) = url2.parse() {
                if tv::open_web(&app2, &id2, &tile2, u).is_ok() {
                    return;
                }
            }
        }
        let active = app2.state::<Shell>().tv.lock().unwrap().active;
        if active {
            tv::go_home(&app2);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn env(k: &str) -> Option<String> {
        match k {
            "LOCALAPPDATA" => Some(r"C:\Users\u\AppData\Local".into()),
            "ProgramFiles" => Some(r"C:\Program Files".into()),
            "ProgramFiles(x86)" => Some(r"C:\Program Files (x86)".into()),
            _ => None,
        }
    }

    #[cfg(windows)]
    #[test]
    fn brave_preferred_over_vivaldi() {
        let f = detect_with(&env, &|_| true, &[]);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind, Kind::Brave);
        assert!(f[0].path.to_string_lossy().ends_with(r"Local\BraveSoftware\Brave-Browser\Application\brave.exe"));
        assert_eq!(f[1].kind, Kind::Vivaldi);
    }

    #[cfg(windows)]
    #[test]
    fn only_vivaldi_in_program_files() {
        let want = PathBuf::from(r"C:\Program Files\Vivaldi\Application\vivaldi.exe");
        let f = detect_with(&env, &|p| p == want, &[]);
        assert_eq!(f, vec![Found { kind: Kind::Vivaldi, path: want }]);
    }

    #[cfg(windows)]
    #[test]
    fn nothing_installed_and_registry_extra() {
        assert!(detect_with(&env, &|_| false, &[]).is_empty());
        let extra = [(Kind::Brave, PathBuf::from(r"D:\b\brave.exe"))];
        let f = detect_with(&env, &|p| p.starts_with("D:"), &extra);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, Kind::Brave);
    }

    #[cfg(windows)]
    #[test]
    fn resolve_choices() {
        let f = detect_with(&env, &|_| true, &[]);
        assert_eq!(resolve("auto", &f).unwrap().kind, Kind::Brave);
        assert_eq!(resolve("vivaldi", &f).unwrap().kind, Kind::Vivaldi);
        assert!(resolve("builtin", &f).is_none());
        assert!(resolve("auto", &[]).is_none());
        assert!(resolve("brave", &f[1..]).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn reg_output_parsing() {
        let out = "\r\nHKEY_CURRENT_USER\\SOFTWARE\\X\\brave.exe\r\n    (Default)    REG_SZ    C:\\Brave\\brave.exe\r\n";
        assert_eq!(parse_reg_default(out), Some(PathBuf::from(r"C:\Brave\brave.exe")));
        assert_eq!(parse_reg_default("nope"), None);
    }

    #[test]
    fn command_line() {
        let m = MonInfo { index: 1, name: String::new(), x: 1920, y: -10, w: 3840, h: 2160 };
        let a = build_args("https://www.youtube.com/tv?hl=fr", Some("UA x"), Path::new(r"C:\p"), Some(&m), Some("fr"));
        assert_eq!(a[0], "--app=https://www.youtube.com/tv?hl=fr");
        assert!(a.contains(&r"--user-data-dir=C:\p".to_string()));
        assert!(a.contains(&"--user-agent=UA x".to_string()));
        assert!(a.contains(&"--window-position=1920,-10".to_string()));
        assert!(a.contains(&"--window-size=3840,2160".to_string()));
        assert!(a.contains(&"--start-fullscreen".to_string()));
        assert!(a.contains(&"--no-first-run".to_string()));
        assert!(a.contains(&"--lang=fr".to_string()));
        let b = build_args("https://x", None, Path::new("p"), None, Some("a b"));
        assert!(!b.iter().any(|s| s.starts_with("--user-agent") || s.starts_with("--window-") || s.starts_with("--lang")));
    }

    #[test]
    fn hl_param() {
        assert_eq!(url_hl("https://www.youtube.com/tv?hl=fr"), Some("fr".into()));
        assert_eq!(url_hl("https://x/y"), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_detection_uses_known_paths_and_path() {
        let env = |k: &str| (k == "PATH").then(|| "/x/bin:/y/bin".to_string());
        let c = candidate_paths(&env);
        assert!(c.iter().any(|(k, p)| *k == Kind::Brave && p == Path::new("/x/bin").join("brave-browser").as_path()));
        assert!(c.iter().any(|(k, p)| *k == Kind::Vivaldi && p == Path::new("/y/bin").join("vivaldi").as_path()));
        assert!(!c.iter().any(|(_, p)| p.to_string_lossy().ends_with(".exe")));
        let want = Path::new("/y/bin").join("vivaldi");
        let f = detect_with(&env, &|p| p == want, &[]);
        assert_eq!(f, vec![Found { kind: Kind::Vivaldi, path: want }]);
        assert!(detect_with(&env, &|_| false, &[]).is_empty());
    }
}
