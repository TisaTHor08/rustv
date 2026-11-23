//! Lecteur vidéo/audio : mpv, piloté par son IPC JSON (tube nommé).
//!
//! RusTV ne lie pas libmpv : il lance `mpv.exe` en plein écran sur l'écran de la TV et dialogue avec lui
//! (zapping des chaînes, titre et état de lecture pour le téléphone). La télécommande envoie de vraies
//! touches, que mpv reçoit grâce à un `input.conf` généré ici.
//! mpv est détecté s'il est déjà installé ; sinon il est téléchargé (avec l'accord de l'utilisateur) depuis
//! les builds Windows référencés par mpv.io (shinchiro/mpv-winbuild-cmake), après vérification du SHA-256.
pub mod config;
pub mod history;
pub mod menu;
pub mod queue;
pub mod timeshift;

use crate::{settings, tv, Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const LATEST_API: &str = "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest";
const MAX_ARCHIVE: u64 = 120 * 1024 * 1024;

#[derive(Clone, Default, Serialize)]
pub struct PlayerState {
    pub active: bool,
    pub kind: String,
    pub title: String,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    /// Direct différé : secondes disponibles en arrière dans le cache, et retard sur le direct (0 = en direct).
    pub timeshift_available_secs: f64,
    pub behind_live_secs: f64,
    pub audio_tracks: Vec<menu::TrackInfo>,
    pub sub_tracks: Vec<menu::TrackInfo>,
    pub queue_index: usize,
    pub queue_len: usize,
}

#[derive(Deserialize, Clone)]
pub struct Item {
    pub name: String,
    pub url: String,
    /// Affiche (chemin ou URL) pour la rangée « Reprendre la lecture ».
    #[serde(default)]
    pub poster: Option<String>,
}

#[derive(Deserialize)]
pub struct PlayRequest {
    /// "live" (zapping haut/bas), "video" (avance/retour, reprise) ou "audio".
    pub kind: String,
    pub items: Vec<Item>,
    #[serde(default)]
    pub index: usize,
    /// Position de départ en secondes (reprise depuis l'historique).
    #[serde(default)]
    pub start: Option<f64>,
    /// Langue de l'interface ("fr" | "en"), pour les menus et les préférences de pistes.
    #[serde(default)]
    pub lang: Option<String>,
}

/// Dernières valeurs exactes reçues de mpv (non limitées par le filtre de 5 s de `PlayerState`).
#[derive(Default, Clone)]
struct Props {
    pos: f64,
    cache_end: Option<f64>,
    cache_start: Option<f64>,
    ended: bool,
}

#[derive(Default)]
pub struct Player {
    child: Mutex<Option<Child>>,
    pipe: Mutex<Option<rustv_core::platform::ipc::Stream>>,
    items: Mutex<Vec<Item>>,
    index: Mutex<usize>,
    pub state: Mutex<PlayerState>,
    props: Mutex<Props>,
    lang: Mutex<String>,
    hist: Mutex<Option<history::History>>,
    /// Incrémenté à chaque lecture et à chaque fermeture : invalide les fils de compte à rebours obsolètes.
    gen: AtomicU64,
    /// Génération des comptes à rebours (changer d'élément ou fermer les annule).
    cd: AtomicU64,
    skip_wait: AtomicBool,
    waiting: AtomicBool,
}

fn player_dir() -> PathBuf {
    settings::dir().join("player")
}

pub fn find_mpv() -> Option<PathBuf> {
    let exe = rustv_core::platform::exe_name("mpv");
    let mut cands: Vec<PathBuf> = vec![player_dir().join(&exe)];
    if let Some(path) = std::env::var_os("PATH") {
        cands.extend(std::env::split_paths(&path).map(|p| p.join(&exe)));
    }
    // Linux/macOS : emplacements usuels hors PATH (le bit d'exécution est vérifié par `which_in`).
    if !cfg!(windows) {
        cands.extend(rustv_core::platform::default_bin_dirs().into_iter().map(|d| d.join(&exe)));
        if cfg!(target_os = "macos") {
            cands.push(PathBuf::from("/Applications/mpv.app/Contents/MacOS/mpv"));
        }
    }
    for var in ["ProgramFiles", "LOCALAPPDATA"] {
        if let Some(b) = std::env::var_os(var) {
            let b = PathBuf::from(b);
            cands.push(b.join("mpv").join("mpv.exe"));
            cands.push(b.join("Programs").join("mpv").join("mpv.exe"));
        }
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        cands.push(PathBuf::from(h).join("scoop").join("apps").join("mpv").join("current").join("mpv.exe"));
    }
    cands.into_iter().find(|p| p.is_file())
}

// ---------- installation ----------

pub fn install(app: &AppHandle) -> Result<PathBuf, String> {
    // L'archive officielle est un build Windows : sous Linux/macOS, mpv s'installe avec le gestionnaire de paquets
    // (apt, dnf, flatpak, brew : voir docs/PORTING.md) ; on ne télécharge jamais un binaire d'un autre OS.
    if !cfg!(windows) {
        return Err(crate::features::install_hint("mpv"));
    }
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).user_agent("RusTV").build();
    let mut body = String::new();
    agent
        .get(LATEST_API)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub inaccessible : {e}"))?
        .into_reader()
        .take(2 * 1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    let rel: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let asset = rel["assets"]
        .as_array()
        .and_then(|a| {
            a.iter().find(|x| {
                let n = x["name"].as_str().unwrap_or("");
                n.starts_with("mpv-x86_64-") && n.ends_with(".7z") && !["v3", "dev", "debug", "ffmpeg"].iter().any(|k| n.contains(k))
            })
        })
        .ok_or("aucune archive mpv trouvée dans la dernière version")?;
    let url = asset["browser_download_url"].as_str().ok_or("URL manquante")?;
    let size = asset["size"].as_u64().unwrap_or(0);
    let digest = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).ok_or("GitHub ne fournit pas d'empreinte SHA-256 : installation refusée")?.to_lowercase();
    if size == 0 || size > MAX_ARCHIVE {
        return Err("taille d'archive inattendue".into());
    }

    let dir = player_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join("download.7z");
    {
        let resp = agent.get(url).call().map_err(|e| format!("téléchargement impossible : {e}"))?;
        let mut rd = resp.into_reader().take(MAX_ARCHIVE + 1);
        let mut out = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        let (mut done, mut last) = (0u64, 0u64);
        loop {
            let n = rd.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if done - last > 1024 * 1024 {
                last = done;
                let _ = app.emit("player-install", json!({ "pct": (done * 100 / size).min(99) }));
            }
        }
        let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != digest {
            let _ = std::fs::remove_file(&archive);
            return Err("l'empreinte SHA-256 du téléchargement ne correspond pas".into());
        }
    }

    let _ = app.emit("player-install", json!({ "pct": 99, "stage": "extract" }));
    let stage = dir.join("extract");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    sevenz_rust::decompress_file_with_extract_fn(&archive, &stage, |entry, reader, dest| {
        // Refuse les chemins qui sortiraient du dossier d'extraction.
        let rel = entry.name();
        if rel.contains("..") || rel.contains(':') || rel.starts_with('/') || rel.starts_with('\\') {
            return Ok(false);
        }
        sevenz_rust::default_entry_extract_fn(entry, reader, dest)
    })
    .map_err(|e| format!("extraction impossible : {e}"))?;
    let _ = std::fs::remove_file(&archive);

    let exe = stage.join("mpv.exe");
    if !exe.is_file() {
        return Err("mpv.exe introuvable dans l'archive".into());
    }
    // On remplace l'ancienne installation par la nouvelle.
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        if e.file_name() != "extract" && e.path().is_file() {
            let _ = std::fs::remove_file(e.path());
        }
    }
    for e in std::fs::read_dir(&stage).map_err(|e| e.to_string())?.flatten() {
        let _ = std::fs::rename(e.path(), dir.join(e.file_name()));
    }
    let _ = std::fs::remove_dir_all(&stage);
    let _ = app.emit("player-install", json!({ "pct": 100 }));
    find_mpv().ok_or_else(|| "installation incomplète".to_string())
}

