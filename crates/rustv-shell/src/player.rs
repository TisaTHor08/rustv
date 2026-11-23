//! Lecteur vidéo/audio : mpv, piloté par son IPC JSON (tube nommé).
//!
//! RusTV ne lie pas libmpv : il lance `mpv.exe` en plein écran sur l'écran de la TV et dialogue avec lui
//! (zapping des chaînes, titre et état de lecture pour le téléphone). La télécommande envoie de vraies
//! touches, que mpv reçoit grâce à un `input.conf` généré ici.
//! mpv est détecté s'il est déjà installé ; sinon il est téléchargé (avec l'accord de l'utilisateur) depuis
//! les builds Windows référencés par mpv.io (shinchiro/mpv-winbuild-cmake), après vérification du SHA-256.
use crate::{settings, tv, Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
}

#[derive(Deserialize, Clone)]
pub struct Item {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct PlayRequest {
    /// "live" (zapping haut/bas), "video" (avance/retour, reprise) ou "audio".
    pub kind: String,
    pub items: Vec<Item>,
    #[serde(default)]
    pub index: usize,
}

#[derive(Default)]
pub struct Player {
    child: Mutex<Option<Child>>,
    pipe: Mutex<Option<std::fs::File>>,
    items: Mutex<Vec<Item>>,
    index: Mutex<usize>,
    pub state: Mutex<PlayerState>,
}

fn player_dir() -> PathBuf {
    settings::dir().join("player")
}

pub fn find_mpv() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = vec![player_dir().join("mpv.exe")];
    if let Some(path) = std::env::var_os("PATH") {
        cands.extend(std::env::split_paths(&path).map(|p| p.join("mpv.exe")));
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

fn write_conf(kind: &str) -> PathBuf {
    let dir = player_dir();
    let _ = std::fs::create_dir_all(&dir);
    let conf = if kind == "live" {
        // Haut/bas : chaîne suivante/précédente. Gauche/droite : volume.
        "UP script-message rustv zap -1\nDOWN script-message rustv zap 1\nLEFT add volume -5\nRIGHT add volume 5\nENTER show-text \"${media-title}\" 3000\nESC quit\nBS quit\n"
    } else {
        "LEFT seek -10\nRIGHT seek 10\nUP seek 60\nDOWN seek -60\nENTER cycle pause\nESC quit\nBS quit\n"
    };
    let p = dir.join(format!("{kind}.conf"));
    let _ = std::fs::write(&p, conf);
    p
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

pub fn play(app: &AppHandle, req: PlayRequest) -> Result<(), String> {
    if req.items.is_empty() {
        return Err("rien à lire".into());
    }
    let mpv = find_mpv().ok_or("mpv_missing")?;
    stop(app);
    let shell = app.state::<Shell>();
    let player = shell.player.clone();
    let index = req.index.min(req.items.len() - 1);
    let first = req.items[index].clone();
    *player.items.lock().unwrap() = req.items;
    *player.index.lock().unwrap() = index;

    let mons = tv::monitors(app);
    let tvmon = if shell.tv.lock().unwrap().active { shell.tv.lock().unwrap().monitor } else { 0 };
    let pipe = format!(r"\\.\pipe\rustv-mpv-{}", std::process::id());
    let conf = write_conf(&req.kind);
    let mut cmd = Command::new(&mpv);
    cmd.args([
        "--no-terminal",
        "--force-window=immediate",
        "--fullscreen",
        "--osd-level=1",
        "--osd-duration=2500",
        "--osd-font-size=36",
        "--idle=no",
        "--keep-open=no",
        "--hwdec=auto-safe",
        "--ytdl=no",
        "--title=RusTV player",
    ]);
    cmd.arg(format!("--input-ipc-server={pipe}")).arg(format!("--input-conf={}", conf.display()));
    if let Some(m) = mons.get(tvmon).filter(|m| !m.name.is_empty()) {
        cmd.arg(format!("--screen-name={}", m.name)).arg(format!("--fs-screen-name={}", m.name));
    }
    match req.kind.as_str() {
        "video" => {
            let wl = player_dir().join("watch_later");
            let _ = std::fs::create_dir_all(&wl);
            cmd.arg("--save-position-on-quit").arg(format!("--watch-later-directory={}", wl.display()));
        }
        "audio" => {
            cmd.arg("--audio-display=embedded-first").arg("--osd-level=3");
        }
        _ => {
            cmd.arg("--cache=yes").arg("--demuxer-readahead-secs=20");
        }
    }
    cmd.arg("--").arg(&first.url);
    let child = cmd.spawn().map_err(|e| format!("impossible de lancer mpv : {e}"))?;
    *player.child.lock().unwrap() = Some(child);
    *player.state.lock().unwrap() = PlayerState { active: true, kind: req.kind.clone(), title: first.name, ..Default::default() };
    shell.core.set_player_info(Some(info_of(&player.state.lock().unwrap())));

    // Dialogue avec mpv (IPC) puis retour à l'accueil quand il se ferme.
    let app2 = app.clone();
    let player2 = player.clone();
    std::thread::spawn(move || {
        ipc_loop(&app2, &player2, &pipe);
        // mpv s'est fermé (ou n'a jamais répondu) : on nettoie et on revient à l'accueil.
        if let Some(mut c) = player2.child.lock().unwrap().take() {
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

fn send(player: &Player, v: Value) {
    if let Some(f) = player.pipe.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{v}");
    }
}

fn ipc_loop(app: &AppHandle, player: &Arc<Player>, pipe: &str) {
    // mpv met un instant à créer le tube : on réessaie quelques secondes.
    let mut file = None;
    for _ in 0..60 {
        if let Ok(f) = OpenOptions::new().read(true).write(true).open(pipe) {
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
    for (i, p) in ["media-title", "pause", "time-pos", "duration"].iter().enumerate() {
        send(player, json!({ "command": ["observe_property", i + 1, p] }));
    }
    let shell = app.state::<Shell>();
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        match v["event"].as_str() {
            Some("property-change") => {
                let mut changed = false;
                {
                    let mut s = player.state.lock().unwrap();
                    match v["name"].as_str() {
                        Some("media-title") => {
                            if let Some(t) = v["data"].as_str() {
                                // Pour la TV en direct, le nom de la chaîne vaut mieux que le titre du flux.
                                if s.kind != "live" {
                                    s.title = t.to_string();
                                    changed = true;
                                }
                            }
                        }
                        Some("pause") => {
                            s.paused = v["data"].as_bool().unwrap_or(false);
                            changed = true;
                        }
                        Some("time-pos") => {
                            let p = v["data"].as_f64().unwrap_or(0.0);
                            changed = (p - s.position).abs() >= 5.0;
                            if changed {
                                s.position = p;
                            }
                        }
                        Some("duration") => s.duration = v["data"].as_f64().unwrap_or(0.0),
                        _ => {}
                    }
                }
                if changed {
                    shell.core.set_player_info(Some(info_of(&player.state.lock().unwrap())));
                }
            }
            Some("client-message") => {
                let a: Vec<&str> = v["args"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
                if a.len() == 3 && a[0] == "rustv" && a[1] == "zap" {
                    zap(player, a[2].parse().unwrap_or(1));
                }
            }
            _ => {}
        }
    }
}

/// Chaîne suivante (+1) ou précédente (-1) dans la liste en cours de lecture.
pub fn zap(player: &Player, dir: i32) {
    let items = player.items.lock().unwrap().clone();
    if items.len() < 2 {
        return;
    }
    let mut idx = player.index.lock().unwrap();
    *idx = (*idx as i32 + dir).rem_euclid(items.len() as i32) as usize;
    let it = &items[*idx];
    player.state.lock().unwrap().title = it.name.clone();
    send(player, json!({ "command": ["loadfile", it.url, "replace"] }));
    send(player, json!({ "command": ["show-text", format!("{}  ({}/{})", it.name, *idx + 1, items.len()), 4000] }));
}
