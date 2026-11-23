//! Médiathèques privées de l'utilisateur : Plex et Jellyfin (adresse + jeton saisis dans les réglages).
//! Les requêtes ne partent que vers l'adresse configurée ; le jeton est envoyé dans un en-tête, jamais dans
//! l'URL ni dans les journaux. Les résultats ouvrent la page du titre dans l'application web du serveur
//! (tuile Plex / Jellyfin), sans jamais mettre le jeton dans une adresse de lecture.
use super::text::percent_encode;
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;

const MAX_BODY: u64 = 1024 * 1024;
const MAX_RESULTS: usize = 8;

/// `http(s)://hote[:port]` sans chemin, sans identifiants ; `None` si l'adresse n'est pas acceptable.
pub fn origin(url: &str) -> Option<String> {
    let u = url.trim();
    let (scheme, rest) = if let Some(r) = u.strip_prefix("https://") {
        ("https", r)
    } else if let Some(r) = u.strip_prefix("http://") {
        ("http", r)
    } else {
        return None;
    };
    let host = rest.split(['/', '?', '#']).next()?;
    let ok = !host.is_empty()
        && host.len() <= 120
        && !host.contains(['@', ' ', '\\'])
        && host.chars().all(|c| c.is_ascii_alphanumeric() || "-.:[]".contains(c));
    ok.then(|| format!("{scheme}://{host}"))
}

pub fn valid_secret(s: &str) -> bool {
    (8..=200).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.~+=".contains(c))
}

fn get_json(url: &str, headers: &[(&str, &str)], secs: u64) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(secs))
        .redirects(0)
        .user_agent("RusTV")
        .build();
    let mut req = agent.get(url).set("Accept", "application/json");
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(c, _)) => return Err(format!("http_{c}")),
        Err(_) => return Err("offline".into()),
    };
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_BODY)
        .read_to_string(&mut body)
        .map_err(|_| "bad_response".to_string())?;
    serde_json::from_str(&body).map_err(|_| "bad_response".to_string())
}

// ---------- Plex ----------

/// Réponse de `/hubs/search` → éléments `{ type, title, year, ratingKey, parent? }` (films, séries, épisodes).
pub fn parse_plex(v: &Value) -> Vec<Value> {
    let mut out = vec![];
    for hub in v["MediaContainer"]["Hub"].as_array().into_iter().flatten() {
        for m in hub["Metadata"].as_array().into_iter().flatten() {
            let kind = match m["type"].as_str() {
                Some(k @ ("movie" | "show" | "episode")) => k,
                _ => continue,
            };
            let (Some(title), Some(key)) = (
                m["title"].as_str(),
                m["ratingKey"]
                    .as_str()
                    .map(String::from)
                    .or_else(|| m["ratingKey"].as_u64().map(|n| n.to_string())),
            ) else {
                continue;
            };
            if !key.chars().all(|c| c.is_ascii_digit()) || key.is_empty() || key.len() > 12 {
                continue;
            }
            out.push(json!({ "type": kind, "title": title, "year": m["year"].as_u64(), "ratingKey": key, "parent": m["grandparentTitle"].as_str() }));
            if out.len() >= MAX_RESULTS {
                return out;
            }
        }
    }
    out
}

/// Lien profond vers la fiche d'un titre dans l'application web Plex (le compte est celui de la tuile Plex).
pub fn plex_link(machine_id: &str, rating_key: &str) -> Option<String> {
    let ok = |s: &str| !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    (ok(machine_id) && ok(rating_key)).then(|| {
        format!("https://app.plex.tv/desktop/#!/server/{machine_id}/details?key=%2Flibrary%2Fmetadata%2F{rating_key}")
    })
}

