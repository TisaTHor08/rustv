//! Système de mods : dossiers dans %APPDATA%\RusTV\mods\<id>\ avec un manifeste `mod.json`.
//!
//! Sécurité :
//! - un mod ne peut rien faire sans permission déclarée dans son manifeste et acceptée par l'utilisateur ;
//! - le code d'un mod qui s'exécute dans l'interface (widgets, pages) est isolé dans une iframe sandbox,
//!   servie avec une CSP qui interdit tout accès réseau : seules les API de l'hôte (stockage, fetch vers
//!   des origines autorisées, lancement) sont disponibles, et chacune vérifie la permission ici ;
//! - l'installation depuis le catalogue ne télécharge que des commits GitHub complets (contenu non modifiable).
use crate::settings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const REGISTRY_URL: &str = "https://raw.githubusercontent.com/TisaTHor08/rustv/main/registry/index.json";
const MAX_TEXT: u64 = 512 * 1024;
const MAX_DOWNLOAD: u64 = 30 * 1024 * 1024;
const MAX_FILES: usize = 800;

pub fn root() -> PathBuf {
    settings::dir().join("mods")
}
fn data_dir() -> PathBuf {
    settings::dir().join("mods-data")
}
fn staging_dir() -> PathBuf {
    settings::dir().join("mods-staging")
}
fn state_path() -> PathBuf {
    settings::dir().join("mods-state.json")
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct ModState {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    granted: Vec<String>,
}

fn load_states() -> HashMap<String, ModState> {
    std::fs::read_to_string(state_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_states(s: &HashMap<String, ModState>) {
    let _ = std::fs::create_dir_all(settings::dir());
    if let Ok(j) = serde_json::to_string_pretty(s) {
        let tmp = settings::dir().join("mods-state.json.tmp");
        if std::fs::write(&tmp, j).is_ok() {
            let _ = std::fs::rename(&tmp, state_path());
        }
    }
}

pub fn valid_id(id: &str) -> bool {
    (2..=40).contains(&id.len()) && !id.starts_with('-') && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn valid_perm(p: &str) -> bool {
    p == "storage"
        || p == "launch"
        || (p.starts_with("network:https://") && p.len() < 200 && !p.contains(char::is_whitespace) && !p.ends_with('/'))
}

/// Chemin relatif sûr (pas de `..`, pas de chemin absolu).
fn safe_rel(rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if p.is_absolute() || rel.is_empty() || rel.contains(':') {
        return None;
    }
    p.components().all(|c| matches!(c, std::path::Component::Normal(_))).then(|| p.to_path_buf())
}

pub fn read_manifest(dir: &Path) -> Result<Value, String> {
    let s = std::fs::read_to_string(dir.join("mod.json")).map_err(|_| "mod.json introuvable".to_string())?;
    let m: Value = serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| format!("mod.json invalide : {e}"))?;
    let id = m["id"].as_str().unwrap_or("");
    if !valid_id(id) {
        return Err("id invalide (a-z, 0-9, tirets ; 2 à 40 caractères)".into());
    }
    if m["name"].as_str().map(str::is_empty).unwrap_or(true) || m["version"].as_str().map(str::is_empty).unwrap_or(true) {
        return Err("name et version sont obligatoires".into());
    }
    if let Some(perms) = m["permissions"].as_array() {
        for p in perms {
            if !p.as_str().map(valid_perm).unwrap_or(false) {
                return Err(format!("permission inconnue : {p}"));
            }
        }
    }
    Ok(m)
}

/// Permissions réellement nécessaires : celles déclarées + celles impliquées par les contributions.
pub fn effective_permissions(m: &Value) -> Vec<String> {
    let mut out: Vec<String> = m["permissions"].as_array().map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect()).unwrap_or_default();
    if let Some(ads) = m["contributes"]["adapters"].as_array() {
        for a in ads {
            for pat in a["match"].as_array().into_iter().flatten().filter_map(|p| p.as_str()) {
                out.push(format!("inject:{pat}"));
            }
        }
    }
    if let Some(tiles) = m["contributes"]["tiles"].as_array() {
        if tiles.iter().any(|t| t["kind"] == "native") && !out.iter().any(|p| p == "launch") {
            out.push("launch".into());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn is_enabled(st: Option<&ModState>, eff: &[String]) -> bool {
    st.map(|s| s.enabled && eff.iter().all(|p| s.granted.contains(p))).unwrap_or(false)
}

pub fn list() -> Vec<Value> {
    let states = load_states();
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(root()) else { return out };
    for e in rd.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let dirname = e.file_name().to_string_lossy().into_owned();
        match read_manifest(&e.path()) {
            Ok(m) if m["id"] == dirname.as_str() => {
                let eff = effective_permissions(&m);
                let enabled = is_enabled(states.get(&dirname), &eff);
                out.push(json!({ "id": dirname, "manifest": m, "permissions": eff, "enabled": enabled, "error": null }));
            }
            Ok(_) => out.push(json!({ "id": dirname, "manifest": null, "permissions": [], "enabled": false, "error": "le dossier doit porter le même nom que l'id du mod" })),
            Err(err) => out.push(json!({ "id": dirname, "manifest": null, "permissions": [], "enabled": false, "error": err })),
        }
    }
    out.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    out
}

// ---------- contributions à la télécommande du téléphone ----------

const REMOTE_KEYS: [&str; 9] = ["up", "down", "left", "right", "enter", "back", "backlong", "menu", "search"];
const REMOTE_MEDIA: [&str; 7] = ["playPause", "next", "prev", "stop", "volUp", "volDown", "mute"];
const MAX_REMOTE_CSS: u64 = 64 * 1024;

/// Une action de bouton : uniquement des commandes déclaratives connues (jamais de code).
fn sanitize_action(a: &Value) -> Option<Value> {
    let t = a["t"].as_str()?;
    match t {
        "key" => REMOTE_KEYS.contains(&a["k"].as_str()?).then(|| json!({"t":"key","k":a["k"]})),
        "media" => REMOTE_MEDIA.contains(&a["a"].as_str()?).then(|| json!({"t":"media","a":a["a"]})),
        "setVolume" => Some(json!({"t":"setVolume","level":a["level"].as_u64()?.min(100)})),
        "openTile" => {
            let id = a["id"].as_str()?;
            (id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || "-_:".contains(c))).then(|| json!({"t":"openTile","id":id}))
        }
        "home" => Some(json!({"t":"home"})),
        "tv" => Some(json!({"t":"tv","on":a["on"].as_bool()?})),
        "altTab" => ["open", "left", "right", "confirm"].contains(&a["a"].as_str()?).then(|| json!({"t":"altTab","a":a["a"]})),
        "text" => {
            let s = a["text"].as_str()?;
            (s.chars().count() <= 200).then(|| json!({"t":"text","backspaces":0,"text":s}))
        }
        "wait" => Some(json!({"t":"wait","ms":a["ms"].as_u64()?.min(5000)})),
        _ => None,
    }
}

/// Texte court ou `{ "fr": "...", "en": "..." }`.
fn sanitize_label(v: &Value, max: usize) -> Option<Value> {
    match v {
        Value::String(s) if !s.is_empty() && s.chars().count() <= max => Some(v.clone()),
        Value::Object(o) => {
            let m: serde_json::Map<String, Value> = o
                .iter()
                .filter(|(k, x)| k.len() <= 5 && x.as_str().map(|s| s.chars().count() <= max).unwrap_or(false))
                .map(|(k, x)| (k.clone(), x.clone()))
                .collect();
            (!m.is_empty()).then(|| Value::Object(m))
        }
        _ => None,
    }
}

fn css_is_safe(css: &str) -> bool {
    let l = css.to_ascii_lowercase();
    !l.contains("@import") && !l.contains("http:") && !l.contains("https:") && !l.contains("//")
}

/// Thèmes et panneaux de boutons des mods actifs, nettoyés, prêts à être envoyés au téléphone.
pub fn remote_bundle() -> Value {
    let mut out = vec![];
    for entry in list() {
        if entry["enabled"] != true {
            continue;
        }
        let m = &entry["manifest"];
        let r = &m["contributes"]["remote"];
        if !r.is_object() {
            continue;
        }
        let id = entry["id"].as_str().unwrap_or_default();
        let dir = root().join(id);
        let mut themes = vec![];
        for t in r["themes"].as_array().into_iter().flatten().take(8) {
            let (Some(tid), Some(name)) = (t["id"].as_str(), sanitize_label(&t["name"], 40)) else { continue };
            let vars: serde_json::Map<String, Value> = t["vars"]
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(k, v)| {
                    k.starts_with("--") && k.len() <= 40 && v.as_str().map(|s| s.len() <= 120 && css_is_safe(s) && !s.contains(['{', '}', ';'])).unwrap_or(false)
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let css = t["css"]
                .as_str()
                .and_then(safe_rel)
                .map(|p| dir.join(p))
                .filter(|p| std::fs::metadata(p).map(|md| md.len() <= MAX_REMOTE_CSS).unwrap_or(false))
                .and_then(|p| std::fs::read_to_string(p).ok())
                .filter(|c| css_is_safe(c))
                .unwrap_or_default();
            themes.push(json!({ "id": tid, "name": name, "vars": vars, "css": css }));
        }
        let mut panels = vec![];
        for p in r["panels"].as_array().into_iter().flatten().take(6) {
            let (Some(pid), Some(name)) = (p["id"].as_str(), sanitize_label(&p["name"], 30)) else { continue };
            let buttons: Vec<Value> = p["buttons"]
                .as_array()
                .into_iter()
                .flatten()
                .take(24)
                .filter_map(|b| {
                    let label = sanitize_label(&b["label"], 24)?;
                    let actions: Vec<Value> = b["actions"].as_array()?.iter().take(20).filter_map(sanitize_action).collect();
                    (!actions.is_empty()).then(|| json!({ "label": label, "glyph": b["glyph"].as_str().unwrap_or(""), "actions": actions }))
                })
                .collect();
            if !buttons.is_empty() {
                panels.push(json!({ "id": pid, "name": name, "glyph": p["glyph"].as_str().unwrap_or(""), "buttons": buttons }));
            }
        }
        if themes.is_empty() && panels.is_empty() {
            continue;
        }
        out.push(json!({ "id": id, "name": m["name"], "i18n": m["i18n"], "themes": themes, "panels": panels }));
    }
    Value::Array(out)
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id invalide".into());
    }
    let m = read_manifest(&root().join(id))?;
    let mut st = load_states();
    let e = st.entry(id.to_string()).or_default();
    e.enabled = enabled;
    if enabled {
        e.granted = effective_permissions(&m); // l'utilisateur a validé la liste affichée avant d'activer
    }
    save_states(&st);
    Ok(())
}

pub fn uninstall(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id invalide".into());
    }
    let dir = root().join(id);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let mut st = load_states();
    st.remove(id);
    save_states(&st);
    let _ = std::fs::remove_file(data_dir().join(format!("{id}.json")));
    Ok(())
}

pub fn text(id: &str, rel: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err("id invalide".into());
    }
    let rel = safe_rel(rel).ok_or("chemin invalide")?;
    let base = root().join(id).canonicalize().map_err(|_| "mod introuvable")?;
    let p = base.join(rel).canonicalize().map_err(|_| "fichier introuvable")?;
    if !p.starts_with(&base) || !p.is_file() {
        return Err("fichier hors du mod".into());
    }
    if p.metadata().map(|m| m.len()).unwrap_or(0) > MAX_TEXT {
        return Err("fichier trop volumineux".into());
    }
    std::fs::read_to_string(p).map_err(|e| e.to_string())
}

fn granted(id: &str, perm: &str) -> bool {
    let st = load_states();
    let Some(s) = st.get(id) else { return false };
    s.enabled && s.granted.iter().any(|g| g == perm)
}

// ---------- stockage clé/valeur par mod ----------

fn read_data(id: &str) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(data_dir().join(format!("{id}.json"))).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn storage_get(id: &str, key: &str) -> Result<Value, String> {
    if !granted(id, "storage") {
        return Err("permission « storage » manquante".into());
    }
    Ok(read_data(id).get(key).cloned().unwrap_or(Value::Null))
}

pub fn storage_set(id: &str, key: &str, value: Value) -> Result<(), String> {
    if !granted(id, "storage") {
        return Err("permission « storage » manquante".into());
    }
    let mut d = read_data(id);
    d.insert(key.to_string(), value);
    let s = serde_json::to_string(&d).map_err(|e| e.to_string())?;
    if s.len() > 256 * 1024 {
        return Err("stockage du mod plein (256 Ko)".into());
    }
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    std::fs::write(data_dir().join(format!("{id}.json")), s).map_err(|e| e.to_string())
}

// ---------- réseau ----------

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(12)).user_agent("RusTV").build()
}

