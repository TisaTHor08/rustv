//! Télécommande web : petit serveur HTTP + WebSocket sur le réseau local privé.
//!
//! * Sert la coquille web de la télécommande (fichiers fournis par un `Assets`).
//! * `GET /ws` (mise à niveau WebSocket) transporte EXACTEMENT le même flux d'octets que la liaison TCP
//!   (trames `u32 | payload`, handshake HMAC mutuel, AES-256-GCM) : on réutilise `Session::accept` et
//!   `serve_session`. Aucune voie moins sûre : appareil non jumelé = refus, clé inconnue = refus.
//! * N'écoute que le loopback et les adresses IPv4 privées des interfaces ; refuse tout pair hors LAN,
//!   tout en-tête `Host` public (rebinding DNS) et toute `Origin` étrangère ; limite de débit par adresse.
//!
//! Désactivé par défaut : c'est le shell qui démarre et arrête ce serveur selon le réglage.
use crate::core::Core;
use crate::server::serve_session;
use rustv_protocol::session::Session;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const DEFAULT_PORT: u16 = 47822;

const MAX_HEAD: usize = 8 * 1024;
/// Plus gros message WebSocket accepté (une trame du protocole fait au plus 1 Mo).
const MAX_WS_PAYLOAD: u64 = 2 * 1024 * 1024;
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNS: usize = 32;
const MAX_PER_IP: usize = 8;
/// Échecs d'authentification tolérés par adresse et par fenêtre avant blocage.
const FAIL_LIMIT: u32 = 6;
const WINDOW: Duration = Duration::from_secs(60);
const BLOCK: Duration = Duration::from_secs(60);
/// Requêtes (statiques ou WebSocket) par adresse et par fenêtre.
const HIT_LIMIT: u32 = 300;

// ---------- fichiers statiques ----------

pub struct Asset {
    pub body: Vec<u8>,
    pub mime: String,
}

/// Fournisseur des fichiers de la coquille web (chemin relatif sans `/` initial, ex. `index.html`).
pub trait Assets: Send + Sync {
    fn get(&self, path: &str) -> Option<Asset>;
}

// ---------- réseau privé ----------

/// Vrai pour une adresse du réseau privé (RFC 1918, lien local, ULA IPv6) ou le loopback.
pub fn is_lan(ip: IpAddr) -> bool {
    let ip = match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    };
    match ip {
        IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local(),
        IpAddr::V6(v) => {
            let s = v.segments();
            v.is_loopback() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Valide l'en-tête `Host` : IP privée, `localhost`, nom `.local` ou nom sans point. Un nom public est refusé
/// (protection contre le rebinding DNS : une page d'un autre site ne peut pas atteindre ce serveur).
pub fn host_ok(host: &str) -> bool {
    let h = host.trim();
    let name = if let Some(rest) = h.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        match h.rsplit_once(':') {
            Some((n, p)) if p.chars().all(|c| c.is_ascii_digit()) && !n.contains(':') => n,
            _ => h,
        }
    }
    .to_ascii_lowercase();
    if name.is_empty() {
        return false;
    }
    if let Ok(ip) = name.parse::<IpAddr>() {
        return is_lan(ip);
    }
    name == "localhost" || name.ends_with(".local") || !name.contains('.')
}

/// Adresses sur lesquelles écouter : loopback + IPv4 privées des interfaces (jamais 0.0.0.0).
pub fn listen_ips() -> Vec<Ipv4Addr> {
    let mut v = vec![Ipv4Addr::LOCALHOST];
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        if let IpAddr::V4(ip) = i.ip() {
            if !ip.is_loopback() && ip.is_private() && !v.contains(&ip) {
                v.push(ip);
            }
        }
    }
    v
}

// ---------- limiteur de débit ----------

#[derive(Default)]
struct Entry {
    fails: u32,
    fail_since: Option<Instant>,
    blocked_until: Option<Instant>,
    hits: u32,
    hit_since: Option<Instant>,
    conns: usize,
}

#[derive(Default)]
struct Limiter {
    map: HashMap<IpAddr, Entry>,
}

