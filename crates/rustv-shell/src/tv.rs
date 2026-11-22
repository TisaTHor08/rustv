//! Gestion des fenêtres du mode TV : sélecteur d'écran, accueil, écrans noirs, apps web.
use crate::{settings, Shell};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const CATALOG: &str = include_str!("catalog.json");
const SPATNAV: &str = include_str!("spatnav.js");

pub fn catalog() -> Vec<Value> {
    serde_json::from_str(CATALOG).expect("catalog.json valide")
}

#[derive(Clone, Serialize)]
pub struct MonInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Écrans triés de gauche à droite, puis de haut en bas.
pub fn monitors(app: &AppHandle) -> Vec<MonInfo> {
    let mut v: Vec<MonInfo> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| MonInfo {
            index: 0,
            name: m.name().cloned().unwrap_or_default(),
            x: m.position().x,
            y: m.position().y,
            w: m.size().width,
            h: m.size().height,
        })
        .collect();
    v.sort_by_key(|m| (m.x, m.y));
    for (i, m) in v.iter_mut().enumerate() {
        m.index = i;
    }
    v
}

fn place_full(win: &WebviewWindow, m: &MonInfo) {
    let _ = win.set_position(PhysicalPosition::new(m.x, m.y));
    let _ = win.set_size(PhysicalSize::new(m.w, m.h));
    let _ = win.set_fullscreen(true);
}

/// Windows refuse souvent de donner le focus à un process en arrière-plan : on passe par rustv-core.
pub fn force_focus(win: &WebviewWindow) {
    if let Ok(h) = win.hwnd() {
        rustv_core::input::force_foreground(h.0 as isize);
    }
    let _ = win.set_focus();
}

fn ui_url(query: &str) -> WebviewUrl {
    WebviewUrl::App(format!("index.html?{query}").into())
}

// ---------- démarrage / arrêt du mode TV ----------

pub fn start(app: &AppHandle) {
    let shell = app.state::<Shell>();
    if shell.tv.lock().unwrap().active {
        if let Some(w) = app.get_webview_window("tv") {
            force_focus(&w);
        }
        return;
    }
    let mons = monitors(app);
    if mons.len() <= 1 {
        choose(app, 0);
        return;
    }
    for m in &mons {
        let label = format!("picker-{}", m.index);
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        if let Ok(w) = WebviewWindowBuilder::new(app, &label, ui_url(&format!("view=picker&i={}&n={}", m.index, mons.len())))
            .title("RusTV — choix de l'écran")
            .decorations(false)
            .visible(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .build()
        {
            place_full(&w, m);
            let _ = w.show();
        }
    }
    let first = shell.settings.lock().unwrap().monitor.unwrap_or(0).min(mons.len() - 1);
    if let Some(w) = app.get_webview_window(&format!("picker-{first}")) {
        force_focus(&w);
    }
}

pub fn picker_move(app: &AppHandle, dir: i32) {
    let n = monitors(app).len() as i32;
    let cur = (0..n).find(|i| app.get_webview_window(&format!("picker-{i}")).map(|w| w.is_focused().unwrap_or(false)).unwrap_or(false)).unwrap_or(0);
    let next = (cur + dir).clamp(0, n - 1);
    if let Some(w) = app.get_webview_window(&format!("picker-{next}")) {
        force_focus(&w);
    }
}

pub fn choose(app: &AppHandle, index: usize) {
    let shell = app.state::<Shell>();
    for (label, w) in app.webview_windows() {
        if label.starts_with("picker-") {
            let _ = w.destroy();
        }
    }
    let mons = monitors(app);
    let index = index.min(mons.len().saturating_sub(1));
    let Some(m) = mons.get(index).cloned() else { return };
    {
        let mut s = shell.settings.lock().unwrap();
        s.monitor = Some(index);
        s.save();
    }
    {
        let mut t = shell.tv.lock().unwrap();
        t.active = true;
        t.monitor = index;
    }
    if shell.settings.lock().unwrap().blackout {
        for o in mons.iter().filter(|o| o.index != index) {
            let label = format!("black-{}", o.index);
            if app.get_webview_window(&label).is_some() {
                continue;
            }
            if let Ok(w) = WebviewWindowBuilder::new(app, &label, ui_url("view=black"))
                .decorations(false)
                .visible(false)
                .skip_taskbar(true)
                .always_on_top(true)
                .focused(false)
                .build()
            {
                place_full(&w, o);
                let _ = w.set_cursor_visible(false);
                let _ = w.show();
            }
        }
    }
    if app.get_webview_window("tv").is_none() {
        if let Ok(w) = WebviewWindowBuilder::new(app, "tv", ui_url("view=home")).title("RusTV").decorations(false).visible(false).build() {
            place_full(&w, &m);
            let _ = w.show();
            force_focus(&w);
        }
    }
    shell.core.set_tv_mode(true);
    let _ = app.emit("tv-changed", true);
}

pub fn exit(app: &AppHandle) {
    let shell = app.state::<Shell>();
    for (label, w) in app.webview_windows() {
        if label == "tv" || label.starts_with("black-") || label.starts_with("picker-") || label.starts_with("app-") {
            let _ = w.destroy();
        }
    }
    shell.tv.lock().unwrap().active = false;
    shell.core.set_tv_mode(false);
}

/// Ferme l'app au premier plan et revient à l'accueil.
pub fn go_home(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("app-") {
            let _ = w.destroy();
        }
    }
    if let Some(w) = app.get_webview_window("tv") {
        force_focus(&w);
    } else {
        start(app);
    }
}

