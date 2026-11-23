//! Casting vers RusTV : découverte SSDP/UPnP, DIAL (YouTube) et rendu DLNA (MediaRenderer).
//! Désactivé par défaut (il ouvre des ports) ; actif seulement si l'utilisateur l'active dans la page « Cast and DLNA ».
//! Le serveur HTTP n'écoute que sur les interfaces IPv4 du réseau privé (port 47840) et n'accepte que des sources locales.
pub mod dial;
pub mod dlna;
pub mod http;
pub mod ssdp;

use dlna::{Renderer, Service, Sub};
use http::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ssdp::Iface;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub const PORT: u16 = 47840;
const MAX_CONNS: usize = 16;

// ---------- configuration ----------

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct Cfg {
    enabled: bool,
    /// Nom affiché sur le téléphone ; vide = « RusTV (<nom du PC>) ».
    name: String,
    /// UUID stable de l'appareil (généré une fois).
    uuid: String,
}

fn load_cfg() -> Cfg {
    let mut c: Cfg = crate::store::load("cast");
    if c.uuid.len() != 36 {
        c.uuid = new_uuid();
        let _ = crate::store::save("cast", &c);
    }
    c
}

pub fn new_uuid() -> String {
    let mut b: [u8; 16] = rand::random();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

fn friendly_name(custom: &str, pc: &str) -> String {
    let c: String = custom.chars().filter(|c| !c.is_control()).take(60).collect();
    let c = c.trim();
    if c.is_empty() {
        format!("RusTV ({})", if pc.trim().is_empty() { "PC" } else { pc.trim() })
    } else {
        c.to_string()
    }
}

// ---------- interface avec le reste de l'application ----------

#[derive(Clone, Default)]
pub struct PlayerSnap {
    pub active: bool,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
}

/// Actions réelles (lecteur, volume, YouTube). Une implémentation factice sert aux tests.
pub trait Host: Send + Sync {
    fn play(&self, kind: &str, title: &str, url: &str) -> Result<(), String>;
    fn stop(&self);
    fn pause(&self, paused: bool);
    fn seek(&self, secs: f64);
    fn player(&self) -> PlayerSnap;
    fn volume(&self) -> (u8, bool);
    fn set_volume(&self, v: u8);
    /// Ouvre la tuile YouTube ; `url` remplace l'adresse de la tuile (lancement DIAL avec paramètres).
    fn launch_youtube(&self, url: Option<String>) -> Result<(), String>;
    fn youtube_running(&self) -> bool;
    fn stop_youtube(&self);
}

struct AppHost {
    app: AppHandle,
    yt_launched: Mutex<Option<Instant>>,
}

impl Host for AppHost {
    fn play(&self, kind: &str, title: &str, url: &str) -> Result<(), String> {
        let kind = if kind == "audio" { "audio" } else { "video" };
        crate::player::play(&self.app, crate::player::PlayRequest { kind: kind.into(), items: vec![crate::player::Item { name: title.to_string(), url: url.to_string() }], index: 0 })
    }
    fn stop(&self) {
        crate::player::stop(&self.app);
    }
    fn pause(&self, paused: bool) {
        crate::player::ipc(&self.app, json!({ "command": ["set_property", "pause", paused] }));
    }
    fn seek(&self, secs: f64) {
        crate::player::ipc(&self.app, json!({ "command": ["seek", secs, "absolute"] }));
    }
    fn player(&self) -> PlayerSnap {
        let st = self.app.state::<crate::Shell>().player.state.lock().unwrap().clone();
        PlayerSnap { active: st.active, paused: st.paused, position: st.position, duration: st.duration }
    }
    fn volume(&self) -> (u8, bool) {
        rustv_core::audio::get().unwrap_or((0, false))
    }
    fn set_volume(&self, v: u8) {
        rustv_core::audio::set(v.min(100));
    }
    fn launch_youtube(&self, url: Option<String>) -> Result<(), String> {
        let mut tile = crate::tv::catalog().into_iter().find(|t| t["id"] == "youtube").ok_or("youtube tile missing")?;
        if let Some(u) = url {
            // La spec de la Leanback YouTube TV : les paramètres de lancement vont dans la query de /tv.
            tile["url"] = Value::String(u);
        }
        crate::tv::launch_value(&self.app, &tile, None)?;
        *self.yt_launched.lock().unwrap() = Some(Instant::now());
        Ok(())
    }
    fn youtube_running(&self) -> bool {
        // Fenêtre WebView2 ouverte, ou lancement très récent (le navigateur anti-pub n'a pas de fenêtre suivie).
        self.app.get_webview_window("app-youtube").is_some() || self.yt_launched.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(15))
    }
    fn stop_youtube(&self) {
        *self.yt_launched.lock().unwrap() = None;
        if self.app.state::<crate::Shell>().tv.lock().unwrap().active {
            crate::tv::go_home(&self.app);
        } else {
            crate::browser::close();
            for (label, w) in self.app.webview_windows() {
                if label.starts_with("app-") {
                    let _ = w.destroy();
                }
            }
        }
    }
}

