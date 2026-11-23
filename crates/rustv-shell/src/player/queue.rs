//! File de lecture : épisode suivant automatique et touches suivant/précédent (logique pure).

/// Faut-il garder mpv ouvert (`--idle=yes`) à la fin d'un fichier pour enchaîner ?
pub fn idle_needed(kind: &str, len: usize, auto_next: bool) -> bool {
    kind != "live" && len > 1 && auto_next
}

/// Élément à lire quand le fichier courant se termine (None = fin de la lecture).
pub fn next_after_eof(kind: &str, idx: usize, len: usize, auto_next: bool) -> Option<usize> {
    if kind == "live" || !auto_next || idx + 1 >= len {
        return None;
    }
    Some(idx + 1)
}

/// Suivant (+1) / précédent (-1) sans boucler ; None aux extrémités.
pub fn step(idx: usize, len: usize, dir: i32) -> Option<usize> {
    let n = idx as i64 + dir as i64;
    if n < 0 || n >= len as i64 {
        None
    } else {
        Some(n as usize)
    }
}

/// Texte de l'incrustation de compte à rebours.
pub fn next_notice(lang: &str, title: &str, secs: u32) -> String {
    if lang == "en" {
        format!("Next: {title} in {secs} s (Back to cancel)")
    } else {
        format!("Suivant : {title} dans {secs} s (Retour pour annuler)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eof_selection() {
        assert_eq!(next_after_eof("video", 0, 3, true), Some(1));
        assert_eq!(next_after_eof("video", 2, 3, true), None);
        assert_eq!(next_after_eof("video", 0, 3, false), None);
        assert_eq!(next_after_eof("live", 0, 3, true), None);
        assert_eq!(next_after_eof("audio", 0, 2, true), Some(1));
    }

    #[test]
    fn stepping() {
        assert_eq!(step(0, 3, -1), None);
        assert_eq!(step(0, 3, 1), Some(1));
        assert_eq!(step(2, 3, 1), None);
        assert_eq!(step(2, 3, -1), Some(1));
    }

    #[test]
    fn idle_and_notice() {
        assert!(idle_needed("video", 2, true));
        assert!(!idle_needed("video", 1, true));
        assert!(!idle_needed("live", 9, true));
        assert_eq!(next_notice("en", "S01E02", 5), "Next: S01E02 in 5 s (Back to cancel)");
        assert!(next_notice("fr", "X", 3).contains("dans 3 s"));
    }
}
