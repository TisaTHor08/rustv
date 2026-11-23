//! TMDB : recherche d'un titre (`search/multi`) puis services de streaming par pays
//! (`/{movie|tv}/{id}/watch/providers`, données fournies par JustWatch).
//! Réseau : uniquement https://api.themoviedb.org, 5 s de délai, réponses bornées, au plus 4 requêtes par recherche.
//! Le jeton n'est jamais journalisé. Les réponses sont gardées 24 h sur disque.
use super::text::{fold, percent_encode, score};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

const API: &str = "https://api.themoviedb.org/3";
pub const IMG: &str = "https://image.tmdb.org/t/p/w185";
const MAX_BODY: u64 = 512 * 1024;
const TTL_SECS: u64 = 24 * 3600;
const MAX_CACHE: usize = 200;
/// Titres pour lesquels on demande les services (une requête chacun).
const MAX_TITLES: usize = 3;

pub fn valid_token(s: &str) -> bool {
    // Jeton « Read Access Token » (JWT) : trois segments base64url séparés par des points.
    (20..=600).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

pub fn valid_country(s: &str) -> bool {
    s.len() == 2 && s.chars().all(|c| c.is_ascii_uppercase())
}

/// Langue de l'interface → étiquette TMDB.
pub fn lang_tag(lang: &str) -> &'static str {
    if lang == "en" {
        "en-US"
    } else {
        "fr-FR"
    }
}

/// Pays par défaut déduit de la langue de l'interface.
pub fn default_country(lang: &str) -> &'static str {
    if lang == "en" {
        "US"
    } else {
        "FR"
    }
}

fn year_of(date: &Value) -> Option<u32> {
    date.as_str()?
        .get(..4)?
        .parse()
        .ok()
        .filter(|y| (1870..=2200).contains(y))
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

/// Réponse de `search/multi` → titres (films et séries uniquement, dans l'ordre de TMDB).
pub fn parse_search(v: &Value) -> Vec<Value> {
    v["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let kind = r["media_type"].as_str().filter(|k| *k == "movie" || *k == "tv")?;
            let id = r["id"].as_u64()?;
            let title = r["title"].as_str().or_else(|| r["name"].as_str()).filter(|t| !t.is_empty())?;
            let year = year_of(if kind == "movie" { &r["release_date"] } else { &r["first_air_date"] });
            let poster = r["poster_path"].as_str().filter(|p| p.starts_with('/') && p.len() < 80 && !p.contains("..") && p.chars().all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))).map(|p| format!("{IMG}{p}"));
            Some(json!({ "id": id, "kind": kind, "title": clip(title, 120), "year": year, "poster": poster, "overview": clip(r["overview"].as_str().unwrap_or(""), 280) }))
        })
        .take(8)
        .collect()
}

/// Réponse de `watch/providers` → services du pays : `[{ name, kind }]`, kind = flatrate | free | rent | buy.
/// Un service présent sous plusieurs formes garde la meilleure (abonnement avant gratuit avant location avant achat).
pub fn parse_providers(v: &Value, country: &str) -> Vec<Value> {
    let c = &v["results"][country];
    let mut best: Vec<(String, usize)> = vec![];
    for (key, rank) in [("flatrate", 0usize), ("free", 1), ("ads", 1), ("rent", 2), ("buy", 3)] {
        for p in c[key].as_array().into_iter().flatten() {
            let Some(name) = p["provider_name"].as_str().filter(|n| !n.is_empty() && n.len() < 80) else {
                continue;
            };
            match best.iter_mut().find(|(n, _)| n == name) {
                Some(e) => e.1 = e.1.min(rank),
                None => best.push((name.to_string(), rank)),
            }
        }
    }
    best.sort_by_key(|(_, r)| *r);
    const KINDS: [&str; 4] = ["flatrate", "free", "rent", "buy"];
    best.into_iter()
        .map(|(n, r)| json!({ "name": n, "kind": KINDS[r] }))
        .collect()
}

// ---------- cache disque (24 h) ----------

#[derive(Default)]
pub struct Cache {
    map: HashMap<String, (u64, Value)>,
}

impl Cache {
    pub fn get(&self, key: &str, now: u64) -> Option<Value> {
        self.map
            .get(key)
            .filter(|(t, _)| now.saturating_sub(*t) < TTL_SECS)
            .map(|(_, v)| v.clone())
    }

