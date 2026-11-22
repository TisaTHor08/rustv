//! HMAC-SHA256, HKDF et canal AES-256-GCM avec compteur anti-rejeu.
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac accepte toute taille");
    for p in parts {
        m.update(p);
    }
    m.finalize().into_bytes().into()
}

/// Comparaison en temps constant.
pub fn hmac_verify(key: &[u8], parts: &[&[u8]], tag: &[u8]) -> bool {
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac accepte toute taille");
    for p in parts {
        m.update(p);
    }
    m.verify_slice(tag).is_ok()
}

pub fn derive_key(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), secret);
    let mut out = [0u8; 32];
    hk.expand(info, &mut out).expect("32 octets valides");
    out
}

/// Sens unique : un compteur strictement croissant sert de nonce.
/// Le transport étant ordonné (TCP/RFCOMM), le compteur est implicite : tout rejeu,
/// perte ou réordonnancement fait échouer le déchiffrement.
pub struct Channel {
    cipher: Aes256Gcm,
    counter: u64,
}

impl Channel {
    pub fn new(key: &[u8; 32]) -> Self {
        Self { cipher: Aes256Gcm::new(key.into()), counter: 0 }
    }
    fn nonce(&self) -> [u8; 12] {
        let mut n = [0u8; 12];
        n[4..].copy_from_slice(&self.counter.to_be_bytes());
        n
    }
    pub fn seal(&mut self, plain: &[u8]) -> Vec<u8> {
        let n = self.nonce();
        let ct = self.cipher.encrypt(Nonce::from_slice(&n), plain).expect("chiffrement");
        self.counter += 1;
        ct
    }
    pub fn open(&mut self, ct: &[u8]) -> Option<Vec<u8>> {
        let n = self.nonce();
        let pt = self.cipher.decrypt(Nonce::from_slice(&n), ct).ok()?;
        self.counter += 1; // n'avance que si l'authentification réussit
        Some(pt)
    }
}
