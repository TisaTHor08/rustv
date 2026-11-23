//! Recherche universelle : un titre → où le regarder (TMDB / JustWatch), dans quelles médiathèques privées
//! (Plex, Jellyfin, UGREEN Theater) et dans quels fichiers des dossiers choisis (index local).
//!
//! Vie privée : seule la saisie part vers api.themoviedb.org (si un jeton TMDB est configuré) et vers les
//! serveurs Plex / Jellyfin de l'utilisateur. Les noms de fichiers restent sur le PC. Jetons protégés par DPAPI.
//! Rien n'est actif sans configuration : sans jeton ni serveur, seuls les dossiers déjà choisis sont indexés.
pub mod index;
pub mod providers;
pub mod secret;
pub mod servers;
pub mod text;
pub mod tmdb;

use crate::{settings, tv, Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

const MAX_QUERY: usize = 100;
const MAX_FOLDERS: usize = 32;
const MAX_FILE_HITS: usize = 30;

pub fn dir() -> PathBuf {
    settings::dir().join("search")
}

/// Écriture atomique (fichier temporaire puis renommage).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

// ---------- configuration ----------

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Sources {
    pub tmdb: bool,
    pub library: bool,
    pub files: bool,
    pub plex: bool,
    pub jellyfin: bool,
    pub ugreen: bool,
}

impl Default for Sources {
    fn default() -> Self {
        Sources {
            tmdb: true,
            library: true,
            files: true,
            plex: true,
            jellyfin: true,
            ugreen: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    /// Jeton TMDB protégé par DPAPI (hexadécimal) ; vide = non configuré.
    pub tmdb: String,
    pub country: String,
    pub sources: Sources,
    pub folders: Vec<String>,
    pub plex_url: String,
    pub plex_token: String,
    pub jellyfin_url: String,
    pub jellyfin_key: String,
}

fn load_config() -> Config {
    crate::store::load("usearch")
}

fn unseal(sealed: &str) -> Option<String> {
    if sealed.is_empty() {
        return None;
    }
    secret::open(&secret::Dpapi, sealed).ok().filter(|s| !s.is_empty())
}

fn ugreen_url(app: &AppHandle) -> String {
    app.state::<Shell>()
        .settings
        .lock()
        .map(|s| s.urls.get("ugreen").cloned().unwrap_or_default())
        .unwrap_or_default()
}

fn jellyfin_tile_url(app: &AppHandle) -> String {
    app.state::<Shell>()
        .settings
        .lock()
        .map(|s| s.urls.get("jellyfin").cloned().unwrap_or_default())
        .unwrap_or_default()
}

/// Dossiers de la médiathèque locale (films, séries, photos, musique) déjà choisis par l'utilisateur.
fn library_folders() -> Vec<String> {
    let p = crate::prefs::get();
    ["movies", "series", "photos", "music"]
        .iter()
        .flat_map(|k| p["library"][*k].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

fn all_roots(cfg: &Config) -> Vec<String> {
    index::merge_roots(&[&cfg.folders, &library_folders()])
}

fn public_config(app: &AppHandle, cfg: &Config) -> Value {
    json!({
        "sources": cfg.sources,
        "country": cfg.country,
        "folders": cfg.folders,
        "libraryFolders": index::merge_roots(&[&library_folders()]),
        "hasTmdb": unseal(&cfg.tmdb).is_some(),
        "plex": { "url": cfg.plex_url, "hasToken": unseal(&cfg.plex_token).is_some() },
        "jellyfin": { "url": cfg.jellyfin_url, "hasKey": unseal(&cfg.jellyfin_key).is_some(), "tileUrl": jellyfin_tile_url(app) },
        "ugreen": { "url": ugreen_url(app) },
    })
}

/// Applique un correctif partiel à la configuration. Les jetons ne sont jamais relus : `""` les efface.
pub fn apply_patch(cfg: &mut Config, p: &Value) -> Result<(), String> {
    if let Some(t) = p["tmdbToken"].as_str() {
        let t = t.trim();
        cfg.tmdb = if t.is_empty() {
            String::new()
        } else if tmdb::valid_token(t) {
            secret::seal(&secret::Dpapi, t)?
        } else {
            return Err("tmdb_token_format".into());
        };
    }
    if let Some(c) = p["country"].as_str() {
        let c = c.trim().to_uppercase();
        if !c.is_empty() && !tmdb::valid_country(&c) {
            return Err("tmdb_country".into());
        }
        cfg.country = c;
    }
    if let Some(s) = p["sources"].as_object() {
        let src = &mut cfg.sources;
        for (k, v) in s {
            let Some(b) = v.as_bool() else { continue };
            match k.as_str() {
                "tmdb" => src.tmdb = b,
                "library" => src.library = b,
                "files" => src.files = b,
                "plex" => src.plex = b,
                "jellyfin" => src.jellyfin = b,
                "ugreen" => src.ugreen = b,
                _ => {}
            }
        }
    }
    if let Some(f) = p["folders"].as_array() {
        let list: Vec<String> = f.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        let merged = index::merge_roots(&[&list]);
        if merged.len() > MAX_FOLDERS {
            return Err("too_many_folders".into());
        }
        cfg.folders = merged;
    }
    for (name, url_key, secret_key) in [("plex", "url", "token"), ("jellyfin", "url", "key")] {
        let o = &p[name];
        if let Some(u) = o[url_key].as_str() {
            let u = u.trim();
            let v = if u.is_empty() {
                String::new()
            } else {
                servers::origin(u).ok_or("bad_url")?
            };
            if name == "plex" {
                cfg.plex_url = v;
            } else {
                cfg.jellyfin_url = v;
            }
        }
        if let Some(s) = o[secret_key].as_str() {
            let s = s.trim();
            let v = if s.is_empty() {
                String::new()
            } else if servers::valid_secret(s) {
                secret::seal(&secret::Dpapi, s)?
            } else {
                return Err("bad_token".into());
            };
            if name == "plex" {
                cfg.plex_token = v;
            } else {
                cfg.jellyfin_key = v;
            }
        }
    }
    Ok(())
}

// ---------- requête ----------

/// Numéro de la dernière recherche demandée : une recherche plus ancienne s'arrête avant sa prochaine requête.
static LATEST: AtomicU64 = AtomicU64::new(0);

fn is_stale(seq: u64) -> bool {
    LATEST.load(Ordering::SeqCst) > seq
}

fn kind_label(k: char) -> &'static str {
    match k {
        'a' => "audio",
        'p' => "photo",
        _ => "video",
    }
}

/// Résultats de l'index local, prêts pour l'interface.
fn file_results(q: &str) -> Vec<Value> {
    let fq = text::fold(q);
    index::with_index(|i| i.search(&fq, MAX_FILE_HITS))
        .into_iter()
        .map(|h| {
            let e = index::Entry::new(h.path.clone(), h.kind);
            let (title, year) = e.title();
            json!({ "source": "files", "title": title, "year": year, "kind": kind_label(h.kind), "sub": e.folder(), "path": h.path, "playable": h.kind != 'p', "score": h.score })
        })
        .collect()
}

/// Associe les services renvoyés par TMDB aux tuiles de RusTV et construit l'URL de recherche du titre.
pub fn attach_providers(titles: &mut [Value], provs: &[providers::Provider], lang: &str) {
    for t in titles.iter_mut() {
        let name = t["title"].as_str().unwrap_or("").to_string();
        let mut seen: Vec<String> = vec![];
        let mut out: Vec<Value> = vec![];
        for p in t["providers"].as_array().cloned().unwrap_or_default() {
            let n = p["name"].as_str().unwrap_or("");
            match providers::find(provs, n) {
                Some(f) => {
                    if seen.contains(&f.id) {
                        continue;
                    }
                    seen.push(f.id.clone());
                    out.push(json!({ "name": f.name, "kind": p["kind"], "id": f.id, "tileId": f.tile, "url": f.url(&name, lang), "mapped": true }));
                }
                None => out.push(json!({ "name": n, "kind": p["kind"], "mapped": false })),
            }
        }
        t["providers"] = Value::Array(out);
    }
}

fn run_query(app: &AppHandle, q: &str, seq: u64, lang: &str, ui_country: &str) -> Result<Value, String> {
    let q = q.trim();
    let n = q.chars().count();
    if !(2..=MAX_QUERY).contains(&n) {
        return Ok(json!({ "seq": seq, "titles": [], "library": [], "files": [], "services": [], "tmdb": "idle" }));
    }
    let cfg = load_config();
    let provs = providers::all();

    // ---- partie locale, immédiate ----
    let files = if cfg.sources.files { file_results(q) } else { vec![] };
    let mut library: Vec<Value> = vec![];
    let ug = ugreen_url(app);
    if cfg.sources.ugreen && !ug.is_empty() {
        library.push(
            json!({ "source": "ugreen", "title": "UGREEN Theater", "kind": "app", "tileId": "ugreen", "url": null }),
        );
    }
    let services: Vec<Value> = provs.iter().map(|p| json!({ "id": p.id, "name": p.name, "tileId": p.tile, "url": p.url(q, lang), "origin": p.origin, "kinds": p.kinds })).collect();
    let _ = app.emit(
        "usearch-partial",
        json!({ "seq": seq, "files": files, "library": library, "services": services }),
    );

    // ---- partie distante, en parallèle ----
    let token = if cfg.sources.tmdb { unseal(&cfg.tmdb) } else { None };
    let country = if tmdb::valid_country(&cfg.country) {
        cfg.country.clone()
    } else if tmdb::valid_country(ui_country) {
        ui_country.to_string()
    } else {
        tmdb::default_country(lang).to_string()
    };
    let plex = if cfg.sources.plex {
        servers::origin(&cfg.plex_url).zip(unseal(&cfg.plex_token))
    } else {
        None
    };
    let jf_base = servers::origin(&cfg.jellyfin_url).or_else(|| servers::origin(&jellyfin_tile_url(app)));
    let jellyfin = if cfg.sources.jellyfin {
        jf_base.zip(unseal(&cfg.jellyfin_key))
    } else {
        None
    };
    let stale = move || is_stale(seq);

    let (tm, px, jf) = std::thread::scope(|s| {
        let tm = token
            .as_ref()
            .map(|t| s.spawn(|| tmdb::lookup(q, lang, &country, t, &stale)));
        let px = plex
            .as_ref()
            .map(|(b, t)| s.spawn(|| servers::plex_search(b, t, q, &stale)));
        let jf = jellyfin
            .as_ref()
            .map(|(b, k)| s.spawn(|| servers::jellyfin_search(b, k, q)));
        let join = |h: Option<std::thread::ScopedJoinHandle<'_, Result<Vec<Value>, String>>>| {
            h.map(|h| h.join().unwrap_or_else(|_| Err("failed".into())))
        };
        (join(tm), join(px), join(jf))
    });
    if is_stale(seq) {
        return Err("cancelled".into());
    }

    let mut errors = serde_json::Map::new();
    let mut titles: Vec<Value> = vec![];
    let tmdb_state = match (cfg.sources.tmdb, tm) {
        (false, _) => "off",
        (true, None) => "nokey",
        (true, Some(Ok(t))) => {
            titles = t;
            "ok"
        }
        (true, Some(Err(e))) => {
            errors.insert("tmdb".into(), json!(e));
            "error"
        }
    };
    attach_providers(&mut titles, &provs, lang);
    for (name, r) in [("plex", px), ("jellyfin", jf)] {
        match r {
            Some(Ok(v)) => library.extend(v),
            Some(Err(e)) => {
                errors.insert(name.into(), json!(e));
            }
            None => {}
        }
    }
    Ok(json!({
        "seq": seq,
        "titles": titles,
        "library": library,
        "files": files,
        "services": services,
        "tmdb": tmdb_state,
        "country": country,
        "errors": errors,
        "attribution": if tmdb_state == "ok" { json!({ "justwatch": "Data provided by JustWatch", "tmdb": "This product uses the TMDB API but is not endorsed or certified by TMDB." }) } else { Value::Null },
    }))
}

// ---------- commandes ----------

/// Recherche un titre. `opts` : `{ seq, lang: "fr"|"en", country? }`. Renvoie
/// `{ seq, titles:[{id,kind,title,year,poster,overview,providers:[{name,kind,id?,tileId?,url?,mapped}]}],
///    library:[{source,title,year?,kind,parent?,tileId,url}], files:[{source:"files",title,year,kind,sub,path,playable}],
///    services:[{id,name,tileId,url,origin}], tmdb:"idle|off|nokey|ok|error", country, errors:{tmdb?,plex?,jellyfin?}, attribution }`.
/// Un événement `usearch-partial` `{ seq, files, library, services }` part avant la partie réseau.
/// `Err("cancelled")` si une recherche plus récente a été lancée.
#[tauri::command]
pub async fn usearch_query(app: AppHandle, q: String, opts: Option<Value>) -> Result<Value, String> {
    let o = opts.unwrap_or(Value::Null);
    let seq = o["seq"].as_u64().unwrap_or(0);
    if seq < LATEST.fetch_max(seq, Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    let lang = (if o["lang"] == "en" { "en" } else { "fr" }).to_string();
    let country = o["country"].as_str().unwrap_or("").to_uppercase();
    tauri::async_runtime::spawn_blocking(move || run_query(&app, &q, seq, &lang, &country))
        .await
        .map_err(|_| "failed".to_string())?
}

/// Lance le service d'une tuile, éventuellement sur une adresse précise (page du titre ou recherche).
/// `tile` n'est lu que pour les tuiles absentes du catalogue (tuiles perso ou de mods, décrites par l'interface).
/// Ne mémorise pas l'adresse (contrairement à `launch_tile`). Renvoie `"ok"` ou `"needs_url"`.
#[tauri::command]
pub async fn usearch_launch(
    app: AppHandle,
    tile_id: String,
    url: Option<String>,
    tile: Option<Value>,
) -> Result<String, String> {
    let mut t = match tv::catalog().into_iter().find(|t| t["id"] == tile_id.as_str()) {
        Some(t) => t,
        None => {
            let t = tile.ok_or("unknown_tile")?;
            if t["id"] != tile_id.as_str() {
                return Err("unknown_tile".into());
            }
            t
        }
    };
    if let Some(u) = url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        if t["kind"] != "web" {
            return Err("not_web".into());
        }
        if !safe_launch_url(u) {
            return Err("bad_url".into());
        }
        t["url"] = Value::String(u.to_string());
    }
    tv::launch_value(&app, &t, None)
}

/// Adresse http(s) raisonnable (pas d'espace ni de caractère de contrôle, taille bornée).
pub fn safe_launch_url(u: &str) -> bool {
    u.len() <= 2000
        && (u.starts_with("https://") || u.starts_with("http://"))
        && !u.chars().any(|c| c.is_whitespace() || c.is_control())
        && u.parse::<tauri::Url>().is_ok()
}

/// Configuration sans secrets : `{ sources, country, folders, libraryFolders, hasTmdb, plex:{url,hasToken},
/// jellyfin:{url,hasKey,tileUrl}, ugreen:{url} }`.
#[tauri::command]
pub async fn usearch_get_config(app: AppHandle) -> Result<Value, String> {
    Ok(public_config(&app, &load_config()))
}

/// Correctif partiel : `{ tmdbToken?, country?, sources?:{tmdb,library,files,plex,jellyfin,ugreen}, folders?:[..],
/// plex?:{url?,token?}, jellyfin?:{url?,key?} }`. Une chaîne vide efface une valeur. Erreurs : codes courts.
#[tauri::command]
pub async fn usearch_set_config(patch: Value) -> Result<(), String> {
    let mut cfg = load_config();
    apply_patch(&mut cfg, &patch)?;
    crate::store::save("usearch", &cfg)
}

fn emit_status(app: &AppHandle) {
    let _ = app.emit("usearch-scan", status_json());
}

fn status_json() -> Value {
    let running = index::SCANNER.running.load(Ordering::SeqCst);
    let (roots, total) = index::with_index(|i| {
        (
            i.roots
                .iter()
                .map(|r| json!({ "path": r.path, "status": r.status.as_str(), "count": r.count, "scanned": r.scanned }))
                .collect::<Vec<_>>(),
            i.len(),
        )
    });
    json!({
        "running": running,
        "scanning": index::SCANNER.current.lock().map(|c| c.clone()).unwrap_or_default(),
        "progress": index::SCANNER.files.load(Ordering::Relaxed),
        "total": total,
        "lastScan": roots.iter().filter_map(|r| r["scanned"].as_u64()).max().unwrap_or(0),
        "roots": roots,
    })
}

/// `{ running, scanning, progress, total, lastScan, roots:[{path,status:"ok|truncated|unreachable|cancelled",count,scanned}] }`.
#[tauri::command]
pub async fn usearch_status() -> Result<Value, String> {
    Ok(status_json())
}

/// Lance (ou relance) l'indexation des dossiers listés et de ceux de la médiathèque. Sans `force`, une racine
/// parcourue il y a moins d'une heure est laissée telle quelle. Renvoie `false` si un parcours est déjà en cours.
/// Événement `usearch-scan` (même forme que `usearch_status`) à chaque racine et à la fin.
#[tauri::command]
pub async fn usearch_scan(app: AppHandle, force: Option<bool>) -> Result<bool, String> {
    let cfg = load_config();
    if !cfg.sources.files {
        return Ok(false);
    }
    let a = app.clone();
    Ok(index::start_scan(
        all_roots(&cfg),
        force.unwrap_or(false),
        Arc::new(move |_| emit_status(&a)),
    ))
}

#[tauri::command]
pub async fn usearch_scan_cancel() -> Result<(), String> {
    index::SCANNER.cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Efface le cache des réponses TMDB (24 h) et l'index des fichiers.
#[tauri::command]
pub async fn usearch_clear_cache() -> Result<(), String> {
    tmdb::clear_cache();
    index::with_index(|i| *i = index::Index::default());
    let _ = std::fs::remove_file(dir().join("index.tsv"));
    Ok(())
}

/// Teste une source : `"tmdb" | "plex" | "jellyfin" | "ugreen" | "files"`. `Ok(détail court)` ou `Err(code)`
/// (tmdb_unauthorized, tmdb_offline, plex_unauthorized, plex_offline, jellyfin_unauthorized, not_configured, unreachable…).
#[tauri::command]
pub async fn usearch_test(app: AppHandle, source: String) -> Result<String, String> {
    let cfg = load_config();
    let ug = ugreen_url(&app);
    let jf_tile = jellyfin_tile_url(&app);
    tauri::async_runtime::spawn_blocking(move || match source.as_str() {
        "tmdb" => {
            let t = unseal(&cfg.tmdb).ok_or("not_configured")?;
            tmdb::test(&t).map(|_| "ok".to_string())
        }
        "plex" => {
            let (b, t) = servers::origin(&cfg.plex_url)
                .zip(unseal(&cfg.plex_token))
                .ok_or("not_configured")?;
            servers::plex_test(&b, &t)
        }
        "jellyfin" => {
            let b = servers::origin(&cfg.jellyfin_url)
                .or_else(|| servers::origin(&jf_tile))
                .ok_or("not_configured")?;
            let k = unseal(&cfg.jellyfin_key).ok_or("not_configured")?;
            servers::jellyfin_test(&b, &k)
        }
        "ugreen" => {
            let b = servers::origin(&ug).ok_or("not_configured")?;
            test_tcp(&b).map(|_| "ok".to_string())
        }
        "files" => {
            let roots = all_roots(&cfg);
            if roots.is_empty() {
                return Err("not_configured".into());
            }
            let ok = roots
                .iter()
                .filter(|r| {
                    index::run_with_timeout(std::time::Duration::from_secs(3), {
                        let r: String = (*r).clone();
                        move || std::fs::read_dir(r).is_ok()
                    })
                    .unwrap_or(false)
                })
                .count();
            if ok == 0 {
                Err("unreachable".into())
            } else {
                Ok(format!("{ok}/{}", roots.len()))
            }
        }
        _ => Err("unknown_source".into()),
    })
    .await
    .map_err(|_| "failed".to_string())?
}

/// Connexion TCP seule (UGREEN : pas d'API de recherche, on vérifie juste que le NAS répond).
fn test_tcp(origin: &str) -> Result<(), String> {
    use std::net::ToSocketAddrs;
    let (https, host) = match origin.strip_prefix("https://") {
        Some(h) => (true, h),
        None => (false, origin.strip_prefix("http://").unwrap_or(origin)),
    };
    let target = if host.contains(':') && !host.starts_with('[') {
        host.to_string()
    } else {
        format!("{host}:{}", if https { 443 } else { 80 })
    };
    let addr = target
        .to_socket_addrs()
        .map_err(|_| "unreachable".to_string())?
        .next()
        .ok_or("unreachable")?;
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3))
        .map(|_| ())
        .map_err(|_| "unreachable".to_string())
}

/// Au démarrage : charge l'index et, après un délai (le boot de la TV passe avant), met à jour les dossiers
/// qui n'ont pas été parcourus depuis plus d'une heure. Aucun parcours périodique ensuite.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(45));
        let cfg = load_config();
        if cfg.sources.files {
            let a = app.clone();
            index::start_scan(all_roots(&cfg), false, Arc::new(move |_| emit_status(&a)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_ordering() {
        LATEST.store(0, Ordering::SeqCst);
        assert!(!is_stale(5));
        LATEST.store(7, Ordering::SeqCst);
        assert!(is_stale(5) && !is_stale(7) && !is_stale(8));
        LATEST.store(0, Ordering::SeqCst);
    }

    #[test]
    fn providers_attached_to_titles() {
        let provs = providers::builtin();
        let mut titles = vec![json!({
            "title": "Amélie & Co", "providers": [
                { "name": "Netflix", "kind": "flatrate" },
                { "name": "Netflix basic with Ads", "kind": "flatrate" },
                { "name": "Canal+", "kind": "rent" },
                { "name": "Filmo", "kind": "buy" }
            ]
        })];
        attach_providers(&mut titles, &provs, "fr");
        let p = titles[0]["providers"].as_array().unwrap();
        assert_eq!(p.len(), 3, "deux noms du meme service = une puce");
        assert_eq!(p[0]["tileId"], "netflix");
        assert_eq!(p[0]["url"], "https://www.netflix.com/search?q=Am%C3%A9lie%20%26%20Co");
        assert_eq!(p[1]["tileId"], "canal");
        assert_eq!(p[1]["kind"], "rent");
        assert_eq!(p[2]["mapped"], false);
        assert!(p[2]["tileId"].is_null());
    }

    #[test]
    fn config_patch() {
        let mut c = Config::default();
        apply_patch(&mut c, &json!({ "country": " be ", "sources": { "plex": false, "bogus": true, "tmdb": "x" }, "folders": ["D:\\Films\\", "d:\\films", "\\\\NAS\\Video"],
            "plex": { "url": "http://192.168.1.5:32400/web", "token": "" }, "jellyfin": { "url": "" } }))
        .unwrap();
        assert_eq!(c.country, "BE");
        assert!(!c.sources.plex && c.sources.tmdb && c.sources.files);
        assert_eq!(c.folders, vec!["D:\\Films", "\\\\NAS\\Video"]);
        assert_eq!(c.plex_url, "http://192.168.1.5:32400");
        // erreurs
        assert_eq!(
            apply_patch(&mut c, &json!({ "country": "FRA" })),
            Err("tmdb_country".into())
        );
        assert_eq!(
            apply_patch(&mut c, &json!({ "plex": { "url": "ftp://x" } })),
            Err("bad_url".into())
        );
        assert_eq!(
            apply_patch(&mut c, &json!({ "plex": { "token": "a b" } })),
            Err("bad_token".into())
        );
        assert_eq!(
            apply_patch(&mut c, &json!({ "tmdbToken": "bad token" })),
            Err("tmdb_token_format".into())
        );
        let many: Vec<String> = (0..40).map(|i| format!("D:\\d{i}")).collect();
        assert_eq!(
            apply_patch(&mut c, &json!({ "folders": many })),
            Err("too_many_folders".into())
        );
        // effacer
        c.tmdb = "abcd".into();
        apply_patch(&mut c, &json!({ "tmdbToken": "" })).unwrap();
        assert!(c.tmdb.is_empty());
    }

    #[test]
    fn launch_urls() {
        assert!(safe_launch_url("https://www.netflix.com/search?q=Am%C3%A9lie"));
        assert!(safe_launch_url(
            "http://192.168.1.5:8096/web/index.html#!/details?id=1&serverId=2"
        ));
        assert!(
            !safe_launch_url("javascript:alert(1)")
                && !safe_launch_url("file:///C:/x")
                && !safe_launch_url("https://a b.c")
        );
        assert!(!safe_launch_url(&format!("https://x.com/{}", "a".repeat(2100))));
    }

    #[test]
    fn config_is_serializable_without_secrets() {
        let c = Config {
            tmdb: "deadbeef".into(),
            ..Default::default()
        };
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.tmdb, "deadbeef");
        assert!(serde_json::from_str::<Config>("{}").unwrap().sources.files);
    }
}
