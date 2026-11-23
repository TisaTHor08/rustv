//! Compte Google partagé : on se connecte une fois dans un profil WebView dédié (« google »), puis les cookies de
//! session Google sont recopiés dans le profil de chaque application web autorisée au lancement, pour que
//! « Se connecter avec Google » y trouve la session. RusTV ne voit ni ne stocke jamais le mot de passe ; les cookies
//! ne vivent que dans les profils WebView locaux (et en mémoire quelques minutes), et sont effacés à la déconnexion.
use crate::settings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::webview::cookie::{time::OffsetDateTime, Cookie, SameSite};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const SIGNIN_JS: &str = include_str!("google_signin.js");
const LOGIN_URL: &str = "https://accounts.google.com/ServiceLogin?hl=fr&continue=https%3A%2F%2Fmyaccount.google.com%2F";
/// Adresses interrogées pour récupérer les cookies Google applicables (chaque appel renvoie aussi ceux du domaine parent).
const HARVEST_URLS: [&str; 5] = [
    "https://accounts.google.com/",
    "https://www.google.com/",
    "https://myaccount.google.com/",
    "https://www.youtube.com/",
    "https://accounts.youtube.com/",
];
const CACHE_TTL: Duration = Duration::from_secs(600);

// ---------- fonctions pures ----------

/// UA de navigateur Chromium standard (sans « Edg/ » ni « WebView ») : évite le blocage « navigateur non sécurisé ».
pub fn chrome_ua() -> &'static str {
    if cfg!(target_os = "macos") {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"
    } else if cfg!(target_os = "linux") {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"
    } else {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"
    }
}

/// Domaine de cookie propagé : google.com et youtube.com (et leurs sous-domaines) uniquement, jamais un autre site.
pub fn is_google_domain(domain: &str) -> bool {
    let d = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    ["google.com", "youtube.com"].iter().any(|b| d == *b || d.ends_with(&format!(".{b}")))
}

/// Hôte des pages de connexion Google (popup OAuth, Google Identity Services).
pub fn is_google_auth_host(host: &str) -> bool {
    matches!(host.to_ascii_lowercase().as_str(), "accounts.google.com" | "accounts.youtube.com")
}

/// Cookie simplifié (testable sans WebView).
#[derive(Clone, Debug, PartialEq)]
pub struct Ck {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// Expiration (secondes Unix) ; None = cookie de session.
    pub expires: Option<i64>,
    pub same_site: Option<&'static str>,
}

/// Garde les cookies Google valides (domaine autorisé, valeur non vide, non expirés), sans doublon (nom, domaine, chemin).
pub fn filter_cookies(all: Vec<Ck>, now: i64) -> Vec<Ck> {
    let mut seen = std::collections::HashSet::new();
    all.into_iter()
        .filter(|c| is_google_domain(&c.domain) && !c.name.is_empty() && !c.value.is_empty() && c.expires.is_none_or(|e| e > now))
        .filter(|c| seen.insert((c.name.clone(), c.domain.to_ascii_lowercase(), c.path.clone())))
        .collect()
}

/// Une session Google est-elle présente ? (cookies d'authentification connus)
pub fn has_session(cks: &[Ck]) -> bool {
    cks.iter().any(|c| is_google_domain(&c.domain) && matches!(c.name.as_str(), "SID" | "SAPISID" | "__Secure-1PSID" | "__Secure-3PSID"))
}

/// Le partage est-il actif pour cette application ? Réglage de l'utilisateur d'abord, sinon valeur par défaut de la tuile.
pub fn app_active(enabled: bool, connected: bool, app: &str, tile_default: bool, overrides: &HashMap<String, bool>) -> bool {
    enabled && connected && overrides.get(app).copied().unwrap_or(tile_default)
}

fn from_cookie(c: &Cookie<'_>) -> Ck {
    Ck {
        name: c.name().to_string(),
        value: c.value().to_string(),
        domain: c.domain().unwrap_or("").to_string(),
        path: c.path().unwrap_or("/").to_string(),
        secure: c.secure().unwrap_or(false),
        http_only: c.http_only().unwrap_or(false),
        expires: c.expires_datetime().map(|d| d.unix_timestamp()),
        same_site: match c.same_site() {
            Some(SameSite::Strict) => Some("Strict"),
            Some(SameSite::Lax) => Some("Lax"),
            Some(SameSite::None) => Some("None"),
            None => None,
        },
    }
}

