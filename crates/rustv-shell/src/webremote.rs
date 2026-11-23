//! Télécommande web : démarrage et arrêt à chaud du serveur HTTP + WebSocket (voir `rustv_core::webremote`
//! et docs/WEB-REMOTE.md). Désactivée par défaut ; réglage dans Réglages > Intégrations.
use rustv_core::webremote::{Asset, Assets, WebRemote, DEFAULT_PORT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Cfg {
    enabled: bool,
    port: u16,
}

impl Default for Cfg {
    fn default() -> Self {
        Self { enabled: false, port: DEFAULT_PORT }
    }
}

/// Ports réservés par RusTV (télécommande TCP, beacon, cast).
fn port_ok(p: u16) -> bool {
    p >= 1024 && ![47820, 47821, 47840].contains(&p)
}

fn load_cfg() -> Cfg {
    let mut c: Cfg = crate::store::load("webremote");
    if !port_ok(c.port) {
        c.port = DEFAULT_PORT;
    }
    c
}

static RUN: Mutex<Option<(WebRemote, u16)>> = Mutex::new(None);
static LAST_ERROR: Mutex<String> = Mutex::new(String::new());

// ---------- fichiers de la coquille web ----------

pub fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

struct Dir(PathBuf);

impl Assets for Dir {
    fn get(&self, path: &str) -> Option<Asset> {
        let f = self.0.join(path);
        if !f.is_file() {
            return None;
        }
        let body = std::fs::read(&f).ok()?;
        Some(Asset { body, mime: mime(path).to_string() })
    }
}

/// Dossier contenant `index.html` : ressources de l'application, à côté de l'exécutable, ou (développement)
/// dossier du dépôt / build de `remote/`.
fn locate(app: &AppHandle) -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = vec![];
    if let Ok(r) = app.path().resource_dir() {
        c.push(r.join("remote-web"));
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        c.push(d.join("remote-web"));
    }
    let m = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.push(m.join("remote-web"));
    c.push(m.join("../../remote/dist"));
    c.into_iter().find(|d| d.join("index.html").is_file())
}

// ---------- démarrage / arrêt ----------

pub fn stop() {
    let w = RUN.lock().unwrap().take();
    if let Some((w, _)) = w {
        w.stop();
    }
}

fn start_with(app: &AppHandle, port: u16) -> Result<(), String> {
    stop();
    let assets: Arc<dyn Assets> = Arc::new(Dir(locate(app).unwrap_or_else(|| PathBuf::from("remote-web-absent"))));
    let core = app.state::<crate::Shell>().core.clone();
    let w = WebRemote::start(core, assets, port).map_err(|_| "port_in_use".to_string())?;
    *RUN.lock().unwrap() = Some((w, port));
    Ok(())
}

pub fn start(app: &AppHandle) {
    let c = load_cfg();
    if c.enabled {
        if let Err(e) = start_with(app, c.port) {
            *LAST_ERROR.lock().unwrap() = e;
        }
    }
}

// ---------- vue ----------

fn qr_svg(data: &str, size: u32) -> Result<String, String> {
    use qrcode::{render::svg, QrCode};
    Ok(QrCode::new(data.as_bytes())
        .map_err(|e| e.to_string())?
        .render::<svg::Color>()
        .min_dimensions(size, size)
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// Adresses privées (hors loopback) sur lesquelles le serveur écoute, avec le port.
fn lan_urls(port: u16) -> Vec<String> {
    let run = RUN.lock().unwrap();
    let Some((w, _)) = run.as_ref() else { return vec![] };
    w.addrs().iter().filter(|a| !a.ip().is_loopback()).map(|a| format!("http://{}:{}", a.ip(), port)).collect()
}

fn view(app: &AppHandle) -> Value {
    let c = load_cfg();
    let running = RUN.lock().unwrap().is_some();
    let urls = lan_urls(c.port);
    let clients = RUN.lock().unwrap().as_ref().map(|(w, _)| w.clients()).unwrap_or_default();
    let err = LAST_ERROR.lock().unwrap().clone();
    json!({
        "enabled": c.enabled,
        "port": c.port,
        "running": running,
        "bundled": locate(app).is_some(),
        "urls": urls,
        "qr": urls.first().and_then(|u| qr_svg(u, 360).ok()).unwrap_or_default(),
        "clients": clients,
        "error": err,
    })
}

#[tauri::command]
pub async fn webremote_get(app: AppHandle) -> Result<Value, String> {
    Ok(view(&app))
}

#[derive(Deserialize)]
pub struct WebSet {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
}

#[tauri::command]
pub async fn webremote_set(app: AppHandle, cfg: WebSet) -> Result<Value, String> {
    let mut c = load_cfg();
    if let Some(p) = cfg.port {
        if !port_ok(p) {
            return Err("port_invalid".into());
        }
        c.port = p;
    }
    if let Some(e) = cfg.enabled {
        c.enabled = e;
    }
    crate::store::save("webremote", &c)?;
    let (app2, c2) = (app.clone(), c.clone());
    // Le démarrage et l'arrêt (attente des fils) bloquent : hors du fil asynchrone.
    let res = tauri::async_runtime::spawn_blocking(move || {
        if c2.enabled {
            start_with(&app2, c2.port)
        } else {
            stop();
            Ok(())
        }
    })
    .await
    .map_err(|_| "webremote_failed".to_string())?;
    *LAST_ERROR.lock().unwrap() = res.clone().err().unwrap_or_default();
    res?;
    Ok(view(&app))
}

/// Jumelage d'un navigateur : nouvelle clé révocable, encodée dans le fragment (`#p=`) d'une adresse que l'on
/// scanne ou ouvre. Le fragment n'est jamais envoyé au serveur ; la clé ne circule pas sur le réseau.
#[tauri::command]
pub async fn webremote_pair(app: AppHandle) -> Result<Value, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let c = load_cfg();
    let base = lan_urls(c.port).into_iter().next().ok_or("not_running")?;
    let shell = app.state::<crate::Shell>();
    let (mut info, device) = shell.core.new_pairing();
    info.ips = vec![];
    info.bt = None;
    let code = URL_SAFE_NO_PAD.encode(info.to_qr_string());
    let url = format!("{base}/#p={code}");
    Ok(json!({ "svg": qr_svg(&url, 420)?, "device": device, "code": code, "url": base }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_ports_refused() {
        assert!(port_ok(47822) && port_ok(8080));
        assert!(!port_ok(47820) && !port_ok(47821) && !port_ok(47840) && !port_ok(80) && !port_ok(0));
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime("assets/index-1.js"), "text/javascript; charset=utf-8");
        assert!(mime("manifest.webmanifest").starts_with("application/manifest+json"));
        assert_eq!(mime("icons/a.png"), "image/png");
        assert_eq!(mime("x.bin"), "application/octet-stream");
    }

    #[test]
    fn disabled_by_default() {
        let c = Cfg::default();
        assert!(!c.enabled && c.port == 47822);
    }
}