// ---------- lecture ----------

/// Secondes d'attente avant l'épisode suivant.
const COUNTDOWN: u32 = 5;

/// Écrit `input.conf` et le script Lua du menu dans le dossier du lecteur.
fn write_support(kind: &str) -> (PathBuf, PathBuf) {
    let dir = player_dir();
    let _ = std::fs::create_dir_all(&dir);
    let conf = dir.join(format!("{kind}.conf"));
    let _ = std::fs::write(&conf, menu::input_conf(kind));
    let lua = dir.join("rustv-menu.lua");
    let _ = std::fs::write(&lua, menu::lua_script());
    (conf, lua)
}

/// Envoie une commande JSON brute à mpv (module cast : pause, reprise, déplacement).
pub fn ipc(app: &AppHandle, v: Value) {
    let p = app.state::<Shell>().player.clone();
    send(&p, v);
}

pub fn stop(app: &AppHandle) {
    let p = app.state::<Shell>().player.clone();
    let child = p.child.lock().unwrap().take();
    if let Some(mut c) = child {
        let _ = c.kill();
    }
}

fn with_hist<R>(player: &Player, f: impl FnOnce(&mut history::History) -> R) -> R {
    let mut g = player.hist.lock().unwrap();
    f(g.get_or_insert_with(|| crate::store::load::<history::History>("history")))
}