impl Limiter {
    /// Admet une connexion de `ip` (et la compte) ; faux si bloquée, trop active ou trop de connexions ouvertes.
    fn admit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        if self.map.len() > 1024 {
            self.map.retain(|_, e| e.conns > 0 || e.blocked_until.is_some_and(|t| t > now));
        }
        let e = self.map.entry(ip).or_default();
        if e.blocked_until.is_some_and(|t| t > now) {
            return false;
        }
        if e.hit_since.is_none_or(|t| now.duration_since(t) > WINDOW) {
            e.hit_since = Some(now);
            e.hits = 0;
        }
        e.hits += 1;
        if e.hits > HIT_LIMIT || e.conns >= MAX_PER_IP {
            return false;
        }
        e.conns += 1;
        true
    }
    fn release(&mut self, ip: IpAddr) {
        if let Some(e) = self.map.get_mut(&ip) {
            e.conns = e.conns.saturating_sub(1);
        }
    }
    fn fail(&mut self, ip: IpAddr) {
        let now = Instant::now();
        let e = self.map.entry(ip).or_default();
        if e.fail_since.is_none_or(|t| now.duration_since(t) > WINDOW) {
            e.fail_since = Some(now);
            e.fails = 0;
        }
        e.fails += 1;
        if e.fails >= FAIL_LIMIT {
            e.blocked_until = Some(now + BLOCK);
            e.fails = 0;
        }
    }
}

// ---------- WebSocket (côté serveur) ----------

/// Flux d'octets au-dessus d'un WebSocket : chaque `write` devient un message binaire, la lecture
/// concatène les charges utiles des messages de données. Le protocole RusTV redécoupe lui-même en trames.
pub struct WsStream {
    sock: TcpStream,
    left: u64,
    mask: [u8; 4],
    mpos: usize,
}

impl WsStream {
    pub fn new(sock: TcpStream) -> Self {
        Self { sock, left: 0, mask: [0; 4], mpos: 0 }
    }
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self { sock: self.sock.try_clone()?, left: 0, mask: [0; 4], mpos: 0 })
    }
    pub fn socket(&self) -> &TcpStream {
        &self.sock
    }

    fn skip(&mut self, mut n: u64) -> io::Result<()> {
        let mut b = [0u8; 256];
        while n > 0 {
            let k = self.sock.read(&mut b[..n.min(256) as usize])?;
            if k == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            n -= k as u64;
        }
        Ok(())
    }
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

impl Read for WsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            if self.left > 0 {
                let n = buf.len().min(self.left as usize);
                let r = self.sock.read(&mut buf[..n])?;
                if r == 0 {
                    return Ok(0);
                }
                for b in &mut buf[..r] {
                    *b ^= self.mask[self.mpos & 3];
                    self.mpos += 1;
                }
                self.left -= r as u64;
                return Ok(r);
            }
            let mut h = [0u8; 2];
            match self.sock.read_exact(&mut h) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(0),
                Err(e) => return Err(e),
            }
            if h[0] & 0x70 != 0 {
                return Err(bad("bits RSV non gérés"));
            }
            if h[1] & 0x80 == 0 {
                return Err(bad("trame cliente non masquée"));
            }
            let op = h[0] & 0x0f;
            let len = match h[1] & 0x7f {
                126 => {
                    let mut b = [0u8; 2];
                    self.sock.read_exact(&mut b)?;
                    u64::from(u16::from_be_bytes(b))
                }
                127 => {
                    let mut b = [0u8; 8];
                    self.sock.read_exact(&mut b)?;
                    u64::from_be_bytes(b)
                }
                n => u64::from(n),
            };
            let mut m = [0u8; 4];
            self.sock.read_exact(&mut m)?;
            if len > MAX_WS_PAYLOAD {
                return Err(bad("message WebSocket trop grand"));
            }
            match op {
                0 | 1 | 2 => {
                    self.mask = m;
                    self.mpos = 0;
                    self.left = len;
                }
                8 => return Ok(0), // fermeture
                9 | 10 => {
                    if len > 125 {
                        return Err(bad("trame de contrôle trop grande"));
                    }
                    self.skip(len)?; // ping/pong ignorés : le protocole a son propre ping
                }
                _ => return Err(bad("opcode inconnu")),
            }
        }
    }
}

