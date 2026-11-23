//! Linux : applications installées (fichiers `.desktop` XDG, flatpak, snap) et lancement.
//!
//! `list` renvoie d'abord les applis perso (`id = "c:<index>"`) puis les applis installées triées par nom
//! (`id = "s:<hash>"`, `target` = chemin du fichier `.desktop`). `launch(target)` ouvre :
//! - un fichier `.desktop` : son `Exec` nettoyé (codes %f %u… retirés), lancé sans shell, détaché ;
//! - une URL ou un fichier/dossier existant : `xdg-open` ;
//! - `steam://open/bigpicture` : `steam -bigpicture` si Steam est installé ;
//! - un nom de programme (`vlc`) ou un chemin exécutable : lancé directement.
use crate::config::CustomApp;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct App {
    pub id: String,
    pub name: String,
    pub target: String,
}

/// Entrée `.desktop` utile.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Entry {
    pub name: String,
    pub exec: Vec<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub path: Option<String>,
}

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// Découpe un `Exec` (guillemets doubles et antislash, selon la spécification Desktop Entry) et retire les codes
/// de champ (`%f %F %u %U %d %D %n %N %i %c %k %v %m`) ; `%%` devient `%`.
pub fn clean_exec(exec: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                    started = true;
                }
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            '%' => match chars.next() {
                Some('%') => {
                    cur.push('%');
                    started = true;
                }
                _ => {} // code de champ : supprimé
            },
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(cur);
    }
    // Un code de champ isolé (ex. `%U`) a laissé un argument vide : on le retire.
    args.retain(|a| !a.is_empty());
    args
}

/// Valeur `Name[<lang>]` si présente, sinon `Name`.
fn localized(map: &[(String, String)], key: &str, lang: Option<&str>) -> Option<String> {
    if let Some(l) = lang {
        let k = format!("{key}[{l}]");
        if let Some((_, v)) = map.iter().find(|(kk, _)| *kk == k) {
            return Some(v.clone());
        }
    }
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Analyse un fichier `.desktop`. `None` si ce n'est pas une application visible dans un lanceur
/// (`Type` autre qu'Application, `NoDisplay`, `Hidden`, `Terminal=true`, `Exec` absent).
pub fn parse_desktop(text: &str, lang: Option<&str>) -> Option<Entry> {
    let mut in_entry = false;
    let mut kv: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry || l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            kv.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let get = |k: &str| kv.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.as_str());
    let is_true = |k: &str| get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if get("Type").is_some_and(|t| t != "Application") || is_true("NoDisplay") || is_true("Hidden") || is_true("Terminal") {
        return None;
    }
    let name = localized(&kv, "Name", lang).filter(|n| !n.is_empty())?;
    let exec = clean_exec(get("Exec")?);
    if exec.is_empty() {
        return None;
    }
    Some(Entry {
        name,
        exec,
        icon: get("Icon").filter(|i| !i.is_empty()).map(str::to_string),
        categories: get("Categories").map(|c| c.split(';').filter(|s| !s.is_empty()).map(str::to_string).collect()).unwrap_or_default(),
        path: get("Path").filter(|p| !p.is_empty()).map(str::to_string),
    })
}

/// Dossiers de données XDG : `XDG_DATA_HOME` (défaut `~/.local/share`), `XDG_DATA_DIRS` (défaut
/// `/usr/local/share:/usr/share`), flatpak (système et utilisateur), snap.
pub fn data_dirs(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let nonempty = |k: &str| env(k).filter(|v| !v.is_empty());
    let mut v: Vec<PathBuf> = Vec::new();
    let home = nonempty("HOME").map(PathBuf::from);
    match nonempty("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()) {
        Some(p) => v.push(p),
        None => v.extend(home.iter().map(|h| h.join(".local/share"))),
    }
    let dirs = nonempty("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    v.extend(dirs.split(':').filter(|d| d.starts_with('/')).map(PathBuf::from));
    if let Some(h) = &home {
        v.push(h.join(".local/share/flatpak/exports/share"));
    }
    v.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    v.push(PathBuf::from("/var/lib/snapd/desktop"));
    let mut seen: Vec<PathBuf> = Vec::new();
    v.retain(|p| {
        let dup = seen.contains(p);
        seen.push(p.clone());
        !dup
    });
    v
}

fn scan(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 2 {
                scan(&p, depth + 1, out);
            }
        } else if p.extension().is_some_and(|x| x == "desktop") {
            out.push(p);
        }
    }
}