// ---------- lancement d'apps ----------

fn find_vlc() -> Option<String> {
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(var) {
            let path = std::path::PathBuf::from(p).join(r"VideoLAN\VLC\vlc.exe");
            if path.exists() {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Lance une tuile du catalogue. Renvoie "ok", "needs_url" ou une erreur lisible.
pub fn launch(app: &AppHandle, id: &str, url: Option<String>) -> Result<String, String> {
    let tile = catalog().into_iter().find(|t| t["id"] == id).ok_or("Application inconnue")?;
    launch_value(app, &tile, url)
}

/// Lance une tuile décrite par un objet JSON (catalogue ou tuile personnalisée de l'utilisateur).
pub fn launch_value(app: &AppHandle, tile: &Value, url: Option<String>) -> Result<String, String> {
    let shell = app.state::<Shell>();
    let id = tile["id"].as_str().filter(|i| i.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')).ok_or("Tuile invalide")?;
    if let Some(u) = url.filter(|u| !u.trim().is_empty()) {
        let u = if u.contains("://") { u } else { format!("http://{u}") };
        u.parse::<Url>().map_err(|_| "Adresse invalide".to_string())?;
        let mut s = shell.settings.lock().unwrap();
        s.urls.insert(id.to_string(), u);
        s.save();
    }
    let tile = tile.clone();
    match tile["kind"].as_str().unwrap_or("") {
        "native" => {
            let target = match id {
                "vlc" => find_vlc().ok_or("VLC n'est pas installé (https://www.videolan.org)")?,
                _ => tile["target"].as_str().unwrap_or_default().to_string(),
            };
            rustv_core::apps::launch(&target);
            Ok("ok".into())
        }
        _ => {
            let url = tile["url"].as_str().filter(|u| !u.is_empty()).map(str::to_string).or_else(|| shell.settings.lock().unwrap().urls.get(id).cloned());
            let Some(url) = url else { return Ok("needs_url".into()) };
            let parsed: Url = url.parse().map_err(|_| "Adresse invalide")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("Seules les adresses http(s) sont autorisées".into());
            }
            open_web(app, id, &tile, parsed).map_err(|e| e.to_string())?;
            Ok("ok".into())
        }
    }
}

fn open_web(app: &AppHandle, id: &str, tile: &Value, url: Url) -> tauri::Result<()> {
    let shell = app.state::<Shell>();
    for (label, w) in app.webview_windows() {
        if label.starts_with("app-") {
            let _ = w.destroy();
        }
    }
    let label = format!("app-{id}");
    // Option `allow_insecure_lan` : accepte les certificats auto-signés, uniquement pour un hôte du réseau local.
    let insecure = tile["allow_insecure_lan"].as_bool().unwrap_or(false) && url.scheme() == "https" && url.host_str().is_some_and(crate::lan::is_private_host);
    let mod_scripts = crate::mods::adapters_for(url.as_str());
    // Un profil WebView2 par application : sessions, cookies et comptes isolés.
    let profile = settings::dir().join("profiles").join(id);
    let mut b = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(tile["name"].as_str().unwrap_or("RusTV"))
        .data_directory(profile)
        .decorations(false)
        .visible(false);
    if insecure {
        // Remplace les arguments par défaut de wry (voir la doc de additional_browser_args) ; profil dédié par tuile.
        b = b.additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --ignore-certificate-errors");
    }
    if tile["spatnav"].as_bool().unwrap_or(false) {
        b = b.initialization_script(SPATNAV);
    }
    // Adaptateurs de sites apportés par les mods activés (une permission « inject » a été acceptée).
    if !mod_scripts.is_empty() {
        b = b.initialization_script(&mod_scripts);
    }
    if let Some(ua) = tile["ua"].as_str() {
        b = b.user_agent(ua);
    }
    let w = b.build()?;
    let mons = monitors(app);
    let idx = if shell.tv.lock().unwrap().active { shell.tv.lock().unwrap().monitor } else { 0 };
    if let Some(m) = mons.get(idx) {
        place_full(&w, m);
    }
    let _ = w.show();
    force_focus(&w);
    Ok(())
}
