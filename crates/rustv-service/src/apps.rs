//! Lanceur : raccourcis du menu Démarrer + applis perso (URL / exécutables).
use crate::config::CustomApp;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

#[derive(Clone)]
pub struct App {
    pub id: String,
    pub name: String,
    pub target: String,
}

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn scan(dir: &Path, depth: u32, out: &mut Vec<App>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                scan(&p, depth + 1, out);
            }
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
            let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let low = name.to_lowercase();
            if low.contains("uninstall") || low.contains("désinstall") || low.contains("desinstall") {
                continue;
            }
            let target = p.to_string_lossy().to_string();
            out.push(App { id: format!("s:{}", fnv(&target)), name, target });
        }
    }
}

pub fn list(custom: &[CustomApp]) -> Vec<App> {
    let mut v: Vec<App> = custom
        .iter()
        .enumerate()
        .map(|(i, c)| App { id: format!("c:{i}"), name: c.name.clone(), target: c.target.clone() })
        .collect();
    let mut found = Vec::new();
    let roots: Vec<PathBuf> = [
        std::env::var_os("ProgramData").map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs")),
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs")),
    ]
    .into_iter()
    .flatten()
    .collect();
    for r in roots {
        scan(&r, 0, &mut found);
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    v.extend(found);
    v
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn launch(target: &str) {
    let t = wide(target);
    let op = wide("open");
    unsafe {
        ShellExecuteW(None, PCWSTR(op.as_ptr()), PCWSTR(t.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}
