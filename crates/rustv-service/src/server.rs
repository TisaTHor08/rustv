//! Transport TCP + beacon UDP de découverte. Le traitement d'une session est générique
//! (`serve_session`) pour pouvoir brancher aussi Bluetooth RFCOMM.
use crate::core::Core;
use rustv_protocol::messages::{PcMsg, PhoneMsg};
use rustv_protocol::session::Session;
use rustv_protocol::{BEACON_PORT, TCP_PORT};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Sans ping du téléphone pendant ce délai, la connexion est considérée morte.
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

pub fn serve_session<S: Read + Write + Send + 'static>(
    core: Arc<Core>,
    sess: Session<S>,
    read_half: S,
    close: impl Fn() + Send + 'static,
) {
    let device = sess.peer_id.clone();
    let (mut tx, mut rx) = sess.split(read_half);
    let (out_tx, out_rx) = mpsc::channel::<PcMsg>();
    let id = core.add_conn(out_tx.clone(), &device);

    // Écrivain : seul propriétaire de la moitié émission.
    std::thread::spawn(move || {
        while let Ok(m) = out_rx.recv() {
            if tx.send(&m).is_err() {
                break;
            }
        }
        close();
    });

    // Lecteur : bloque sur le flux ; une erreur (timeout, coupure, trame altérée) termine tout.
    loop {
        match rx.recv::<PhoneMsg>() {
            Ok(m) => {
                // Appairage révoqué depuis la fenêtre : on coupe à la prochaine trame (≤ 4 s : ping).
                if core.cfg.lock().unwrap().key_of(&device).is_none() {
                    break;
                }
                core.handle(m, &out_tx)
            }
            Err(_) => break,
        }
    }
    core.remove_conn(id);
    // Fermer le canal réveille l'écrivain, qui ferme le socket.
}

pub fn run_tcp(core: Arc<Core>, listener: TcpListener) {
    for stream in listener.incoming().flatten() {
        let core = core.clone();
        std::thread::spawn(move || {
            let _ = stream.set_nodelay(true);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5))); // handshake
            let Ok(rd) = stream.try_clone() else { return };
            let Ok(closer) = stream.try_clone() else { return };
            let lookup = {
                let core = core.clone();
                move |id: &str| core.cfg.lock().unwrap().key_of(id)
            };
            let Ok(sess) = Session::accept(stream, lookup) else { return };
            let _ = rd.set_read_timeout(Some(IDLE_TIMEOUT));
            serve_session(core, sess, rd, move || {
                let _ = closer.shutdown(std::net::Shutdown::Both);
            });
        });
    }
}

pub fn bind() -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], TCP_PORT)))
}

pub fn local_ips() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(v4) if !v4.is_link_local() => Some(v4.to_string()),
            _ => None,
        })
        .collect()
}

/// Beacon UDP broadcast toutes les 2 s sur chaque interface IPv4.
pub fn run_beacon(core: Arc<Core>) {
    loop {
        let payload = {
            let c = core.cfg.lock().unwrap();
            serde_json::json!({ "app": "rustv", "id": c.pc_id, "name": c.pc_name, "port": TCP_PORT }).to_string()
        };
        for iface in if_addrs::get_if_addrs().unwrap_or_default() {
            if let if_addrs::IfAddr::V4(v4) = &iface.addr {
                if v4.ip.is_loopback() {
                    continue;
                }
                let Some(bc) = v4.broadcast else { continue };
                if let Ok(sock) = UdpSocket::bind((v4.ip, 0)) {
                    let _ = sock.set_broadcast(true);
                    let _ = sock.send_to(payload.as_bytes(), (bc, BEACON_PORT));
                }
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}
