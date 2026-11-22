//! Mini serveur HTTP local (127.0.0.1) qui sert uniquement les fichiers que l'interface a enregistrés
//! (fonds d'écran, diaporamas, plus tard la médiathèque). Chaque fichier a une URL secrète aléatoire ;
//! les requêtes Range sont gérées pour que les vidéos puissent boucler et avancer.
use rand::RngCore;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct Media {
    port: u16,
    files: Mutex<HashMap<String, PathBuf>>,
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

impl Media {
    pub fn start() -> Arc<Media> {
        let listener = TcpListener::bind("127.0.0.1:0").expect("port local libre");
        let port = listener.local_addr().unwrap().port();
        let media = Arc::new(Media { port, files: Mutex::new(HashMap::new()) });
        let m = media.clone();
        std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                let m = m.clone();
                std::thread::spawn(move || {
                    let _ = handle(s, &m);
                });
            }
        });
        media
    }

    /// Enregistre un fichier et renvoie son URL locale secrète.
    pub fn url(&self, path: &Path) -> Option<String> {
        if !path.is_file() {
            return None;
        }
        let mut files = self.files.lock().unwrap();
        // Même fichier => même URL (évite de remplir la table à chaque diaporama).
        if let Some((tok, _)) = files.iter().find(|(_, p)| p.as_path() == path) {
            return Some(format!("http://127.0.0.1:{}/{}", self.port, tok));
        }
        let mut b = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut b);
        let tok: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
        let key = format!("{tok}.{ext}");
        files.insert(key.clone(), path.to_path_buf());
        Some(format!("http://127.0.0.1:{}/{}", self.port, key))
    }
}

fn respond(s: &mut TcpStream, status: &str, extra: &str, body: &[u8]) -> std::io::Result<()> {
    write!(s, "HTTP/1.1 {status}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n{extra}Content-Length: {}\r\n\r\n", body.len())?;
    s.write_all(body)
}

fn handle(mut s: TcpStream, m: &Media) -> std::io::Result<()> {
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut rd = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    rd.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut range: Option<String> = None;
    loop {
        let mut h = String::new();
        if rd.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("range:") {
            range = Some(v.trim().to_string());
        }
    }
    if method != "GET" && method != "HEAD" {
        return respond(&mut s, "405 Method Not Allowed", "", b"");
    }
    let key = target.trim_start_matches('/');
    let path = m.files.lock().unwrap().get(key).cloned();
    let Some(path) = path else { return respond(&mut s, "404 Not Found", "", b"") };
    let mut f = File::open(&path)?;
    let len = f.metadata()?.len();
    let (start, end, partial) = match range.as_deref().and_then(|r| r.strip_prefix("bytes=")) {
        Some(r) => {
            let mut it = r.splitn(2, '-');
            let a = it.next().unwrap_or("");
            let b = it.next().unwrap_or("");
            let (st, en) = if a.is_empty() {
                let n: u64 = b.parse().unwrap_or(0);
                (len.saturating_sub(n), len.saturating_sub(1))
            } else {
                (a.parse().unwrap_or(0), b.parse().unwrap_or(len.saturating_sub(1)))
            };
            (st.min(len.saturating_sub(1)), en.min(len.saturating_sub(1)), true)
        }
        None => (0, len.saturating_sub(1), false),
    };
    let n = if len == 0 { 0 } else { end - start + 1 };
    let status = if partial { "206 Partial Content" } else { "200 OK" };
    let mut head = format!("Content-Type: {}\r\nAccept-Ranges: bytes\r\n", mime(&path));
    if partial {
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{len}\r\n"));
    }
    write!(s, "HTTP/1.1 {status}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n{head}Content-Length: {n}\r\n\r\n")?;
    if method == "GET" && n > 0 {
        f.seek(SeekFrom::Start(start))?;
        std::io::copy(&mut f.take(n), &mut s)?;
    }
    Ok(())
}