/// Domaine à donner à WebView2. L'API cookie de Tauri retire le point initial à la lecture (cookie « de domaine » ou
/// « d'hôte » indiscernables) et le getter ne le rend jamais ; or WebView2 distingue `google.com` (hôte seul) de
/// `.google.com` (domaine). Règle : `__Host-*` = hôte seul ; google.com / youtube.com = cookie de domaine (le préfixe
/// supplémentaire compense le retrait d'un point par le getter) ; sous-domaines = hôte seul.
pub fn set_domain(name: &str, domain: &str) -> String {
    let d = domain.trim().trim_start_matches('.');
    if !name.starts_with("__Host-") && (d.eq_ignore_ascii_case("google.com") || d.eq_ignore_ascii_case("youtube.com")) {
        format!("..{d}")
    } else {
        d.to_string()
    }
}

fn to_cookie(c: &Ck) -> Cookie<'static> {
    let mut b = Cookie::build((c.name.clone(), c.value.clone())).domain(set_domain(&c.name, &c.domain)).path(c.path.clone()).secure(c.secure).http_only(c.http_only);
    b = match c.same_site {
        Some("Strict") => b.same_site(SameSite::Strict),
        Some("Lax") => b.same_site(SameSite::Lax),
        Some("None") => b.same_site(SameSite::None),
        _ => b,
    };
    if let Some(d) = c.expires.and_then(|e| OffsetDateTime::from_unix_timestamp(e).ok()) {
        b = b.expires(d);
    }
    b.build()
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

// ---------- état persistant ----------

#[derive(Serialize, Deserialize, Default, Clone)]
struct GState {
    /// Fonction activée par l'utilisateur (désactivée par défaut).
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    connected: bool,
    /// Profils d'apps qui ont reçu des cookies Google (à nettoyer à la déconnexion).
    #[serde(default)]
    applied: Vec<String>,
    /// Réglage par app (absent = valeur par défaut de la tuile).
    #[serde(default)]
    apps: HashMap<String, bool>,
}

fn state() -> &'static Mutex<GState> {
    static S: OnceLock<Mutex<GState>> = OnceLock::new();
    S.get_or_init(|| {
        let g = std::fs::read_to_string(settings::dir().join("google.json")).ok().and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()).unwrap_or_default();
        Mutex::new(g)
    })
}

fn save(g: &GState) {
    let d = settings::dir();
    let _ = std::fs::create_dir_all(&d);
    if let Ok(s) = serde_json::to_string_pretty(g) {
        let tmp = d.join("google.json.tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, d.join("google.json"));
        }
    }
}

/// Cookies récupérés récemment (mémoire seulement, jamais écrits sur disque par RusTV).
fn cache() -> &'static Mutex<Option<(Instant, Vec<Ck>)>> {
    static C: OnceLock<Mutex<Option<(Instant, Vec<Ck>)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

// ---------- accès aux profils WebView ----------

fn profile_dir(id: &str) -> std::path::PathBuf {
    settings::dir().join("profiles").join(id)
}

/// Ouvre une fenêtre cachée sur le profil `id`, exécute `f`, puis la détruit. À appeler depuis un fil dédié
/// (l'API cookies de WebView2 se bloque sur le fil principal).
fn with_profile<T>(app: &AppHandle, id: &str, f: impl FnOnce(&WebviewWindow) -> T) -> Result<T, String> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let label = format!("gtmp-{}", SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let url = "about:blank".parse::<Url>().map_err(|e| e.to_string())?;
    let w = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .data_directory(profile_dir(id))
        .visible(false)
        .skip_taskbar(true)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;
    let r = f(&w);
    let _ = w.destroy();
    Ok(r)
}

fn read_google_cookies(w: &WebviewWindow) -> Vec<Ck> {
    let mut v = Vec::new();
    for u in HARVEST_URLS {
        if let Ok(list) = w.cookies_for_url(u.parse().unwrap()) {
            v.extend(list.iter().map(from_cookie));
        }
    }
    filter_cookies(v, now())
}