fn origin_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains('@')).then(|| format!("https://{host}"))
}

/// GET https vers une origine déclarée dans les permissions du mod (`network:https://exemple.org`).
pub fn fetch(id: &str, url: &str) -> Result<Value, String> {
    let origin = origin_of(url).ok_or("seules les adresses https sont autorisées")?;
    if !granted(id, &format!("network:{origin}")) {
        return Err(format!("permission « network:{origin} » manquante"));
    }
    let resp = match agent().get(url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return Err(e.to_string()),
    };
    let status = resp.status();
    let mut body = String::new();
    resp.into_reader().take(2 * 1024 * 1024).read_to_string(&mut body).map_err(|e| e.to_string())?;
    Ok(json!({ "status": status, "body": body }))
}

// ---------- adaptateurs de sites ----------

fn glob(pat: &str, s: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pat.chars().collect(), s.chars().collect());
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Script d'initialisation regroupant les adaptateurs des mods activés qui correspondent à `url`.
pub fn adapters_for(url: &str) -> String {
    let mut out = String::new();
    for m in list() {
        if m["enabled"] != true {
            continue;
        }
        let id = m["id"].as_str().unwrap_or("");
        for a in m["manifest"]["contributes"]["adapters"].as_array().into_iter().flatten() {
            if !a["match"].as_array().into_iter().flatten().filter_map(|p| p.as_str()).any(|p| glob(p, url)) {
                continue;
            }
            let ready = a["runAt"].as_str() != Some("start");
            let mut body = String::new();
            if let Some(css) = a["css"].as_str().and_then(|r| text(id, r).ok()) {
                body.push_str(&format!("var s=document.createElement('style');s.textContent={};(document.head||document.documentElement).appendChild(s);", serde_json::to_string(&css).unwrap_or_default()));
            }
            if let Some(js) = a["js"].as_str().and_then(|r| text(id, r).ok()) {
                body.push_str(&js);
            }
            if body.is_empty() {
                continue;
            }
            let wrapped = format!("try{{{body}}}catch(e){{console.error('[rustv mod {id}]',e)}}");
            if ready {
                out.push_str(&format!("(function(){{var run=function(){{{wrapped}}};if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',run);else run();}})();\n"));
            } else {
                out.push_str(&format!("(function(){{{wrapped}}})();\n"));
            }
        }
    }
    out
}

