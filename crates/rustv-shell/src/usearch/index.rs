//! Index des noms de fichiers des dossiers choisis (disques réseau, UNC, dossiers de la médiathèque).
//! Parcours en arrière-plan, borné (profondeur, nombre de fichiers, durée), annulable, sans suivre les
//! liens symboliques ; chaque lecture de dossier a un délai : un disque réseau absent ne bloque rien.
//! L'index est écrit dans un fichier compact à nous (%APPDATA%\RusTV\search\index.tsv, écriture atomique).
//! Seuls les noms et chemins sont gardés : rien ne quitte le PC.
use super::text::{clean_title, fold, score};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

pub const PHOTO: &[&str] = &["jpg", "jpeg", "png", "webp", "heic", "bmp", "tif", "tiff"];
pub const MAX_TOTAL: usize = 100_000;
pub const MAX_ROOT: usize = 60_000;
pub const MAX_DEPTH: usize = 12;
const MAX_PER_DIR: usize = 20_000;
const MAX_FILE_BYTES: usize = 40 * 1024 * 1024;
/// Un dossier racine ne sera pas reparcouru automatiquement plus d'une fois par heure.
pub const AUTO_REFRESH_SECS: u64 = 3600;

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_depth: usize,
    pub max_files: usize,
    /// Délai accordé à la lecture d'un dossier (un disque réseau mort ne répond jamais).
    pub dir_budget: Duration,
    /// Durée maximale d'un parcours de racine.
    pub total_budget: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_depth: MAX_DEPTH,
            max_files: MAX_ROOT,
            dir_budget: Duration::from_secs(4),
            total_budget: Duration::from_secs(600),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    /// Une limite (nombre de fichiers, durée) a été atteinte : index partiel.
    Truncated,
    /// Racine introuvable ou ne répondant pas.
    Unreachable,
    Cancelled,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Truncated => "truncated",
            Status::Unreachable => "unreachable",
            Status::Cancelled => "cancelled",
        }
    }
    fn parse(s: &str) -> Status {
        match s {
            "ok" => Status::Ok,
            "truncated" => Status::Truncated,
            "cancelled" => Status::Cancelled,
            _ => Status::Unreachable,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    /// 'v' vidéo, 'a' audio, 'p' photo.
    pub kind: char,
    /// « titre nettoyé | nom du dossier parent » passés dans `fold`.
    key: String,
    root: u16,
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

fn parent_name(path: &str) -> &str {
    let mut it = path.rsplit(['\\', '/']);
    it.next();
    it.next().unwrap_or("")
}

impl Entry {
    pub fn new(path: String, kind: char) -> Entry {
        let key = format!(
            "{}|{}",
            fold(&clean_title(file_stem(&path)).0),
            fold(parent_name(&path))
        );
        Entry {
            path,
            kind,
            key,
            root: 0,
        }
    }
    /// Titre affichable et année déduits du nom de fichier.
    pub fn title(&self) -> (String, Option<u32>) {
        clean_title(file_stem(&self.path))
    }
    pub fn folder(&self) -> &str {
        match self.path.rfind(['\\', '/']) {
            Some(i) => &self.path[..i],
            None => "",
        }
    }
}

// ---------- liste d'un dossier avec délai ----------

#[derive(Debug, PartialEq, Eq)]
pub enum ListErr {
    Timeout,
    Io,
}

/// Exécute `f` sur un thread dédié et abandonne l'attente après `budget` : le thread bloqué (lecture sur un
/// disque réseau mort) finira tout seul quand Windows rendra la main, sans geler le parcours ni l'interface.
pub fn run_with_timeout<R: Send + 'static>(
    budget: Duration,
    f: impl FnOnce() -> R + Send + 'static,
) -> Result<R, ListErr> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("usearch-io".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .map_err(|_| ListErr::Io)?;
    rx.recv_timeout(budget).map_err(|_| ListErr::Timeout)
}

/// (nom, est un dossier) ; les liens symboliques et points de jonction sont ignorés (pas de boucle possible).
fn read_entries(dir: &Path) -> Result<Vec<(String, bool)>, ()> {
    let rd = std::fs::read_dir(dir).map_err(|_| ())?;
    let mut out = vec![];
    for e in rd.flatten().take(MAX_PER_DIR) {
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let Some(name) = e.file_name().to_str().map(String::from) else {
            continue;
        };
        out.push((name, ft.is_dir()));
    }
    Ok(out)
}

fn skip_name(n: &str) -> bool {
    n.starts_with('.')
        || n.starts_with('$')
        || n.starts_with('@')
        || n.eq_ignore_ascii_case("System Volume Information")
        || n.eq_ignore_ascii_case("#recycle")
        || n.eq_ignore_ascii_case("lost+found")
}

fn kind_of(name: &str) -> Option<char> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    if crate::fs::VIDEO.contains(&ext.as_str()) {
        Some('v')
    } else if crate::fs::AUDIO.contains(&ext.as_str()) {
        Some('a')
    } else if PHOTO.contains(&ext.as_str()) {
        Some('p')
    } else {
        None
    }
}