fn harvest(app: &AppHandle) -> Result<Vec<Ck>, String> {
    if let Some((t, v)) = cache().lock().unwrap().as_ref() {
        if t.elapsed() < CACHE_TTL {
            return Ok(v.clone());
        }
    }
    let cks = with_profile(app, "google", read_google_cookies)?;
    *cache().lock().unwrap() = Some((Instant::now(), cks.clone()));
    Ok(cks)
}

/// Recopie les cookies Google dans la fenêtre `w` (profil de l'app `id`).
fn inject(app: &AppHandle, id: &str, w: &WebviewWindow) -> Result<usize, String> {
    let cks = harvest(app)?;
    let mut n = 0;
    for c in &cks {
        if w.set_cookie(to_cookie(c)).is_ok() {
            n += 1;
        }
    }
    let mut g = state().lock().unwrap();
    if !g.applied.iter().any(|a| a == id) {
        g.applied.push(id.to_string());
        save(&g);
    }
    Ok(n)
}

fn delete_google_cookies(w: &WebviewWindow, everything: bool) {
    if let Ok(list) = w.cookies() {
        for c in list {
            if everything || is_google_domain(c.domain().unwrap_or("")) {
                // Suppression exacte côté WebView2 (nom + domaine + chemin) : on essaie la forme « domaine » puis « hôte seul ».
                let ck = from_cookie(&c);
                let _ = w.delete_cookie(to_cookie(&ck));
                let _ = w.delete_cookie(c);
            }
        }
    }
}

fn emit(app: &AppHandle) {
    let _ = app.emit("google-state", ());
}

// ---------- intégration dans open_web ----------

/// Cette app doit-elle recevoir la session Google ? (tuile : champ `googleSignIn` ; réglage utilisateur prioritaire)
pub fn wants(id: &str, tile: &Value) -> bool {
    let g = state().lock().unwrap();
    app_active(g.enabled, g.connected, id, tile["googleSignIn"].as_bool().unwrap_or(false), &g.apps)
}

/// Prépare une fenêtre d'app web : propage les cookies Google puis charge `url`. Fil dédié ; en cas d'échec, la page se charge quand même.
pub fn prepare_and_navigate(app: &AppHandle, id: &str, w: WebviewWindow, url: Url) {
    let (app, id) = (app.clone(), id.to_string());
    std::thread::spawn(move || {
        let _ = inject(&app, &id, &w);
        let _ = w.navigate(url);
    });
}

/// Gestionnaire `window.open` : seuls les schémas http(s)/about sont permis ; pour une app autorisée, les popups de connexion
/// Google deviennent une fenêtre RusTV (même profil, script d'aide, UA Chromium, lien avec la fenêtre d'origine conservé pour OAuth).
/// Le reste garde le comportement par défaut.
pub fn on_new_window(app: &AppHandle, id: &str, google: bool, url: Url, features: tauri::webview::NewWindowFeatures) -> tauri::webview::NewWindowResponse<tauri::Wry> {
    use tauri::webview::NewWindowResponse;
    if !matches!(url.scheme(), "http" | "https" | "about") {
        return NewWindowResponse::Deny;
    }
    if !google || !url.host_str().is_some_and(is_google_auth_host) {
        return NewWindowResponse::Allow;
    }
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let label = format!("app-popup-{}", SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::External("about:blank".parse().unwrap()))
        .window_features(features)
        .data_directory(profile_dir(id))
        .user_agent(chrome_ua())
        .initialization_script(SIGNIN_JS)
        .title("Google")
        .build();
    match built {
        Ok(window) => NewWindowResponse::Create { window },
        Err(_) => NewWindowResponse::Allow,
    }
}

// ---------- connexion / déconnexion ----------

