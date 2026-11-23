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
    crate::hooks::tv_mode_changed(app, true);
    let _ = app.emit("tv-changed", true);
}

pub fn exit(app: &AppHandle) {
    crate::browser::close();
    let shell = app.state::<Shell>();
    for (label, w) in app.webview_windows() {
        if label == "tv" || label.starts_with("black-") || label.starts_with("picker-") || label.starts_with("app-") {
            let _ = w.destroy();
        }
    }
    shell.tv.lock().unwrap().active = false;
    shell.core.set_tv_mode(false);
    crate::hooks::tv_mode_changed(app, false);
}

/// Démarrage automatique (`rustv --boot`, avec ou sans `--tv`) : choisit l'écran mémorisé sans afficher le
/// sélecteur. Si la TV n'est pas encore là (réveil lent), attend jusqu'à 60 s qu'elle apparaisse, puis se
/// rabat sur l'index mémorisé. `screen` : `--screen N` explicite.
pub fn boot(app: &AppHandle, screen: Option<usize>) {
    use crate::guard::{boot_decision, Boot, MonDesc, SavedMon};
    let app = app.clone();
    std::thread::spawn(move || {
        let saved = match screen {
            Some(i) => Some(SavedMon { index: i, name: String::new(), w: 0, h: 0, count: i + 1 }),
            None => crate::guard::saved_monitor().or_else(|| {
                let i = app.state::<Shell>().settings.lock().ok()?.monitor?;
                Some(SavedMon { index: i, name: String::new(), w: 0, h: 0, count: i + 1 })
            }),
        };
        let t0 = std::time::Instant::now();
        loop {
            if app.state::<Shell>().tv.lock().map(|t| t.active).unwrap_or(true) {
                return;
            }
            let descs: Vec<MonDesc> = monitors(&app).iter().map(|m| MonDesc { name: m.name.clone(), w: m.w, h: m.h }).collect();
            let waited = t0.elapsed().as_millis() as u64;
            match boot_decision(saved.as_ref(), &descs, waited, 60_000) {
                Boot::Use(i) | Boot::Fallback(i) => return choose(&app, i),
                Boot::Pick => return start(&app),
                Boot::Wait if waited > 600_000 => return,
                Boot::Wait => std::thread::sleep(std::time::Duration::from_secs(1)),
            }
        }
    });
}

/// Replace les fenêtres TV après un changement d'écrans (TV réveillée, résolution modifiée, veille).
pub fn reflow(app: &AppHandle) {
    let shell = app.state::<Shell>();
    let (active, idx) = shell.tv.lock().map(|t| (t.active, t.monitor)).unwrap_or((false, 0));
    if !active {
        return;
    }
    let mons = monitors(app);
    let Some(m) = mons.get(idx.min(mons.len().saturating_sub(1))).cloned() else { return };
    for (label, w) in app.webview_windows() {
        if label == "tv" || label.starts_with("app-") {
            place_full(&w, &m);
        } else if let Some(o) = label.strip_prefix("black-").and_then(|i| i.parse::<usize>().ok()).and_then(|i| mons.get(i)) {
            place_full(&w, o);
        }
    }
}

/// Ferme l'app au premier plan et revient à l'accueil.
pub fn go_home(app: &AppHandle) {
    crate::browser::close();
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
/// `browser` : choix de l'utilisateur pour les tuiles `adblock_browser` (auto | builtin | brave | vivaldi).
pub fn launch(app: &AppHandle, id: &str, url: Option<String>, browser: Option<String>) -> Result<String, String> {
    let mut tile = catalog().into_iter().find(|t| t["id"] == id).ok_or("Application inconnue")?;
    if let Some(b) = browser {
        tile["browser"] = Value::String(b);
    }
    launch_value(app, &tile, url)
}

/// Lance une tuile décrite par un objet JSON (catalogue ou tuile personnalisée de l'utilisateur).
pub fn launch_value(app: &AppHandle, tile: &Value, url: Option<String>) -> Result<String, String> {
    let shell = app.state::<Shell>();
    let id = tile["id"].as_str().filter(|i| i.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')).ok_or("Tuile invalide")?;
    if let Some(u) = url.filter(|u| !u.trim().is_empty()) {
        // Plusieurs adresses possibles (séparées par un espace, une virgule ou un point-virgule) : la première qui répond est utilisée.
        let list = split_addresses(&u);
        if list.is_empty() {
            return Err("Adresse invalide".into());
        }
        let u = list.join(" ");
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
            crate::guard::note_launch(&target); // la garde TV autorisera cette fenêtre (lancée exprès)
            rustv_core::apps::launch(&target);
            Ok("ok".into())
        }
        _ => {
            let url = tile["url"].as_str().filter(|u| !u.is_empty()).map(str::to_string).or_else(|| shell.settings.lock().unwrap().urls.get(id).cloned());
            let Some(url) = url else { return Ok("needs_url".into()) };
            let url = pick_reachable(&url)?;
            let parsed: Url = url.parse().map_err(|_| "Adresse invalide")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("Seules les adresses http(s) sont autorisées".into());
            }
            if tile["adblock_browser"].as_bool().unwrap_or(false) {
                let choice = tile["browser"].as_str().unwrap_or("auto");
                if let Some(found) = crate::browser::resolve(choice, &crate::browser::detect()) {
                    for (label, w) in app.webview_windows() {
                        if label.starts_with("app-") {
                            let _ = w.destroy();
                        }
                    }
                    if crate::browser::launch(app, &found, id, &tile, parsed.as_str()).is_ok() {
                        return Ok("ok".into());
                    }
                }
            }
            open_web(app, id, &tile, parsed).map_err(|e| e.to_string())?;
            Ok("ok".into())
        }
    }
}