    pub fn put(&mut self, key: String, now: u64, v: Value) {
        self.map.retain(|_, (t, _)| now.saturating_sub(*t) < TTL_SECS);
        if self.map.len() >= MAX_CACHE {
            if let Some(oldest) = self.map.iter().min_by_key(|(_, (t, _))| *t).map(|(k, _)| k.clone()) {
                self.map.remove(&oldest);
            }
        }
        self.map.insert(key, (now, v));
    }

    fn to_json(&self) -> Value {
        Value::Object(
            self.map
                .iter()
                .map(|(k, (t, v))| (k.clone(), json!({ "t": t, "v": v })))
                .collect(),
        )
    }

    fn from_json(v: &Value) -> Cache {
        let map = v
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, e)| Some((k.clone(), (e["t"].as_u64()?, e["v"].clone()))))
            .take(MAX_CACHE)
            .collect();
        Cache { map }
    }
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn cache_file() -> std::path::PathBuf {
    super::dir().join("tmdb-cache.json")
}

fn with_cache<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let c = g.get_or_insert_with(|| {
        std::fs::read_to_string(cache_file())
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .map(|v| Cache::from_json(&v))
            .unwrap_or_default()
    });
    f(c)
}

fn persist() {
    let Some(s) = with_cache(|c| serde_json::to_string(&c.to_json()).ok()) else {
        return;
    };
    if s.len() > 2 * 1024 * 1024 {
        return;
    }
    let _ = super::write_atomic(&cache_file(), s.as_bytes());
}

pub fn clear_cache() {
    with_cache(|c| c.map.clear());
    let _ = std::fs::remove_file(cache_file());
}

// ---------- réseau ----------

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .redirects(0)
        .user_agent("RusTV")
        .build()
}

fn get_json(path_and_query: &str, token: &str) -> Result<Value, String> {
    let url = format!("{API}{path_and_query}");
    let resp = match agent()
        .get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/json")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => return Err("tmdb_unauthorized".into()),
        Err(ureq::Error::Status(429, _)) => return Err("tmdb_rate".into()),
        Err(ureq::Error::Status(_, _)) => return Err("tmdb_failed".into()),
        Err(_) => return Err("tmdb_offline".into()),
    };
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_BODY)
        .read_to_string(&mut body)
        .map_err(|_| "tmdb_failed".to_string())?;
    serde_json::from_str(&body).map_err(|_| "tmdb_failed".to_string())
}