pub struct Walk {
    pub entries: Vec<Entry>,
    pub status: Status,
}

/// Parcourt `root` (profondeur et nombre de fichiers bornés). `progress` reçoit le nombre de fichiers trouvés.
pub fn walk_root(root: &Path, lim: &Limits, cancel: &AtomicBool, progress: &mut dyn FnMut(usize)) -> Walk {
    let started = Instant::now();
    let mut entries: Vec<Entry> = vec![];
    let mut status = Status::Ok;
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    let mut first = true;
    'outer: while let Some((dir, depth)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            status = Status::Cancelled;
            break;
        }
        if started.elapsed() >= lim.total_budget {
            status = Status::Truncated;
            break;
        }
        let d = dir.clone();
        let list = match run_with_timeout(lim.dir_budget, move || read_entries(&d)) {
            Ok(Ok(l)) => l,
            Ok(Err(())) => {
                if first {
                    status = Status::Unreachable;
                    break;
                }
                continue; // dossier illisible (droits…) : on passe
            }
            Err(_) => {
                // Pas de réponse dans le délai : on abandonne la racine plutôt que d'enchaîner les attentes.
                status = Status::Unreachable;
                break;
            }
        };
        first = false;
        for (name, is_dir) in list {
            if skip_name(&name) {
                continue;
            }
            if is_dir {
                if depth < lim.max_depth {
                    stack.push((dir.join(&name), depth + 1));
                }
            } else if let Some(k) = kind_of(&name) {
                let p = dir.join(&name);
                let Some(ps) = p.to_str().filter(|s| !s.chars().any(|c| c.is_control())) else {
                    continue;
                };
                if entries.len() >= lim.max_files {
                    status = Status::Truncated;
                    break 'outer;
                }
                entries.push(Entry::new(ps.to_string(), k));
                if entries.len() % 500 == 0 {
                    progress(entries.len());
                }
            }
        }
    }
    progress(entries.len());
    Walk { entries, status }
}

// ---------- index persistant ----------

#[derive(Clone, Debug)]
pub struct RootInfo {
    pub path: String,
    pub status: Status,
    pub count: usize,
    /// Secondes Unix du dernier parcours (réussi ou non).
    pub scanned: u64,
}

#[derive(Default, Clone, Debug)]
pub struct Index {
    pub roots: Vec<RootInfo>,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub path: String,
    pub kind: char,
    pub score: u32,
}

const HEADER: &str = "RUSTV-SEARCH-INDEX 1";

impl Index {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn info(&self, root: &str) -> Option<&RootInfo> {
        self.roots.iter().find(|r| r.path.eq_ignore_ascii_case(root))
    }

    /// Remplace tout le contenu d'une racine (la plus récente gagne).
    pub fn replace_root(&mut self, root: &str, entries: Vec<Entry>, status: Status, now: u64) {
        let idx = match self.roots.iter().position(|r| r.path.eq_ignore_ascii_case(root)) {
            Some(i) => i,
            None => {
                self.roots.push(RootInfo {
                    path: root.to_string(),
                    status,
                    count: 0,
                    scanned: now,
                });
                self.roots.len() - 1
            }
        };
        self.entries.retain(|e| e.root as usize != idx);
        let room = MAX_TOTAL.saturating_sub(self.entries.len());
        let mut n = 0;
        for mut e in entries.into_iter().take(room) {
            e.root = idx as u16;
            self.entries.push(e);
            n += 1;
        }
        self.roots[idx] = RootInfo {
            path: root.to_string(),
            status,
            count: n,
            scanned: now,
        };
    }