// ---------- état partagé ----------

#[derive(Default, Clone)]
pub struct Status {
    pub last_client: String,
    pub last_event: String,
}

pub struct Shared {
    pub host: Arc<dyn Host>,
    pub uuid: String,
    pub name: String,
    pub server: String,
    pub stop: AtomicBool,
    pub ifaces: Mutex<Vec<Iface>>,
    pub status: Mutex<Status>,
    pub renderer: Mutex<Renderer>,
    pub subs: Mutex<Vec<Sub>>,
    pub sub_times: Mutex<Vec<Instant>>,
    pub conns: AtomicUsize,
}

impl Shared {
    pub fn new(host: Arc<dyn Host>, uuid: String, name: String) -> Self {
        Self {
            host,
            uuid,
            name,
            server: format!("Windows/10.0 UPnP/1.0 RusTV/{}", env!("CARGO_PKG_VERSION")),
            stop: AtomicBool::new(false),
            ifaces: Mutex::new(Vec::new()),
            status: Mutex::new(Status::default()),
            renderer: Mutex::new(Renderer::default()),
            subs: Mutex::new(Vec::new()),
            sub_times: Mutex::new(Vec::new()),
            conns: AtomicUsize::new(0),
        }
    }
}

// ---------- routage HTTP ----------

pub fn route(sh: &Shared, req: &Request) -> Response {
    if !http::is_lan(req.peer.ip()) {
        return Response::text(403, "Forbidden");
    }
    let base = format!("http://{}", req.local);
    if req.path == "/dd.xml" {
        return match req.method.as_str() {
            "GET" => Response::xml(200, dlna::device_xml(&sh.name, &sh.uuid)).header("Application-URL", dial::application_url(&base)),
            _ => Response::text(405, "Method not allowed"),
        };
    }
    if let Some(r) = dial::handle(sh, req) {
        sh.status.lock().unwrap().last_client = req.peer.ip().to_string();
        return r;
    }
    let mut parts = req.path.trim_start_matches('/').split('/');
    if parts.next() == Some("dlna") {
        if let (Some(svc), Some(what), None) = (parts.next().and_then(Service::from_path), parts.next(), parts.next()) {
            return match (what, req.method.as_str()) {
                ("scpd.xml", "GET") => Response::xml(200, dlna::scpd(svc)),
                ("control", _) => {
                    sh.status.lock().unwrap().last_client = req.peer.ip().to_string();
                    dlna::control(sh, svc, req)
                }
                ("event", "SUBSCRIBE") => dlna::subscribe(sh, svc, req),
                ("event", "UNSUBSCRIBE") => dlna::unsubscribe(sh, svc, req),
                _ => Response::text(405, "Method not allowed"),
            };
        }
    }
    Response::text(404, "Not found")
}

