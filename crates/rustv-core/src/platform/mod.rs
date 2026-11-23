//! Couche plateforme : chemins, recherche de binaires et sélection des backends.
//!
//! Convention (voir docs/PORTING.md) :
//! - l'implémentation Windows reste dans le fichier d'origine (`input.rs`, `audio.rs`…) ;
//! - sur les autres OS, le module est remplacé par `platform/stub/<module>.rs` via
//!   `#[cfg_attr(not(windows), path = "platform/stub/<module>.rs")]` dans `lib.rs` ;
//! - les stubs ont exactement la même API publique ; ils renvoient `Err("unsupported_platform")`,
//!   `None`, `false` ou une valeur vide, jamais un faux succès.
//!
//! Ce fichier contient uniquement du code portable (testé sous Windows et Linux).
use std::path::{Path, PathBuf};

/// Erreur stable renvoyée par les stubs des plateformes non gérées.
pub const UNSUPPORTED: &str = "unsupported_platform";

/// Système ciblé à la compilation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

/// OS de la compilation courante (les autres OS Unix sont traités comme Linux).
pub const fn current_os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    }
}

/// Nom du dossier d'application dans les répertoires de config/données.
pub const APP_DIR: &str = "RusTV";

/// Absolu au sens XDG (commence par `/`) ou au sens de l'OS courant.
fn is_abs(p: &Path) -> bool {
    p.is_absolute() || p.to_string_lossy().starts_with('/')
}

fn nonempty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.is_empty())
}

/// Dossier personnel (`USERPROFILE` sous Windows, `HOME` ailleurs).
pub fn resolve_home(os: Os, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let key = if os == Os::Windows { "USERPROFILE" } else { "HOME" };
    nonempty(env(key)).map(PathBuf::from)
}

/// Dossier de configuration/état : `%APPDATA%\RusTV` (Windows, inchangé),
/// `$XDG_CONFIG_HOME/RusTV` ou `~/.config/RusTV` (Linux), `~/Library/Application Support/RusTV` (macOS).
/// À défaut de variable d'environnement : dossier temporaire (comportement historique).
pub fn resolve_config_dir(os: Os, env: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    let base = match os {
        Os::Windows => nonempty(env("APPDATA")).map(PathBuf::from),
        Os::Linux => nonempty(env("XDG_CONFIG_HOME"))
            .map(PathBuf::from)
            .filter(|p| is_abs(p))
            .or_else(|| resolve_home(os, env).map(|h| h.join(".config"))),
        Os::MacOs => resolve_home(os, env).map(|h| h.join("Library").join("Application Support")),
    };
    base.unwrap_or_else(std::env::temp_dir).join(APP_DIR)
}

/// Dossier des données volumineuses (profils, index…) : identique au dossier de config sous Windows et
/// macOS ; `$XDG_DATA_HOME/RusTV` ou `~/.local/share/RusTV` sous Linux. Pas encore utilisé par le shell
/// (tout vit dans `config_dir()` pour l'instant) : prévu pour l'étape Linux.
pub fn resolve_data_dir(os: Os, env: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    match os {
        Os::Linux => nonempty(env("XDG_DATA_HOME"))
            .map(PathBuf::from)
            .filter(|p| is_abs(p))
            .or_else(|| resolve_home(os, env).map(|h| h.join(".local").join("share")))
            .unwrap_or_else(std::env::temp_dir)
            .join(APP_DIR),
        _ => resolve_config_dir(os, env),
    }
}

/// Dossier vidéos de l'utilisateur : `~/Videos` (Windows, Linux ; `$XDG_VIDEOS_DIR` prioritaire sous Linux),
/// `~/Movies` (macOS). `None` sans dossier personnel.
pub fn resolve_video_dir(os: Os, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if os == Os::Linux {
        if let Some(p) = nonempty(env("XDG_VIDEOS_DIR")).map(PathBuf::from).filter(|p| is_abs(p)) {
            return Some(p);
        }
    }
    let home = resolve_home(os, env)?;
    Some(home.join(if os == Os::MacOs { "Movies" } else { "Videos" }))
}

fn real_env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

pub fn home_dir() -> Option<PathBuf> {
    resolve_home(current_os(), &real_env)
}
pub fn config_dir() -> PathBuf {
    resolve_config_dir(current_os(), &real_env)
}
#[allow(dead_code)]
pub fn data_dir() -> PathBuf {
    resolve_data_dir(current_os(), &real_env)
}
pub fn video_dir() -> Option<PathBuf> {
    resolve_video_dir(current_os(), &real_env)
}