impl Write for WsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut f = Vec::with_capacity(buf.len() + 10);
        f.push(0x82); // FIN + binaire
        match buf.len() {
            n if n < 126 => f.push(n as u8),
            n if n <= 0xffff => {
                f.push(126);
                f.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                f.push(127);
                f.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        f.extend_from_slice(buf);
        self.sock.write_all(&f)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.sock.flush()
    }
}

fn accept_key(key: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let mut h = Sha1::new();
    h.update(key.trim().as_bytes());
    h.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    STANDARD.encode(h.finalize())
}

// ---------- HTTP minimal ----------

struct Req {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

/// Lit l'en-tête de la requête (octet par octet : aucun octet de trop n'est consommé).
fn read_head(s: &mut TcpStream) -> Result<Req, u16> {
    let mut head = Vec::with_capacity(512);
    let mut b = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= MAX_HEAD {
            return Err(431);
        }
        match s.read(&mut b) {
            Ok(1) => head.push(b[0]),
            _ => return Err(0),
        }
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let mut lines = text.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (Some(method), Some(target), Some(ver)) = (first.next(), first.next(), first.next()) else { return Err(400) };
    if !ver.starts_with("HTTP/1.") {
        return Err(400);
    }
    let mut headers = HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();
    Ok(Req { method: method.to_string(), path, headers })
}

fn reason(c: u16) -> &'static str {
    match c {
        101 => "Switching Protocols",
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; \
connect-src 'self' ws: wss:; media-src 'self' blob:; worker-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";

fn respond(s: &mut TcpStream, code: u16, mime: &str, cache: &str, body: &[u8], head_only: bool) {
    let mut h = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: {CSP}\r\nConnection: close\r\n\r\n",
        reason(code),
        body.len()
    )
    .into_bytes();
    if !head_only {
        h.extend_from_slice(body);
    }
    let _ = s.write_all(&h);
    let _ = s.flush();
}

fn text(s: &mut TcpStream, code: u16, msg: &str) {
    respond(s, code, "text/plain; charset=utf-8", "no-store", msg.as_bytes(), false);
}

// ---------- serveur ----------

struct Client {
    id: u64,
    device: String,
    ip: IpAddr,
    since: Instant,
    sock: TcpStream,
}

/// Appareil connecté par la télécommande web.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClientInfo {
    pub device: String,
    pub name: String,
    pub ip: String,
    pub secs: u64,
}

struct Shared {
    core: Arc<Core>,
    assets: Arc<dyn Assets>,
    stop: AtomicBool,
    limiter: Mutex<Limiter>,
    clients: Mutex<Vec<Client>>,
    total: AtomicUsize,
    next: AtomicU64,
    bound: Mutex<Vec<SocketAddr>>,
}