fn open_login(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("gauth") {
        crate::tv::force_focus(&w);
        return Ok(());
    }
    let url: Url = LOGIN_URL.parse().map_err(|_| "invalid")?;
    // Aucun accès IPC : la fenêtre n'est dans aucune capacité, la page Google ne peut rien appeler de RusTV.
    let w = WebviewWindowBuilder::new(app, "gauth", WebviewUrl::External(url))
        .title("Google")
        .data_directory(profile_dir("google"))
        .user_agent(chrome_ua())
        .decorations(false)
        .background_color(tauri::window::Color(255, 255, 255, 255))
        .initialization_script(crate::tv::SPATNAV)
        .initialization_script(SIGNIN_JS)
        .build()
        .map_err(|e| e.to_string())?;
    crate::tv::place_on_tv(app, &w);
    crate::tv::force_focus(&w);
    let h = app.clone();
    w.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            let h = h.clone();
            // Fenêtre fermée : on regarde si une session Google existe maintenant.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500)); // laisse WebView2 écrire les cookies
                *cache().lock().unwrap() = None;
                let ok = with_profile(&h, "google", |w| has_session(&read_google_cookies(w))).unwrap_or(false);
                {
                    let mut g = state().lock().unwrap();
                    g.connected = ok;
                    save(&g);
                }
                emit(&h);
            });
        }
    });
    Ok(())
}

fn do_disconnect(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("app-") || label == "gauth" {
            let _ = w.destroy();
        }
    }
    std::thread::sleep(Duration::from_millis(700));
    *cache().lock().unwrap() = None;
    let applied = {
        let mut g = state().lock().unwrap();
        g.connected = false;
        std::mem::take(&mut g.applied)
    };
    let _ = with_profile(app, "google", |w| {
        delete_google_cookies(w, true);
        let _ = w.clear_all_browsing_data();
    });
    for id in applied.iter().filter(|i| i.as_str() != "google") {
        let _ = with_profile(app, id, |w| delete_google_cookies(w, false));
    }
    save(&state().lock().unwrap());
    emit(app);
}

// ---------- commandes ----------

#[tauri::command]
pub async fn google_get(app: AppHandle) -> Result<Value, String> {
    let g = state().lock().unwrap().clone();
    let apps: Vec<Value> = crate::tv::catalog()
        .iter()
        .filter(|t| t["kind"] == "web" && t["googleSignIn"].as_bool().unwrap_or(false))
        .filter_map(|t| {
            let id = t["id"].as_str()?;
            Some(json!({ "id": id, "name": t["name"], "on": g.apps.get(id).copied().unwrap_or(true) }))
        })
        .collect();
    Ok(json!({ "enabled": g.enabled, "connected": g.connected, "connecting": app.get_webview_window("gauth").is_some(), "apps": apps, "overrides": g.apps }))
}

#[tauri::command]
pub async fn google_set_enabled(app: AppHandle, on: bool) -> Result<(), String> {
    {
        let mut g = state().lock().unwrap();
        g.enabled = on;
        save(&g);
    }
    emit(&app);
    Ok(())
}

#[tauri::command]
pub async fn google_connect(app: AppHandle) -> Result<(), String> {
    if !state().lock().unwrap().enabled {
        return Err("disabled".into());
    }
    open_login(&app)
}

#[tauri::command]
pub async fn google_disconnect(app: AppHandle) -> Result<(), String> {
    std::thread::spawn(move || do_disconnect(&app));
    Ok(())
}