/// Nom de fichier exécutable : ajoute `.exe` sous Windows uniquement.
pub fn exe_name(base: &str) -> String {
    if cfg!(windows) && !base.to_ascii_lowercase().ends_with(".exe") {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Cherche `name` (sans suffixe) dans la liste de dossiers donnée (équivalent de `which`).
/// Sous Windows le suffixe `.exe` est ajouté ; sous Unix le fichier doit porter un bit d'exécution.
pub fn which_in<I: IntoIterator<Item = PathBuf>>(name: &str, dirs: I) -> Option<PathBuf> {
    let file = exe_name(name);
    dirs.into_iter().map(|d| d.join(&file)).find(|p| p.is_file() && is_executable(p))
}

/// Cherche `name` dans le `PATH` puis dans `extra` (emplacements usuels), dans cet ordre.
pub fn which_with(name: &str, path_var: Option<std::ffi::OsString>, extra: &[PathBuf]) -> Option<PathBuf> {
    let from_path: Vec<PathBuf> = path_var.map(|p| std::env::split_paths(&p).filter(|d| !d.as_os_str().is_empty()).collect()).unwrap_or_default();
    which_in(name, from_path.into_iter().chain(extra.iter().cloned()))
}

/// `which` sur le vrai `PATH` plus les emplacements usuels de l'OS (`/usr/local/bin`, `/opt/homebrew/bin`…).
pub fn which(name: &str) -> Option<PathBuf> {
    which_with(name, std::env::var_os("PATH"), &default_bin_dirs())
}

/// Dossiers de binaires usuels hors `PATH` (vide sous Windows : l'appelant a ses propres chemins).
pub fn default_bin_dirs() -> Vec<PathBuf> {
    match current_os() {
        Os::Windows => vec![],
        Os::Linux => ["/usr/bin", "/usr/local/bin", "/snap/bin", "/var/lib/flatpak/exports/bin"].iter().map(PathBuf::from).collect(),
        Os::MacOs => ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/Applications"].iter().map(PathBuf::from).collect(),
    }
}

/// IPC local avec un processus enfant (mpv `--input-ipc-server`) : tube nommé sous Windows, socket Unix ailleurs.
pub mod ipc {
    use std::path::PathBuf;

    /// Flux bidirectionnel (`Read + Write`, clonable avec `try_clone`).
    #[cfg(windows)]
    pub type Stream = std::fs::File;
    #[cfg(unix)]
    pub type Stream = std::os::unix::net::UnixStream;

    /// Chemin/nom IPC à passer au processus enfant : `\.\pipe\<name>` (Windows) ou
    /// `$XDG_RUNTIME_DIR/<name>.sock` (Unix ; dossier temporaire à défaut).
    pub fn path(name: &str) -> String {
        path_with(cfg!(windows), name, std::env::var("XDG_RUNTIME_DIR").ok())
    }

    /// Version pure de `path` (testable).
    pub fn path_with(windows: bool, name: &str, runtime_dir: Option<String>) -> String {
        if windows {
            format!(r"\\.\pipe\{name}")
        } else {
            let dir = runtime_dir.filter(|d| d.starts_with('/')).map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
            dir.join(format!("{name}.sock")).to_string_lossy().into_owned()
        }
    }

    /// Ouvre le canal (échoue tant que le serveur n'est pas prêt : l'appelant réessaie).
    pub fn connect(path: &str) -> std::io::Result<Stream> {
        #[cfg(windows)]
        {
            std::fs::OpenOptions::new().read(true).write(true).open(path)
        }
        #[cfg(unix)]
        {
            std::os::unix::net::UnixStream::connect(path)
        }
    }
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
#[cfg(not(unix))]
fn is_executable(_: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn config_dir_windows_is_appdata() {
        let e = env_of(&[("APPDATA", "C:\\Users\\u\\AppData\\Roaming")]);
        assert_eq!(resolve_config_dir(Os::Windows, &e), PathBuf::from("C:\\Users\\u\\AppData\\Roaming").join("RusTV"));
    }

    #[test]
    fn config_dir_linux_prefers_xdg_then_home() {
        let e = env_of(&[("XDG_CONFIG_HOME", "/x/cfg"), ("HOME", "/home/u")]);
        assert_eq!(resolve_config_dir(Os::Linux, &e), PathBuf::from("/x/cfg").join("RusTV"));
        let e = env_of(&[("HOME", "/home/u")]);
        assert_eq!(resolve_config_dir(Os::Linux, &e), PathBuf::from("/home/u").join(".config").join("RusTV"));
        // XDG relatif : ignoré (spécification XDG).
        let e = env_of(&[("XDG_CONFIG_HOME", "rel"), ("HOME", "/home/u")]);
        assert_eq!(resolve_config_dir(Os::Linux, &e), PathBuf::from("/home/u").join(".config").join("RusTV"));
    }

    #[test]
    fn config_dir_macos_and_fallback() {
        let e = env_of(&[("HOME", "/Users/u")]);
        assert_eq!(resolve_config_dir(Os::MacOs, &e), PathBuf::from("/Users/u").join("Library").join("Application Support").join("RusTV"));
        let none = env_of(&[]);
        assert_eq!(resolve_config_dir(Os::Linux, &none), std::env::temp_dir().join("RusTV"));
        assert_eq!(resolve_config_dir(Os::Windows, &none), std::env::temp_dir().join("RusTV"));
    }

    #[test]
    fn data_dir_linux_uses_xdg_data() {
        let e = env_of(&[("XDG_DATA_HOME", "/d"), ("HOME", "/home/u")]);
        assert_eq!(resolve_data_dir(Os::Linux, &e), PathBuf::from("/d").join("RusTV"));
        let e = env_of(&[("HOME", "/home/u")]);
        assert_eq!(resolve_data_dir(Os::Linux, &e), PathBuf::from("/home/u").join(".local").join("share").join("RusTV"));
        let e = env_of(&[("HOME", "/Users/u")]);
        assert_eq!(resolve_data_dir(Os::MacOs, &e), resolve_config_dir(Os::MacOs, &e));
    }

    #[test]
    fn video_dir_per_os() {
        let e = env_of(&[("HOME", "/home/u"), ("XDG_VIDEOS_DIR", "/media/vid")]);
        assert_eq!(resolve_video_dir(Os::Linux, &e), Some(PathBuf::from("/media/vid")));
        let e = env_of(&[("HOME", "/home/u")]);
        assert_eq!(resolve_video_dir(Os::Linux, &e), Some(PathBuf::from("/home/u").join("Videos")));
        let e = env_of(&[("HOME", "/Users/u")]);
        assert_eq!(resolve_video_dir(Os::MacOs, &e), Some(PathBuf::from("/Users/u").join("Movies")));
        let e = env_of(&[("USERPROFILE", r"C:\Users\u")]);
        assert_eq!(resolve_video_dir(Os::Windows, &e), Some(PathBuf::from(r"C:\Users\u").join("Videos")));
        assert_eq!(resolve_video_dir(Os::Windows, &env_of(&[])), None);
    }

    #[test]
    fn ipc_paths() {
        assert_eq!(ipc::path_with(true, "rustv-mpv-1", None), r"\\.\pipe\rustv-mpv-1");
        let p = ipc::path_with(false, "rustv-mpv-1", Some("/run/user/1000".into()));
        assert_eq!(PathBuf::from(p), PathBuf::from("/run/user/1000").join("rustv-mpv-1.sock"));
        // Dossier d'exécution relatif ou absent : dossier temporaire.
        let q = ipc::path_with(false, "x", Some("rel".into()));
        assert_eq!(PathBuf::from(q), std::env::temp_dir().join("x.sock"));
    }

    #[test]
    fn exe_name_suffix() {
        if cfg!(windows) {
            assert_eq!(exe_name("mpv"), "mpv.exe");
            assert_eq!(exe_name("MPV.EXE"), "MPV.EXE");
        } else {
            assert_eq!(exe_name("mpv"), "mpv");
        }
    }

    #[test]
    fn which_finds_binary_on_temp_path() {
        let dir = std::env::temp_dir().join(format!("rustv-which-{}", std::process::id()));
        let other = dir.join("other");
        let bin = dir.join("bin");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let file = bin.join(exe_name("fakebin"));
        std::fs::write(&file, b"#!/bin/sh
").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
            // Fichier non exécutable : ignoré.
            std::fs::write(other.join("noexec"), b"x").unwrap();
        }
        let path = std::env::join_paths([other.clone(), bin.clone()]).unwrap();
        assert_eq!(which_with("fakebin", Some(path.clone()), &[]), Some(file.clone()));
        assert_eq!(which_with("absent", Some(path), &[]), None);
        // Emplacements supplémentaires quand PATH est vide.
        assert_eq!(which_with("fakebin", None, &[bin]), Some(file));
        #[cfg(unix)]
        assert_eq!(which_with("noexec", Some(std::env::join_paths([other]).unwrap()), &[]), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
