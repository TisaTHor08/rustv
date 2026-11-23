//! Détection des champs de saisie via UI Automation.
//!
//! On interroge l'élément qui a le focus clavier toutes les 350 ms plutôt que de s'abonner à
//! `FocusChangedEvent` : un abonnement COM dans un process sans boucle dédiée rate des événements
//! (notamment dans les navigateurs) ; le sondage est plus fiable et son coût est négligeable.
//! Il est suspendu tant qu'aucun téléphone n'est connecté.
use crate::core::Core;
use crate::textfocus::{is_text_input, Ctl, Probe};
use std::sync::Arc;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;

fn is_editable(el: &IUIAutomationElement) -> bool {
    unsafe {
        let enabled = el.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(true);
        let ctl = el.CurrentControlType().map(|c| Ctl::from_uia(c.0)).unwrap_or(Ctl::Other);
        let value_writable = el
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .ok()
            .and_then(|p| p.CurrentIsReadOnly().ok())
            .map(|ro| !ro.as_bool())
            .unwrap_or(false);
        let has_text_pattern = el.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId).is_ok();
        is_text_input(&Probe { enabled, ctl, value_writable, has_text_pattern })
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
            let mut editable = auto.GetFocusedElement().map(|e| is_editable(&e)).unwrap_or(false);
            // Apparition : on la confirme par un second échantillon (un focus qui ne fait que passer ne doit pas
            // faire surgir le clavier du téléphone). La disparition est immédiate.
            if editable {
                std::thread::sleep(Duration::from_millis(120));
                editable = auto.GetFocusedElement().map(|e| is_editable(&e)).unwrap_or(false);
            }
            core.set_text_focus(editable);
            std::thread::sleep(Duration::from_millis(350));
        }
    }
}