/// `on` = None : retour à la valeur par défaut de la tuile. Désactiver pour une app efface ses cookies Google.
#[tauri::command]
pub async fn google_set_app(app: AppHandle, id: String, on: Option<bool>) -> Result<(), String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':') {
        return Err("invalid".into());
    }
    let cleanup = {
        let mut g = state().lock().unwrap();
        match on {
            Some(v) => g.apps.insert(id.clone(), v),
            None => g.apps.remove(&id),
        };
        let off = on == Some(false) && g.applied.contains(&id);
        if off {
            g.applied.retain(|a| a != &id);
        }
        save(&g);
        off
    };
    if cleanup {
        std::thread::spawn(move || {
            let _ = with_profile(&app, &id, |w| delete_google_cookies(w, false));
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ck(name: &str, domain: &str, value: &str, exp: Option<i64>) -> Ck {
        Ck { name: name.into(), value: value.into(), domain: domain.into(), path: "/".into(), secure: true, http_only: true, expires: exp, same_site: None }
    }

    #[test]
    fn google_domains_only() {
        for d in [".google.com", "accounts.google.com", "google.com", ".youtube.com", "WWW.YouTube.com"] {
            assert!(is_google_domain(d), "{d}");
        }
        for d in ["", "evil-google.com", "google.com.evil.net", "notyoutube.com", "netflix.com", "google.fr", "accounts.google.com.attacker.io"] {
            assert!(!is_google_domain(d), "{d}");
        }
    }

    #[test]
    fn auth_hosts() {
        assert!(is_google_auth_host("accounts.google.com"));
        assert!(is_google_auth_host("Accounts.Google.Com"));
        assert!(!is_google_auth_host("www.google.com"));
        assert!(!is_google_auth_host("accounts.google.com.evil.net"));
    }

    #[test]
    fn filter_drops_foreign_expired_empty_and_duplicates() {
        let v = vec![
            ck("SID", ".google.com", "a", Some(2000)),
            ck("SID", ".google.com", "a2", Some(2000)),
            ck("OLD", ".google.com", "x", Some(500)),
            ck("EMPTY", ".google.com", "", None),
            ck("SESSION", "accounts.google.com", "s", None),
            ck("nf", ".netflix.com", "n", None),
            ck("yt", ".youtube.com", "y", Some(9999)),
        ];
        let out = filter_cookies(v, 1000);
        let names: Vec<&str> = out.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["SID", "SESSION", "yt"]);
        assert_eq!(out[0].value, "a");
    }

    #[test]
    fn session_detection() {
        assert!(!has_session(&[ck("NID", ".google.com", "x", None)]));
        assert!(has_session(&[ck("__Secure-1PSID", ".google.com", "x", None)]));
        assert!(!has_session(&[ck("SID", ".example.com", "x", None)]));
        assert!(!has_session(&[]));
    }

    #[test]
    fn per_app_rules() {
        let mut o = HashMap::new();
        assert!(app_active(true, true, "spotify", true, &o));
        assert!(!app_active(true, true, "netflix", false, &o));
        assert!(!app_active(false, true, "spotify", true, &o), "désactivé par défaut");
        assert!(!app_active(true, false, "spotify", true, &o), "pas connecté");
        o.insert("spotify".into(), false);
        o.insert("netflix".into(), true);
        assert!(!app_active(true, true, "spotify", true, &o));
        assert!(app_active(true, true, "netflix", false, &o));
    }

    #[test]
    fn cookie_roundtrip_keeps_attributes() {
        let c = Ck { name: "SAPISID".into(), value: "v".into(), domain: "google.com".into(), path: "/".into(), secure: true, http_only: false, expires: Some(2_000_000_000), same_site: Some("Lax") };
        let back = from_cookie(&to_cookie(&c));
        assert_eq!(back.domain, ".google.com", "le point de domaine est conservé pour WebView2");
        assert_eq!(Ck { domain: "google.com".into(), ..back }, c);
    }

    #[test]
    fn domain_cookies_keep_their_leading_dot_for_webview2() {
        // Ce que reçoit WebView2 (via le getter de l'API cookie) : « .google.com » pour un cookie de domaine.
        let dom = |name: &str, d: &str| to_cookie(&ck(name, d, "v", None)).domain().map(str::to_string);
        assert_eq!(dom("SAPISID", "google.com").as_deref(), Some(".google.com"));
        assert_eq!(dom("SAPISID", ".google.com").as_deref(), Some(".google.com"));
        assert_eq!(dom("PREF", "youtube.com").as_deref(), Some(".youtube.com"));
        assert_eq!(dom("__Host-GAPS", "accounts.google.com").as_deref(), Some("accounts.google.com"));
        assert_eq!(dom("__Host-X", "google.com").as_deref(), Some("google.com"));
        assert_eq!(dom("NID", "accounts.google.com").as_deref(), Some("accounts.google.com"));
    }

    #[test]
    fn ua_is_plain_chromium() {
        let ua = chrome_ua();
        assert!(ua.contains("Chrome/") && ua.starts_with("Mozilla/5.0"));
        assert!(!ua.contains("Edg/") && !ua.contains("wv"));
    }

    #[test]
    fn catalog_tiles_declaring_google_are_web() {
        for t in crate::tv::catalog() {
            if t["googleSignIn"].as_bool().unwrap_or(false) {
                assert_eq!(t["kind"], "web", "{}", t["id"]);
            }
        }
    }
}