pub struct WebRemote {
    sh: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl WebRemote {
    /// Écoute le loopback et les IPv4 privées sur `port`. Erreur si aucune adresse n'a pu être ouverte.
    pub fn start(core: Arc<Core>, assets: Arc<dyn Assets>, port: u16) -> io::Result<Self> {
        Self::start_with(core, assets, port, listen_ips, true)
    }

    /// Variante pour les tests : adresses fixes, sans rescan des interfaces.
    pub fn start_on(core: Arc<Core>, assets: Arc<dyn Assets>, ips: Vec<Ipv4Addr>, port: u16) -> io::Result<Self> {
        Self::start_with(core, assets, port, move || ips.clone(), false)
    }

    fn start_with(
        core: Arc<Core>,
        assets: Arc<dyn Assets>,
        port: u16,
        ips: impl Fn() -> Vec<Ipv4Addr> + Send + 'static,
        rescan: bool,
    ) -> io::Result<Self> {
        let sh = Arc::new(Shared {
            core,
            assets,
            stop: AtomicBool::new(false),
            limiter: Mutex::new(Limiter::default()),
            clients: Mutex::new(vec![]),
            total: AtomicUsize::new(0),
            next: AtomicU64::new(0),
            bound: Mutex::new(vec![]),
        });
        let mut ls: Vec<TcpListener> = vec![];
        bind_new(&sh, &ips(), port, &mut ls);
        if ls.is_empty() {
            return Err(io::Error::new(io::ErrorKind::AddrInUse, "port indisponible"));
        }
        let s2 = sh.clone();
        let thread = std::thread::Builder::new().name("webremote".into()).spawn(move || run(s2, ls, port, ips, rescan))?;
        Ok(Self { sh, thread: Some(thread) })
    }

    pub fn addrs(&self) -> Vec<SocketAddr> {
        self.sh.bound.lock().unwrap().clone()
    }

    pub fn clients(&self) -> Vec<ClientInfo> {
        let cfg = self.sh.core.cfg.lock().unwrap();
        self.sh
            .clients
            .lock()
            .unwrap()
            .iter()
            .map(|c| ClientInfo {
                device: c.device.clone(),
                name: cfg.devices.iter().find(|d| d.id == c.device).map(|d| d.name.clone()).unwrap_or_default(),
                ip: c.ip.to_string(),
                secs: c.since.elapsed().as_secs(),
            })
            .collect()
    }

    /// Arrête l'écoute, coupe les sessions web et attend la fin du fil d'acceptation.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.sh.stop.store(true, Ordering::SeqCst);
        for c in self.sh.clients.lock().unwrap().iter() {
            let _ = c.sock.shutdown(Shutdown::Both);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        for _ in 0..30 {
            if self.sh.total.load(Ordering::SeqCst) == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for WebRemote {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn bind_new(sh: &Shared, ips: &[Ipv4Addr], port: u16, ls: &mut Vec<TcpListener>) {
    for ip in ips {
        if sh.bound.lock().unwrap().iter().any(|a| a.ip() == IpAddr::V4(*ip)) {
            continue;
        }
        if let Ok(l) = TcpListener::bind(SocketAddr::from((*ip, port))) {
            if l.set_nonblocking(true).is_ok() {
                if let Ok(a) = l.local_addr() {
                    sh.bound.lock().unwrap().push(a);
                    ls.push(l);
                }
            }
        }
    }
}

fn run(sh: Arc<Shared>, mut ls: Vec<TcpListener>, port: u16, ips: impl Fn() -> Vec<Ipv4Addr>, rescan: bool) {
    let mut last = Instant::now();
    while !sh.stop.load(Ordering::SeqCst) {
        let mut idle = true;
        for l in &ls {
            if let Ok((s, peer)) = l.accept() {
                idle = false;
                if !is_lan(peer.ip()) {
                    continue; // jamais d'hôte hors réseau local
                }
                if sh.total.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
                    sh.total.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let s2 = sh.clone();
                if std::thread::Builder::new().name("webremote-conn".into()).spawn(move || {
                    handle(&s2, s, peer.ip());
                    s2.total.fetch_sub(1, Ordering::SeqCst);
                }).is_err()
                {
                    sh.total.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
        if rescan && last.elapsed() > Duration::from_secs(5) {
            last = Instant::now();
            bind_new(&sh, &ips(), port, &mut ls);
        }
        if idle {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn handle(sh: &Arc<Shared>, mut s: TcpStream, ip: IpAddr) {
    let _ = s.set_nonblocking(false);
    let _ = s.set_nodelay(true);
    let _ = s.set_read_timeout(Some(IO_TIMEOUT));
    let _ = s.set_write_timeout(Some(IO_TIMEOUT));
    if !sh.limiter.lock().unwrap().admit(ip) {
        text(&mut s, 429, "Too many requests");
        return;
    }
    serve(sh, s, ip);
    sh.limiter.lock().unwrap().release(ip);
}

fn serve(sh: &Arc<Shared>, mut s: TcpStream, ip: IpAddr) {
    let req = match read_head(&mut s) {
        Ok(r) => r,
        Err(0) => return,
        Err(c) => return text(&mut s, c, ""),
    };
    let host = req.headers.get("host").map(String::as_str).unwrap_or("");
    if !host_ok(host) {
        return text(&mut s, 403, "Forbidden");
    }
    let is_ws = req.headers.get("upgrade").is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
    if is_ws {
        if req.path != "/ws" || req.method != "GET" {
            return text(&mut s, 404, "Not found");
        }
        // Origin : si présente, elle doit être celle du serveur lui-même (pas de page tierce).
        if let Some(o) = req.headers.get("origin") {
            let same = o.strip_prefix("http://").or_else(|| o.strip_prefix("https://")) == Some(host);
            if !same {
                return text(&mut s, 403, "Forbidden");
            }
        }
        let ok = req.headers.get("sec-websocket-version").map(String::as_str) == Some("13");
        let Some(key) = req.headers.get("sec-websocket-key").filter(|_| ok) else { return text(&mut s, 400, "Bad request") };
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept_key(key)
        );
        if s.write_all(resp.as_bytes()).is_err() {
            return;
        }
        return ws_session(sh, s, ip);
    }
    if req.method != "GET" && req.method != "HEAD" {
        return text(&mut s, 405, "Method not allowed");
    }
    let rel = req.path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel.split('/').any(|p| p.is_empty() || p == "." || p == "..") || rel.contains(['\\', '\0', ':']) {
        return text(&mut s, 404, "Not found");
    }
    match sh.assets.get(rel) {
        Some(a) => {
            let cache = if rel.starts_with("assets/") { "public, max-age=86400" } else { "no-cache" };
            respond(&mut s, 200, &a.mime, cache, &a.body, req.method == "HEAD");
        }
        None if rel == "index.html" => text(&mut s, 503, "La telecommande web n'est pas incluse dans cette compilation de RusTV."),
        None => text(&mut s, 404, "Not found"),
    }
}

fn ws_session(sh: &Arc<Shared>, s: TcpStream, ip: IpAddr) {
    let ws = WsStream::new(s);
    let Ok(read_half) = ws.try_clone() else { return };
    let Ok(closer) = ws.socket().try_clone() else { return };
    let Ok(reg) = ws.socket().try_clone() else { return };
    let core = sh.core.clone();
    let lookup = {
        let core = core.clone();
        move |id: &str| core.cfg.lock().unwrap().key_of(id)
    };
    let sess = match Session::accept(ws, lookup) {
        Ok(s) => s,
        Err(_) => {
            sh.limiter.lock().unwrap().fail(ip);
            return;
        }
    };
    let _ = read_half.socket().set_read_timeout(Some(IDLE_TIMEOUT));
    let id = sh.next.fetch_add(1, Ordering::SeqCst);
    sh.clients.lock().unwrap().push(Client { id, device: sess.peer_id.clone(), ip, since: Instant::now(), sock: reg });
    if sh.stop.load(Ordering::SeqCst) {
        let _ = closer.shutdown(Shutdown::Both);
    }
    serve_session(core, sess, read_half, move || {
        let _ = closer.shutdown(Shutdown::Both);
    });
    sh.clients.lock().unwrap().retain(|c| c.id != id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Device};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use rustv_protocol::messages::{PcMsg, PhoneMsg};

    const KEY: [u8; 32] = [5u8; 32];

    struct Files;
    impl Assets for Files {
        fn get(&self, p: &str) -> Option<Asset> {
            match p {
                "index.html" => Some(Asset { body: b"<html>ok</html>".to_vec(), mime: "text/html; charset=utf-8".into() }),
                "assets/a.js" => Some(Asset { body: b"1".to_vec(), mime: "text/javascript".into() }),
                _ => None,
            }
        }
    }
    struct NoFiles;
    impl Assets for NoFiles {
        fn get(&self, _: &str) -> Option<Asset> {
            None
        }
    }

    fn core() -> Arc<Core> {
        let dev = Device { id: "phone1".into(), name: "Navigateur test".into(), key: STANDARD.encode(KEY), used: true };
        Arc::new(Core::new(Config { pc_id: "t".into(), pc_name: "t".into(), devices: vec![dev], custom_apps: vec![] }))
    }

    fn server(assets: Arc<dyn Assets>) -> (WebRemote, SocketAddr, Arc<Core>) {
        let c = core();
        let w = WebRemote::start_on(c.clone(), assets, vec![Ipv4Addr::LOCALHOST], 0).unwrap();
        let a = w.addrs()[0];
        (w, a, c)
    }

    /// Client WebSocket minimal : écritures masquées, lectures non masquées (RFC 6455).
    struct WsClient {
        s: TcpStream,
        left: usize,
    }
    impl WsClient {
        fn connect_with(addr: SocketAddr, host: &str, origin: Option<&str>) -> Result<Self, String> {
            let mut s = TcpStream::connect(addr).map_err(|e| e.to_string())?;
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let o = origin.map(|o| format!("Origin: {o}\r\n")).unwrap_or_default();
            s.write_all(format!("GET /ws HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n{o}\r\n").as_bytes()).unwrap();
            let mut head = Vec::new();
            let mut b = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if s.read(&mut b).map_err(|e| e.to_string())? == 0 {
                    return Err("fermé".into());
                }
                head.push(b[0]);
            }
            let h = String::from_utf8_lossy(&head).to_string();
            if !h.starts_with("HTTP/1.1 101") {
                return Err(h.lines().next().unwrap_or("").to_string());
            }
            // Exemple de la RFC 6455 §1.3.
            assert!(h.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="), "{h}");
            Ok(Self { s, left: 0 })
        }
        fn connect(addr: SocketAddr) -> Result<Self, String> {
            Self::connect_with(addr, &addr.to_string(), None)
        }
    }
    impl Write for WsClient {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mask = [1u8, 2, 3, 4];
            let mut f = vec![0x82];
            match buf.len() {
                n if n < 126 => f.push(0x80 | n as u8),
                n => {
                    f.push(0x80 | 126);
                    f.extend_from_slice(&(n as u16).to_be_bytes());
                }
            }
            f.extend_from_slice(&mask);
            f.extend(buf.iter().enumerate().map(|(i, b)| b ^ mask[i & 3]));
            self.s.write_all(&f)?;
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Read for WsClient {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.left == 0 {
                let mut h = [0u8; 2];
                self.s.read_exact(&mut h)?;
                self.left = match h[1] & 0x7f {
                    126 => {
                        let mut b = [0u8; 2];
                        self.s.read_exact(&mut b)?;
                        u16::from_be_bytes(b) as usize
                    }
                    n => n as usize,
                };
            }
            let n = buf.len().min(self.left);
            let r = self.s.read(&mut buf[..n])?;
            self.left -= r;
            Ok(r)
        }
    }

    fn http_get(addr: SocketAddr, path: &str, host: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).unwrap();
        let mut out = Vec::new();
        let _ = s.read_to_end(&mut out);
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn paired_device_handshake_and_ping() {
        let (w, addr, _c) = server(Arc::new(Files));
        let ws = WsClient::connect(addr).unwrap();
        let mut sess = Session::connect(ws, &KEY, "phone1").expect("handshake");
        sess.send(&PhoneMsg::Ping).unwrap();
        // Le serveur pousse d'abord son état ; on attend le Pong.
        let mut got = false;
        for _ in 0..20 {
            if sess.recv::<PcMsg>().unwrap() == PcMsg::Pong {
                got = true;
                break;
            }
        }
        assert!(got);
        assert_eq!(w.clients().len(), 1);
        assert_eq!(w.clients()[0].name, "Navigateur test");
        w.stop();
    }

    #[test]
    fn unpaired_or_wrong_key_refused_then_blocked() {
        let (w, addr, _c) = server(Arc::new(Files));
        for (id, key) in [("intrus", KEY), ("phone1", [9u8; 32])] {
            let ws = WsClient::connect(addr).unwrap();
            assert!(Session::connect(ws, &key, id).is_err(), "{id} doit être refusé");
        }
        assert!(w.clients().is_empty());
        // Les échecs répétés bloquent l'adresse (limite de débit).
        for _ in 0..FAIL_LIMIT {
            if let Ok(ws) = WsClient::connect(addr) {
                let _ = Session::connect(ws, &KEY, "inconnu");
            }
        }
        assert!(WsClient::connect(addr).is_err(), "adresse bloquée");
        assert!(http_get(addr, "/", &addr.to_string()).contains("429"));
        w.stop();
    }

    #[test]
    fn rejects_foreign_host_and_origin() {
        let (w, addr, _c) = server(Arc::new(Files));
        assert!(http_get(addr, "/", "evil.example.com").starts_with("HTTP/1.1 403"));
        assert!(http_get(addr, "/", "8.8.8.8").starts_with("HTTP/1.1 403"));
        assert!(WsClient::connect_with(addr, &addr.to_string(), Some("http://evil.example.com")).is_err());
        assert!(WsClient::connect_with(addr, &addr.to_string(), Some(&format!("http://{addr}"))).is_ok());
        w.stop();
    }

    #[test]
    fn static_files_and_traversal() {
        let (w, addr, _c) = server(Arc::new(Files));
        let host = addr.to_string();
        let r = http_get(addr, "/", &host);
        assert!(r.starts_with("HTTP/1.1 200") && r.contains("<html>ok</html>") && r.contains("Content-Security-Policy"), "{r}");
        assert!(http_get(addr, "/assets/a.js?v=1", &host).contains("max-age"));
        assert!(http_get(addr, "/../Cargo.toml", &host).starts_with("HTTP/1.1 404"));
        assert!(http_get(addr, "/a/../index.html", &host).starts_with("HTTP/1.1 404"));
        assert!(http_get(addr, "/nope", &host).starts_with("HTTP/1.1 404"));
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_all(format!("POST / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).unwrap();
        let mut o = String::new();
        let _ = s.read_to_string(&mut o);
        assert!(o.starts_with("HTTP/1.1 405"));
        w.stop();
        let (w, addr, _c) = server(Arc::new(NoFiles));
        assert!(http_get(addr, "/", &addr.to_string()).starts_with("HTTP/1.1 503"));
        w.stop();
    }

    #[test]
    fn stop_frees_port_and_closes_sessions() {
        let (w, addr, _c) = server(Arc::new(Files));
        let ws = WsClient::connect(addr).unwrap();
        let mut sess = Session::connect(ws, &KEY, "phone1").unwrap();
        w.stop();
        assert!((0..30).any(|_| sess.recv::<PcMsg>().is_err()), "session coupée à l'arrêt");
        assert!(TcpListener::bind(addr).is_ok(), "port libéré");
    }

    #[test]
    fn host_and_lan_rules() {
        for h in ["192.168.1.4:47822", "10.0.0.2", "localhost:1", "[::1]:80", "tv-pc", "tv.local:47822", "127.0.0.1"] {
            assert!(host_ok(h), "{h}");
        }
        for h in ["", "example.com", "8.8.8.8:80", "a.b.c", "[2001:db8::1]:80"] {
            assert!(!host_ok(h), "{h}");
        }
        assert!(is_lan("::ffff:192.168.0.1".parse().unwrap()) && !is_lan("1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn websocket_framing() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let mut ws = WsStream::new(s);
            let mut out = Vec::new();
            let mut b = [0u8; 64];
            loop {
                match ws.read(&mut b) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => out.extend_from_slice(&b[..n]),
                }
            }
            out
        });
        let mut c = TcpStream::connect(addr).unwrap();
        let m = [9u8, 8, 7, 6];
        let mask = |p: &[u8]| -> Vec<u8> { p.iter().enumerate().map(|(i, b)| b ^ m[i & 3]).collect() };
        // Message fragmenté (texte puis continuation), ping intercalé, puis message binaire.
        c.write_all(&[&[0x01, 0x80 | 3][..], &m, &mask(b"abc")].concat()).unwrap();
        c.write_all(&[&[0x89, 0x80 | 2][..], &m, &mask(b"hi")].concat()).unwrap();
        c.write_all(&[&[0x80, 0x80 | 2][..], &m, &mask(b"de")].concat()).unwrap();
        c.write_all(&[&[0x82, 0x80 | 1][..], &m, &mask(b"f")].concat()).unwrap();
        c.write_all(&[0x88, 0x80, 0, 0, 0, 0]).unwrap();
        assert_eq!(t.join().unwrap(), b"abcdef");
        // Trame non masquée : refusée.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let t = std::thread::spawn(move || WsStream::new(l.accept().unwrap().0).read(&mut [0u8; 8]).is_err());
        TcpStream::connect(addr).unwrap().write_all(&[0x82, 0x01, 0x00]).unwrap();
        assert!(t.join().unwrap());
    }
}
