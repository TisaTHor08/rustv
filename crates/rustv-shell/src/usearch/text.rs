//! Texte : normalisation (accents, casse, ponctuation), classement des correspondances et nettoyage
//! des noms de fichiers (« The.Movie.2019.1080p.BluRay.x264-GRP.mkv » → « The Movie », 2019).

/// Remplace une lettre accentuée par sa base (latin courant), en minuscule.
fn fold_char(c: char, out: &mut String) {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
            out.push('a')
        }
        'ç' | 'ć' | 'č' | 'Ç' | 'Ć' | 'Č' => out.push('c'),
        'ď' | 'đ' | 'Ď' | 'Đ' => out.push('d'),
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ę' | 'Ě' => {
            out.push('e')
        }
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'İ' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'ı' => out.push('i'),
        'ł' | 'Ł' => out.push('l'),
        'ñ' | 'ń' | 'ň' | 'Ñ' | 'Ń' | 'Ň' => out.push('n'),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ő' => {
            out.push('o')
        }
        'œ' | 'Œ' => out.push_str("oe"),
        'æ' | 'Æ' => out.push_str("ae"),
        'ř' | 'Ř' => out.push('r'),
        'ś' | 'š' | 'ş' | 'Ś' | 'Š' | 'Ş' => out.push('s'),
        'ß' => out.push_str("ss"),
        'ť' | 'ţ' | 'Ť' | 'Ţ' => out.push('t'),
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ű' | 'Ů' => out.push('u'),
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' => out.push('y'),
        'ź' | 'ż' | 'ž' | 'Ź' | 'Ż' | 'Ž' => out.push('z'),
        _ => {
            for l in c.to_lowercase() {
                out.push(l);
            }
        }
    }
}

/// « Amélie Poulain : 2001 ! » → « amelie poulain 2001 » (lettres et chiffres séparés par un espace).
pub fn fold(s: &str) -> String {
    let mut tmp = String::with_capacity(s.len());
    for c in s.chars() {
        fold_char(c, &mut tmp);
    }
    let mut out = String::with_capacity(tmp.len());
    let mut pending_space = false;
    for c in tmp.chars() {
        if c.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        } else {
            pending_space = true;
        }
    }
    out
}

/// Score d'une correspondance entre la requête et un titre, tous deux déjà passés dans `fold`.
/// `None` : pas de correspondance. Plus c'est grand, mieux c'est.
pub fn score(query: &str, title: &str) -> Option<u32> {
    if query.is_empty() || title.is_empty() {
        return None;
    }
    // Pénalité de longueur : à correspondance égale, le titre le plus court gagne.
    let pen = (title.len().saturating_sub(query.len()) as u32).min(40);
    if title == query {
        return Some(1000);
    }
    if title.starts_with(query) {
        // Début de titre ; mieux si la requête se termine sur une frontière de mot.
        let boundary = title[query.len()..].starts_with(' ');
        let base = if boundary { 800 } else { 700 };
        return Some(base - pen);
    }
    let qt: Vec<&str> = query.split(' ').collect();
    let tw: Vec<&str> = title.split(' ').collect();
    if title.contains(&format!(" {query}")) {
        return Some(600 - pen);
    }
    // Tous les mots de la requête sont des débuts de mots du titre (dans n'importe quel ordre).
    if qt.iter().all(|q| tw.iter().any(|w| w.starts_with(q))) {
        return Some(450 - pen);
    }
    if title.contains(query) {
        return Some(300 - pen);
    }
    None
}

const TAGS: &[&str] = &[
    "480p",
    "576p",
    "720p",
    "1080p",
    "1080i",
    "2160p",
    "4k",
    "uhd",
    "bluray",
    "blu-ray",
    "bdrip",
    "brrip",
    "webrip",
    "web-dl",
    "webdl",
    "web",
    "hdtv",
    "dvdrip",
    "dvdscr",
    "x264",
    "x265",
    "h264",
    "h265",
    "hevc",
    "xvid",
    "divx",
    "multi",
    "vff",
    "vfq",
    "vf",
    "vo",
    "vostfr",
    "truefrench",
    "french",
    "subfrench",
    "remux",
    "hdr",
    "hdr10",
    "10bit",
    "internal",
    "proper",
    "repack",
    "extended",
    "unrated",
    "dts",
    "ac3",
    "aac",
    "dd5",
    "atmos",
];

/// Nom de fichier sans extension → (titre, année). Le titre s'arrête à l'année ou au premier marqueur technique.
pub fn clean_title(stem: &str) -> (String, Option<u32>) {
    let s: String = stem
        .chars()
        .map(|c| if c == '.' || c == '_' { ' ' } else { c })
        .collect();
    let mut year = None;
    let mut words: Vec<&str> = Vec::new();
    for w in s.split_whitespace() {
        let lw = w.to_lowercase();
        let bare = lw.trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']');
        if bare.len() == 4 && bare.chars().all(|c| c.is_ascii_digit()) {
            let y: u32 = bare.parse().unwrap_or(0);
            if (1900..=2100).contains(&y) && !words.is_empty() {
                year = Some(y);
                break;
            }
        }
        // « x264-GRP » : on teste aussi la partie avant le tiret.
        let head = bare.split('-').next().unwrap_or(bare);
        if TAGS.contains(&bare) || TAGS.contains(&head) || (w.starts_with('[') && w.ends_with(']')) {
            if !words.is_empty() {
                break;
            }
            continue;
        }
        words.push(w);
    }
    let t = words.join(" ");
    (if t.is_empty() { stem.to_string() } else { t }, year)
}

/// Encodage pour une URL (RFC 3986 : seuls les caractères non réservés restent tels quels, espace → %20).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding() {
        assert_eq!(fold("Amélie Poulain : 2001 !"), "amelie poulain 2001");
        assert_eq!(fold("  ÉTÉ  –  Œuvre "), "ete oeuvre");
        assert_eq!(fold("L'Été d'Ève"), "l ete d eve");
        assert_eq!(fold("STRASSE straße"), "strasse strasse");
    }

    #[test]
    fn ranking() {
        let q = fold("amelie");
        let exact = score(&fold("Amélie"), &fold("Amélie")).unwrap();
        let prefix = score(&q, &fold("Amélie Poulain")).unwrap();
        let word = score(&q, &fold("Le Fabuleux Destin d'Amélie Poulain")).unwrap();
        let sub = score(&q, &fold("Zamelie")).unwrap();
        assert!(exact > prefix && prefix > word && word > sub);
        assert!(score("zzz", "amelie").is_none());
        // mots dans le désordre
        assert!(score("poulain amelie", "amelie poulain").is_some());
        // le titre le plus court gagne à correspondance égale
        assert!(score("matrix", "matrix reloaded").unwrap() > score("matrix", "matrix revolutions extended").unwrap());
        assert!(score("", "x").is_none());
    }

    #[test]
    fn file_names() {
        assert_eq!(
            clean_title("The.Movie.2019.1080p.BluRay.x264-GRP"),
            ("The Movie".into(), Some(2019))
        );
        assert_eq!(clean_title("Mon Film (2005)"), ("Mon Film".into(), Some(2005)));
        assert_eq!(clean_title("Some_Movie_1080p_x264-GRP"), ("Some Movie".into(), None));
        assert_eq!(clean_title("2001").0, "2001");
        assert_eq!(clean_title("[1080p] Show Name").0, "Show Name");
    }

    #[test]
    fn encoding() {
        assert_eq!(percent_encode("Amélie & Co/2"), "Am%C3%A9lie%20%26%20Co%2F2");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }
}