pub fn plex_search(base: &str, token: &str, q: &str, cancelled: &dyn Fn() -> bool) -> Result<Vec<Value>, String> {
    let hdr = [("X-Plex-Token", token)];
    let v = get_json(
        &format!(
            "{base}/hubs/search?query={}&limit={MAX_RESULTS}",
            percent_encode(q.trim())
        ),
        &hdr,
        4,
    )
    .map_err(|e| plex_err(&e))?;
    let items = parse_plex(&v);
    if items.is_empty() || cancelled() {
        return Ok(vec![]);
    }
    let id = get_json(&format!("{base}/identity"), &[], 4).map_err(|e| plex_err(&e))?;
    let machine = id["MediaContainer"]["machineIdentifier"].as_str().unwrap_or("");
    Ok(items
        .into_iter()
        .filter_map(|i| {
            let url = plex_link(machine, i["ratingKey"].as_str()?)?;
            Some(json!({ "source": "plex", "title": i["title"], "year": i["year"], "kind": i["type"], "parent": i["parent"], "tileId": "plex", "url": url }))
        })
        .collect())
}

fn plex_err(e: &str) -> String {
    match e {
        "http_401" | "http_403" => "plex_unauthorized".into(),
        "offline" => "plex_offline".into(),
        _ => "plex_failed".into(),
    }
}

pub fn plex_test(base: &str, token: &str) -> Result<String, String> {
    let id = get_json(&format!("{base}/identity"), &[], 4).map_err(|e| plex_err(&e))?;
    // /identity est public : on vérifie le jeton avec une route protégée.
    get_json(&format!("{base}/library/sections"), &[("X-Plex-Token", token)], 4).map_err(|e| plex_err(&e))?;
    Ok(id["MediaContainer"]["version"].as_str().unwrap_or("").to_string())
}

// ---------- Jellyfin ----------

/// Réponse de `/Items` → éléments `{ type, title, year, id, serverId, parent? }`.
pub fn parse_jellyfin(v: &Value) -> Vec<Value> {
    v["Items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let kind = match m["Type"].as_str() {
                Some(k @ ("Movie" | "Series" | "Episode")) => k,
                _ => return None,
            };
            let (title, id) = (m["Name"].as_str()?, m["Id"].as_str()?);
            let server = m["ServerId"].as_str().unwrap_or("");
            let hex = |s: &str| s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
            (!title.is_empty() && !id.is_empty() && hex(id) && hex(server)).then(|| json!({ "type": kind, "title": title, "year": m["ProductionYear"].as_u64(), "id": id, "serverId": server, "parent": m["SeriesName"].as_str() }))
        })
        .take(MAX_RESULTS)
        .collect()
}

pub fn jellyfin_link(base: &str, id: &str, server_id: &str) -> String {
    if server_id.is_empty() {
        format!("{base}/web/index.html#!/details?id={id}")
    } else {
        format!("{base}/web/index.html#!/details?id={id}&serverId={server_id}")
    }
}

fn jf_err(e: &str) -> String {
    match e {
        "http_401" | "http_403" => "jellyfin_unauthorized".into(),
        "offline" => "jellyfin_offline".into(),
        _ => "jellyfin_failed".into(),
    }
}

pub fn jellyfin_search(base: &str, key: &str, q: &str) -> Result<Vec<Value>, String> {
    let url = format!("{base}/Items?searchTerm={}&IncludeItemTypes=Movie,Series,Episode&Recursive=true&Limit={MAX_RESULTS}&Fields=ProductionYear", percent_encode(q.trim()));
    let v = get_json(&url, &[("X-Emby-Token", key)], 4).map_err(|e| jf_err(&e))?;
    Ok(parse_jellyfin(&v)
        .into_iter()
        .map(|i| {
            let link = jellyfin_link(base, i["id"].as_str().unwrap_or(""), i["serverId"].as_str().unwrap_or(""));
            json!({ "source": "jellyfin", "title": i["title"], "year": i["year"], "kind": i["type"], "parent": i["parent"], "tileId": "jellyfin", "url": link })
        })
        .collect())
}

