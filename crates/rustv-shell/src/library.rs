//! Médiathèque locale : films, séries, photos et musique, d'après les dossiers choisis par l'utilisateur.
//! Aucun accès réseau : titres déduits des noms de fichiers, jaquettes locales (poster.jpg, folder.jpg…).
use crate::fs::{AUDIO, IMAGE, VIDEO};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 30_000;
const MAX_DEPTH: usize = 8;
const ART: &[&str] = &["poster", "folder", "cover", "front", "thumb"];
const TAGS: &[&str] = &[
    "480p", "576p", "720p", "1080p", "2160p", "4k", "uhd", "bluray", "blu-ray", "bdrip", "brrip", "webrip", "web-dl", "webdl", "hdtv", "dvdrip", "x264", "x265", "h264", "h265", "hevc", "xvid", "multi", "vff", "vfq", "vostfr", "truefrench", "french", "remux", "hdr", "10bit",
];

fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default()
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            walk(&p, depth + 1, out);
        } else {
            out.push(p);
        }
        if out.len() >= MAX_FILES {
            return;
        }
    }
}

/// « Mon.Film.2019.1080p.BluRay.x264 » → ("Mon Film", Some(2019)).
fn clean(stem: &str) -> (String, Option<u32>) {
    let s: String = stem.chars().map(|c| if c == '.' || c == '_' { ' ' } else { c }).collect();
    let mut year = None;
    let mut words: Vec<&str> = Vec::new();
    for w in s.split_whitespace() {
        let lw = w.to_lowercase();
        let bare = lw.trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']');
        if bare.len() == 4 && bare.chars().all(|c| c.is_ascii_digit()) {
            let y: u32 = bare.parse().unwrap_or(0);
            if (1900..=2100).contains(&y) && !words.is_empty() {
                year = Some(y);
                break;
            }
        }
        if TAGS.contains(&bare) || (w.starts_with('[') && w.ends_with(']')) {
            if !words.is_empty() {
                break;
            }
            continue;
        }
        words.push(w);
    }
    let t = words.join(" ");
    (if t.is_empty() { stem.to_string() } else { t }, year)
}

/// Cherche « SxxExx » ou « 1x02 » : renvoie (saison, épisode, position du motif).
fn episode(stem: &str) -> Option<(u32, u32, usize)> {
    let lower: Vec<char> = stem.to_lowercase().chars().collect();
    for i in 0..lower.len() {
        if lower[i] == 's' && i + 1 < lower.len() && lower[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            let mut s = String::new();
            while j < lower.len() && lower[j].is_ascii_digit() && s.len() < 2 {
                s.push(lower[j]);
                j += 1;
            }
            if j < lower.len() && lower[j] == 'e' {
                let mut k = j + 1;
                let mut e = String::new();
                while k < lower.len() && lower[k].is_ascii_digit() && e.len() < 3 {
                    e.push(lower[k]);
                    k += 1;
                }
                if !e.is_empty() {
                    return Some((s.parse().ok()?, e.parse().ok()?, i));
                }
            }
        }
    }
    for i in 1..lower.len().saturating_sub(2) {
        if lower[i] == 'x' && lower[i - 1].is_ascii_digit() && lower[i + 1].is_ascii_digit() && lower[i + 2].is_ascii_digit() {
            let s = lower[i - 1].to_digit(10)?;
            let e = (lower[i + 1].to_digit(10)? * 10 + lower[i + 2].to_digit(10)?) as u32;
            return Some((s, e, i - 1));
        }
    }
    None
}