/// Enregistre la position de l'élément en cours dans l'historique (si c'est une vidéo ou un long audio).
fn commit(player: &Player, ended: bool) {
    let (kind, duration) = {
        let s = player.state.lock().unwrap();
        (s.kind.clone(), s.duration)
    };
    if !history::eligible(&kind, duration) {
        return;
    }
    let (pos, was_ended) = {
        let p = player.props.lock().unwrap();
        (p.pos, p.ended)
    };
    let idx = *player.index.lock().unwrap();
    let Some(it) = player.items.lock().unwrap().get(idx).cloned() else { return };
    let e = history::Entry { url: it.url, title: it.name, kind, position: pos, duration, updated: history::now_secs(), poster: it.poster, finished: false };
    with_hist(player, |h| {
        if h.record(e, ended || was_ended) {
            let _ = crate::store::save("history", &*h);
        }
    });
}

pub fn play(app: &AppHandle, req: PlayRequest) -> Result<(), String> {
    if req.items.is_empty() {
        return Err("rien à lire".into());
    }
    let mpv = find_mpv().ok_or("mpv_missing")?;
    let shell = app.state::<Shell>();
    let player = shell.player.clone();
    // l'élément qu'on quitte garde sa position dans l'historique
    commit(&player, false);
    stop(app);
    let cfg = config::load();
    let ui = match req.lang.as_deref() {
        Some("en") => "en",
        _ => "fr",
    }
    .to_string();
    let index = req.index.min(req.items.len() - 1);
    let first = req.items[index].clone();
    let start = if cfg.resume && req.kind == "video" { req.start.filter(|s| s.is_finite() && *s >= 1.0) } else { None };
    let queue_len = req.items.len();
    let idle = queue::idle_needed(&req.kind, queue_len, cfg.auto_next);
    *player.items.lock().unwrap() = req.items;
    *player.index.lock().unwrap() = index;
    *player.lang.lock().unwrap() = ui.clone();
    *player.props.lock().unwrap() = Props::default();
    player.skip_wait.store(false, Ordering::SeqCst);
    player.waiting.store(false, Ordering::SeqCst);
    player.cd.fetch_add(1, Ordering::SeqCst);
    let my_gen = player.gen.fetch_add(1, Ordering::SeqCst) + 1;

    let mons = tv::monitors(app);
    let tvmon = if shell.tv.lock().unwrap().active { shell.tv.lock().unwrap().monitor } else { 0 };
    let pipe = rustv_core::platform::ipc::path(&format!("rustv-mpv-{}", std::process::id()));
    let (conf, lua) = write_support(&req.kind);
    let wl = player_dir().join("watch_later");
    let _ = std::fs::create_dir_all(&wl);
    let mut cmd = Command::new(&mpv);
    cmd.args([
        "--no-terminal",
        "--force-window=immediate",
        "--fullscreen",
        "--osd-level=1",
        "--osd-duration=2500",
        "--osd-font-size=36",
        "--keep-open=no",
        "--hwdec=auto-safe",
        "--ytdl=no",
        "--title=RusTV player",
    ]);
    // idle=yes seulement pour enchaîner une file : on décide nous-mêmes de quitter à la fin.
    cmd.arg(if idle { "--idle=yes" } else { "--idle=no" });
    cmd.arg(format!("--input-ipc-server={pipe}")).arg(format!("--input-conf={}", conf.display()));
    cmd.arg(format!("--script={}", lua.display())).arg(format!("--script-opts=rustv-kind={},rustv-lang={}", req.kind, ui));
    if let Some(m) = mons.get(tvmon).filter(|m| !m.name.is_empty()) {
        cmd.arg(format!("--screen-name={}", m.name)).arg(format!("--fs-screen-name={}", m.name));
    }
    cmd.args(config::lang_args(&cfg, &ui)).args(config::kind_args(&req.kind, &cfg, &wl));
    if let Some(s) = start {
        cmd.arg(format!("--start={s:.1}"));
    }
    cmd.arg("--").arg(&first.url);
    let child = cmd.spawn().map_err(|e| format!("impossible de lancer mpv : {e}"))?;
    *player.child.lock().unwrap() = Some(child);
    *player.state.lock().unwrap() = PlayerState { active: true, kind: req.kind.clone(), title: first.name, queue_index: index, queue_len, ..Default::default() };
    shell.core.set_player_info(Some(info_of(&player.state.lock().unwrap())));

    // Dialogue avec mpv (IPC) puis retour à l'accueil quand il se ferme.
    let app2 = app.clone();
    let player2 = player.clone();
    std::thread::spawn(move || {
        ipc_loop(&app2, &player2, &pipe);
        if player2.gen.load(Ordering::SeqCst) != my_gen {
            return; // une autre lecture a pris le relais
        }
        commit(&player2, false);
        player2.cd.fetch_add(1, Ordering::SeqCst);
        // mpv s'est fermé (ou n'a jamais répondu) : on nettoie et on revient à l'accueil.
        if let Some(mut c) = player2.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        *player2.pipe.lock().unwrap() = None;
        *player2.state.lock().unwrap() = PlayerState::default();
        let shell = app2.state::<Shell>();
        shell.core.set_player_info(None);
        let _ = app2.emit("player-closed", ());
        if shell.tv.lock().unwrap().active {
            tv::go_home(&app2);
        }
    });
    Ok(())
}

