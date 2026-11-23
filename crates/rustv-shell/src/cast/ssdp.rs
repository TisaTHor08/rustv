//! SSDP (découverte UPnP/DIAL) : texte des messages (pur, testé) et sockets multicast.
use socket2::{Domain, Protocol, SockAddr, SockRef, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const PORT: u16 = 1900;
pub const MAX_AGE: u32 = 1800;

pub const T_ROOT: &str = "upnp:rootdevice";
pub const T_MR: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";
pub const T_AVT: &str = "urn:schemas-upnp-org:service:AVTransport:1";
pub const T_RC: &str = "urn:schemas-upnp-org:service:RenderingControl:1";
pub const T_CM: &str = "urn:schemas-upnp-org:service:ConnectionManager:1";
pub const T_DIAL: &str = "urn:dial-multiscreen-org:service:dial:1";

/// Interface IPv4 privée de ce PC.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iface {
    pub ip: Ipv4Addr,
    pub mask: Ipv4Addr,
}

/// Interfaces IPv4 du réseau privé (RFC 1918) ; jamais d'adresse publique, de loopback ni de lien local.
pub fn list_ifaces() -> Vec<Iface> {
    let mut v: Vec<Iface> = Vec::new();
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        if let if_addrs::IfAddr::V4(a) = i.addr {
            if a.ip.is_private() && !a.ip.is_loopback() && !v.iter().any(|x| x.ip == a.ip) {
                v.push(Iface { ip: a.ip, mask: a.netmask });
            }
        }
    }
    v
}

/// Choisit l'interface dont le sous-réseau contient le demandeur ; à défaut, la première.
pub fn pick_iface(req: Ipv4Addr, ifaces: &[Iface]) -> Option<Iface> {
    let m = |i: &Iface| u32::from(i.ip) & u32::from(i.mask) == u32::from(req) & u32::from(i.mask);
    ifaces.iter().find(|i| m(i)).or_else(|| ifaces.first()).copied()
}

/// (cible NT/ST, USN) annoncés par l'appareil.
pub fn targets(uuid: &str) -> Vec<(String, String)> {
    let u = format!("uuid:{uuid}");
    let mut v = vec![(T_ROOT.to_string(), format!("{u}::{T_ROOT}")), (u.clone(), u.clone())];
    for t in [T_MR, T_AVT, T_RC, T_CM, T_DIAL] {
        v.push((t.to_string(), format!("{u}::{t}")));
    }
    v
}

#[derive(Debug, PartialEq)]
pub struct MSearch {
    pub st: String,
    pub mx: u32,
}

/// Analyse un M-SEARCH. `None` si ce n'en est pas un ou s'il est mal formé (MAN absent).
pub fn parse_msearch(text: &str) -> Option<MSearch> {
    let mut lines = text.split("\r\n");
    let first = lines.next()?;
    if !first.trim().eq_ignore_ascii_case("M-SEARCH * HTTP/1.1") {
        return None;
    }
    let (mut st, mut mx, mut man) = (None, 3u32, false);
    for l in lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        match k.trim().to_ascii_lowercase().as_str() {
            "st" => st = Some(v.to_string()),
            "mx" => mx = v.parse().unwrap_or(3).clamp(1, 5),
            "man" => man = v.trim_matches('"').eq_ignore_ascii_case("ssdp:discover"),
            _ => {}
        }
    }
    let st = st.filter(|s| !s.is_empty() && s.len() < 200)?;
    man.then_some(MSearch { st, mx })
}

/// Cibles qui répondent à un ST donné.
pub fn matching(uuid: &str, st: &str) -> Vec<(String, String)> {
    let all = targets(uuid);
    if st.eq_ignore_ascii_case("ssdp:all") {
        return all;
    }
    all.into_iter().filter(|(t, _)| t.eq_ignore_ascii_case(st)).collect()
}

pub fn search_response(st: &str, usn: &str, location: &str, server: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age={MAX_AGE}\r\nEXT:\r\nLOCATION: {location}\r\nSERVER: {server}\r\nST: {st}\r\nUSN: {usn}\r\n\r\n")
}

