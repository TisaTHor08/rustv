//! Détection des champs de saisie via UI Automation.
//!
//! On interroge l'élément qui a le focus clavier toutes les 350 ms plutôt que de s'abonner à
//! `FocusChangedEvent` : un abonnement COM dans un process sans boucle dédiée rate des événements
//! (notamment dans les navigateurs) ; le sondage est plus fiable et son coût est négligeable.
//! Il est suspendu tant qu'aucun téléphone n'est connecté.
use crate::core::Core;
use std::sync::Arc;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;

fn is_editable(el: &IUIAutomationElement) -> bool {
    unsafe {
        if el.CurrentIsEnabled().map(|b| !b.as_bool()).unwrap_or(false) {
            return false;
        }
        // ValuePattern non lecture seule : champs texte, zones de saisie, barres d'adresse…
        if let Ok(p) = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
            if p.CurrentIsReadOnly().map(|b| !b.as_bool()).unwrap_or(false) {
                return true;
            }
        }
        // TextPattern : seulement pour les contrôles de saisie (sinon toute page web compterait).
        if el.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId).is_ok() {
            if let Ok(ct) = el.CurrentControlType() {
                return ct == UIA_EditControlTypeId;
            }
        }
        false
    }
}

pub fn run(core: Arc<Core>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let auto: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
            Ok(a) => a,
            Err(_) => return,
        };
        loop {
            if core.conn_count() == 0 {
                std::thread::sleep(Duration::from_millis(1000));
                continue;
            }
            let editable = auto.GetFocusedElement().map(|e| is_editable(&e)).unwrap_or(false);
            core.set_text_focus(editable);
            std::thread::sleep(Duration::from_millis(350));
        }
    }
}
