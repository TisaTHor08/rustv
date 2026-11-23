//! Décision « le contrôle qui a le focus est-il un champ de saisie ? » (logique pure, testable sans UI Automation).
//!
//! L'ancienne règle (« ValuePattern non lecture seule ») était trop large : curseurs, listes déroulantes non
//! éditables, panneaux de navigateur ou d'application exposent aussi ce motif, et la télécommande affichait
//! alors une zone de texte alors qu'aucun champ n'existait.

/// Types de contrôle UI Automation qui nous intéressent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctl {
    Edit,
    ComboBox,
    Document,
    Other,
}

impl Ctl {
    /// Identifiants `UIA_*ControlTypeId`.
    pub fn from_uia(id: i32) -> Ctl {
        match id {
            50004 => Ctl::Edit,
            50003 => Ctl::ComboBox,
            50030 => Ctl::Document,
            _ => Ctl::Other,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub enabled: bool,
    pub ctl: Ctl,
    /// ValuePattern présent et non lecture seule.
    pub value_writable: bool,
    pub has_text_pattern: bool,
}

pub fn is_text_input(p: &Probe) -> bool {
    if !p.enabled {
        return false;
    }
    match p.ctl {
        Ctl::Edit => p.value_writable || p.has_text_pattern,
        // Liste déroulante : seulement si elle est éditable (zone de texte intégrée).
        Ctl::ComboBox | Ctl::Document => p.value_writable,
        Ctl::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(ctl: Ctl, w: bool, t: bool) -> Probe {
        Probe { enabled: true, ctl, value_writable: w, has_text_pattern: t }
    }

    #[test]
    fn edit_fields_count() {
        assert!(is_text_input(&p(Ctl::Edit, true, false)));
        assert!(is_text_input(&p(Ctl::Edit, false, true)));
        assert!(!is_text_input(&p(Ctl::Edit, false, false)));
    }

    #[test]
    fn non_text_controls_with_writable_value_do_not_count() {
        // Curseur, spinner, panneau, bouton… : ValuePattern inscriptible mais pas de saisie de texte.
        assert!(!is_text_input(&p(Ctl::Other, true, true)));
        assert!(is_text_input(&p(Ctl::ComboBox, true, false)));
        assert!(!is_text_input(&p(Ctl::ComboBox, false, false)));
        // Page web en lecture : TextPattern seul ne suffit pas pour un document.
        assert!(!is_text_input(&p(Ctl::Document, false, true)));
    }

    #[test]
    fn disabled_never_counts() {
        let mut x = p(Ctl::Edit, true, true);
        x.enabled = false;
        assert!(!is_text_input(&x));
    }

    #[test]
    fn control_type_mapping() {
        assert_eq!(Ctl::from_uia(50004), Ctl::Edit);
        assert_eq!(Ctl::from_uia(50015), Ctl::Other);
    }
}
