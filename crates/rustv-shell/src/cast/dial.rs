//! DIAL : ressource d'application `/apps/YouTube` (GET, POST, DELETE). Le téléphone découvre le « téléviseur » par SSDP
//! (`urn:dial-multiscreen-org:service:dial:1`), lit l'en-tête `Application-URL` de /dd.xml puis interroge `<Application-URL>YouTube`.
use super::http::{Request, Response};
use super::Shared;

const MAX_PARAMS: usize = 4096;
pub const YOUTUBE_TV: &str = "https://www.youtube.com/tv?";

/// `Application-URL` renvoyé avec /dd.xml (se termine par « / » comme l'exige la spec).
pub fn application_url(base: &str) -> String {
    format!("{base}/apps/")
}

/// Document de la ressource d'application (spec DIAL 1.7).
pub fn app_xml(name: &str, running: bool) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<service xmlns=\"urn:dial-multiscreen-org:schemas:dial\" dialVer=\"1.7\">\r\n<name>{name}</name>\r\n<options allowStop=\"true\"/>\r\n<state>{}</state>\r\n{}</service>\r\n",
        if running { "running" } else { "stopped" },
        if running { "<link rel=\"run\" href=\"run\"/>\r\n" } else { "" }
    )
}

/// Paramètres de lancement : uniquement des caractères d'une query encodée en pourcent. `None` si autre chose.
pub fn sanitize_params(body: &str) -> Option<String> {
    let b = body.trim();
    if b.len() > MAX_PARAMS || !b.chars().all(|c| c.is_ascii_alphanumeric() || "=&%_-.~+,:".contains(c)) {
        return None;
    }
    Some(b.to_string())
}

/// URL de l'application YouTube TV pour une requête de lancement (`None` = URL par défaut de la tuile).
pub fn youtube_url(params: &str) -> Option<String> {
    (!params.is_empty()).then(|| format!("{YOUTUBE_TV}{params}"))
}

/// Un navigateur (origine http/https) ne doit pas pouvoir lancer ou arrêter l'app : protection contre le CSRF.
pub fn origin_ok(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(o) => {
            let o = o.to_ascii_lowercase();
            !(o.starts_with("http://") || o.starts_with("https://") || o == "null")
        }
    }
}

pub fn handle(sh: &Shared, req: &Request) -> Option<Response> {
    let rest = req.path.strip_prefix("/apps/")?;
    let (app, sub) = rest.split_once('/').unwrap_or((rest, ""));
    // Seule YouTube est prise en charge. Netflix n'est volontairement pas déclarée : on ne sait pas la lancer.
    if app != "YouTube" || !(sub.is_empty() || sub == "run") {
        return Some(Response::text(404, "Not found"));
    }
    let host = &sh.host;
    Some(match req.method.as_str() {
        "GET" if sub.is_empty() => {
            Response::xml(200, app_xml("YouTube", host.youtube_running()))
        }
        "POST" if sub.is_empty() => {
            if !origin_ok(req.header("origin")) {
                return Some(Response::text(403, "Forbidden"));
            }
            let Some(params) = sanitize_params(&req.body_str()) else { return Some(Response::text(400, "Bad parameters")) };
            match host.launch_youtube(youtube_url(&params)) {
                Ok(()) => {
                    sh.status.lock().unwrap().last_event = "DIAL: YouTube".into();
                    Response::new(201).header("Location", format!("http://{}/apps/YouTube/run", req.local))
                }
                Err(_) => Response::text(503, "Launch failed"),
            }
        }
        "DELETE" => {
            if !origin_ok(req.header("origin")) {
                return Some(Response::text(403, "Forbidden"));
            }
            if host.youtube_running() {
                host.stop_youtube();
                sh.status.lock().unwrap().last_event = "DIAL: stop YouTube".into();
                Response::new(200)
            } else {
                Response::text(404, "Not running")
            }
        }
        _ => Response::text(405, "Method not allowed"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_states() {
        let r = app_xml("YouTube", true);
        assert!(r.contains("<state>running</state>") && r.contains("<link rel=\"run\" href=\"run\"/>") && r.contains("dialVer=\"1.7\""));
        let s = app_xml("YouTube", false);
        assert!(s.contains("<state>stopped</state>") && !s.contains("rel=\"run\""));
        assert!(s.contains("<name>YouTube</name>") && s.contains("allowStop=\"true\""));
    }

    #[test]
    fn app_url_ends_with_slash() {
        assert_eq!(application_url("http://192.168.1.2:47840"), "http://192.168.1.2:47840/apps/");
    }

    #[test]
    fn params() {
        assert_eq!(sanitize_params("pairingCode=abc-123&theme=cl&v=dQw4w9WgXcQ&t=12").as_deref(), Some("pairingCode=abc-123&theme=cl&v=dQw4w9WgXcQ&t=12"));
        assert_eq!(sanitize_params("  ").as_deref(), Some(""));
        assert!(sanitize_params("a=b#frag").is_none());
        assert!(sanitize_params("a=<script>").is_none());
        assert!(sanitize_params("a=b c").is_none());
        assert!(sanitize_params(&"a".repeat(5000)).is_none());
        assert_eq!(youtube_url("v=1&t=2").as_deref(), Some("https://www.youtube.com/tv?v=1&t=2"));
        assert_eq!(youtube_url(""), None);
    }

    #[test]
    fn origin() {
        assert!(origin_ok(None));
        assert!(origin_ok(Some("package:com.google.android.youtube")));
        assert!(!origin_ok(Some("https://evil.example")));
        assert!(!origin_ok(Some("http://192.168.1.3")));
        assert!(!origin_ok(Some("null")));
    }
}