fn lang_from_env() -> Option<String> {
    let l = std::env::var("LC_MESSAGES").or_else(|_| std::env::var("LANG")).ok()?;
    let short: String = l.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    (short.len() >= 2 && short != "C").then_some(short)
}

pub fn list(custom: &[CustomApp]) -> Vec<App> {
    let mut v: Vec<App> = custom.iter().enumerate().map(|(i, c)| App { id: format!("c:{i}"), name: c.name.clone(), target: c.target.clone() }).collect();
    let env = |k: &str| std::env::var(k).ok();
    let lang = lang_from_env();
    let mut files = Vec::new();
    for d in data_dirs(&env) {
        scan(&d.join("applications"), 0, &mut files);
    }
    let mut found: Vec<App> = Vec::new();
    for f in files {
        // RusTV ne se liste pas lui-même.
        if f.file_stem().is_some_and(|s| s == "rustv" || s == "RusTV") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let Some(e) = parse_desktop(&text, lang.as_deref()) else { continue };
        let target = f.to_string_lossy().into_owned();
        found.push(App { id: format!("s:{}", fnv(&target)), name: e.name, target });
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    v.extend(found);
    v
}

/// Lance `argv` détaché (nouveau groupe de processus, sorties coupées) ; un thread récupère le code de sortie.
fn spawn_detached(argv: &[String], cwd: Option<&str>) -> bool {
    let Some((prog, rest)) = argv.split_first() else { return false };
    let mut c = Command::new(prog);
    c.args(rest).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    if let Some(d) = cwd.filter(|d| Path::new(d).is_dir()) {
        c.current_dir(d);
    }
    match c.spawn() {
        Ok(mut ch) => {
            std::thread::spawn(move || {
                let _ = ch.wait();
            });
            true
        }
        Err(_) => false,
    }
}

fn xdg_open(target: &str) {
    let prog = crate::platform::which("xdg-open").map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "xdg-open".into());
    if !spawn_detached(&[prog, target.to_string()], None) {
        eprintln!("rustv-core: apps tool_missing (xdg-open)");
    }
}

/// Ce que `launch` fera d'une cible (testable sans lancer de processus).
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    Desktop(String),
    Open(String),
    Steam,
    Program(String),
}

pub fn plan(target: &str, is_existing_path: bool) -> Plan {
    if target.ends_with(".desktop") && target.starts_with('/') {
        Plan::Desktop(target.to_string())
    } else if target.starts_with("steam://") {
        Plan::Steam
    } else if target.contains("://") || is_existing_path {
        Plan::Open(target.to_string())
    } else {
        Plan::Program(target.to_string())
    }
}

