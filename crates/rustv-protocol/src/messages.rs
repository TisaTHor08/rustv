use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key { Up, Down, Left, Right, Enter, Back, BackLong, Menu }

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
    State { pinned: bool, text_focus: bool, tv_mode: bool },
    Pong,
}