// ---------- catalogue communautaire ----------

pub fn registry() -> Result<Value, String> {
    let resp = agent().get(REGISTRY_URL).call().map_err(|e| format!("catalogue inaccessible : {e}"))?;
    let mut body = String::new();
    resp.into_reader().take(1024 * 1024).read_to_string(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| format!("catalogue invalide : {e}"))
}

fn is_commit(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn is_repo(s: &str) -> bool {
    let mut it = s.split('/');
    let (a, b, c) = (it.next(), it.next(), it.next());
    let ok = |x: &str| !x.is_empty() && x.len() < 100 && x.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    matches!((a, b, c), (Some(a), Some(b), None) if ok(a) && ok(b))
}

/// Télécharge le commit épinglé et en extrait le dossier du mod dans mods-staging/<id>. Renvoie son manifeste.
pub fn install_prepare(entry: &Value) -> Result<Value, String> {
    let id = entry["id"].as_str().unwrap_or("");
    let src = &entry["source"];
    let (repo, commit, path) = (src["repo"].as_str().unwrap_or(""), src["commit"].as_str().unwrap_or(""), src["path"].as_str().unwrap_or(""));
    if !valid_id(id) || !is_repo(repo) || !is_commit(commit) {
        return Err("entrée de catalogue invalide (commit complet exigé)".into());
    }
    let sub = if path.is_empty() { PathBuf::new() } else { safe_rel(path).ok_or("chemin invalide")? };

    let url = format!("https://codeload.github.com/{repo}/zip/{commit}");
    let resp = agent().get(&url).call().map_err(|e| format!("téléchargement impossible : {e}"))?;
    let mut buf = Vec::new();
    resp.into_reader().take(MAX_DOWNLOAD + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > MAX_DOWNLOAD {
        return Err("archive trop volumineuse".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(buf)).map_err(|e| e.to_string())?;

    let dest = staging_dir().join(id);
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let mut count = 0usize;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = f.enclosed_name() else { continue };
        // On retire le dossier racine « <repo>-<commit>/ » puis le sous-dossier du mod.
        let mut comps = name.components();
        comps.next();
        let rel: PathBuf = comps.as_path().to_path_buf();
        let Ok(inner) = rel.strip_prefix(&sub) else { continue };
        if inner.as_os_str().is_empty() || f.is_dir() {
            continue;
        }
        count += 1;
        total += f.size();
        if count > MAX_FILES || total > 40 * 1024 * 1024 {
            return Err("archive de mod trop grosse".into());
        }
        let out = dest.join(inner);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(|e| e.to_string())?;
        std::fs::write(&out, data).map_err(|e| e.to_string())?;
    }
    let m = read_manifest(&dest).map_err(|e| format!("manifeste du mod : {e}"))?;
    if m["id"] != id {
        let _ = std::fs::remove_dir_all(&dest);
        return Err("l'id du manifeste ne correspond pas au catalogue".into());
    }
    let eff = effective_permissions(&m);
    Ok(json!({ "manifest": m, "permissions": eff }))
}

pub fn install_commit(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id invalide".into());
    }
    let from = staging_dir().join(id);
    let m = read_manifest(&from)?;
    let to = root().join(id);
    std::fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    if to.exists() {
        std::fs::remove_dir_all(&to).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    let mut st = load_states();
    st.insert(id.to_string(), ModState { enabled: true, granted: effective_permissions(&m) });
    save_states(&st);
    Ok(())
}

pub fn install_cancel(id: &str) {
    if valid_id(id) {
        let _ = std::fs::remove_dir_all(staging_dir().join(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches() {
        assert!(glob("*://*.netflix.com/*", "https://www.netflix.com/browse"));
        assert!(!glob("*://*.netflix.com/*", "https://www.example.com/netflix.com/"));
        assert!(glob("https://example.com/*", "https://example.com/"));
    }

    #[test]
    fn ids_and_paths() {
        assert!(valid_id("my-theme") && !valid_id("../x") && !valid_id("A") && !valid_id("-a"));
        assert!(safe_rel("a/b.css").is_some());
        assert!(safe_rel("../x").is_none() && safe_rel("C:\\x").is_none() && safe_rel("/etc").is_none());
        assert_eq!(origin_of("https://a.b/c?d").as_deref(), Some("https://a.b"));
        assert!(origin_of("http://a.b").is_none() && origin_of("https://u@a.b/").is_none());
    }

    #[test]
    fn remote_actions_are_whitelisted() {
        assert!(sanitize_action(&json!({"t":"key","k":"enter"})).is_some());
        assert!(sanitize_action(&json!({"t":"key","k":"f4"})).is_none());
        assert!(sanitize_action(&json!({"t":"vaultFill","id":"x"})).is_none());
        assert!(sanitize_action(&json!({"t":"launch","appId":"calc"})).is_none());
        assert_eq!(sanitize_action(&json!({"t":"wait","ms":999999})).unwrap()["ms"], 5000);
        assert_eq!(sanitize_action(&json!({"t":"setVolume","level":400})).unwrap()["level"], 100);
        assert!(sanitize_action(&json!({"t":"openTile","id":"../x"})).is_none());
        assert!(css_is_safe(".a{color:red}") && !css_is_safe("@import 'x'") && !css_is_safe("a{background:url(https://x)}"));
    }

    #[test]
    fn effective_permissions_include_adapters() {
        let m = json!({"permissions":["storage"],"contributes":{"adapters":[{"match":["*://x.com/*"]}],"tiles":[{"kind":"native"}]}});
        let e = effective_permissions(&m);
        assert!(e.contains(&"inject:*://x.com/*".to_string()) && e.contains(&"launch".to_string()) && e.contains(&"storage".to_string()));
    }
}