/// Découpe une saisie en adresses http(s) valides (le schéma `http://` est ajouté s'il manque), sans doublon, 5 au plus.
pub(crate) fn split_addresses(input: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in input.split(|c: char| c.is_whitespace() || c == ',' || c == ';').filter(|p| !p.is_empty()) {
        let u = if part.contains("://") { part.to_string() } else { format!("http://{part}") };
        if let Ok(parsed) = u.parse::<Url>() {
            if matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some() && !out.contains(&u) && out.len() < 5 {
                out.push(u);
            }
        }
    }
    out
}

/// L'hôte de l'adresse répond-il (résolution du nom puis connexion TCP en moins de 2 s) ?
fn reachable(u: &str) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(parsed) = u.parse::<Url>() else { return false };
    let Some(host) = parsed.host_str() else { return false };
    let Some(port) = parsed.port_or_known_default() else { return false };
    let Ok(addrs) = (host, port).to_socket_addrs() else { return false };
    addrs.take(4).any(|a| std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_millis(1200)).is_ok())
}

/// Choisit la première adresse (dans l'ordre saisi) qui répond. Les adresses sont testées en parallèle et l'attente totale est
/// bornée (1,5 s) : un nom introuvable ne bloque donc pas le lancement. Une seule adresse : gardée telle quelle.
fn pick_reachable(stored: &str) -> Result<String, String> {
    let list = split_addresses(stored);
    match list.len() {
        0 => return Err("Adresse invalide".into()),
        1 => return Ok(list.into_iter().next().unwrap()),
        _ => {}
    }
    let (tx, rx) = std::sync::mpsc::channel::<(usize, bool)>();
    for (i, u) in list.iter().enumerate() {
        let (tx, u) = (tx.clone(), u.clone());
        std::thread::spawn(move || {
            let _ = tx.send((i, reachable(&u)));
        });
    }
    drop(tx);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
    let mut res: Vec<Option<bool>> = vec![None; list.len()];
    while let Ok((i, ok)) = rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
        res[i] = Some(ok);
        // Gagnante : la première adresse positive dont toutes les précédentes ont déjà échoué.
        for (j, r) in res.iter().enumerate() {
            match r {
                Some(true) => return Ok(list[j].clone()),
                Some(false) => continue,
                None => break,
            }
        }
    }
    // Délai écoulé : on prend la première qui a répondu, même si une adresse plus prioritaire n'a pas encore répondu.
    res.iter().position(|r| *r == Some(true)).map(|j| list[j].clone()).ok_or_else(|| "Aucune des adresses ne répond (NAS éteint ou hors réseau ?)".to_string())
}

pub(crate) fn open_web(app: &AppHandle, id: &str, tile: &Value, url: Url) -> tauri::Result<()> {
    crate::browser::close();
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

#[cfg(test)]
mod address_tests {
    use super::split_addresses;

    #[test]
    fn splits_and_normalizes() {
        let l = split_addresses("http://DXP2800-8E19, dxp2800-8e19.eur81.ug.link  https://a.example/x;http://DXP2800-8E19");
        assert_eq!(l, vec!["http://DXP2800-8E19", "http://dxp2800-8e19.eur81.ug.link", "https://a.example/x"]);
    }

    #[test]
    fn rejects_other_schemes_and_garbage() {
        assert!(split_addresses("file:///c:/x ftp://h javascript:alert(1)").is_empty());
        assert!(split_addresses("   ").is_empty());
        assert_eq!(split_addresses("1 2 3 4 5 6 7").len(), 5);
    }
}