struct ConnGuard<'a>(&'a AtomicUsize);
impl Drop for ConnGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn handle_conn(sh: Arc<Shared>, mut stream: std::net::TcpStream) {
    let _g = ConnGuard(&sh.conns);
    let _ = stream.set_nonblocking(false);
    match http::read_request(&mut stream) {
        Ok(req) => {
            let resp = route(&sh, &req);
            http::write_response(&mut stream, &resp);
        }
        Err(0) => {}
        Err(code) => http::write_response(&mut stream, &Response::text(code, "")),
    }
}

fn listen(sh: Arc<Shared>, l: TcpListener) {
    while !sh.stop.load(Ordering::Relaxed) {
        match l.accept() {
            Ok((s, peer)) => {
                if !http::is_lan(peer.ip()) {
                    continue; // jamais d'hôte hors réseau local
                }
                if sh.conns.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
                    sh.conns.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let sh2 = sh.clone();
                if std::thread::Builder::new().name("cast-conn".into()).spawn(move || handle_conn(sh2, s)).is_err() {
                    sh.conns.fetch_sub(1, Ordering::SeqCst);
                }
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

// ---------- démarrage / arrêt ----------

type Listeners = Arc<Mutex<Vec<(Ipv4Addr, JoinHandle<()>)>>>;

struct Running {
    sh: Arc<Shared>,
    sock: Arc<UdpSocket>,
    threads: Vec<JoinHandle<()>>,
    listeners: Listeners,
}

static RUN: Mutex<Option<Running>> = Mutex::new(None);

fn bind_listener(sh: &Arc<Shared>, ip: Ipv4Addr, listeners: &Listeners) -> bool {
    // Uniquement l'adresse privée de l'interface, jamais 0.0.0.0.
    let Ok(l) = TcpListener::bind(SocketAddr::from((ip, PORT))) else { return false };
    if l.set_nonblocking(true).is_err() {
        return false;
    }
    let sh2 = sh.clone();
    match std::thread::Builder::new().name("cast-http".into()).spawn(move || listen(sh2, l)) {
        Ok(h) => {
            listeners.lock().unwrap().push((ip, h));
            true
        }
        Err(_) => false,
    }
}

/// Gère les interfaces (arrivée/départ de réseaux) et émet les NOTIFY périodiques.
fn manage(sh: Arc<Shared>, sock: Arc<UdpSocket>, listeners: Listeners) {
    let mut last_scan: Option<Instant> = None;
    let mut last_alive: Option<Instant> = None;
    let mut again: Option<Instant> = None;
    while !sh.stop.load(Ordering::Relaxed) {
        if last_scan.is_none_or(|t| t.elapsed() >= Duration::from_secs(5)) {
            last_scan = Some(Instant::now());
            let now = ssdp::list_ifaces();
            let mut new_ip = false;
            for i in &now {
                let known = listeners.lock().unwrap().iter().any(|(ip, _)| *ip == i.ip);
                if !known {
                    ssdp::join(&sock, i.ip);
                    if bind_listener(&sh, i.ip, &listeners) {
                        new_ip = true;
                    }
                }
            }
            let bound: Vec<Iface> = now.into_iter().filter(|i| listeners.lock().unwrap().iter().any(|(ip, _)| *ip == i.ip)).collect();
            *sh.ifaces.lock().unwrap() = bound;
            if new_ip {
                last_alive = None;
            }
        }
        if last_alive.is_none_or(|t| t.elapsed() >= Duration::from_secs(u64::from(ssdp::MAX_AGE) / 2)) {
            ssdp::send_notifies(&sh, &sock, true);
            last_alive = Some(Instant::now());
            // Un second envoi une seconde plus tard : UDP peut perdre des paquets.
            again = Some(Instant::now() + Duration::from_secs(1));
        } else if again.is_some_and(|t| Instant::now() >= t) {
            ssdp::send_notifies(&sh, &sock, true);
            again = None;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn start_server(host: Arc<dyn Host>, cfg: &Cfg, pc_name: &str) -> Result<(), String> {
    cast_stop();
    let sh = Arc::new(Shared::new(host, cfg.uuid.clone(), friendly_name(&cfg.name, pc_name)));
    let ifaces = ssdp::list_ifaces();
    *sh.ifaces.lock().unwrap() = Vec::new();
    let sock = Arc::new(ssdp::open_socket(&ifaces).map_err(|_| "ssdp_unavailable".to_string())?);
    let listeners: Listeners = Arc::new(Mutex::new(Vec::new()));
    let mut threads = Vec::new();
    let spawn = |name: &str, f: Box<dyn FnOnce() + Send>| std::thread::Builder::new().name(name.into()).spawn(f).map_err(|_| "thread".to_string());
    {
        let (s, k) = (sh.clone(), sock.clone());
        threads.push(spawn("cast-ssdp", Box::new(move || ssdp::run_recv(s, k)))?);
    }
    {
        let s = sh.clone();
        threads.push(spawn("cast-gena", Box::new(move || dlna::run_gena(s)))?);
    }
    {
        let (s, k, l) = (sh.clone(), sock.clone(), listeners.clone());
        threads.push(spawn("cast-manage", Box::new(move || manage(s, k, l)))?);
    }
    *RUN.lock().unwrap() = Some(Running { sh, sock, threads, listeners });
    Ok(())
}

/// Arrête tout : byebye SSDP, fermeture des sockets, fin et attente des fils.
pub fn cast_stop() {
    let Some(run) = RUN.lock().unwrap().take() else { return };
    // Le fil de gestion peut encore émettre : on laisse `stop` agir puis on envoie les byebye nous-mêmes.
    run.sh.stop.store(true, Ordering::SeqCst);
    for h in run.threads {
        let _ = h.join();
    }
    let ls: Vec<_> = run.listeners.lock().unwrap().drain(..).collect();
    for (_, h) in ls {
        let _ = h.join();
    }
    for _ in 0..2 {
        ssdp::send_notifies(&run.sh, &run.sock, false);
    }
    run.sh.subs.lock().unwrap().clear();
    // Les connexions en cours se terminent seules (délais courts) ; on les attend brièvement.
    for _ in 0..30 {
        if run.sh.conns.load(Ordering::SeqCst) == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Point d'entrée : démarre le casting seulement s'il est activé dans la configuration.
pub fn start(app: &AppHandle) {
    let cfg = load_cfg();
    if cfg.enabled {
        let _ = start_with(app, &cfg);
    }
}

fn start_with(app: &AppHandle, cfg: &Cfg) -> Result<(), String> {
    let pc = app.state::<crate::Shell>().core.cfg.lock().unwrap().pc_name.clone();
    let host: Arc<dyn Host> = Arc::new(AppHost { app: app.clone(), yt_launched: Mutex::new(None) });
    start_server(host, cfg, &pc)
}

// ---------- commandes ----------

fn view(app: &AppHandle) -> Value {
    let cfg = load_cfg();
    let pc = app.state::<crate::Shell>().core.cfg.lock().unwrap().pc_name.clone();
    let run = RUN.lock().unwrap();
    let (running, last, ev, ip) = match run.as_ref() {
        Some(r) => {
            let st = r.sh.status.lock().unwrap().clone();
            let ip = r.sh.ifaces.lock().unwrap().first().map(|i| i.ip.to_string()).unwrap_or_default();
            (!r.listeners.lock().unwrap().is_empty(), st.last_client, st.last_event, ip)
        }
        None => (false, String::new(), String::new(), ssdp::list_ifaces().first().map(|i| i.ip.to_string()).unwrap_or_default()),
    };
    json!({
        "enabled": cfg.enabled,
        "name": friendly_name(&cfg.name, &pc),
        "port": PORT,
        "ip": ip,
        "status": { "running": running, "lastClient": last, "lastEvent": ev },
    })
}

#[tauri::command]
pub async fn cast_get(app: AppHandle) -> Result<Value, String> {
    Ok(view(&app))
}

#[derive(Deserialize)]
pub struct CastSet {
    pub enabled: Option<bool>,
    pub name: Option<String>,
}

#[tauri::command]
pub async fn cast_set(app: AppHandle, cfg: CastSet) -> Result<Value, String> {
    let mut c = load_cfg();
    if let Some(e) = cfg.enabled {
        c.enabled = e;
    }
    if let Some(n) = cfg.name {
        c.name = n.chars().filter(|c| !c.is_control()).take(60).collect::<String>().trim().to_string();
    }
    crate::store::save("cast", &c)?;
    // Le démarrage (sockets, fils) et l'arrêt (attente des fils) bloquent : hors du fil asynchrone.
    let (app2, c2) = (app.clone(), c.clone());
    let res = tauri::async_runtime::spawn_blocking(move || {
        cast_stop();
        if c2.enabled {
            start_with(&app2, &c2)
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|_| "cast_failed".to_string())?;
    res?;
    Ok(view(&app))
}

#[cfg(test)]
pub mod testutil {
    use super::*;

    pub struct MockHost {
        pub log: Mutex<Vec<String>>,
        pub vol: Mutex<u8>,
        pub playing: Mutex<bool>,
        pub paused: Mutex<bool>,
        pub yt: Mutex<bool>,
    }

    impl Default for MockHost {
        fn default() -> Self {
            Self { log: Mutex::new(vec![]), vol: Mutex::new(40), playing: Mutex::new(false), paused: Mutex::new(false), yt: Mutex::new(false) }
        }
    }

    impl MockHost {
        pub fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
        fn push(&self, s: String) {
            self.log.lock().unwrap().push(s);
        }
    }

    impl Host for MockHost {
        fn play(&self, kind: &str, title: &str, url: &str) -> Result<(), String> {
            self.push(format!("play {kind} {title} {url}"));
            *self.playing.lock().unwrap() = true;
            *self.paused.lock().unwrap() = false;
            Ok(())
        }
        fn stop(&self) {
            self.push("stop".into());
            *self.playing.lock().unwrap() = false;
        }
        fn pause(&self, p: bool) {
            self.push(format!("pause {p}"));
            *self.paused.lock().unwrap() = p;
        }
        fn seek(&self, s: f64) {
            self.push(format!("seek {s}"));
        }
        fn player(&self) -> PlayerSnap {
            PlayerSnap { active: *self.playing.lock().unwrap(), paused: *self.paused.lock().unwrap(), position: 12.0, duration: 600.0 }
        }
        fn volume(&self) -> (u8, bool) {
            (*self.vol.lock().unwrap(), false)
        }
        fn set_volume(&self, v: u8) {
            self.push(format!("volume {v}"));
            *self.vol.lock().unwrap() = v;
        }
        fn launch_youtube(&self, url: Option<String>) -> Result<(), String> {
            self.push(format!("youtube {}", url.unwrap_or_default()));
            *self.yt.lock().unwrap() = true;
            Ok(())
        }
        fn youtube_running(&self) -> bool {
            *self.yt.lock().unwrap()
        }
        fn stop_youtube(&self) {
            self.push("stop youtube".into());
            *self.yt.lock().unwrap() = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::MockHost;
    use super::*;
    use std::collections::HashMap;

    fn shared() -> (Arc<MockHost>, Shared) {
        let h = Arc::new(MockHost::default());
        (h.clone(), Shared::new(h, "11111111-2222-4333-8444-555555555555".into(), "RusTV (PC)".into()))
    }

    fn req(method: &str, path: &str, peer: &str, headers: &[(&str, &str)], body: &str) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            query: String::new(),
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>(),
            body: body.as_bytes().to_vec(),
            peer: format!("{peer}:50000").parse().unwrap(),
            local: "192.168.1.2:47840".parse().unwrap(),
        }
    }

    fn header<'a>(r: &'a Response, k: &str) -> Option<&'a str> {
        r.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).map(|(_, v)| v.as_str())
    }

    #[test]
    fn uuid_and_names() {
        let u = new_uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert_ne!(u, new_uuid());
        assert_eq!(friendly_name("", "SALON"), "RusTV (SALON)");
        assert_eq!(friendly_name("  Cinéma\n", "x"), "Cinéma");
        assert_eq!(friendly_name("", " "), "RusTV (PC)");
    }

    #[test]
    fn description_and_application_url() {
        let (_, sh) = shared();
        let r = route(&sh, &req("GET", "/dd.xml", "192.168.1.9", &[], ""));
        assert_eq!(r.status, 200);
        assert_eq!(header(&r, "Application-URL"), Some("http://192.168.1.2:47840/apps/"));
        assert!(String::from_utf8_lossy(&r.body).contains("RusTV (PC)"));
    }

    #[test]
    fn refuses_public_sources() {
        let (h, sh) = shared();
        let body = "<s:Envelope><s:Body><u:Stop><InstanceID>0</InstanceID></u:Stop></s:Body></s:Envelope>";
        assert_eq!(route(&sh, &req("POST", "/dlna/avt/control", "8.8.8.8", &[], body)).status, 403);
        assert_eq!(route(&sh, &req("POST", "/apps/YouTube", "8.8.8.8", &[], "v=1")).status, 403);
        assert!(h.log().is_empty());
    }

    #[test]
    fn dial_lifecycle() {
        let (h, sh) = shared();
        let g = route(&sh, &req("GET", "/apps/YouTube", "192.168.1.9", &[], ""));
        assert!(String::from_utf8_lossy(&g.body).contains("<state>stopped</state>"));
        let p = route(&sh, &req("POST", "/apps/YouTube", "192.168.1.9", &[], "pairingCode=abc&theme=cl&v=XYZ&t=3"));
        assert_eq!(p.status, 201);
        assert_eq!(header(&p, "Location"), Some("http://192.168.1.2:47840/apps/YouTube/run"));
        assert_eq!(h.log(), vec!["youtube https://www.youtube.com/tv?pairingCode=abc&theme=cl&v=XYZ&t=3"]);
        let g = route(&sh, &req("GET", "/apps/YouTube", "192.168.1.9", &[], ""));
        assert!(String::from_utf8_lossy(&g.body).contains("<state>running</state>"));
        assert_eq!(route(&sh, &req("DELETE", "/apps/YouTube/run", "192.168.1.9", &[], "")).status, 200);
        assert_eq!(route(&sh, &req("DELETE", "/apps/YouTube/run", "192.168.1.9", &[], "")).status, 404);
        assert_eq!(route(&sh, &req("GET", "/apps/Netflix", "192.168.1.9", &[], "")).status, 404);
    }

    #[test]
    fn dial_blocks_browser_origin_and_bad_params() {
        let (h, sh) = shared();
        assert_eq!(route(&sh, &req("POST", "/apps/YouTube", "192.168.1.9", &[("origin", "https://evil.example")], "v=1")).status, 403);
        assert_eq!(route(&sh, &req("POST", "/apps/YouTube", "192.168.1.9", &[], "v=1<script>")).status, 400);
        assert!(h.log().is_empty());
    }

    #[test]
    fn dlna_routes() {
        let (h, sh) = shared();
        assert_eq!(route(&sh, &req("GET", "/dlna/avt/scpd.xml", "10.0.0.7", &[], "")).status, 200);
        assert_eq!(route(&sh, &req("GET", "/dlna/zzz/scpd.xml", "10.0.0.7", &[], "")).status, 404);
        let set = |u: &str| {
            format!("<s:Envelope xmlns:s=\"x\"><s:Body><u:SetAVTransportURI xmlns:u=\"y\"><InstanceID>0</InstanceID><CurrentURI>{u}</CurrentURI><CurrentURIMetaData></CurrentURIMetaData></u:SetAVTransportURI></s:Body></s:Envelope>")
        };
        let bad = route(&sh, &req("POST", "/dlna/avt/control", "10.0.0.7", &[], &set("file:///C:/a.mp4")));
        assert_eq!(bad.status, 500);
        assert!(String::from_utf8_lossy(&bad.body).contains("<errorCode>716</errorCode>"));
        let ok = route(&sh, &req("POST", "/dlna/avt/control", "10.0.0.7", &[("soapaction", "\"urn:schemas-upnp-org:service:AVTransport:1#SetAVTransportURI\"")], &set("http://10.0.0.7/a.mp4")));
        assert_eq!(ok.status, 200);
        assert!(String::from_utf8_lossy(&ok.body).contains("SetAVTransportURIResponse"));
        // SOAPACTION incohérent avec le corps.
        let mism = route(&sh, &req("POST", "/dlna/avt/control", "10.0.0.7", &[("soapaction", "\"urn:x#Play\"")], &set("http://10.0.0.7/a.mp4")));
        assert_eq!(mism.status, 500);
        let play = "<s:Envelope xmlns:s=\"x\"><s:Body><u:Play xmlns:u=\"y\"><InstanceID>0</InstanceID><Speed>1</Speed></u:Play></s:Body></s:Envelope>";
        assert_eq!(route(&sh, &req("POST", "/dlna/avt/control", "10.0.0.7", &[], play)).status, 200);
        assert_eq!(h.log(), vec!["play video a.mp4 http://10.0.0.7/a.mp4"]);
        assert_eq!(sh.status.lock().unwrap().last_client, "10.0.0.7");
    }

    #[test]
    fn subscribe_flow_and_limits() {
        let (_, sh) = shared();
        let cb = [("callback", "<http://10.0.0.7:5000/e>"), ("nt", "upnp:event"), ("timeout", "Second-120")];
        let r = route(&sh, &req("SUBSCRIBE", "/dlna/avt/event", "10.0.0.7", &cb, ""));
        assert_eq!(r.status, 200);
        assert_eq!(header(&r, "TIMEOUT"), Some("Second-120"));
        let sid = header(&r, "SID").unwrap().to_string();
        assert!(sid.starts_with("uuid:"));
        // Renouvellement, puis rappel vers un autre hôte refusé.
        assert_eq!(route(&sh, &req("SUBSCRIBE", "/dlna/avt/event", "10.0.0.7", &[("sid", &sid)], "")).status, 200);
        assert_eq!(route(&sh, &req("SUBSCRIBE", "/dlna/avt/event", "10.0.0.8", &cb, "")).status, 412);
        assert_eq!(route(&sh, &req("SUBSCRIBE", "/dlna/rc/event", "10.0.0.7", &[("sid", &sid)], "")).status, 412);
        assert_eq!(route(&sh, &req("UNSUBSCRIBE", "/dlna/avt/event", "10.0.0.7", &[("sid", &sid)], "")).status, 200);
        assert_eq!(route(&sh, &req("UNSUBSCRIBE", "/dlna/avt/event", "10.0.0.7", &[("sid", &sid)], "")).status, 412);
        // Limite de débit : au plus 20 nouveaux abonnements par minute.
        let mut codes = Vec::new();
        for _ in 0..25 {
            codes.push(route(&sh, &req("SUBSCRIBE", "/dlna/avt/event", "10.0.0.7", &cb, "")).status);
        }
        assert!(codes.contains(&503) || codes.contains(&429));
        assert!(sh.subs.lock().unwrap().len() <= 16);
    }
}