/// NOTIFY `ssdp:alive` (avec `location`) ou `ssdp:byebye` (sans).
pub fn notify(nt: &str, usn: &str, location: Option<&str>, server: &str) -> String {
    match location {
        Some(loc) => format!(
            "NOTIFY * HTTP/1.1\r\nHOST: {GROUP}:{PORT}\r\nCACHE-CONTROL: max-age={MAX_AGE}\r\nLOCATION: {loc}\r\nNT: {nt}\r\nNTS: ssdp:alive\r\nSERVER: {server}\r\nUSN: {usn}\r\n\r\n"
        ),
        None => format!("NOTIFY * HTTP/1.1\r\nHOST: {GROUP}:{PORT}\r\nNT: {nt}\r\nNTS: ssdp:byebye\r\nUSN: {usn}\r\n\r\n"),
    }
}

// ---------- sockets ----------

pub fn open_socket(ifaces: &[Iface]) -> std::io::Result<UdpSocket> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    // Le service SSDP de Windows écoute déjà sur ce port : REUSEADDR permet de partager.
    s.set_reuse_address(true)?;
    s.bind(&SockAddr::from(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT)))?;
    s.set_multicast_ttl_v4(2)?;
    let _ = s.set_multicast_loop_v4(false);
    s.set_read_timeout(Some(Duration::from_millis(500)))?;
    let sock: UdpSocket = s.into();
    for i in ifaces {
        join(&sock, i.ip);
    }
    Ok(sock)
}

/// Rejoint le groupe sur une interface (l'erreur « déjà membre » est ignorée).
pub fn join(sock: &UdpSocket, ip: Ipv4Addr) {
    let _ = SockRef::from(sock).join_multicast_v4(&GROUP, &ip);
}

fn location(i: &Iface) -> String {
    format!("http://{}:{}/dd.xml", i.ip, super::PORT)
}

/// Envoie alive (`true`) ou byebye (`false`) pour toutes les cibles, sur chaque interface.
pub fn send_notifies(sh: &super::Shared, sock: &UdpSocket, alive: bool) {
    let dest = SocketAddr::V4(SocketAddrV4::new(GROUP, PORT));
    for i in sh.ifaces.lock().unwrap().clone() {
        if SockRef::from(sock).set_multicast_if_v4(&i.ip).is_err() {
            continue;
        }
        let loc = location(&i);
        for (nt, usn) in targets(&sh.uuid) {
            let msg = notify(&nt, &usn, alive.then_some(loc.as_str()), &sh.server);
            let _ = sock.send_to(msg.as_bytes(), dest);
        }
    }
}

