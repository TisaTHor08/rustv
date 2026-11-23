//! Services de streaming : table embarquée (search-providers.json) + fournisseurs déclarés par les mods
//! (`contributes.search`). Associe un nom de service renvoyé par TMDB à une tuile RusTV et construit l'URL
//! de recherche du titre.
use super::text::{fold, percent_encode};
use serde::Deserialize;
use std::collections::HashMap;

const TABLE: &str = include_str!("../search-providers.json");

#[derive(Clone, Debug, Default)]
pub struct Provider {
    pub id: String,
    pub tile: String,
    pub name: String,
    pub names: Vec<String>,
    pub search: String,
    pub search_lang: HashMap<String, String>,
    pub kinds: Vec<String>,
    /// "builtin" ou "mod:<id>".
    pub origin: String,
}

#[derive(Deserialize)]
struct Raw {
    id: String,
    tile: String,
    name: String,
    #[serde(default)]
    names: Vec<String>,
    search: String,
    #[serde(default, rename = "searchLang")]
    search_lang: HashMap<String, String>,
    #[serde(default)]
    kinds: Vec<String>,
}

#[derive(Deserialize)]
struct Table {
    providers: Vec<Raw>,
}

/// Modèle d'URL accepté : https, un `{q}` au moins, jamais dans le nom d'hôte, sans espace ni identifiants.
pub fn valid_template(t: &str) -> bool {
    if t.len() > 300
        || !t.starts_with("https://")
        || !t.contains("{q}")
        || t.contains(char::is_whitespace)
        || t.chars().any(|c| c.is_control())
    {
        return false;
    }
    let rest = &t["https://".len()..];
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..host_end];
    // `{q}` ne doit pas décider de la destination ; pas d'identifiants dans l'adresse.
    !host.is_empty() && !host.contains(['{', '}', '@']) && host.contains('.')
}

/// Remplace chaque `{q}` par la requête encodée (RFC 3986).
pub fn build_url(template: &str, q: &str) -> String {
    template.replace("{q}", &percent_encode(q.trim()))
}

pub fn builtin() -> Vec<Provider> {
    let t: Table = match serde_json::from_str(TABLE) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    t.providers
        .into_iter()
        .filter(|r| valid_template(&r.search) && r.search_lang.values().all(|u| valid_template(u)))
        .map(|r| Provider {
            id: r.id,
            tile: r.tile,
            name: r.name,
            names: r.names,
            search: r.search,
            search_lang: r.search_lang,
            kinds: r.kinds,
            origin: "builtin".into(),
        })
        .collect()
}