/// Vérifie le jeton sans rien chercher.
pub fn test(token: &str) -> Result<(), String> {
    if !valid_token(token) {
        return Err("tmdb_token_format".into());
    }
    get_json("/configuration", token).map(|_| ())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Titres correspondant à `q` avec leurs services dans `country`. `cancelled` est consulté avant chaque requête.
pub fn lookup(
    q: &str,
    lang: &str,
    country: &str,
    token: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<Value>, String> {
    if !valid_token(token) {
        return Err("tmdb_token_format".into());
    }
    if !valid_country(country) {
        return Err("tmdb_country".into());
    }
    let key = format!("{}|{}|{}", fold(q), lang_tag(lang), country);
    if let Some(v) = with_cache(|c| c.get(&key, now())) {
        if let Some(a) = v.as_array() {
            return Ok(a.clone());
        }
    }
    if cancelled() {
        return Err("cancelled".into());
    }
    let sr = get_json(
        &format!(
            "/search/multi?query={}&include_adult=false&language={}&page=1",
            percent_encode(q.trim()),
            lang_tag(lang)
        ),
        token,
    )?;
    let mut titles = parse_search(&sr);
    // Les titres qui ressemblent le plus à la saisie d'abord (tri stable : l'ordre de TMDB départage).
    let fq = fold(q);
    titles.sort_by_key(|t| std::cmp::Reverse(score(&fq, &fold(t["title"].as_str().unwrap_or(""))).unwrap_or(0)));
    titles.truncate(MAX_TITLES);
    for t in titles.iter_mut() {
        if cancelled() {
            return Err("cancelled".into());
        }
        let kind = if t["kind"] == "movie" { "movie" } else { "tv" };
        let id = t["id"].as_u64().unwrap_or(0);
        let pr = get_json(&format!("/{kind}/{id}/watch/providers"), token).unwrap_or(Value::Null);
        t["providers"] = Value::Array(parse_providers(&pr, country));
    }
    let v = Value::Array(titles.clone());
    with_cache(|c| c.put(key, now(), v));
    persist();
    Ok(titles)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = r#"{"page":1,"results":[
      {"id":603,"media_type":"movie","title":"The Matrix","release_date":"1999-03-30","poster_path":"/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg","overview":"Un hacker découvre la vérité."},
      {"id":2288,"media_type":"person","name":"Keanu Reeves"},
      {"id":1396,"media_type":"tv","name":"Breaking Bad","first_air_date":"2008-01-20","poster_path":null,"overview":""},
      {"id":5,"media_type":"movie","title":"","release_date":""},
      {"id":6,"media_type":"movie","title":"Sans date","release_date":"","poster_path":"/../evil"}
    ]}"#;

    const PROVIDERS: &str = r#"{"id":603,"results":{
      "FR":{"link":"https://www.themoviedb.org/movie/603/watch?locale=FR",
        "flatrate":[{"provider_id":8,"provider_name":"Netflix"},{"provider_id":119,"provider_name":"Amazon Prime Video"}],
        "rent":[{"provider_id":2,"provider_name":"Apple TV"},{"provider_id":8,"provider_name":"Netflix"}],
        "buy":[{"provider_id":2,"provider_name":"Apple TV"},{"provider_id":3,"provider_name":"Google Play Movies"}],
        "ads":[{"provider_id":99,"provider_name":"Pluto TV"}]},
      "US":{"flatrate":[{"provider_id":1,"provider_name":"Max"}]}}}"#;

    #[test]
    fn search_parsing() {
        let t = parse_search(&serde_json::from_str(SEARCH).unwrap());
        assert_eq!(t.len(), 3, "personnes et titres vides ignores");
        assert_eq!(t[0]["title"], "The Matrix");
        assert_eq!(t[0]["kind"], "movie");
        assert_eq!(t[0]["year"], 1999);
        assert_eq!(
            t[0]["poster"],
            "https://image.tmdb.org/t/p/w185/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg"
        );
        assert_eq!(t[1]["title"], "Breaking Bad");
        assert_eq!(t[1]["kind"], "tv");
        assert_eq!(t[1]["year"], 2008);
        assert!(t[1]["poster"].is_null());
        assert!(t[2]["year"].is_null());
        assert!(t[2]["poster"].is_null(), "chemin d'affiche suspect refuse");
        assert!(parse_search(&json!({})).is_empty());
    }

    #[test]
    fn provider_parsing() {
        let v: Value = serde_json::from_str(PROVIDERS).unwrap();
        let p = parse_providers(&v, "FR");
        let get = |n: &str| {
            p.iter()
                .find(|x| x["name"] == n)
                .map(|x| x["kind"].as_str().unwrap().to_string())
        };
        assert_eq!(get("Netflix").as_deref(), Some("flatrate"), "meilleure forme gardee");
        assert_eq!(get("Amazon Prime Video").as_deref(), Some("flatrate"));
        assert_eq!(get("Pluto TV").as_deref(), Some("free"));
        assert_eq!(get("Apple TV").as_deref(), Some("rent"));
        assert_eq!(get("Google Play Movies").as_deref(), Some("buy"));
        assert_eq!(p.len(), 5);
        // abonnement en premier
        assert_eq!(p[0]["kind"], "flatrate");
        assert_eq!(parse_providers(&v, "US").len(), 1);
        assert!(parse_providers(&v, "DE").is_empty());
        assert!(parse_providers(&Value::Null, "FR").is_empty());
    }

    #[test]
    fn inputs() {
        assert!(valid_token("eyJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJ4In0.abc-def_123"));
        assert!(
            !valid_token("short") && !valid_token("has space has space has space") && !valid_token(&"a".repeat(700))
        );
        assert!(valid_country("FR") && !valid_country("fr") && !valid_country("FRA"));
        assert_eq!(lang_tag("en"), "en-US");
        assert_eq!(default_country("fr"), "FR");
    }

    #[test]
    fn cache_ttl_and_cap() {
        let mut c = Cache::default();
        c.put("a".into(), 1000, json!([1]));
        assert!(c.get("a", 1000 + TTL_SECS - 1).is_some());
        assert!(c.get("a", 1000 + TTL_SECS).is_none());
        for i in 0..(MAX_CACHE + 20) {
            c.put(format!("k{i}"), 2000 + i as u64, json!(i));
        }
        assert!(c.map.len() <= MAX_CACHE);
        let back = Cache::from_json(&c.to_json());
        assert_eq!(back.map.len(), c.map.len());
    }
}
