//! Handshake mutuel par challenge-réponse HMAC puis session chiffrée.
//!
//! 1. C→S  Hello  { v, device_id, nc }                       (clair)
//! 2. S→C  Challenge { ns, proof_s = HMAC(K,"S"|id|nc|ns) }  (le PC prouve qu'il connaît K)
//! 3. C→S  Response  { proof_c = HMAC(K,"C"|id|nc|ns) }
//! 4. S→C  Verdict
//! Clés de session : HKDF(K, salt = nc|ns, info = "c2s" / "s2c"). La clé K ne circule jamais.
use crate::crypto::{derive_key, hmac, hmac_verify, Channel};
use crate::frame::{read_frame, write_frame};
use rand::RngCore;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Serialize, Deserialize)]
struct Hello {
    v: u32,
    device_id: String,
    nc: String,
}
#[derive(Serialize, Deserialize)]
struct Challenge {
    ns: String,
    proof_s: String,
}
#[derive(Serialize, Deserialize)]
struct Response {
    proof_c: String,
}
#[derive(Serialize, Deserialize)]
struct Verdict {
    ok: bool,
}

fn err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, msg.to_string())
}

fn b64(b: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(b)
}
fn unb64(s: &str) -> io::Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(s).map_err(|_| err("base64 invalide"))
}
fn send_json<W: Write, T: Serialize>(w: &mut W, v: &T) -> io::Result<()> {
    write_frame(w, &serde_json::to_vec(v).map_err(|e| err(&e.to_string()))?)
}
fn recv_json<R: Read, T: DeserializeOwned>(r: &mut R) -> io::Result<T> {
    serde_json::from_slice(&read_frame(r)?).map_err(|_| err("trame de handshake invalide"))
}

pub struct Session<S> {
    stream: S,
    tx: Channel,
    rx: Channel,
    pub peer_id: String,
}

impl<S: Read + Write> Session<S> {
    fn finish(stream: S, key: &[u8], nc: &[u8], ns: &[u8], peer_id: String, is_client: bool) -> Self {
        let salt = [nc, ns].concat();
        let c2s = Channel::new(&derive_key(key, &salt, b"c2s"));
        let s2c = Channel::new(&derive_key(key, &salt, b"s2c"));
        let (tx, rx) = if is_client { (c2s, s2c) } else { (s2c, c2s) };
        Self { stream, tx, rx, peer_id }
    }

    /// Côté téléphone (et tests).
    pub fn connect(mut stream: S, key: &[u8; 32], device_id: &str) -> io::Result<Self> {
        let mut nc = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nc);
        send_json(
            &mut stream,
            &Hello { v: crate::PROTOCOL_VERSION, device_id: device_id.into(), nc: b64(&nc) },
        )?;
        let ch: Challenge = recv_json(&mut stream)?;
        let ns = unb64(&ch.ns)?;
        if !hmac_verify(key, &[b"S", device_id.as_bytes(), &nc, &ns], &unb64(&ch.proof_s)?) {
            return Err(err("le serveur ne connaît pas la clé"));
        }
        let proof_c = hmac(key, &[b"C", device_id.as_bytes(), &nc, &ns]);
        send_json(&mut stream, &Response { proof_c: b64(&proof_c) })?;
        let v: Verdict = recv_json(&mut stream)?;
        if !v.ok {
            return Err(err("refusé par le PC"));
        }
        Ok(Self::finish(stream, key, &nc, &ns, device_id.into(), true))
    }

    /// Côté PC. `lookup` renvoie la clé de l'appareil s'il est appairé (sinon refus).
    pub fn accept(mut stream: S, lookup: impl Fn(&str) -> Option<[u8; 32]>) -> io::Result<Self> {
        let hello: Hello = recv_json(&mut stream)?;
        if hello.v != crate::PROTOCOL_VERSION {
            return Err(err("version incompatible"));
        }
        let nc = unb64(&hello.nc)?;
        let key = match lookup(&hello.device_id) {
            Some(k) => k,
            None => {
                let _ = send_json(&mut stream, &Verdict { ok: false });
                return Err(err("appareil non appairé"));
            }
        };
        let mut ns = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut ns);
        let proof_s = hmac(&key, &[b"S", hello.device_id.as_bytes(), &nc, &ns]);
        send_json(&mut stream, &Challenge { ns: b64(&ns), proof_s: b64(&proof_s) })?;
        let resp: Response = recv_json(&mut stream)?;
        if !hmac_verify(&key, &[b"C", hello.device_id.as_bytes(), &nc, &ns], &unb64(&resp.proof_c)?) {
            let _ = send_json(&mut stream, &Verdict { ok: false });
            return Err(err("preuve invalide"));
        }
        send_json(&mut stream, &Verdict { ok: true })?;
        Ok(Self::finish(stream, &key, &nc, &ns, hello.device_id, false))
    }

    pub fn send<T: Serialize>(&mut self, msg: &T) -> io::Result<()> {
        let plain = serde_json::to_vec(msg).map_err(|e| err(&e.to_string()))?;
        let ct = self.tx.seal(&plain);
        write_frame(&mut self.stream, &ct)
    }

    pub fn recv<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        let ct = read_frame(&mut self.stream)?;
        let pt = self
            .rx
            .open(&ct)
            .ok_or_else(|| err("déchiffrement échoué (altération ou rejeu)"))?;
        serde_json::from_slice(&pt).map_err(|_| err("message invalide"))
    }

    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Sépare en moitiés émission / réception. `read_half` doit être un clone du même flux
    /// (ex. `TcpStream::try_clone`) : l'original sert à écrire, le clone à lire.
    pub fn split(self, read_half: S) -> (SessionTx<S>, SessionRx<S>) {
        (
            SessionTx { stream: self.stream, tx: self.tx },
            SessionRx { stream: read_half, rx: self.rx },
        )
    }
}

