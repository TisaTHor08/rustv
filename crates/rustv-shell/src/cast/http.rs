//! Mini serveur HTTP/1.1 (std::net) pour le casting : une requête par connexion, tailles bornées.
//! Gère les méthodes non standard d'UPnP (SUBSCRIBE, UNSUBSCRIBE) ; pas de « chunked » ni de keep-alive.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

pub const MAX_HEAD: usize = 16 * 1024;
pub const MAX_BODY: usize = 256 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Request {
    pub method: String,
    /// Chemin sans la query.
    pub path: String,
    pub query: String,
    /// Noms en minuscules.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub peer: SocketAddr,
    /// Adresse locale de la connexion (sert à construire les URL renvoyées).
    pub local: SocketAddr,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self { status, headers: Vec::new(), body: Vec::new() }
    }
    pub fn xml(status: u16, body: String) -> Self {
        Self { status, headers: vec![("Content-Type".into(), "text/xml; charset=\"utf-8\"".into())], body: body.into_bytes() }
    }
    pub fn header(mut self, k: &str, v: impl Into<String>) -> Self {
        self.headers.push((k.to_string(), v.into()));
        self
    }
    pub fn text(status: u16, msg: &str) -> Self {
        Self { status, headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())], body: msg.as_bytes().to_vec() }
    }
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

/// Sérialise une réponse (toujours `Connection: close`).
pub fn response_bytes(r: &Response) -> Vec<u8> {
    let mut s = format!("HTTP/1.1 {} {}\r\n", r.status, reason(r.status));
    for (k, v) in &r.headers {
        // Pas d'injection de lignes dans les en-têtes.
        let v: String = v.chars().filter(|c| *c != '\r' && *c != '\n').collect();
        s.push_str(&format!("{k}: {v}\r\n"));
    }
    s.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", r.body.len()));
    let mut out = s.into_bytes();
    out.extend_from_slice(&r.body);
    out
}

/// Analyse la tête d'une requête (sans le corps) : méthode, chemin, query, en-têtes.
pub fn parse_head(head: &str) -> Result<(String, String, String, HashMap<String, String>), u16> {
    let mut lines = head.split("\r\n");
    let first = lines.next().ok_or(400u16)?;
    let mut parts = first.split(' ');
    let method = parts.next().filter(|m| !m.is_empty() && m.len() <= 16 && m.chars().all(|c| c.is_ascii_uppercase())).ok_or(400u16)?;
    let target = parts.next().ok_or(400u16)?;
    let version = parts.next().ok_or(400u16)?;
    if !version.starts_with("HTTP/1.") || parts.next().is_some() {
        return Err(400);
    }
    // Seule la forme « origin » (/chemin?query) est acceptée.
    if !target.starts_with('/') || target.len() > 1024 || target.chars().any(|c| c.is_control()) {
        return Err(400);
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut headers = HashMap::new();
    for l in lines {
        if l.is_empty() {
            break;
        }
        let (k, v) = l.split_once(':').ok_or(400u16)?;
        if headers.len() >= 40 {
            return Err(400);
        }
        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
    }
    Ok((method.to_string(), path.to_string(), query.to_string(), headers))
}

/// Lit une requête complète depuis le flux. Erreur = code HTTP à renvoyer (0 = couper sans répondre).
pub fn read_request(stream: &mut TcpStream) -> Result<Request, u16> {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let peer = stream.peer_addr().map_err(|_| 0u16)?;
    let local = stream.local_addr().map_err(|_| 0u16)?;
    let started = Instant::now();
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEAD {
            return Err(413);
        }
        if started.elapsed() > TOTAL_TIMEOUT {
            return Err(0);
        }
        let n = stream.read(&mut chunk).map_err(|_| 0u16)?;
        if n == 0 {
            return Err(0);
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = std::str::from_utf8(&buf[..head_end]).map_err(|_| 400u16)?;
    let (method, path, query, headers) = parse_head(head)?;
    if headers.get("transfer-encoding").is_some() {
        return Err(501);
    }
    let len: usize = match headers.get("content-length") {
        Some(v) => v.parse().map_err(|_| 400u16)?,
        None => 0,
    };
    if len > MAX_BODY {
        return Err(413);
    }
    let mut body = buf[head_end + 4..].to_vec();
    if body.len() > len {
        body.truncate(len);
    }
    while body.len() < len {
        if started.elapsed() > TOTAL_TIMEOUT {
            return Err(0);
        }
        let n = stream.read(&mut chunk).map_err(|_| 0u16)?;
        if n == 0 {
            return Err(0);
        }
        body.extend_from_slice(&chunk[..n]);
        body.truncate(len);
    }
    Ok(Request { method, path, query, headers, body, peer, local })
}

pub fn write_response(stream: &mut TcpStream, r: &Response) {
    let _ = stream.write_all(&response_bytes(r));
    let _ = stream.flush();
}

/// Vrai si l'adresse source est du réseau local (privée, lien local ou loopback). Réutilise `crate::lan`.
pub fn is_lan(ip: IpAddr) -> bool {
    // Une IPv4 mappée en IPv6 est ramenée à l'IPv4.
    let ip = match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    };
    crate::lan::is_private_host(&ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_ok() {
        let (m, p, q, h) = parse_head("SUBSCRIBE /a/b?x=1 HTTP/1.1\r\nHost: x\r\nCALLBACK: <http://1.2.3.4/>\r\n").unwrap();
        assert_eq!((m.as_str(), p.as_str(), q.as_str()), ("SUBSCRIBE", "/a/b", "x=1"));
        assert_eq!(h["callback"], "<http://1.2.3.4/>");
    }

    #[test]
    fn head_bad() {
        assert!(parse_head("get / HTTP/1.1\r\n").is_err());
        assert!(parse_head("GET http://evil/ HTTP/1.1\r\n").is_err());
        assert!(parse_head("GET / SPDY/3\r\n").is_err());
        assert!(parse_head("GET /\r\n").is_err());
        assert!(parse_head("GET / HTTP/1.1\r\nbroken\r\n").is_err());
    }

    #[test]
    fn response_has_length_and_no_injection() {
        let r = Response::new(200).header("X-A", "a\r\nInjected: 1");
        let s = String::from_utf8(response_bytes(&r)).unwrap();
        assert!(s.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(!s.contains("\r\nInjected"));
        assert!(s.contains("Content-Length: 0\r\n"));
    }

    #[test]
    fn lan_only() {
        assert!(is_lan("192.168.1.5".parse().unwrap()));
        assert!(is_lan("10.1.2.3".parse().unwrap()));
        assert!(is_lan("::ffff:192.168.1.5".parse().unwrap()));
        assert!(!is_lan("8.8.8.8".parse().unwrap()));
        assert!(!is_lan("2001:db8::1".parse().unwrap()));
        assert!(!is_lan("::ffff:8.8.8.8".parse().unwrap()));
    }
}