    /// Racine injoignable : on garde l'ancien contenu (un NAS éteint ne vide pas l'index) et on note l'échec.
    pub fn mark_failed(&mut self, root: &str, status: Status, now: u64) {
        match self.roots.iter_mut().find(|r| r.path.eq_ignore_ascii_case(root)) {
            Some(r) => {
                r.status = status;
                r.scanned = now;
            }
            None => self.roots.push(RootInfo {
                path: root.to_string(),
                status,
                count: 0,
                scanned: now,
            }),
        }
    }

    /// Retire les racines qui ne sont plus demandées.
    pub fn retain_roots(&mut self, wanted: &[String]) {
        let keep: Vec<bool> = self
            .roots
            .iter()
            .map(|r| wanted.iter().any(|w| w.eq_ignore_ascii_case(&r.path)))
            .collect();
        if keep.iter().all(|k| *k) {
            return;
        }
        let mut remap = vec![u16::MAX; self.roots.len()];
        let mut next = 0u16;
        for (i, k) in keep.iter().enumerate() {
            if *k {
                remap[i] = next;
                next += 1;
            }
        }
        self.entries
            .retain(|e| keep.get(e.root as usize).copied().unwrap_or(false));
        for e in self.entries.iter_mut() {
            e.root = remap[e.root as usize];
        }
        let mut i = 0;
        self.roots.retain(|_| {
            i += 1;
            keep[i - 1]
        });
    }