fn is_exec_file(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.is_file() && p.metadata().map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

pub fn launch(target: &str) {
    let p = Path::new(target);
    match plan(target, p.exists() && !is_exec_file(p)) {
        Plan::Desktop(path) => {
            let Ok(text) = std::fs::read_to_string(&path) else { return };
            let Some(e) = parse_desktop(&text, None) else { return };
            if !spawn_detached(&e.exec, e.path.as_deref()) {
                // Repli : gtk-launch avec l'identifiant du fichier.
                if let Some(id) = Path::new(&path).file_stem().and_then(|s| s.to_str()) {
                    let _ = spawn_detached(&["gtk-launch".to_string(), id.to_string()], None);
                }
            }
        }
        Plan::Steam => match crate::platform::which("steam") {
            Some(s) => {
                let _ = spawn_detached(&[s.to_string_lossy().into_owned(), "-bigpicture".into()], None);
            }
            None => xdg_open(target),
        },
        Plan::Open(t) => xdg_open(&t),
        Plan::Program(prog) => {
            let exe = if prog.contains('/') { Some(PathBuf::from(&prog)) } else { crate::platform::which(&prog) };
            match exe {
                Some(e) => {
                    let _ = spawn_detached(&[e.to_string_lossy().into_owned()], None);
                }
                None => eprintln!("rustv-core: apps not_found"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX: &str = "[Desktop Entry]\nVersion=1.0\nName=Firefox\nName[fr]=Firefox Web Browser\nExec=/usr/lib/firefox/firefox %u\nIcon=firefox\nTerminal=false\nType=Application\nCategories=GNOME;GTK;Network;WebBrowser;\n\n[Desktop Action new-window]\nName=New Window\nExec=firefox --new-window %u\n";

    #[test]
    fn parses_a_normal_entry() {
        let e = parse_desktop(FIREFOX, None).unwrap();
        assert_eq!(e.name, "Firefox");
        assert_eq!(e.exec, vec!["/usr/lib/firefox/firefox"]);
        assert_eq!(e.icon.as_deref(), Some("firefox"));
        assert!(e.categories.contains(&"WebBrowser".to_string()));
        // les sections « Desktop Action » sont ignorées
        assert!(!e.exec.contains(&"--new-window".to_string()));
    }

    #[test]
    fn localized_name_wins() {
        assert_eq!(parse_desktop(FIREFOX, Some("fr")).unwrap().name, "Firefox Web Browser");
        assert_eq!(parse_desktop(FIREFOX, Some("de")).unwrap().name, "Firefox");
    }

    #[test]
    fn hidden_entries_are_skipped() {
        assert!(parse_desktop("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n", None).is_none());
        assert!(parse_desktop("[Desktop Entry]\nName=X\nExec=x\nHidden=true\n", None).is_none());
        assert!(parse_desktop("[Desktop Entry]\nName=X\nExec=x\nTerminal=true\n", None).is_none());
        assert!(parse_desktop("[Desktop Entry]\nName=X\nType=Link\nURL=http://x\n", None).is_none());
        assert!(parse_desktop("[Desktop Entry]\nName=X\n", None).is_none());
    }

    #[test]
    fn exec_field_codes_are_removed() {
        assert_eq!(clean_exec("vlc --started-from-file %U"), vec!["vlc", "--started-from-file"]);
        assert_eq!(clean_exec("env FOO=1 app %f %F %i %c %k"), vec!["env", "FOO=1", "app"]);
        assert_eq!(clean_exec(r#"sh -c "echo 100%%""#), vec!["sh", "-c", "echo 100%"]);
        assert_eq!(clean_exec(r#""/opt/My App/run" --x "a \"b\"""#), vec!["/opt/My App/run", "--x", "a \"b\""]);
        assert_eq!(clean_exec("   "), Vec::<String>::new());
        assert_eq!(clean_exec("flatpak run --branch=stable org.x.App @@u %U @@"), vec!["flatpak", "run", "--branch=stable", "org.x.App", "@@u", "@@"]);
    }

    #[test]
    fn xdg_dirs_default_and_custom() {
        let env = |k: &str| match k {
            "HOME" => Some("/home/u".to_string()),
            _ => None,
        };
        let d = data_dirs(&env);
        assert_eq!(d[0], Path::new("/home/u/.local/share"));
        assert!(d.contains(&PathBuf::from("/usr/share")) && d.contains(&PathBuf::from("/usr/local/share")));
        assert!(d.contains(&PathBuf::from("/var/lib/flatpak/exports/share")));
        assert!(d.contains(&PathBuf::from("/var/lib/snapd/desktop")));
        let env2 = |k: &str| match k {
            "HOME" => Some("/home/u".to_string()),
            "XDG_DATA_HOME" => Some("/data".to_string()),
            "XDG_DATA_DIRS" => Some("/a:/usr/share:relative".to_string()),
            _ => None,
        };
        let d2 = data_dirs(&env2);
        assert_eq!(d2[0], Path::new("/data"));
        assert!(d2.contains(&PathBuf::from("/a")) && !d2.iter().any(|p| p.ends_with("relative")));
        // pas de doublon
        assert_eq!(d2.iter().filter(|p| **p == Path::new("/usr/share")).count(), 1);
    }

    #[test]
    fn launch_plans() {
        assert_eq!(plan("/usr/share/applications/a.desktop", true), Plan::Desktop("/usr/share/applications/a.desktop".into()));
        assert_eq!(plan("steam://open/bigpicture", false), Plan::Steam);
        assert_eq!(plan("https://example.org", false), Plan::Open("https://example.org".into()));
        assert_eq!(plan("/home/u/Videos", true), Plan::Open("/home/u/Videos".into()));
        assert_eq!(plan("vlc", false), Plan::Program("vlc".into()));
    }

    #[test]
    fn custom_apps_come_first() {
        let l = list(&[CustomApp { name: "Mon appli".into(), target: "https://x.y".into() }]);
        assert_eq!(l[0].id, "c:0");
        assert_eq!(l[0].name, "Mon appli");
    }
}
