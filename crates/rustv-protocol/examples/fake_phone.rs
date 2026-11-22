//! Faux téléphone pour tester le service : cargo run -p rustv-protocol --example fake_phone -- <ip:port> <device_id> <key_base64>
use base64::{engine::general_purpose::STANDARD, Engine};
use rustv_protocol::messages::*;
use rustv_protocol::session::Session;
use std::net::TcpStream;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let key: [u8; 32] = STANDARD.decode(&a[3]).unwrap().try_into().unwrap();
    let s = TcpStream::connect(&a[1]).unwrap();
    let mut sess = match Session::connect(s, &key, &a[2]) {
        Ok(s) => s,
        Err(e) => {
            println!("REFUSE: {e}");
            std::process::exit(2);
        }
    };
    println!("authentifié");
    for _ in 0..2 {
        match sess.recv::<PcMsg>().unwrap() {
            PcMsg::State { pinned, text_focus } => println!("State pinned={pinned} textFocus={text_focus}"),
            PcMsg::Apps { apps } => println!("Apps: {} (ex: {:?})", apps.len(), apps.iter().take(3).map(|a| &a.name).collect::<Vec<_>>()),
            m => println!("{m:?}"),
        }
    }
    sess.send(&PhoneMsg::Ping).unwrap();
    println!("réponse au ping: {:?}", sess.recv::<PcMsg>().unwrap());
}
