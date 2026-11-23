//! Réglages Home Assistant (ha.json). Le mot de passe est stocké protégé par DPAPI (`passwordEnc`, hexadécimal),
//! jamais renvoyé à l'interface (`hasPassword` seulement).
use super::dpapi::{self, Protector};
use serde::{Deserialize, Serialize};

pub const STORE: &str = "ha";
pub const DEFAULT_PREFIX: &str = "homeassistant";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HaConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password_enc: String,
    pub tls: bool,
    pub client_id: String,
    pub base_topic: String,
    pub discovery_prefix: String,
}

impl Default for HaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            port: 1883,
            username: String::new(),
            password_enc: String::new(),
            tls: false,
            client_id: String::new(),
            base_topic: String::new(),
            discovery_prefix: DEFAULT_PREFIX.into(),
        }
    }
}

/// Réglages tels que l'interface les voit : sans mot de passe.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HaConfigView {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub has_password: bool,
    pub tls: bool,
    pub client_id: String,
    pub base_topic: String,
    pub discovery_prefix: String,
    /// Valeurs effectives quand les champs ci-dessus sont vides.
    pub default_base_topic: String,
    pub default_client_id: String,
}

/// Réglages envoyés par l'interface. `password` absent = inchangé ; `""` = effacer.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HaConfigIn {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub base_topic: String,
    #[serde(default)]
    pub discovery_prefix: String,
}

pub fn sanitize_node(s: &str) -> String {
    let v: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if v.is_empty() {
        "pc".into()
    } else {
        v
    }
}

pub fn valid_host(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':' || c == '[' || c == ']')
}

/// Topic ou préfixe : segments non vides de [A-Za-z0-9_.-], sans joker ni `$`, 100 caractères au plus.
pub fn valid_topic(t: &str) -> bool {
    !t.is_empty() && t.len() <= 100 && t.split('/').all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
}

