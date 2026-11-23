//! Navigation dans les fichiers pour le sélecteur de la TV (fonds d'écran, icônes, médiathèque).
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct Root {
    pub name: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub size: u64,
}

#[derive(Serialize)]
pub struct Listing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<Entry>,
}

pub const IMAGE: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "svg"];
pub const VIDEO: &[&str] = &["mp4", "m4v", "webm", "mkv", "avi", "mov", "ts", "m2ts", "wmv"];
pub const AUDIO: &[&str] = &["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus"];

pub fn roots() -> Vec<Root> {
    let mut v = Vec::new();
    if let Some(home) = rustv_core::platform::home_dir() {
        // `Videos` s'appelle `Movies` sous macOS.
        let videos = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
        for (label, dir) in [("Images", "Pictures"), ("Vidéos", videos), ("Musique", "Music"), ("Téléchargements", "Downloads"), ("Bureau", "Desktop"), ("Documents", "Documents")] {
            let p = home.join(dir);
            if p.is_dir() {
                v.push(Root { name: label.into(), path: p.to_string_lossy().into_owned() });
            }
        }
    }
    drive_roots(&mut v);
    v
}

/// Masque des lecteurs fourni par Windows : instantané (tester chaque lettre bloque sur un disque réseau absent).
#[cfg(windows)]
fn drive_roots(v: &mut Vec<Root>) {
    let mask = unsafe { GetLogicalDrives() };
    for i in 2..26u32 {
        if mask & (1 << i) != 0 {
            let l = (b'A' + i as u8) as char;
            v.push(Root { name: format!("Disque {l}:"), path: format!("{l}:\\") });
        }
    }
}

/// Hors Windows : la racine du système de fichiers (le backend pourra y ajouter les volumes montés :
/// /media, /run/media, /Volumes).
#[cfg(not(windows))]
fn drive_roots(v: &mut Vec<Root>) {
    // Linux : volumes montés visibles (/media, /run/media, /mnt) avant la racine.
    #[cfg(target_os = "linux")]
    for m in crate::linux::mounts::read() {
        v.push(Root { name: crate::linux::mounts::label(&m.point), path: m.point });
    }
    v.push(Root { name: "Système".into(), path: "/".into() });
}

#[cfg(windows)]
extern "system" {
    fn GetLogicalDrives() -> u32;
}

fn exts(filter: &str) -> Option<Vec<&'static str>> {
    match filter {
        "image" => Some(IMAGE.to_vec()),
        "video" => Some(VIDEO.to_vec()),
        "audio" => Some(AUDIO.to_vec()),
        "media" => Some([IMAGE, VIDEO, AUDIO].concat()),
        "folder" => Some(vec![]), // dossiers uniquement
        _ => None,                // tout
    }
}

pub fn list(path: &str, filter: &str) -> Result<Listing, String> {
    let p = PathBuf::from(path);
    let rd = std::fs::read_dir(&p).map_err(|e| format!("Dossier illisible : {e}"))?;
    let allowed = exts(filter);
    let mut entries: Vec<Entry> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name.starts_with('$') {
                return None;
            }
            let md = e.metadata().ok()?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if md.file_attributes() & 0x2 != 0 {
                    return None; // fichier caché
                }
            }
            let dir = md.is_dir();
            if !dir {
                match &allowed {
                    Some(list) => {
                        let ext = Path::new(&name).extension().and_then(|x| x.to_str()).map(|x| x.to_lowercase()).unwrap_or_default();
                        if !list.contains(&ext.as_str()) {
                            return None;
                        }
                    }
                    None => {}
                }
            }
            Some(Entry { name, path: e.path().to_string_lossy().into_owned(), dir, size: md.len() })
        })
        .collect();
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(Listing { path: path.to_string(), parent: p.parent().map(|x| x.to_string_lossy().into_owned()).filter(|s| !s.is_empty()), entries })
}

/// Images d'un dossier (pour les diaporamas), triées par nom.
pub fn images_in(path: &str, max: usize) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(path)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().and_then(|x| x.to_str()).map(|x| IMAGE.contains(&x.to_lowercase().as_str())).unwrap_or(false))
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|s| s.to_lowercase());
    v.truncate(max);
    v
}
