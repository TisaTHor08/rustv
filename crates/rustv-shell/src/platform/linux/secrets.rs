//! Linux : secrets dans le trousseau du bureau (Secret Service : GNOME Keyring, KWallet, KeePassXC…).
//!
//! Le secret n'est JAMAIS écrit sur le disque. Le fichier JSON de RusTV ne reçoit qu'un identifiant aléatoire
//! (32 caractères hexadécimaux) ; le secret est rangé dans le trousseau sous les attributs
//! `application=rustv`, `purpose=<module>`, `id=<identifiant>`. Sans démon de trousseau (ou trousseau verrouillé
//! et refusé) : `Err("keyring_unavailable")`, jamais de repli en clair.
use secret_service::blocking::SecretService;
use secret_service::EncryptionType;
use std::collections::HashMap;

pub const UNAVAILABLE: &str = "keyring_unavailable";

/// Identifiant aléatoire de 128 bits.
fn new_id() -> Result<String, String> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).map_err(|_| UNAVAILABLE.to_string())?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Attributs de recherche (testés) d'un secret.
pub fn attributes<'a>(purpose: &'a str, id: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("application", "rustv"), ("purpose", purpose), ("id", id)])
}

/// L'identifiant stocké sur disque est-il bien formé ?
pub fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit())
}

fn connect() -> Result<SecretService<'static>, String> {
    SecretService::connect(EncryptionType::Dh).map_err(|_| UNAVAILABLE.to_string())
}

/// Range `plain` dans le trousseau et renvoie l'identifiant à enregistrer sur disque.
pub fn store(purpose: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let ss = connect()?;
    let coll = ss.get_default_collection().map_err(|_| UNAVAILABLE.to_string())?;
    if coll.is_locked().unwrap_or(true) {
        coll.unlock().map_err(|_| UNAVAILABLE.to_string())?;
    }
    let id = new_id()?;
    let label = format!("RusTV ({purpose})");
    coll.create_item(&label, attributes(purpose, &id), plain, false, "text/plain").map_err(|_| UNAVAILABLE.to_string())?;
    Ok(id.into_bytes())
}

/// Relit le secret correspondant à l'identifiant stocké sur disque.
pub fn fetch(purpose: &str, sealed: &[u8]) -> Result<Vec<u8>, String> {
    let id = std::str::from_utf8(sealed).map_err(|_| UNAVAILABLE.to_string())?;
    if !valid_id(id) {
        return Err(UNAVAILABLE.to_string());
    }
    let ss = connect()?;
    let found = ss.search_items(attributes(purpose, id)).map_err(|_| UNAVAILABLE.to_string())?;
    let item = found.unlocked.into_iter().next().or_else(|| found.locked.into_iter().next()).ok_or_else(|| UNAVAILABLE.to_string())?;
    if item.is_locked().unwrap_or(true) {
        item.unlock().map_err(|_| UNAVAILABLE.to_string())?;
    }
    item.get_secret().map_err(|_| UNAVAILABLE.to_string())
}

/// Un service Secret Service répond-il sur le bus de session ? (sans ouvrir de collection)
pub fn available() -> bool {
    use zbus::blocking::{fdo::DBusProxy, Connection};
    let Ok(conn) = Connection::session() else { return false };
    let Ok(dbus) = DBusProxy::new(&conn) else { return false };
    let Ok(name) = zbus::names::BusName::try_from("org.freedesktop.secrets") else { return false };
    dbus.name_has_owner(name.clone()).unwrap_or(false) || dbus.list_activatable_names().map(|n| n.iter().any(|x| x.as_str() == "org.freedesktop.secrets")).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_identify_application_purpose_and_id() {
        let a = attributes("homeassistant", "00ff");
        assert_eq!(a["application"], "rustv");
        assert_eq!(a["purpose"], "homeassistant");
        assert_eq!(a["id"], "00ff");
    }

    #[test]
    fn id_validation() {
        assert!(valid_id(&"a1".repeat(16)));
        assert!(!valid_id("short") && !valid_id(&"g".repeat(32)));
        assert!(fetch("x", b"not-an-id").is_err());
        assert_eq!(fetch("x", &[0xff, 0xfe]).unwrap_err(), UNAVAILABLE);
    }

    /// Manuel (trousseau réel : gnome-keyring, KWallet…) : `cargo test -p rustv-shell secrets -- --ignored`.
    #[test]
    #[ignore]
    fn live_keyring_round_trip() {
        let id = store("selftest", b"s3cr3t-value").expect("keyring_unavailable");
        assert!(valid_id(std::str::from_utf8(&id).unwrap()));
        assert_eq!(fetch("selftest", &id).unwrap(), b"s3cr3t-value");
        assert!(fetch("other-purpose", &id).is_err());
    }

    #[test]
    fn generated_ids_are_valid_and_distinct() {
        let (a, b) = (new_id().unwrap(), new_id().unwrap());
        assert!(valid_id(&a) && valid_id(&b) && a != b);
    }
}