    /// Meilleures correspondances pour `q` (déjà passé dans `fold`) : le nom du fichier compte plus que celui du dossier.
    pub fn search(&self, q: &str, limit: usize) -> Vec<Hit> {
        if q.chars().count() < 2 {
            return vec![];
        }
        let mut hits: Vec<Hit> = vec![];
        for e in &self.entries {
            let (name, dir) = e.key.split_once('|').unwrap_or((e.key.as_str(), ""));
            let s = score(q, name).or_else(|| score(q, dir).map(|s| s / 2));
            if let Some(s) = s {
                hits.push(Hit {
                    path: e.path.clone(),
                    kind: e.kind,
                    score: s,
                });
                if hits.len() > 4000 {
                    hits.sort_by(|a, b| b.score.cmp(&a.score));
                    hits.truncate(limit.max(200));
                }
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        hits.truncate(limit);
        hits
    }

    pub fn to_text(&self) -> String {
        let mut s = String::with_capacity(self.entries.len() * 80 + 256);
        s.push_str(HEADER);
        s.push('\n');
        for (i, r) in self.roots.iter().enumerate() {
            s.push_str(&format!(
                "#root\t{}\t{}\t{}\t{}\n",
                r.path,
                r.scanned,
                r.status.as_str(),
                r.count
            ));
            for e in self.entries.iter().filter(|e| e.root as usize == i) {
                s.push(e.kind);
                s.push('\t');
                s.push_str(&e.path);
                s.push('\n');
            }
        }
        s
    }

    pub fn from_text(text: &str) -> Index {
        let mut idx = Index::default();
        let mut lines = text.lines();
        if lines.next().map(|l| l.trim_start_matches('\u{feff}')) != Some(HEADER) {
            return idx;
        }
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            if f.first() == Some(&"#root") && f.len() == 5 {
                if idx.roots.len() >= u16::MAX as usize - 1 {
                    break;
                }
                idx.roots.push(RootInfo {
                    path: f[1].to_string(),
                    scanned: f[2].parse().unwrap_or(0),
                    status: Status::parse(f[3]),
                    count: 0,
                });
            } else if f.len() == 2 && f[0].len() == 1 && !idx.roots.is_empty() && idx.entries.len() < MAX_TOTAL {
                let k = f[0].chars().next().unwrap_or('?');
                if matches!(k, 'v' | 'a' | 'p') {
                    let mut e = Entry::new(f[1].to_string(), k);
                    e.root = (idx.roots.len() - 1) as u16;
                    idx.roots.last_mut().map(|r| r.count += 1);
                    idx.entries.push(e);
                }
            }
        }
        idx
    }
}

// ---------- état global : index en mémoire + parcours en cours ----------

static INDEX: Mutex<Option<Index>> = Mutex::new(None);

fn index_file() -> PathBuf {
    super::dir().join("index.tsv")
}

pub fn with_index<R>(f: impl FnOnce(&mut Index) -> R) -> R {
    let mut g = INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let idx = g.get_or_insert_with(|| {
        std::fs::read_to_string(index_file())
            .map(|s| Index::from_text(&s))
            .unwrap_or_default()
    });
    f(idx)
}

fn save_index() {
    let text = with_index(|i| i.to_text());
    if text.len() <= MAX_FILE_BYTES {
        let _ = super::write_atomic(&index_file(), text.as_bytes());
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Faut-il reparcourir cette racine ? Jamais plus d'une fois par heure sans demande explicite.
pub fn due(info: Option<&RootInfo>, now: u64, force: bool) -> bool {
    force
        || info
            .map(|i| now.saturating_sub(i.scanned) >= AUTO_REFRESH_SECS)
            .unwrap_or(true)
}

#[derive(Default)]
pub struct Scanner {
    pub running: AtomicBool,
    pub cancel: AtomicBool,
    pub files: AtomicUsize,
    pub current: Mutex<String>,
}

pub static SCANNER: Scanner = Scanner {
    running: AtomicBool::new(false),
    cancel: AtomicBool::new(false),
    files: AtomicUsize::new(0),
    current: Mutex::new(String::new()),
};

/// Lance un parcours en arrière-plan. `false` si un parcours est déjà en cours.
/// `emit` reçoit `true` au début de chaque racine et `false` à la fin du tout (pour l'interface).
pub fn start_scan(roots: Vec<String>, force: bool, emit: Arc<dyn Fn(bool) + Send + Sync>) -> bool {
    if SCANNER.running.swap(true, Ordering::SeqCst) {
        return false;
    }
    SCANNER.cancel.store(false, Ordering::SeqCst);
    SCANNER.files.store(0, Ordering::SeqCst);
    let spawned = std::thread::Builder::new().name("usearch-scan".into()).spawn(move || {
        with_index(|i| i.retain_roots(&roots));
        for root in &roots {
            if SCANNER.cancel.load(Ordering::SeqCst) {
                break;
            }
            let now = now_secs();
            if !with_index(|i| due(i.info(root), now, force)) {
                continue;
            }
            *SCANNER.current.lock().unwrap_or_else(|e| e.into_inner()) = root.clone();
            emit(true);
            let walk = walk_root(Path::new(root), &Limits::default(), &SCANNER.cancel, &mut |n| {
                SCANNER.files.store(n, Ordering::Relaxed)
            });
            let now = now_secs();
            match walk.status {
                Status::Ok | Status::Truncated => with_index(|i| i.replace_root(root, walk.entries, walk.status, now)),
                Status::Unreachable => with_index(|i| i.mark_failed(root, Status::Unreachable, now)),
                Status::Cancelled => break,
            }
            save_index();
        }
        SCANNER.current.lock().unwrap_or_else(|e| e.into_inner()).clear();
        SCANNER.running.store(false, Ordering::SeqCst);
        emit(false);
    });
    if spawned.is_err() {
        SCANNER.running.store(false, Ordering::SeqCst);
        return false;
    }
    true
}

/// Dossiers uniques (ordre conservé, sans doublon à la casse près ni barre finale) à partir de plusieurs listes.
pub fn merge_roots(lists: &[&[String]]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = vec![];
    for l in lists {
        for r in l.iter() {
            let t = r.trim().trim_end_matches(['\\', '/']);
            // « D: » désigne le dossier courant du lecteur : on veut la racine « D:\ ».
            let t = if t.len() == 2 && t.ends_with(':') {
                format!("{t}\\")
            } else {
                t.to_string()
            };
            if t.is_empty() || t.len() > 400 || t.chars().any(|c| c.is_control()) {
                continue;
            }
            if seen.insert(t.to_lowercase()) {
                out.push(t);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier temporaire unique, supprimé à la fin.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let p = std::env::temp_dir().join(format!("rustv-usearch-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
        fn touch(&self, rel: &str) {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn walk(t: &Tmp, lim: &Limits) -> Walk {
        walk_root(&t.0, lim, &AtomicBool::new(false), &mut |_| {})
    }

    #[test]
    fn walker_filters_extensions_and_hidden() {
        let t = Tmp::new("ext");
        t.touch("Films/The.Movie.2019.1080p.BluRay.x264-GRP.mkv");
        t.touch("Films/notes.txt");
        t.touch("Films/cover.JPG");
        t.touch("Musique/Artiste/morceau.flac");
        t.touch(".cache/hidden.mp4");
        t.touch("$RECYCLE.BIN/x.mp4");
        t.touch("System Volume Information/y.mp4");
        let w = walk(&t, &Limits::default());
        assert_eq!(w.status, Status::Ok);
        let mut k: Vec<(char, String)> = w
            .entries
            .iter()
            .map(|e| (e.kind, e.path.rsplit(['\\', '/']).next().unwrap().to_string()))
            .collect();
        k.sort();
        assert_eq!(
            k,
            vec![
                ('a', "morceau.flac".to_string()),
                ('p', "cover.JPG".to_string()),
                ('v', "The.Movie.2019.1080p.BluRay.x264-GRP.mkv".to_string())
            ]
        );
        let movie = w.entries.iter().find(|e| e.kind == 'v').unwrap();
        assert_eq!(movie.title(), ("The Movie".to_string(), Some(2019)));
    }

    #[test]
    fn walker_depth_cap() {
        let t = Tmp::new("depth");
        t.touch("a.mp4");
        t.touch("d1/b.mp4");
        t.touch("d1/d2/c.mp4");
        t.touch("d1/d2/d3/d.mp4");
        let w = walk(
            &t,
            &Limits {
                max_depth: 2,
                ..Limits::default()
            },
        );
        let names: HashSet<String> = w
            .entries
            .iter()
            .map(|e| e.path.rsplit(['\\', '/']).next().unwrap().to_string())
            .collect();
        assert_eq!(
            names,
            ["a.mp4", "b.mp4", "c.mp4"].iter().map(|s| s.to_string()).collect()
        );
    }

    #[test]
    fn walker_count_cap() {
        let t = Tmp::new("count");
        for i in 0..20 {
            t.touch(&format!("d{}/f{i}.mp4", i % 3));
        }
        let w = walk(
            &t,
            &Limits {
                max_files: 7,
                ..Limits::default()
            },
        );
        assert_eq!(w.entries.len(), 7);
        assert_eq!(w.status, Status::Truncated);
    }

    #[test]
    fn walker_unreachable_and_cancel() {
        let t = Tmp::new("gone");
        let missing = t.0.join("nope");
        let w = walk_root(&missing, &Limits::default(), &AtomicBool::new(false), &mut |_| {});
        assert_eq!(w.status, Status::Unreachable);
        assert!(w.entries.is_empty());
        t.touch("a.mp4");
        let w = walk_root(&t.0, &Limits::default(), &AtomicBool::new(true), &mut |_| {});
        assert_eq!(w.status, Status::Cancelled);
    }

    #[test]
    fn walker_total_budget() {
        let t = Tmp::new("budget");
        t.touch("a/x.mp4");
        let w = walk(
            &t,
            &Limits {
                total_budget: Duration::ZERO,
                ..Limits::default()
            },
        );
        // Le budget est consulté avant chaque dossier : rien n'est lu, parcours partiel.
        assert_eq!(w.status, Status::Truncated);
    }

    #[test]
    fn walker_ignores_symlinks() {
        let t = Tmp::new("link");
        t.touch("real/a.mp4");
        // Lien vers le dossier parent : boucle infinie si on le suivait. Sous Windows la création peut être refusée.
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&t.0, t.0.join("real/loop")).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(&t.0, t.0.join("real/loop")).is_ok();
        #[cfg(not(any(unix, windows)))]
        let made = false;
        let w = walk(&t, &Limits::default());
        assert_eq!(w.entries.len(), 1, "le lien (cree : {made}) n'est pas suivi");
        assert_eq!(w.status, Status::Ok);
    }

    #[test]
    fn timeout_wrapper() {
        assert_eq!(run_with_timeout(Duration::from_millis(500), || 5).unwrap(), 5);
        let r = run_with_timeout(Duration::from_millis(30), || {
            std::thread::sleep(Duration::from_millis(400));
            1
        });
        assert_eq!(r, Err(ListErr::Timeout));
    }

    #[test]
    fn index_round_trip_and_search() {
        let mut idx = Index::default();
        idx.replace_root(
            "D:\\Films",
            vec![
                Entry::new("D:\\Films\\Amélie.2001.1080p.mkv".into(), 'v'),
                Entry::new("D:\\Films\\Le Fabuleux Destin d'Amélie Poulain\\video.mkv".into(), 'v'),
                Entry::new("D:\\Films\\Autre.mkv".into(), 'v'),
            ],
            Status::Ok,
            100,
        );
        idx.replace_root(
            "\\\\NAS\\Musique",
            vec![Entry::new(
                "\\\\NAS\\Musique\\Lens\\Amelie Lens - Hypnotized.flac".into(),
                'a',
            )],
            Status::Truncated,
            200,
        );
        let back = Index::from_text(&idx.to_text());
        assert_eq!(back.len(), 4);
        assert_eq!(back.roots.len(), 2);
        assert_eq!(back.roots[1].status, Status::Truncated);
        assert_eq!(back.roots[0].count, 3);
        let q = fold("AMELIE");
        let hits = back.search(&q, 10);
        assert_eq!(hits.len(), 3);
        assert!(
            hits[0].path.ends_with("Amélie.2001.1080p.mkv"),
            "titre exact avant dossier : {:?}",
            hits[0]
        );
        assert!(
            hits.iter().any(|h| h.path.contains("Poulain")),
            "correspond par le nom du dossier"
        );
        assert!(back.search("a", 10).is_empty());
        assert!(Index::from_text("garbage").len() == 0);
    }

    #[test]
    fn index_replace_retain_and_failure() {
        let mut idx = Index::default();
        idx.replace_root("D:\\A", vec![Entry::new("D:\\A\\one.mp4".into(), 'v')], Status::Ok, 1);
        idx.replace_root(
            "E:\\B",
            vec![
                Entry::new("E:\\B\\two.mp4".into(), 'v'),
                Entry::new("E:\\B\\three.mp4".into(), 'v'),
            ],
            Status::Ok,
            2,
        );
        idx.replace_root("d:\\a", vec![Entry::new("D:\\A\\uno.mp4".into(), 'v')], Status::Ok, 3);
        assert_eq!(idx.len(), 3);
        assert_eq!(idx.roots.len(), 2, "meme racine a la casse pres");
        idx.mark_failed("E:\\B", Status::Unreachable, 4);
        assert_eq!(idx.len(), 3, "un echec garde l'ancien contenu");
        assert_eq!(idx.info("E:\\B").unwrap().status, Status::Unreachable);
        idx.retain_roots(&["E:\\B".to_string()]);
        assert_eq!(idx.roots.len(), 1);
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.search("two", 5).len(), 1);
        // le plafond total est respecte
        let many: Vec<Entry> = (0..MAX_TOTAL + 10)
            .map(|i| Entry::new(format!("E:\\B\\f{i}.mp4"), 'v'))
            .collect();
        idx.replace_root("E:\\B", many, Status::Truncated, 5);
        assert_eq!(idx.len(), MAX_TOTAL);
    }

    #[test]
    fn refresh_policy() {
        let info = RootInfo {
            path: "D:\\".into(),
            status: Status::Ok,
            count: 1,
            scanned: 10_000,
        };
        assert!(due(None, 10_001, false));
        assert!(!due(Some(&info), 10_000 + AUTO_REFRESH_SECS - 1, false));
        assert!(due(Some(&info), 10_000 + AUTO_REFRESH_SECS, false));
        assert!(due(Some(&info), 10_001, true));
    }

    #[test]
    fn roots_merge() {
        let a = vec![
            "D:\\Films\\".to_string(),
            " \\\\NAS\\Video ".to_string(),
            "".to_string(),
        ];
        let b = vec!["d:\\films".to_string(), "E:".to_string()];
        assert_eq!(merge_roots(&[&a, &b]), vec!["D:\\Films", "\\\\NAS\\Video", "E:\\"]);
    }
}