fn info_of(s: &PlayerState) -> rustv_core::PlayerInfo {
    rustv_core::PlayerInfo { title: s.title.clone(), playing: !s.paused, position: s.position, duration: s.duration }
}

fn send(player: &Player, v: Value) -> bool {
    match player.pipe.lock().unwrap().as_mut() {
        Some(f) => writeln!(f, "{v}").is_ok(),
        None => false,
    }
}

fn show(player: &Player, text: &str, ms: u32) {
    send(player, json!({ "command": ["show-text", text, ms] }));
}

fn ipc_loop(app: &AppHandle, player: &Arc<Player>, pipe: &str) {
    // mpv met un instant à créer le tube : on réessaie quelques secondes.
    let mut file = None;
    for _ in 0..60 {
        if let Ok(f) = rustv_core::platform::ipc::connect(pipe) {
            file = Some(f);
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
        if player.child.lock().unwrap().as_mut().map(|c| c.try_wait().ok().flatten().is_some()).unwrap_or(true) {
            return;
        }
    }
    let Some(f) = file else {
        // pas d'IPC : on attend simplement la fin du processus
        loop {
            let done = player.child.lock().unwrap().as_mut().map(|c| c.try_wait().ok().flatten().is_some()).unwrap_or(true);
            if done {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    };
    let Ok(reader) = f.try_clone() else { return };
    *player.pipe.lock().unwrap() = Some(f);
    for (i, p) in ["media-title", "pause", "time-pos", "duration", "track-list", "demuxer-cache-time", "demuxer-cache-state"].iter().enumerate() {
        send(player, json!({ "command": ["observe_property", i + 1, p] }));
    }
    let mut last_commit = Instant::now();
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        match v["event"].as_str() {
            Some("property-change") => on_property(app, player, &v, &mut last_commit),
            Some("end-file") => {
                if v["reason"].as_str() == Some("eof") {
                    on_eof(player);
                }
            }
            Some("client-message") => {
                let a: Vec<&str> = v["args"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
                if a.len() >= 2 && a[0] == "rustv" {
                    match a[1] {
                        "zap" if a.len() == 3 => zap(player, a[2].parse().unwrap_or(1)),
                        "next" => step_item(player, 1),
                        "prev" => step_item(player, -1),
                        "skip-wait" => {
                            if player.waiting.load(Ordering::SeqCst) {
                                player.skip_wait.store(true, Ordering::SeqCst);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn on_property(app: &AppHandle, player: &Arc<Player>, v: &Value, last_commit: &mut Instant) {
    let shell = app.state::<Shell>();
    let name = v["name"].as_str().unwrap_or("");
    let mut changed = false;
    let mut ts = false;
    match name {
        "media-title" => {
            if let Some(t) = v["data"].as_str() {
                let mut s = player.state.lock().unwrap();
                // Pour la TV en direct, le nom de la chaîne vaut mieux que le titre du flux.
                if s.kind != "live" {
                    s.title = t.to_string();
                    changed = true;
                }
            }
        }
        "pause" => {
            let paused = v["data"].as_bool().unwrap_or(false);
            let (live, behind) = {
                let mut s = player.state.lock().unwrap();
                s.paused = paused;
                (s.kind == "live", s.behind_live_secs)
            };
            changed = true;
            if live {
                let lang = player.lang.lock().unwrap().clone();
                let word = if lang == "en" { "PAUSED" } else { "PAUSE" };
                let text = if paused { format!("{word}  {}", timeshift::badge(behind)) } else { timeshift::badge(behind) };
                show(player, &text, 2500);
            }
        }
        "time-pos" => {
            let p = v["data"].as_f64().unwrap_or(0.0);
            player.props.lock().unwrap().pos = p;
            {
                let mut s = player.state.lock().unwrap();
                if (p - s.position).abs() >= 5.0 {
                    s.position = p;
                    changed = true;
                }
            }
            ts = true;
            if last_commit.elapsed() >= Duration::from_secs(15) {
                *last_commit = Instant::now();
                commit(player, false);
            }
        }
        "duration" => player.state.lock().unwrap().duration = v["data"].as_f64().unwrap_or(0.0),
        "track-list" => {
            let (a, s) = menu::tracks_from(&v["data"]);
            let mut st = player.state.lock().unwrap();
            st.audio_tracks = a;
            st.sub_tracks = s;
        }
        "demuxer-cache-time" => {
            player.props.lock().unwrap().cache_end = v["data"].as_f64();
            ts = true;
        }
        "demuxer-cache-state" => {
            let ranges: Vec<(f64, f64)> = v["data"]["seekable-ranges"].as_array().map(|a| a.iter().filter_map(|r| Some((r["start"].as_f64()?, r["end"].as_f64()?))).collect()).unwrap_or_default();
            let mut p = player.props.lock().unwrap();
            p.cache_start = timeshift::seekable_start(&ranges, p.pos);
            ts = true;
        }
        _ => {}
    }
    if ts {
        update_timeshift(app, player);
    }
    if changed {
        shell.core.set_player_info(Some(info_of(&player.state.lock().unwrap())));
    }
}

/// Direct : recalcule « disponible en arrière » et « retard sur le direct » ; prévient l'interface si la seconde change.
fn update_timeshift(app: &AppHandle, player: &Player) {
    let p = player.props.lock().unwrap().clone();
    let (back, behind) = timeshift::info(p.pos, p.cache_start, p.cache_end);
    let snapshot = {
        let mut s = player.state.lock().unwrap();
        if s.kind != "live" || ((back - s.timeshift_available_secs).abs() < 1.0 && (behind - s.behind_live_secs).abs() < 1.0) {
            return;
        }
        s.timeshift_available_secs = back;
        s.behind_live_secs = behind;
        s.clone()
    };
    let _ = app.emit("player-state", snapshot);
}

/// Fin de fichier : on enchaîne (compte à rebours) ou on quitte.
fn on_eof(player: &Arc<Player>) {
    player.props.lock().unwrap().ended = true;
    commit(player, true);
    let (kind, len) = {
        let s = player.state.lock().unwrap();
        (s.kind.clone(), s.queue_len)
    };
    let idx = *player.index.lock().unwrap();
    match queue::next_after_eof(&kind, idx, len, config::load().auto_next) {
        Some(n) => start_countdown(player.clone(), n),
        None => {
            send(player, json!({ "command": ["quit"] }));
        }
    }
}

fn start_countdown(player: Arc<Player>, next: usize) {
    let cd = player.cd.load(Ordering::SeqCst);
    player.skip_wait.store(false, Ordering::SeqCst);
    player.waiting.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        let lang = player.lang.lock().unwrap().clone();
        let Some(title) = player.items.lock().unwrap().get(next).map(|i| i.name.clone()) else { return };
        'wait: for secs in (1..=COUNTDOWN).rev() {
            if player.cd.load(Ordering::SeqCst) != cd {
                return;
            }
            show(&player, &queue::next_notice(&lang, &title, secs), 1100);
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(100));
                if player.cd.load(Ordering::SeqCst) != cd {
                    return;
                }
                if player.skip_wait.load(Ordering::SeqCst) {
                    break 'wait;
                }
            }
        }
        if player.cd.load(Ordering::SeqCst) == cd {
            goto_item(&player, next);
        }
    });
}

/// Passe à l'élément `n` de la file : enregistre l'ancien, charge le nouveau dans mpv.
fn goto_item(player: &Player, n: usize) {
    let Some(it) = player.items.lock().unwrap().get(n).cloned() else { return };
    commit(player, false);
    player.cd.fetch_add(1, Ordering::SeqCst); // annule un éventuel compte à rebours
    player.waiting.store(false, Ordering::SeqCst);
    *player.index.lock().unwrap() = n;
    *player.props.lock().unwrap() = Props::default();
    {
        let mut s = player.state.lock().unwrap();
        s.title = it.name.clone();
        s.position = 0.0;
        s.duration = 0.0;
        s.timeshift_available_secs = 0.0;
        s.behind_live_secs = 0.0;
        s.queue_index = n;
    }
    send(player, json!({ "command": ["loadfile", it.url, "replace"] }));
}

/// Touches média suivant/précédent : file de lecture (vidéo/audio) ou zapping (direct).
fn step_item(player: &Player, dir: i32) {
    let (kind, len) = {
        let s = player.state.lock().unwrap();
        (s.kind.clone(), s.queue_len)
    };
    if kind == "live" {
        return zap(player, dir);
    }
    let idx = *player.index.lock().unwrap();
    match queue::step(idx, len, dir) {
        Some(n) => {
            goto_item(player, n);
            let title = player.state.lock().unwrap().title.clone();
            show(player, &format!("{title}  ({}/{len})", n + 1), 3000);
        }
        None => {
            let lang = player.lang.lock().unwrap().clone();
            let msg = match (lang == "en", dir > 0) {
                (true, true) => "Last item",
                (true, false) => "First item",
                (false, true) => "Dernier élément",
                (false, false) => "Premier élément",
            };
            show(player, msg, 2000);
        }
    }
}

/// Chaîne suivante (+1) ou précédente (-1) dans la liste en cours de lecture.
pub fn zap(player: &Player, dir: i32) {
    let len = player.items.lock().unwrap().len();
    if len < 2 {
        return;
    }
    let n = (*player.index.lock().unwrap() as i32 + dir).rem_euclid(len as i32) as usize;
    goto_item(player, n);
    let name = player.state.lock().unwrap().title.clone();
    show(player, &format!("{name}  ({}/{len})", n + 1), 4000);
}

// ---------- commandes Tauri ----------

use tauri::State;

#[tauri::command]
pub async fn continue_list(shell: State<'_, Shell>) -> Result<Vec<history::Entry>, String> {
    Ok(with_hist(&shell.player, |h| h.continue_list()))
}

#[tauri::command]
pub async fn continue_remove(shell: State<'_, Shell>, url: String) -> Result<(), String> {
    with_hist(&shell.player, |h| {
        if h.remove(&url) {
            crate::store::save("history", &*h)
        } else {
            Ok(())
        }
    })
}

#[tauri::command]
pub async fn continue_clear(shell: State<'_, Shell>) -> Result<(), String> {
    with_hist(&shell.player, |h| {
        h.clear();
        crate::store::save("history", &*h)
    })
}

fn active(player: &Player) -> Result<String, String> {
    let s = player.state.lock().unwrap();
    if s.active {
        Ok(s.kind.clone())
    } else {
        Err("not_playing".into())
    }
}

#[tauri::command]
pub async fn player_pause_toggle(shell: State<'_, Shell>) -> Result<(), String> {
    active(&shell.player)?;
    if send(&shell.player, json!({ "command": ["cycle", "pause"] })) {
        Ok(())
    } else {
        Err("no_ipc".into())
    }
}

/// Saut relatif dans le direct différé (secondes ; négatif = en arrière), borné au cache.
/// Renvoie le nouveau retard sur le direct. Erreurs : `not_live`, `nothing_buffered`, `already_live`, `no_ipc`.
#[tauri::command]
pub async fn player_timeshift(shell: State<'_, Shell>, seconds: f64) -> Result<f64, String> {
    let player = &shell.player;
    if active(player)? != "live" {
        return Err("not_live".into());
    }
    let p = player.props.lock().unwrap().clone();
    let target = timeshift::seek_target(seconds, p.pos, p.cache_start, p.cache_end)?;
    if !send(player, json!({ "command": ["seek", target, "absolute"] })) {
        return Err("no_ipc".into());
    }
    let behind = p.cache_end.map(|e| (e - target).max(0.0)).unwrap_or(0.0);
    show(player, &timeshift::badge(if behind < 2.0 { 0.0 } else { behind }), 2500);
    Ok(behind)
}

/// Retour au direct : saute au bord du cache et reprend la lecture.
#[tauri::command]
pub async fn player_go_live(shell: State<'_, Shell>) -> Result<(), String> {
    let player = &shell.player;
    if active(player)? != "live" {
        return Err("not_live".into());
    }
    let end = player.props.lock().unwrap().cache_end;
    send(player, json!({ "command": ["set_property", "pause", false] }));
    let ok = match end {
        Some(e) => send(player, json!({ "command": ["seek", (e - 0.5).max(0.0), "absolute"] })),
        None => {
            // pas de cache connu : on recharge la chaîne, ce qui ramène au direct
            let idx = *player.index.lock().unwrap();
            let url = player.items.lock().unwrap().get(idx).map(|i| i.url.clone());
            url.map(|u| send(player, json!({ "command": ["loadfile", u, "replace"] }))).unwrap_or(false)
        }
    };
    if !ok {
        return Err("no_ipc".into());
    }
    show(player, "LIVE", 2000);
    Ok(())
}

/// Choisit une piste : `kind` = "audio" | "sub", `id` = None pour désactiver (sous-titres).
#[tauri::command]
pub async fn player_set_track(shell: State<'_, Shell>, kind: String, id: Option<i64>) -> Result<(), String> {
    active(&shell.player)?;
    let prop = match kind.as_str() {
        "audio" => "aid",
        "sub" => "sid",
        _ => return Err("invalid".into()),
    };
    let val = id.map(|i| json!(i)).unwrap_or(json!("no"));
    if send(&shell.player, json!({ "command": ["set_property", prop, val] })) {
        Ok(())
    } else {
        Err("no_ipc".into())
    }
}

/// Ouvre le menu du lecteur (comme la touche Menu de la télécommande).
#[tauri::command]
pub async fn player_menu(shell: State<'_, Shell>) -> Result<(), String> {
    active(&shell.player)?;
    if send(&shell.player, json!({ "command": ["script-message", "rustv-menu"] })) {
        Ok(())
    } else {
        Err("no_ipc".into())
    }
}

#[tauri::command]
pub async fn player_get_config() -> Result<config::PlayerConfig, String> {
    Ok(config::load())
}

#[tauri::command]
pub async fn player_set_config(cfg: config::PlayerConfig) -> Result<config::PlayerConfig, String> {
    let c = cfg.sanitized();
    config::save(&c)?;
    Ok(c)
}