fn art_in(dir: &Path, stem: Option<&str>) -> Option<String> {
    let mut names: Vec<String> = ART.iter().map(|s| s.to_string()).collect();
    if let Some(s) = stem {
        names.insert(0, s.to_string());
        names.insert(1, format!("{s}-poster"));
    }
    for n in names {
        for e in ["jpg", "jpeg", "png", "webp"] {
            let p = dir.join(format!("{n}.{e}"));
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn scan(movies: &[String], series: &[String], photos: &[String], music: &[String]) -> Value {
    // ---- films ----
    let mut films = vec![];
    for root in movies {
        let mut files = vec![];
        walk(Path::new(root), 0, &mut files);
        for f in files.iter().filter(|f| VIDEO.contains(&ext(f).as_str())) {
            let stem = f.file_stem().and_then(|x| x.to_str()).unwrap_or("");
            let (title, year) = clean(stem);
            let poster = f.parent().and_then(|d| art_in(d, Some(stem)));
            films.push(json!({ "title": title, "year": year, "path": s(f), "poster": poster }));
        }
    }
    films.sort_by_key(|f| f["title"].as_str().unwrap_or("").to_lowercase());

    // ---- séries ----
    let mut shows: BTreeMap<String, (Option<String>, BTreeMap<u32, Vec<(u32, String, String)>>)> = BTreeMap::new();
    for root in series {
        let rootp = Path::new(root);
        let mut files = vec![];
        walk(rootp, 0, &mut files);
        for f in files.iter().filter(|f| VIDEO.contains(&ext(f).as_str())) {
            let stem = f.file_stem().and_then(|x| x.to_str()).unwrap_or("");
            let (season, ep, pos) = episode(stem).unwrap_or((1, 0, 0));
            let mut show = clean(&stem.chars().take(pos).collect::<String>()).0;
            // Nom de série : le titre avant « SxxExx », sinon le dossier de premier niveau sous la racine.
            let first_dir = f.strip_prefix(rootp).ok().and_then(|r| r.components().next()).map(|c| c.as_os_str().to_string_lossy().into_owned()).filter(|d| !d.eq_ignore_ascii_case(f.file_name().and_then(|n| n.to_str()).unwrap_or("")));
            if show.trim().is_empty() || show == stem {
                show = clean(&first_dir.unwrap_or_else(|| "Séries".into())).0;
            }
            let entry = shows.entry(show.clone()).or_insert((None, BTreeMap::new()));
            if entry.0.is_none() {
                entry.0 = f.parent().and_then(|d| art_in(d, None)).or_else(|| f.parent().and_then(|d| d.parent()).and_then(|d| art_in(d, None)));
            }
            let title = if ep > 0 { format!("S{season:02}E{ep:02} · {}", clean(stem).0) } else { clean(stem).0 };
            entry.1.entry(season).or_default().push((ep, title, s(f)));
        }
    }
    let series_json: Vec<Value> = shows
        .into_iter()
        .map(|(name, (poster, seasons))| {
            let seasons: Vec<Value> = seasons
                .into_iter()
                .map(|(n, mut eps)| {
                    eps.sort_by_key(|e| (e.0, e.1.clone()));
                    json!({ "season": n, "episodes": eps.into_iter().map(|(ep, title, path)| json!({ "ep": ep, "title": title, "path": path })).collect::<Vec<_>>() })
                })
                .collect();
            json!({ "show": name, "poster": poster, "seasons": seasons })
        })
        .collect();

    // ---- photos : un album par dossier ----
    let mut albums: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for root in photos {
        let mut files = vec![];
        walk(Path::new(root), 0, &mut files);
        for f in files.iter().filter(|f| IMAGE.contains(&ext(f).as_str())) {
            let dir = f.parent().map(s).unwrap_or_default();
            albums.entry(dir).or_default().push(s(f));
        }
    }
    let photos_json: Vec<Value> = albums
        .into_iter()
        .map(|(dir, files)| {
            let name = Path::new(&dir).file_name().and_then(|n| n.to_str()).unwrap_or(&dir).to_string();
            json!({ "album": name, "count": files.len(), "cover": files.first(), "photos": files })
        })
        .collect();

    // ---- musique : Artiste/Album/piste ----
    let mut artists: BTreeMap<String, BTreeMap<String, Vec<(String, String)>>> = BTreeMap::new();
    for root in music {
        let rootp = Path::new(root);
        let mut files = vec![];
        walk(rootp, 0, &mut files);
        for f in files.iter().filter(|f| AUDIO.contains(&ext(f).as_str())) {
            let rel: Vec<String> = f.strip_prefix(rootp).map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            let (artist, album) = match rel.len() {
                0 | 1 => ("—".to_string(), "Singles".to_string()),
                2 => ("—".to_string(), rel[0].clone()),
                _ => (rel[0].clone(), rel[1].clone()),
            };
            let title = f.file_stem().and_then(|x| x.to_str()).unwrap_or("").replace('_', " ");
            artists.entry(artist).or_default().entry(album).or_default().push((title, s(f)));
        }
    }
    let music_json: Vec<Value> = artists
        .into_iter()
        .map(|(artist, albums)| {
            json!({ "artist": artist, "albums": albums.into_iter().map(|(album, tracks)| json!({ "album": album, "tracks": tracks.into_iter().map(|(title, path)| json!({ "title": title, "path": path })).collect::<Vec<_>>() })).collect::<Vec<_>>() })
        })
        .collect();

    json!({ "movies": films, "series": series_json, "photos": photos_json, "music": music_json })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaning() {
        assert_eq!(clean("Mon.Film.2019.1080p.BluRay.x264"), ("Mon Film".into(), Some(2019)));
        assert_eq!(clean("Some_Movie_[1080p]").0, "Some Movie");
        assert_eq!(clean("2001").0, "2001");
    }

    #[test]
    fn episodes() {
        assert_eq!(episode("Show.Name.S02E05.720p"), Some((2, 5, 10)));
        assert_eq!(episode("Show 1x07").map(|e| (e.0, e.1)), Some((1, 7)));
        assert!(episode("Just a movie").is_none());
    }
}
