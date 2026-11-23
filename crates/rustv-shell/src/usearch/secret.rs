//! Protection des jetons (TMDB, Plex, Jellyfin) : DPAPI (clé liée au compte Windows), derrière un trait pour
//! tester l'aller-retour sans Windows. Les jetons ne sont jamais journalisés ni renvoyés à l'interface.
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

pub trait Protector {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String>;
    fn unprotect(&self, sealed: &[u8]) -> Result<Vec<u8>, String>;
}

/// Entropie propre à ce module : un autre programme du même compte ne déchiffre pas par accident.
const ENTROPY: &[u8] = b"RusTV-UniversalSearch-v1";

pub struct Dpapi;

fn blob(b: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: b.len() as u32,
        pbData: b.as_ptr() as *mut u8,
    }
}

/// Copie le résultat DPAPI puis libère le tampon alloué par Windows.
unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let v = if out.pbData.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec()
    };
    if !out.pbData.is_null() {
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
    }
    v
}

impl Protector for Dpapi {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(plain);
        let ent = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(&input, None, Some(&ent), None, None, 0, &mut out)
                .map_err(|_| "dpapi_failed".to_string())?;
            Ok(take(out))
        }
    }

    fn unprotect(&self, sealed: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(sealed);
        let ent = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&input, None, Some(&ent), None, None, 0, &mut out)
                .map_err(|_| "dpapi_failed".to_string())?;
            Ok(take(out))
        }
    }
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Jeton en clair → texte hexadécimal protégé (à écrire dans usearch.json).
pub fn seal(p: &dyn Protector, plain: &str) -> Result<String, String> {
    Ok(to_hex(&p.protect(plain.as_bytes())?))
}

/// Texte hexadécimal protégé → jeton en clair.
pub fn open(p: &dyn Protector, sealed_hex: &str) -> Result<String, String> {
    let raw = from_hex(sealed_hex).ok_or_else(|| "dpapi_failed".to_string())?;
    String::from_utf8(p.unprotect(&raw)?).map_err(|_| "dpapi_failed".to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Fausse protection réversible (XOR) : aucun appel à Windows.
    pub struct Fake;
    impl Protector for Fake {
        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
            Ok(plain.iter().map(|b| b ^ 0x5a).collect())
        }
        fn unprotect(&self, sealed: &[u8]) -> Result<Vec<u8>, String> {
            Ok(sealed.iter().map(|b| b ^ 0x5a).collect())
        }
    }

    #[test]
    fn round_trip() {
        let s = seal(&Fake, "eyJ.tok-en_1").unwrap();
        assert!(!s.contains("tok") && s.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(open(&Fake, &s).unwrap(), "eyJ.tok-en_1");
        assert!(open(&Fake, "zz").is_err() && open(&Fake, "abc").is_err());
    }
}
