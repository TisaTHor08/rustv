//! STUB non-Windows : lanceur d'applications.
//!
//! Contrat pour le vrai backend : `list(custom)` renvoie d'abord les applis perso (`id = "c:<index>"`) puis les
//! applis installées triées par nom (`id = "s:<hash>"`, `target` = chemin ou identifiant à passer à `launch`) ;
//! `launch(target)` ouvre une URL, un fichier ou une appli sans bloquer.
//! Linux : fichiers `.desktop` (XDG_DATA_DIRS/applications) + `gtk-launch`/`xdg-open`. macOS : `/Applications`
//! + `open`.
use crate::config::CustomApp;

#[derive(Clone)]
pub struct App {
    pub id: String,
    pub name: String,
    pub target: String,
}

/// Sans backend : seules les applis perso sont listées (aucune appli installée détectée).
pub fn list(custom: &[CustomApp]) -> Vec<App> {
    custom.iter().enumerate().map(|(i, c)| App { id: format!("c:{i}"), name: c.name.clone(), target: c.target.clone() }).collect()
}

/// Sans backend : ne fait rien.
pub fn launch(_target: &str) {}