/// Boucle de réception des M-SEARCH. Rend la main quand `stop` est levé.
pub fn run_recv(sh: Arc<super::Shared>, sock: Arc<UdpSocket>) {
    let mut buf = [0u8; 2048];
    // Limite globale : 30 réponses (rafales de cibles comprises) par seconde, contre l'amplification.
    let (mut window, mut sent) = (Instant::now(), 0u32);
    while !sh.stop.load(Ordering::Relaxed) {
        let Ok((n, src)) = sock.recv_from(&mut buf) else { continue };
        let SocketAddr::V4(src) = src else { continue };
        if !super::http::is_lan(std::net::IpAddr::V4(*src.ip())) || src.port() == 0 {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&buf[..n]) else { continue };
        let Some(ms) = parse_msearch(text) else { continue };
        let found = matching(&sh.uuid, &ms.st);
        if found.is_empty() {
            continue;
        }
        let Some(iface) = pick_iface(*src.ip(), &sh.ifaces.lock().unwrap()) else { continue };
        if window.elapsed() > Duration::from_secs(1) {
            window = Instant::now();
            sent = 0;
        }
        if sent + found.len() as u32 > 30 {
            continue;
        }
        sent += found.len() as u32;
        // Petite attente aléatoire (la norme demande 0..MX s ; on reste court pour ne pas bloquer la boucle).
        std::thread::sleep(Duration::from_millis(rand::random::<u64>() % 150));
        let loc = location(&iface);
        for (st, usn) in found {
            let _ = sock.send_to(search_response(&st, &usn, &loc, &sh.server).as_bytes(), SocketAddr::V4(src));
        }
        sh.status.lock().unwrap().last_client = src.ip().to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U: &str = "11111111-2222-3333-4444-555555555555";

    fn msg(st: &str) -> String {
        format!("M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {st}\r\nUSER-AGENT: x\r\n\r\n")
    }

    #[test]
    fn parse_search() {
        assert_eq!(parse_msearch(&msg("ssdp:all")), Some(MSearch { st: "ssdp:all".into(), mx: 2 }));
        assert!(parse_msearch("NOTIFY * HTTP/1.1\r\n\r\n").is_none());
        assert!(parse_msearch("M-SEARCH * HTTP/1.1\r\nST: ssdp:all\r\n\r\n").is_none(), "MAN requis");
        assert!(parse_msearch("M-SEARCH * HTTP/1.1\r\nMAN: \"ssdp:discover\"\r\n\r\n").is_none(), "ST requis");
        assert_eq!(parse_msearch(&msg("x").replace("MX: 2", "MX: 999")).unwrap().mx, 5);
    }

    #[test]
    fn matching_targets() {
        assert_eq!(matching(U, "ssdp:all").len(), 7);
        for st in [T_ROOT, T_MR, T_AVT, T_RC, T_DIAL, "uuid:11111111-2222-3333-4444-555555555555"] {
            assert_eq!(matching(U, st).len(), 1, "{st}");
        }
        assert!(matching(U, "urn:other:service:x:1").is_empty());
        assert_eq!(matching(U, T_ROOT)[0].1, format!("uuid:{U}::upnp:rootdevice"));
        assert_eq!(matching(U, &format!("uuid:{U}"))[0].1, format!("uuid:{U}"));
    }

    #[test]
    fn response_text() {
        let r = search_response(T_DIAL, "uuid:a::x", "http://192.168.1.2:47840/dd.xml", "S/1");
        assert!(r.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(r.contains("CACHE-CONTROL: max-age=1800\r\n"));
        assert!(r.contains("LOCATION: http://192.168.1.2:47840/dd.xml\r\n"));
        assert!(r.contains(&format!("ST: {T_DIAL}\r\n")) && r.contains("USN: uuid:a::x\r\n") && r.contains("EXT:\r\n"));
        assert!(r.ends_with("\r\n\r\n"));
    }

    #[test]
    fn notify_text() {
        let a = notify(T_MR, "uuid:a::m", Some("http://10.0.0.2:47840/dd.xml"), "S/1");
        assert!(a.starts_with("NOTIFY * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\n"));
        assert!(a.contains("NTS: ssdp:alive\r\n") && a.contains("LOCATION: http://10.0.0.2:47840/dd.xml\r\n") && a.contains("CACHE-CONTROL: max-age=1800"));
        let b = notify(T_MR, "uuid:a::m", None, "S/1");
        assert!(b.contains("NTS: ssdp:byebye\r\n") && !b.contains("LOCATION"));
    }

    #[test]
    fn iface_choice() {
        let a = Iface { ip: "192.168.1.10".parse().unwrap(), mask: "255.255.255.0".parse().unwrap() };
        let b = Iface { ip: "10.0.0.5".parse().unwrap(), mask: "255.0.0.0".parse().unwrap() };
        assert_eq!(pick_iface("10.9.9.9".parse().unwrap(), &[a, b]), Some(b));
        assert_eq!(pick_iface("192.168.1.77".parse().unwrap(), &[b, a]), Some(a));
        assert_eq!(pick_iface("172.16.0.1".parse().unwrap(), &[a, b]), Some(a));
        assert_eq!(pick_iface("172.16.0.1".parse().unwrap(), &[]), None);
    }
}
