//! Linux : détection des champs de saisie (version pragmatique).
//!
//! Une détection générale demanderait AT-SPI (D-Bus `org.a11y.atspi`) et un bureau avec l'accessibilité activée,
//! ce qui n'est pas fiable partout. Ici le focus texte reste toujours `false` : les vues web de RusTV signalent
//! déjà leurs champs de saisie par leur propre canal (spatnav), et le clavier du téléphone ne reste donc jamais
//! ouvert à tort. Limite documentée dans docs/LINUX.md : pas de bascule automatique du clavier du téléphone
//! quand un champ d'une autre application (navigateur externe, appli native) prend le focus.
use crate::core::Core;
use std::sync::Arc;

pub fn run(core: Arc<Core>) {
    core.set_text_focus(false);
}