pub struct SessionTx<S> {
    stream: S,
    tx: Channel,
}
pub struct SessionRx<S> {
    stream: S,
    rx: Channel,
}

impl<S: Write> SessionTx<S> {
    pub fn send<T: Serialize>(&mut self, msg: &T) -> io::Result<()> {
        let plain = serde_json::to_vec(msg).map_err(|e| err(&e.to_string()))?;
        let ct = self.tx.seal(&plain);
        write_frame(&mut self.stream, &ct)
    }
}

impl<S: Read> SessionRx<S> {
    pub fn recv<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        let ct = read_frame(&mut self.stream)?;
        let pt = self
            .rx
            .open(&ct)
            .ok_or_else(|| err("déchiffrement échoué (altération ou rejeu)"))?;
        serde_json::from_slice(&pt).map_err(|_| err("message invalide"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::*;
    use std::net::{TcpListener, TcpStream};

    const KEY: [u8; 32] = [7u8; 32];

    #[test]
    fn handshake_and_messages_over_tcp() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let srv = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let mut sess = Session::accept(s, |id| (id == "phone1").then_some(KEY)).unwrap();
            let m: PhoneMsg = sess.recv().unwrap();
            sess.send(&PcMsg::Pong).unwrap();
            m
        });
        let mut c = Session::connect(TcpStream::connect(addr).unwrap(), &KEY, "phone1").unwrap();
        c.send(&PhoneMsg::Key { k: Key::Up }).unwrap();
        assert_eq!(c.recv::<PcMsg>().unwrap(), PcMsg::Pong);
        assert_eq!(srv.join().unwrap(), PhoneMsg::Key { k: Key::Up });
    }

    #[test]
    fn unknown_device_and_wrong_key_rejected() {
        for (id, key) in [("intrus", KEY), ("phone1", [9u8; 32])] {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = l.local_addr().unwrap();
            let srv = std::thread::spawn(move || {
                let (s, _) = l.accept().unwrap();
                Session::accept(s, |i| (i == "phone1").then_some(KEY)).is_err()
            });
            assert!(Session::connect(TcpStream::connect(addr).unwrap(), &key, id).is_err());
            assert!(srv.join().unwrap());
        }
    }

    #[test]
    fn replay_and_tamper_detected() {
        let mut a = Channel::new(&[1u8; 32]);
        let mut b = Channel::new(&[1u8; 32]);
        let c1 = a.seal(b"un");
        let c2 = a.seal(b"deux");
        assert_eq!(b.open(&c1).unwrap(), b"un");
        assert!(b.open(&c1).is_none(), "rejeu refusé");
        let mut bad = c2.clone();
        bad[0] ^= 1;
        assert!(b.open(&bad).is_none(), "altération refusée");
        assert_eq!(b.open(&c2).unwrap(), b"deux");
    }

    #[test]
    fn message_json_shape() {
        let j = serde_json::to_string(&PhoneMsg::AltTab { a: AltTab::Open }).unwrap();
        assert_eq!(j, r#"{"t":"altTab","a":"open"}"#);
        let m: PhoneMsg = serde_json::from_str(r#"{"t":"launch","appId":"x"}"#).unwrap();
        assert_eq!(m, PhoneMsg::Launch { app_id: "x".into() });
    }
}
