//! Linux : icônes des applications (fichiers `.desktop`) en PNG base64.
//!
//! Recherche de `Icon=` : chemin absolu, puis thèmes d'icônes (`hicolor` d'abord) dans les dossiers `icons/` de
//! `XDG_DATA_HOME`, `XDG_DATA_DIRS`, flatpak, snap et `~/.icons`, puis `/usr/share/pixmaps`. PNG uniquement
//! (les SVG sont ignorés : aucune bibliothèque de rastérisation embarquée).
use base64::Engine;
use std::path::{Path, PathBuf};

pub fn com_init() {}

/// Tailles préférées (px) : proches de l'affichage d'une tuile sur le téléphone, sans fichier énorme.
const SIZES: &[u32] = &[64, 48, 96, 128, 72, 256, 32, 512];
/// Taille maximale d'un fichier d'icône envoyé au téléphone.
const MAX_BYTES: u64 = 150 * 1024;
const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G'];

/// Chemins candidats pour un nom d'icône (sans extension), du plus pertinent au moins pertinent.
pub fn icon_candidates(name: &str, icon_dirs: &[PathBuf], themes: &[String]) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for size in SIZES {
        for dir in icon_dirs {
            for theme in themes {
                v.push(dir.join(theme).join(format!("{size}x{size}")).join("apps").join(format!("{name}.png")));
            }
        }
    }
    for dir in icon_dirs {
        for theme in themes {
            v.push(dir.join(theme).join("48x48").join("apps").join(format!("{name}.png")));
        }
    }
    v
}

/// Dossiers `icons/` et `pixmaps/` selon l'environnement.
pub fn icon_dirs(env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = super::apps::data_dirs(env).into_iter().map(|d| d.join("icons")).collect();
    if let Some(h) = env("HOME").filter(|h| !h.is_empty()) {
        v.push(PathBuf::from(&h).join(".icons"));
        v.push(PathBuf::from(h).join(".local/share/flatpak/exports/share/icons"));
    }
    v.push(PathBuf::from("/var/lib/flatpak/exports/share/icons"));
    v.push(PathBuf::from("/var/lib/snapd/desktop/icons"));
    v
}

/// Thèmes à parcourir : `hicolor`, thèmes courants puis tous ceux trouvés (borné).
fn themes(dirs: &[PathBuf]) -> Vec<String> {
    let mut t: Vec<String> = ["hicolor", "Adwaita", "breeze", "Papirus", "gnome", "elementary", "Yaru"].iter().map(|s| s.to_string()).collect();
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten().take(40) {
                let n = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && !t.contains(&n) {
                    t.push(n);
                }
            }
        }
    }
    t
}

fn read_png(p: &Path) -> Option<Vec<u8>> {
    let md = std::fs::metadata(p).ok()?;
    if !md.is_file() || md.len() == 0 || md.len() > MAX_BYTES {
        return None;
    }
    let b = std::fs::read(p).ok()?;
    b.starts_with(PNG_MAGIC).then_some(b)
}

/// Résout un champ `Icon=` en octets PNG.
fn resolve(icon: &str) -> Option<Vec<u8>> {
    let icon = icon.trim();
    if icon.is_empty() {
        return None;
    }
    if icon.starts_with('/') {
        return read_png(Path::new(icon));
    }
    if icon.contains('/') || icon.contains("..") {
        return None;
    }
    let env = |k: &str| std::env::var(k).ok();
    let dirs = icon_dirs(&env);
    let themes = themes(&dirs);
    for p in icon_candidates(icon, &dirs, &themes) {
        if let Some(b) = read_png(&p) {
            return Some(b);
        }
    }
    read_png(&Path::new("/usr/share/pixmaps").join(format!("{icon}.png")))
}

/// `target` : chemin du fichier `.desktop` (voir `apps::list`).
pub fn icon_png_base64(target: &str) -> Option<String> {
    if !target.ends_with(".desktop") {
        return None;
    }
    let entry = super::apps::parse_desktop(&std::fs::read_to_string(target).ok()?, None)?;
    let bytes = resolve(&entry.icon?)?;
    Some(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_prefer_hicolor_and_64px() {
        let dirs = vec![PathBuf::from("/usr/share/icons")];
        let c = icon_candidates("firefox", &dirs, &["hicolor".into(), "Adwaita".into()]);
        assert_eq!(c[0], Path::new("/usr/share/icons/hicolor/64x64/apps/firefox.png"));
        assert_eq!(c[1], Path::new("/usr/share/icons/Adwaita/64x64/apps/firefox.png"));
        assert!(c.iter().any(|p| p.ends_with("48x48/apps/firefox.png")));
    }

    #[test]
    fn rejects_traversal_and_non_desktop() {
        assert!(resolve("../../etc/passwd").is_none());
        assert!(icon_png_base64("/etc/hostname").is_none());
        assert!(resolve("").is_none());
    }

    #[test]
    fn png_magic_is_required() {
        let d = std::env::temp_dir().join(format!("rustv-icon-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let ok = d.join("a.png");
        let bad = d.join("b.png");
        std::fs::write(&ok, [0x89, b'P', b'N', b'G', 1, 2]).unwrap();
        std::fs::write(&bad, b"GIF89a").unwrap();
        assert!(read_png(&ok).is_some());
        assert!(read_png(&bad).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
