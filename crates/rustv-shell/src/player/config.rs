//! Réglages du lecteur (`player.json`) et traduction en options mpv (logique pure).
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LANGS: [&str; 7] = ["auto", "fr", "en", "de", "es", "it", "ja"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerConfig {
    /// Langue audio préférée : "auto" (langue de l'interface) ou un code de `LANGS`.
    pub audio_lang: String,
    /// Langue des sous-titres.
    pub sub_lang: String,
    /// Sous-titres activés par défaut.
    pub subs_default: bool,
    /// Enchaîner automatiquement l'élément suivant.
    pub auto_next: bool,
    /// Reprendre la lecture là où elle s'était arrêtée.
    pub resume: bool,
    /// Mémoire tampon du direct différé, en Mo (64 à 1024).
    pub timeshift_mb: u32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self { audio_lang: "auto".into(), sub_lang: "auto".into(), subs_default: false, auto_next: true, resume: true, timeshift_mb: 256 }
    }
}

impl PlayerConfig {
    /// Corrige les valeurs hors limites ou inconnues.
    pub fn sanitized(mut self) -> Self {
        if !LANGS.contains(&self.audio_lang.as_str()) {
            self.audio_lang = "auto".into();
        }
        if !LANGS.contains(&self.sub_lang.as_str()) {
            self.sub_lang = "auto".into();
        }
        self.timeshift_mb = self.timeshift_mb.clamp(64, 1024);
        self
    }
}

pub fn load() -> PlayerConfig {
    crate::store::load::<PlayerConfig>("player").sanitized()
}

pub fn save(c: &PlayerConfig) -> Result<(), String> {
    crate::store::save("player", c)
}

/// Liste mpv (`--alang`/`--slang`) pour un code ; "auto" suit la langue de l'interface puis l'anglais.
pub fn lang_list(code: &str, ui: &str) -> String {
    let c = if code == "auto" { if ui == "en" { "en" } else { "fr" } } else { code };
    let base = match c {
        "fr" => "fre,fra,fr",
        "en" => "eng,en",
        "de" => "ger,deu,de",
        "es" => "spa,es",
        "it" => "ita,it",
        "ja" => "jpn,ja",
        _ => "eng,en",
    };
    if code == "auto" && c != "en" {
        format!("{base},eng,en")
    } else {
        base.to_string()
    }
}

/// Options mpv de langue : `--alang`, `--slang`, et `--sid=no` si les sous-titres sont désactivés par défaut.
pub fn lang_args(cfg: &PlayerConfig, ui: &str) -> Vec<String> {
    let mut v = vec![format!("--alang={}", lang_list(&cfg.audio_lang, ui)), format!("--slang={}", lang_list(&cfg.sub_lang, ui)), "--sub-auto=fuzzy".to_string()];
    if !cfg.subs_default {
        v.push("--sid=no".into());
    }
    v
}

/// Options propres au type de lecture.
pub fn kind_args(kind: &str, cfg: &PlayerConfig, watch_later: &Path) -> Vec<String> {
    match kind {
        "video" => {
            let mut v = vec![
                "--save-position-on-quit".to_string(),
                format!("--watch-later-directory={}", watch_later.display()),
                // pistes et réglages mémorisés par fichier (langue audio/sous-titres, décalages, vitesse)
                "--watch-later-options=start,speed,aid,sid,sub-delay,audio-delay,sub-scale".to_string(),
            ];
            if !cfg.resume {
                v.push("--no-resume-playback".into());
            }
            v
        }
        "audio" => vec!["--audio-display=embedded-first".into(), "--osd-level=3".into()],
        _ => {
            let mb = cfg.timeshift_mb.clamp(64, 1024);
            vec!["--cache=yes".into(), format!("--demuxer-max-bytes={mb}MiB"), format!("--demuxer-max-back-bytes={mb}MiB"), "--cache-secs=7200".into()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_sanitize() {
        let c = PlayerConfig { audio_lang: "xx".into(), timeshift_mb: 99999, ..Default::default() }.sanitized();
        assert_eq!(c.audio_lang, "auto");
        assert_eq!(c.timeshift_mb, 1024);
        assert_eq!(PlayerConfig { timeshift_mb: 1, ..Default::default() }.sanitized().timeshift_mb, 64);
        let parsed: PlayerConfig = serde_json::from_str(r#"{"auto_next":false}"#).unwrap();
        assert!(!parsed.auto_next && parsed.resume);
    }

    #[test]
    fn language_args() {
        assert_eq!(lang_list("auto", "fr"), "fre,fra,fr,eng,en");
        assert_eq!(lang_list("auto", "en"), "eng,en");
        assert_eq!(lang_list("ja", "fr"), "jpn,ja");
        let a = lang_args(&PlayerConfig::default(), "fr");
        assert!(a.contains(&"--alang=fre,fra,fr,eng,en".to_string()));
        assert!(a.contains(&"--sid=no".to_string()));
        let on = PlayerConfig { subs_default: true, sub_lang: "en".into(), ..Default::default() };
        let a = lang_args(&on, "fr");
        assert!(a.contains(&"--slang=eng,en".to_string()));
        assert!(!a.contains(&"--sid=no".to_string()));
    }

    #[test]
    fn kind_options() {
        let c = PlayerConfig::default();
        let live = kind_args("live", &c, Path::new("x"));
        assert!(live.contains(&"--demuxer-max-bytes=256MiB".to_string()));
        assert!(live.contains(&"--demuxer-max-back-bytes=256MiB".to_string()));
        assert!(live.contains(&"--cache=yes".to_string()));
        let v = kind_args("video", &PlayerConfig { resume: false, ..Default::default() }, Path::new("wl"));
        assert!(v.contains(&"--no-resume-playback".to_string()));
        assert!(v.contains(&"--save-position-on-quit".to_string()));
        assert!(kind_args("audio", &c, Path::new("x")).contains(&"--osd-level=3".to_string()));
    }
}
