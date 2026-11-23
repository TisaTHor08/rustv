use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key { Up, Down, Left, Right, Enter, Back, BackLong, Menu, Search }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AltTab { Open, Left, Right, Confirm }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton { Left, Right }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum PhoneMsg {
    Key { k: Key },
    AltTab { a: AltTab },
    Pin,
    /// `backspaces` retours arrière puis injection de `text`.
    Text {
        #[serde(default)]
        backspaces: u32,
        #[serde(default)]
        text: String,
    },
    Mouse {
        dx: i32,
        dy: i32,
        #[serde(default)]
        click: Option<MouseButton>,
        #[serde(default)]
        scroll: i32,
    },
    #[serde(rename_all = "camelCase")]
    Launch { app_id: String },
    GetApps,
    /// Retour à l'accueil TV (ferme l'app web au premier plan).
    Home,
    /// Démarre / arrête le mode TV sur le PC.
    Tv { on: bool },
    Media { a: MediaAction },
    /// Règle le volume du PC (0 à 100).
    SetVolume { level: u8 },
    /// Demande les contributions des mods actifs à la télécommande (thèmes, panneaux de boutons).
    GetRemoteMods,
    /// Ouvre une tuile de l'accueil TV (par identifiant), comme un OK sur la tuile.
    OpenTile { id: String },
    /// Coffre Bitwarden : interroge l'état (CLI présente, connecté, verrouillé).
    VaultStatus,
    /// Déverrouille avec le mot de passe maître (jamais conservé côté PC).
    VaultUnlock { password: Secret },
    /// Première connexion : e-mail, mot de passe maître, code 2FA et méthode (0 appli, 1 e-mail, 3 YubiKey) facultatifs.
    VaultLogin {
        email: String,
        password: Secret,
        #[serde(default)]
        code: Option<String>,
        #[serde(default)]
        method: Option<u8>,
    },
    VaultLock,
    /// Recherche : renvoie uniquement des métadonnées. `q` vide = suggestions d'après la fenêtre au premier plan.
    VaultSearch {
        #[serde(default)]
        q: String,
    },
    /// Saisit un secret dans le champ qui a le focus sur le PC (le secret ne revient jamais au téléphone).
    VaultFill {
        id: String,
        what: VaultFill,
        #[serde(default)]
        enter: bool,
    },
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaAction {
    PlayPause,
    Next,
    Prev,
    Stop,
    VolUp,
    VolDown,
    Mute,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    /// PNG encodé en base64 (peut être vide).
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IconEntry {
    pub id: String,
    /// PNG en base64.
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum PcMsg {
    Apps { apps: Vec<AppEntry> },
    /// Icônes envoyées par lots après la liste (une trame est limitée à 1 Mo).
    Icons { icons: Vec<IconEntry> },
    #[serde(rename_all = "camelCase")]
    State {
        pinned: bool,
        text_focus: bool,
        tv_mode: bool,
        /// Style d'accueil TV actif (`googletv`, `tizen`...), absent = `rustv` : le téléphone en déduit le modèle de télécommande.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        os_style: Option<String>,
    },
    /// Média en cours de lecture sur le PC (lecteur RusTV ou n'importe quelle appli via les contrôles média Windows).
    NowPlaying { active: bool, title: String, artist: String, playing: bool, position: f64, duration: f64, source: String },
    /// Contributions des mods actifs pour la télécommande : liste de `{id, name, themes, panels}` (voir docs/MODS.md).
    /// Envoyé à la demande et à chaque changement des mods.
    RemoteMods { mods: serde_json::Value },
    /// Volume principal du PC (0 à 100) et muet : le téléphone s'aligne dessus.
    Volume { level: u8, muted: bool },
    Pong,
    #[serde(rename_all = "camelCase")]
    VaultState { status: VaultStatus, email: String, autolock_min: u32 },
    VaultItems { items: Vec<VaultItem> },
    /// Résultat d'une opération du coffre : `code` est un code d'erreur stable (vide si `ok`), jamais un message brut de `bw`.
    VaultResult { op: String, ok: bool, code: String },
}

/// Chaîne secrète : Debug masqué, mise à zéro à la libération (au mieux), sérialisée comme une chaîne simple.
/// Utilisée pour le mot de passe maître et les secrets du coffre ; ne doit jamais être journalisée.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Self {
        Secret(s)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Secret {
    fn from(s: String) -> Self {
        Secret(s)
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: on n'écrit que des octets nuls (UTF-8 valide) dans le tampon propre de la chaîne.
        unsafe {
            for b in self.0.as_bytes_mut() {
                std::ptr::write_volatile(b, 0);
            }
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

/// Ce que « remplir » doit saisir dans le champ qui a le focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultFill {
    /// Identifiant, Tab, mot de passe.
    Both,
    Username,
    Password,
    Totp,
}

/// État du coffre Bitwarden côté PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultStatus {
    /// Pas encore interrogé.
    Unknown,
    /// La CLI officielle `bw` est introuvable.
    NotInstalled,
    /// `bw` est installé mais aucun compte n'est connecté.
    LoggedOut,
    Locked,
    Unlocked,
}

/// Métadonnées non secrètes d'un identifiant du coffre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultItem {
    pub id: String,
    pub name: String,
    pub username: String,
    /// Hôte de la première URI (vide si aucune).
    pub host: String,
    pub has_totp: bool,
}

/// Messages du coffre envoyés par le téléphone (`PhoneMsg::Vault*`) : voir docs/PROTOCOL.md.
#[cfg(test)]
mod vault_tests {
    use super::*;
    use serde_json::json;

    fn parse(v: serde_json::Value) -> PhoneMsg {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn phone_vault_messages() {
        assert_eq!(parse(json!({"t":"vaultStatus"})), PhoneMsg::VaultStatus);
        assert_eq!(parse(json!({"t":"vaultLock"})), PhoneMsg::VaultLock);
        assert_eq!(parse(json!({"t":"vaultUnlock","password":"pw"})), PhoneMsg::VaultUnlock { password: "pw".to_string().into() });
        assert_eq!(
            parse(json!({"t":"vaultLogin","email":"a@b.c","password":"pw"})),
            PhoneMsg::VaultLogin { email: "a@b.c".into(), password: "pw".to_string().into(), code: None, method: None }
        );
        assert_eq!(
            parse(json!({"t":"vaultLogin","email":"a@b.c","password":"pw","code":"123456","method":0})),
            PhoneMsg::VaultLogin { email: "a@b.c".into(), password: "pw".to_string().into(), code: Some("123456".into()), method: Some(0) }
        );
        assert_eq!(parse(json!({"t":"vaultSearch","q":"git"})), PhoneMsg::VaultSearch { q: "git".into() });
        assert_eq!(parse(json!({"t":"vaultSearch"})), PhoneMsg::VaultSearch { q: String::new() });
        assert_eq!(
            parse(json!({"t":"vaultFill","id":"abc","what":"both","enter":true})),
            PhoneMsg::VaultFill { id: "abc".into(), what: VaultFill::Both, enter: true }
        );
        assert_eq!(
            parse(json!({"t":"vaultFill","id":"abc","what":"totp"})),
            PhoneMsg::VaultFill { id: "abc".into(), what: VaultFill::Totp, enter: false }
        );
    }

    #[test]
    fn pc_vault_messages() {
        let s = serde_json::to_value(PcMsg::VaultState { status: VaultStatus::NotInstalled, email: String::new(), autolock_min: 5 }).unwrap();
        assert_eq!(s, json!({"t":"vaultState","status":"notInstalled","email":"","autolockMin":5}));
        let i = serde_json::to_value(PcMsg::VaultItems {
            items: vec![VaultItem { id: "1".into(), name: "GitHub".into(), username: "me".into(), host: "github.com".into(), has_totp: true }],
        })
        .unwrap();
        assert_eq!(i, json!({"t":"vaultItems","items":[{"id":"1","name":"GitHub","username":"me","host":"github.com","hasTotp":true}]}));
        let r = serde_json::to_value(PcMsg::VaultResult { op: "unlock".into(), ok: false, code: "badPassword".into() }).unwrap();
        assert_eq!(r, json!({"t":"vaultResult","op":"unlock","ok":false,"code":"badPassword"}));
    }

    #[test]
    fn state_os_style_field() {
        let a = serde_json::to_value(PcMsg::State { pinned: false, text_focus: true, tv_mode: false, os_style: None }).unwrap();
        assert_eq!(a, json!({"t":"state","pinned":false,"textFocus":true,"tvMode":false}));
        let b = serde_json::to_value(PcMsg::State { pinned: false, text_focus: false, tv_mode: true, os_style: Some("tizen".into()) }).unwrap();
        assert_eq!(b, json!({"t":"state","pinned":false,"textFocus":false,"tvMode":true,"osStyle":"tizen"}));
        // Un ancien PC (sans le champ) reste lisible.
        let old: PcMsg = serde_json::from_value(json!({"t":"state","pinned":true,"textFocus":false,"tvMode":false})).unwrap();
        assert_eq!(old, PcMsg::State { pinned: true, text_focus: false, tv_mode: false, os_style: None });
        let new: PcMsg = serde_json::from_value(b).unwrap();
        assert!(matches!(new, PcMsg::State { os_style: Some(ref s), .. } if s == "tizen"));
    }

    #[test]
    fn volume_messages() {
        assert_eq!(parse(json!({"t":"setVolume","level":42})), PhoneMsg::SetVolume { level: 42 });
        let v = serde_json::to_value(PcMsg::Volume { level: 30, muted: true }).unwrap();
        assert_eq!(v, json!({"t":"volume","level":30,"muted":true}));
    }

    #[test]
    fn remote_mod_messages() {
        assert_eq!(parse(json!({"t":"getRemoteMods"})), PhoneMsg::GetRemoteMods);
        assert_eq!(parse(json!({"t":"openTile","id":"youtube"})), PhoneMsg::OpenTile { id: "youtube".into() });
        let v = serde_json::to_value(PcMsg::RemoteMods { mods: json!([{"id":"x"}]) }).unwrap();
        assert_eq!(v, json!({"t":"remoteMods","mods":[{"id":"x"}]}));
    }

    #[test]
    fn secret_is_redacted_in_debug() {
        let m = PhoneMsg::VaultUnlock { password: "hunter2".to_string().into() };
        assert!(!format!("{m:?}").contains("hunter2"));
    }
}