/// Fournisseurs des mods actifs : entrées déjà nettoyées par `mods::search_providers`.
pub fn from_mods(entries: &[serde_json::Value]) -> Vec<Provider> {
    entries
        .iter()
        .filter_map(|e| {
            let tpl = e["urlTemplate"].as_str()?;
            if !valid_template(tpl) {
                return None;
            }
            Some(Provider {
                id: e["id"].as_str()?.to_string(),
                tile: e["tile"].as_str()?.to_string(),
                name: e["name"].as_str()?.to_string(),
                names: e["providerNames"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n.as_str().map(String::from))
                    .collect(),
                search: tpl.to_string(),
                search_lang: HashMap::new(),
                kinds: e["kinds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n.as_str().map(String::from))
                    .collect(),
                origin: format!("mod:{}", e["mod"].as_str().unwrap_or("")),
            })
        })
        .collect()
}

pub fn all() -> Vec<Provider> {
    let mut v = builtin();
    v.extend(from_mods(&crate::mods::search_providers()));
    v
}

impl Provider {
    pub fn url(&self, q: &str, lang: &str) -> String {
        build_url(self.search_lang.get(lang).unwrap_or(&self.search), q)
    }
}

/// Mots sans valeur pour l'identification d'un service (« Netflix basic with Ads » = « Netflix »).
const NOISE: &[&str] = &["with", "ads", "basic", "standard", "premium", "channel"];

/// Nom de service normalisé : sans accents ni casse, « + » lu comme « plus », bruit retiré.
pub fn norm_name(s: &str) -> String {
    let s = s.replace('+', " plus ");
    fold(&s)
        .split(' ')
        .filter(|w| !w.is_empty() && !NOISE.contains(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Trouve le fournisseur correspondant à un nom de service TMDB : égalité exacte d'abord, sinon le plus long
/// alias qui est un début de nom (par mots entiers). Les fournisseurs des mods passent après la table.
pub fn find<'a>(providers: &'a [Provider], tmdb_name: &str) -> Option<&'a Provider> {
    let key = norm_name(tmdb_name);
    if key.is_empty() {
        return None;
    }
    let keys = |p: &Provider| -> Vec<String> {
        let mut k: Vec<String> = p.names.iter().map(|n| norm_name(n)).collect();
        k.push(norm_name(&p.name));
        k
    };
    for p in providers {
        if keys(p).iter().any(|k| *k == key) {
            return Some(p);
        }
    }
    let mut best: Option<(&Provider, usize)> = None;
    for p in providers {
        for k in keys(p) {
            if !k.is_empty() && key.starts_with(&format!("{k} ")) && best.map(|b| k.len() > b.1).unwrap_or(true) {
                best = Some((p, k.len()));
            }
        }
    }
    best.map(|b| b.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_valid() {
        let b = builtin();
        assert!(b.len() >= 15, "toute la table doit etre valide");
        let mut ids: Vec<&str> = b.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), b.len(), "ids uniques");
        // chaque tuile existe dans le catalogue
        let cat = crate::tv::catalog();
        for p in &b {
            assert!(
                cat.iter().any(|t| t["id"] == p.tile.as_str()),
                "tuile inconnue : {}",
                p.tile
            );
        }
    }

    #[test]
    fn templates() {
        assert!(valid_template("https://www.arte.tv/fr/search/?q={q}"));
        assert!(valid_template("https://tubitv.com/search/{q}"));
        assert!(!valid_template("http://x.com/?q={q}"));
        assert!(!valid_template("https://x.com/?q="));
        assert!(!valid_template("https://{q}.evil.com/"));
        assert!(!valid_template("https://user@x.com/?q={q}"));
        assert!(!valid_template("https://x.com/?q={q} y"));
        assert!(!valid_template("https://x/?q={q}"));
        assert!(!valid_template(&format!("https://x.com/{}{{q}}", "a".repeat(400))));
    }

    #[test]
    fn url_building() {
        assert_eq!(
            build_url("https://x.com/s?q={q}", "L'Été & Co"),
            "https://x.com/s?q=L%27%C3%89t%C3%A9%20%26%20Co"
        );
        assert_eq!(build_url("https://x.com/{q}/{q}", " a b "), "https://x.com/a%20b/a%20b");
        let p = builtin().into_iter().find(|p| p.id == "arte").unwrap();
        assert!(p.url("film", "en").contains("/en/search/"));
        assert!(p.url("film", "fr").contains("/fr/search/"));
        assert!(p.url("film", "de").contains("/fr/search/"));
    }

    #[test]
    fn name_mapping() {
        let b = builtin();
        let id = |n: &str| find(&b, n).map(|p| p.id.as_str());
        assert_eq!(id("Netflix"), Some("netflix"));
        assert_eq!(id("Netflix basic with Ads"), Some("netflix"));
        assert_eq!(id("Amazon Prime Video"), Some("prime"));
        assert_eq!(id("Amazon Prime Video with Ads"), Some("prime"));
        assert_eq!(id("Disney Plus"), Some("disney"));
        assert_eq!(id("Disney+"), Some("disney"));
        assert_eq!(id("Canal+"), Some("canal"));
        assert_eq!(id("Canal+ Séries"), Some("canal"));
        assert_eq!(id("ARTE"), Some("arte"));
        assert_eq!(id("Apple TV Plus"), Some("appletv"));
        assert_eq!(id("Paramount Plus Apple TV Channel"), Some("paramount"));
        assert_eq!(id("Filmo"), None);
        assert_eq!(id(""), None);
    }

    #[test]
    fn mod_providers() {
        let e = serde_json::json!([
            { "id": "mod:a", "mod": "a", "name": "Foo", "tile": "mod-a-foo", "urlTemplate": "https://foo.example.org/s?q={q}", "providerNames": ["Foo Play"], "kinds": ["movie"] },
            { "id": "mod:b", "mod": "b", "name": "Bad", "tile": "x", "urlTemplate": "http://bad.example.org/s?q={q}", "providerNames": [] }
        ]);
        let v = from_mods(e.as_array().unwrap());
        assert_eq!(v.len(), 1);
        assert_eq!(find(&v, "foo play").map(|p| p.tile.as_str()), Some("mod-a-foo"));
        assert_eq!(v[0].origin, "mod:a");
    }
}
