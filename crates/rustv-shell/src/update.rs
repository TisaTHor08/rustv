//! Mises à jour via GitHub Releases : vérification, téléchargement de l'installeur (empreinte SHA-256 contrôlée),
//! puis lancement de l'installeur. Rien n'est envoyé sur l'utilisateur : une simple requête GET vers l'API GitHub.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const LATEST: &str = "https://api.github.com/repos/TisaTHor08/rustv/releases/latest";
const ASSET_PREFIX: &str = "https://github.com/TisaTHor08/rustv/releases/download/";
const MAX_INSTALLER: u64 = 200 * 1024 * 1024;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).user_agent("RusTV").build()
}

/// "v1.2.3" ou "1.2.3" → (1, 2, 3)
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches('v').split(['.', '-']).map(|x| x.parse::<u64>().ok());
    Some((it.next()??, it.next()??, it.next().flatten().unwrap_or(0)))
}

pub fn check() -> Result<Value, String> {
    let mut body = String::new();
    agent()
        .get(LATEST)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub inaccessible : {e}"))?
        .into_reader()
        .take(2 * 1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    let rel: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let tag = rel["tag_name"].as_str().unwrap_or("");
    let current = env!("CARGO_PKG_VERSION");
    let available = matches!((parse_version(tag), parse_version(current)), (Some(l), Some(c)) if l > c);
    let asset = rel["assets"].as_array().and_then(|a| {
        a.iter().find(|x| x["name"].as_str().map(|n| n.ends_with("-setup.exe")).unwrap_or(false))
    });
    Ok(json!({
        "current": current,
        "latest": tag.trim_start_matches('v'),
        "available": available,
        "page": rel["html_url"],
        "notes": rel["body"].as_str().unwrap_or("").chars().take(600).collect::<String>(),
        "asset": asset.map(|a| json!({ "name": a["name"], "url": a["browser_download_url"], "size": a["size"], "digest": a["digest"] })),
    }))
}

/// Télécharge l'installeur annoncé par `check`, vérifie son empreinte, le lance puis quitte RusTV.
pub fn install(app: &AppHandle, asset: &Value) -> Result<(), String> {
    // L'installeur publié est un `-setup.exe` Windows : jamais exécuté ailleurs (docs/PORTING.md : paquet .deb,
    // AppImage ou .dmg à prévoir).
    if !cfg!(windows) {
        return Err(rustv_core::platform::UNSUPPORTED.into());
    }
    let url = asset["url"].as_str().ok_or("URL manquante")?;
    if !url.starts_with(ASSET_PREFIX) || !url.ends_with("-setup.exe") {
        return Err("installeur hors du dépôt officiel : refusé".into());
    }
    let size = asset["size"].as_u64().unwrap_or(0);
    let digest = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).ok_or("empreinte SHA-256 manquante : installation refusée")?.to_lowercase();
    if size == 0 || size > MAX_INSTALLER {
        return Err("taille d'installeur inattendue".into());
    }
    let path = std::env::temp_dir().join("RusTV-update-setup.exe");
    {
        let resp = agent().get(url).call().map_err(|e| format!("téléchargement impossible : {e}"))?;
        let mut rd = resp.into_reader().take(MAX_INSTALLER + 1);
        let mut out = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        let mut h = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        let (mut done, mut last) = (0u64, 0u64);
        loop {
            let n = rd.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if done - last > 512 * 1024 {
                last = done;
                let _ = app.emit("update-progress", json!({ "pct": (done * 100 / size).min(99) }));
            }
        }
        let got: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != digest {
            let _ = std::fs::remove_file(&path);
            return Err("l'empreinte SHA-256 de l'installeur ne correspond pas".into());
        }
    }
    let _ = app.emit("update-progress", json!({ "pct": 100 }));
    rustv_core::apps::launch(&path.to_string_lossy());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.0"), Some((1, 0, 0)));
        assert!(parse_version("v1.0.1") > parse_version("1.0.0"));
        assert!(parse_version("0.5.0") < parse_version("1.0.0"));
        assert_eq!(parse_version("nope"), None);
    }
}