pub fn valid_client_id(c: &str) -> bool {
    !c.is_empty() && c.len() <= 64 && c.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl HaConfig {
    pub fn default_base(pc_id: &str) -> String {
        format!("rustv/{}", sanitize_node(pc_id))
    }

    pub fn default_client(pc_id: &str) -> String {
        format!("rustv-{}", sanitize_node(pc_id))
    }

    pub fn base(&self, pc_id: &str) -> String {
        if self.base_topic.is_empty() {
            Self::default_base(pc_id)
        } else {
            self.base_topic.trim_matches('/').to_string()
        }
    }

    pub fn client(&self, pc_id: &str) -> String {
        if self.client_id.is_empty() {
            Self::default_client(pc_id)
        } else {
            self.client_id.clone()
        }
    }

    pub fn prefix(&self) -> String {
        if self.discovery_prefix.is_empty() {
            DEFAULT_PREFIX.into()
        } else {
            self.discovery_prefix.trim_matches('/').to_string()
        }
    }

    pub fn view(&self, pc_id: &str) -> HaConfigView {
        HaConfigView {
            enabled: self.enabled,
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            has_password: !self.password_enc.is_empty(),
            tls: self.tls,
            client_id: self.client_id.clone(),
            base_topic: self.base_topic.clone(),
            discovery_prefix: self.discovery_prefix.clone(),
            default_base_topic: Self::default_base(pc_id),
            default_client_id: Self::default_client(pc_id),
        }
    }

    /// Mot de passe en clair (déchiffré à la demande, jamais conservé).
    pub fn password(&self, p: &dyn Protector) -> Option<String> {
        if self.password_enc.is_empty() {
            None
        } else {
            dpapi::open(p, &self.password_enc).ok()
        }
    }

    /// Valide puis applique les réglages reçus ; en cas d'erreur, `self` reste inchangé.
    pub fn apply(&mut self, inp: HaConfigIn, p: &dyn Protector) -> Result<(), &'static str> {
        let host = inp.host.trim().to_string();
        if !(host.is_empty() && !inp.enabled) && !valid_host(&host) {
            return Err("bad_host");
        }
        if inp.port == 0 {
            return Err("bad_port");
        }
        if inp.username.len() > 128 || inp.username.chars().any(|c| c.is_control()) {
            return Err("bad_user");
        }
        let base = inp.base_topic.trim().trim_matches('/').to_string();
        if !base.is_empty() && !valid_topic(&base) {
            return Err("bad_topic");
        }
        let prefix = inp.discovery_prefix.trim().trim_matches('/').to_string();
        if !prefix.is_empty() && !valid_topic(&prefix) {
            return Err("bad_prefix");
        }
        let cid = inp.client_id.trim().to_string();
        if !cid.is_empty() && !valid_client_id(&cid) {
            return Err("bad_client_id");
        }
        let new_pw = match inp.password {
            None => None,
            Some(s) if s.is_empty() => Some(String::new()),
            Some(s) if s.len() <= 256 && !s.chars().any(|c| c.is_control()) => Some(dpapi::seal(p, &s).map_err(|_| "dpapi_failed")?),
            Some(_) => return Err("bad_password"),
        };
        self.enabled = inp.enabled;
        self.host = host;
        self.port = inp.port;
        self.username = inp.username;
        if let Some(pw) = new_pw {
            self.password_enc = pw;
        }
        self.tls = inp.tls;
        self.client_id = cid;
        self.base_topic = base;
        self.discovery_prefix = if prefix.is_empty() { DEFAULT_PREFIX.into() } else { prefix };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::dpapi::tests::Fake;
    use super::*;

    fn inp() -> HaConfigIn {
        HaConfigIn {
            enabled: true,
            host: "192.168.1.5".into(),
            port: 1883,
            username: "u".into(),
            password: Some("secret".into()),
            tls: false,
            client_id: String::new(),
            base_topic: String::new(),
            discovery_prefix: String::new(),
        }
    }

    #[test]
    fn defaults_and_serde() {
        let c = HaConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.discovery_prefix, "homeassistant");
        let j = serde_json::to_string(&c).unwrap();
        assert!(j.contains("\"discoveryPrefix\""));
        let back: HaConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(back, c);
        // champs manquants -> valeurs par défaut
        let partial: HaConfig = serde_json::from_str("{\"host\":\"x\"}").unwrap();
        assert_eq!(partial.port, 1883);
    }

    #[test]
    fn password_never_in_view() {
        let mut c = HaConfig::default();
        c.apply(inp(), &Fake).unwrap();
        assert!(c.password_enc != "secret" && !c.password_enc.is_empty());
        let v = serde_json::to_string(&c.view("abc")).unwrap();
        assert!(!v.contains("secret") && !v.contains(&c.password_enc));
        assert!(v.contains("\"hasPassword\":true"));
        assert_eq!(c.password(&Fake).as_deref(), Some("secret"));
    }

    #[test]
    fn password_only_replaced_when_provided() {
        let mut c = HaConfig::default();
        c.apply(inp(), &Fake).unwrap();
        let mut i = inp();
        i.password = None;
        c.apply(i, &Fake).unwrap();
        assert_eq!(c.password(&Fake).as_deref(), Some("secret"));
        let mut i = inp();
        i.password = Some(String::new());
        c.apply(i, &Fake).unwrap();
        assert!(c.password(&Fake).is_none());
    }

    #[test]
    fn validation() {
        let mut c = HaConfig::default();
        let mut i = inp();
        i.host = "bad host".into();
        assert_eq!(c.apply(i, &Fake), Err("bad_host"));
        let mut i = inp();
        i.port = 0;
        assert_eq!(c.apply(i, &Fake), Err("bad_port"));
        let mut i = inp();
        i.base_topic = "rustv/#".into();
        assert_eq!(c.apply(i, &Fake), Err("bad_topic"));
        let mut i = inp();
        i.discovery_prefix = "a//b".into();
        assert_eq!(c.apply(i, &Fake), Err("bad_prefix"));
        let mut i = inp();
        i.client_id = "a b".into();
        assert_eq!(c.apply(i, &Fake), Err("bad_client_id"));
        assert_eq!(c, HaConfig::default()); // inchangé après erreurs
    }

    #[test]
    fn effective_names() {
        let mut c = HaConfig::default();
        assert_eq!(c.base("ab12"), "rustv/ab12");
        assert_eq!(c.client("ab12"), "rustv-ab12");
        c.base_topic = "/tv/salon/".into();
        assert_eq!(c.base("ab12"), "tv/salon");
        assert_eq!(c.prefix(), "homeassistant");
    }
}