pub fn jellyfin_test(base: &str, key: &str) -> Result<String, String> {
    let v = get_json(&format!("{base}/System/Info"), &[("X-Emby-Token", key)], 4).map_err(|e| jf_err(&e))?;
    Ok(v["ServerName"].as_str().unwrap_or("").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins() {
        assert_eq!(
            origin("http://192.168.1.10:32400/web/index.html").as_deref(),
            Some("http://192.168.1.10:32400")
        );
        assert_eq!(
            origin(" https://plex.example.org/ ").as_deref(),
            Some("https://plex.example.org")
        );
        assert!(
            origin("ftp://x").is_none()
                && origin("http://u:p@x.org").is_none()
                && origin("http://").is_none()
                && origin("x.org").is_none()
        );
        assert!(origin("http://a b").is_none());
        assert!(valid_secret("abcDEF12_-xyz") && !valid_secret("short") && !valid_secret("has space in it ok"));
    }

    #[test]
    fn plex_payload() {
        let v: Value = serde_json::from_str(
            r#"{"MediaContainer":{"size":3,"Hub":[
              {"type":"movie","title":"Movies","size":1,"Metadata":[{"ratingKey":"4521","key":"/library/metadata/4521","type":"movie","title":"Amélie","year":2001}]},
              {"type":"show","title":"Shows","Metadata":[{"ratingKey":"77","type":"show","title":"Dark","year":2017}]},
              {"type":"episode","title":"Episodes","Metadata":[{"ratingKey":"90","type":"episode","title":"Secrets","grandparentTitle":"Dark"}]},
              {"type":"artist","title":"Artists","Metadata":[{"ratingKey":"5","type":"artist","title":"Amelie Lens"}]},
              {"type":"movie","title":"Movies","Metadata":[{"ratingKey":"../x","type":"movie","title":"Bad"}]},
              {"type":"actor","title":"People","size":0}
            ]}}"#,
        )
        .unwrap();
        let r = parse_plex(&v);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0]["title"], "Amélie");
        assert_eq!(r[0]["year"], 2001);
        assert_eq!(r[1]["type"], "show");
        assert_eq!(r[2]["parent"], "Dark");
        assert_eq!(
            plex_link("abc123def", "4521").unwrap(),
            "https://app.plex.tv/desktop/#!/server/abc123def/details?key=%2Flibrary%2Fmetadata%2F4521"
        );
        assert!(plex_link("", "1").is_none() && plex_link("a b", "1").is_none());
        assert!(parse_plex(&Value::Null).is_empty());
    }

    #[test]
    fn jellyfin_payload() {
        let v: Value = serde_json::from_str(
            r#"{"Items":[
              {"Name":"Amélie","Id":"0a1b2c3d4e5f60718293a4b5c6d7e8f9","ServerId":"f00ba4","Type":"Movie","ProductionYear":2001},
              {"Name":"Pilot","Id":"11","ServerId":"f00ba4","Type":"Episode","SeriesName":"Lost"},
              {"Name":"Album","Id":"22","Type":"MusicAlbum"},
              {"Name":"Evil","Id":"../..","Type":"Movie"}
            ],"TotalRecordCount":4}"#,
        )
        .unwrap();
        let r = parse_jellyfin(&v);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["year"], 2001);
        assert_eq!(r[1]["parent"], "Lost");
        assert_eq!(
            jellyfin_link("http://nas:8096", "11", "f00ba4"),
            "http://nas:8096/web/index.html#!/details?id=11&serverId=f00ba4"
        );
        assert_eq!(
            jellyfin_link("http://nas:8096", "11", ""),
            "http://nas:8096/web/index.html#!/details?id=11"
        );
    }

    #[test]
    fn error_mapping() {
        assert_eq!(plex_err("http_401"), "plex_unauthorized");
        assert_eq!(plex_err("offline"), "plex_offline");
        assert_eq!(jf_err("http_500"), "jellyfin_failed");
    }
}
