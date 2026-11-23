//! Linux : injection d'entrées par un périphérique virtuel `/dev/uinput` (fonctionne sous X11 et sous Wayland).
//!
//! Deux périphériques sont créés à la demande : un clavier et une souris (mouvement relatif, clics, molette).
//! Les codes de touches logiques (`VIRTUAL_KEY`, valeurs Windows) sont traduits en codes evdev par `evdev_key`.
//! Saisie de texte : `wtype` (Wayland) ou `xdotool type` (X11) s'ils sont installés, car ils respectent la
//! disposition du clavier ; à défaut, touches uinput pour l'ASCII si la disposition est QWERTY américaine
//! (ou inconnue) ; sinon le texte est ignoré avec le message `tool_missing` (une seule fois sur stderr).
//! Aucune dépendance en plus de `libc` (ioctl bruts).
//!
//! Erreurs stables de `check()` : `uinput_denied` (périphérique absent ou droits insuffisants : voir docs/LINUX.md).
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

#[allow(dead_code)]
#[path = "../stub/input.rs"]
mod base;
pub use base::{keys, HWND, VIRTUAL_KEY};

// ---- constantes evdev (linux/input-event-codes.h) ----
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const REL_HWHEEL: u16 = 0x06;
const REL_WHEEL: u16 = 0x08;
const REL_WHEEL_HI_RES: u16 = 0x0b;
const BTN_LEFT: u16 = 0x110;
const BTN_RIGHT: u16 = 0x111;
const KEY_LEFTSHIFT: u16 = 42;
const BUS_USB: u16 = 0x03;

/// `_IOC` du noyau (direction, type, numéro, taille).
const fn ioc(dir: u32, ty: u8, nr: u32, size: u32) -> u32 {
    (dir << 30) | (size << 16) | ((ty as u32) << 8) | nr
}
const IOC_NONE: u32 = 0;
const IOC_WRITE: u32 = 1;
const UI_DEV_CREATE: u32 = ioc(IOC_NONE, b'U', 1, 0);
const UI_DEV_DESTROY: u32 = ioc(IOC_NONE, b'U', 2, 0);
const UI_DEV_SETUP: u32 = ioc(IOC_WRITE, b'U', 3, 92);
const UI_SET_EVBIT: u32 = ioc(IOC_WRITE, b'U', 100, 4);
const UI_SET_KEYBIT: u32 = ioc(IOC_WRITE, b'U', 101, 4);
const UI_SET_RELBIT: u32 = ioc(IOC_WRITE, b'U', 102, 4);

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    kind: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
struct UinputSetup {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
    name: [u8; 80],
    ff_effects_max: u32,
}

/// Événement brut (type, code, valeur) : forme testable des écritures.
pub type Ev = (u16, u16, i32);

// ---- traduction des touches ----

/// Code evdev d'une touche logique (valeurs Windows de `keys`). `None` si inconnue.
pub fn evdev_key(vk: VIRTUAL_KEY) -> Option<u16> {
    Some(match vk.0 {
        0x08 => 14,  // Retour arrière
        0x09 => 15,  // Tab
        0x0D => 28,  // Entrée
        0x10 => 42,  // Maj
        0x12 => 56,  // Alt gauche
        0x1B => 1,   // Échap
        0x25 => 105, // gauche
        0x26 => 103, // haut
        0x27 => 106, // droite
        0x28 => 108, // bas
        0x5D => 127, // touche Menu (contexte)
        0x72 => 61,  // F3
        0xAD => 113, // muet
        0xAE => 114, // volume -
        0xAF => 115, // volume +
        0xB0 => 163, // piste suivante
        0xB1 => 165, // piste précédente
        0xB2 => 166, // stop
        0xB3 => 164, // lecture/pause
        _ => return None,
    })
}

/// Touche d'un caractère ASCII sur un clavier QWERTY américain : (code evdev, Maj).
pub fn ascii_key(c: char) -> Option<(u16, bool)> {
    const ROW_Q: &str = "qwertyuiop"; // 16..
    const ROW_A: &str = "asdfghjkl"; // 30..
    const ROW_Z: &str = "zxcvbnm"; // 44..
    let lower = c.to_ascii_lowercase();
    if c.is_ascii_alphabetic() {
        let shift = c.is_ascii_uppercase();
        let code = ROW_Q
            .find(lower)
            .map(|i| 16 + i as u16)
            .or_else(|| ROW_A.find(lower).map(|i| 30 + i as u16))
            .or_else(|| ROW_Z.find(lower).map(|i| 44 + i as u16))?;
        return Some((code, shift));
    }
    if let Some(d) = c.to_digit(10).filter(|_| c.is_ascii_digit()) {
        return Some((if d == 0 { 11 } else { 1 + d as u16 }, false));
    }
    // (sans Maj, avec Maj)
    Some(match c {
        ' ' => (57, false),
        '\n' => (28, false),
        '\t' => (15, false),
        '-' => (12, false),
        '_' => (12, true),
        '=' => (13, false),
        '+' => (13, true),
        '[' => (26, false),
        '{' => (26, true),
        ']' => (27, false),
        '}' => (27, true),
        ';' => (39, false),
        ':' => (39, true),
        '\'' => (40, false),
        '"' => (40, true),
        '`' => (41, false),
        '~' => (41, true),
        '\\' => (43, false),
        '|' => (43, true),
        ',' => (51, false),
        '<' => (51, true),
        '.' => (52, false),
        '>' => (52, true),
        '/' => (53, false),
        '?' => (53, true),
        '!' => (2, true),
        '@' => (3, true),
        '#' => (4, true),
        '$' => (5, true),
        '%' => (6, true),
        '^' => (7, true),
        '&' => (8, true),
        '*' => (9, true),
        '(' => (10, true),
        ')' => (11, true),
        _ => return None,
    })
}

/// Événements d'un appui ou d'un relâchement (un `SYN_REPORT` après chacun).
pub fn key_events(code: u16, down: bool) -> Vec<Ev> {
    vec![(EV_KEY, code, i32::from(down)), (EV_SYN, 0, 0)]
}

/// Combinaison : modificateurs enfoncés, touche, relâchement dans l'ordre inverse.
pub fn chord_events(mods: &[u16], code: u16) -> Vec<Ev> {
    let mut v = Vec::new();
    for m in mods {
        v.extend(key_events(*m, true));
    }
    v.extend(key_events(code, true));
    v.extend(key_events(code, false));
    for m in mods.iter().rev() {
        v.extend(key_events(*m, false));
    }
    v
}

/// Mouvement relatif (une seule trame).
pub fn move_events(dx: i32, dy: i32) -> Vec<Ev> {
    let mut v = Vec::new();
    if dx != 0 {
        v.push((EV_REL, REL_X, dx));
    }
    if dy != 0 {
        v.push((EV_REL, REL_Y, dy));
    }
    v.push((EV_SYN, 0, 0));
    v
}

/// Clic (appui puis relâchement).
pub fn click_events(right: bool) -> Vec<Ev> {
    let b = if right { BTN_RIGHT } else { BTN_LEFT };
    let mut v = key_events(b, true);
    v.extend(key_events(b, false));
    v
}

/// Molette : `delta` en unités Windows (120 par cran, > 0 vers le haut). Au moins un cran si `delta != 0`.
pub fn scroll_events(delta: i32) -> Vec<Ev> {
    if delta == 0 {
        return Vec::new();
    }
    let notches = if delta.abs() < 120 { delta.signum() } else { delta / 120 };
    vec![(EV_REL, REL_WHEEL, notches), (EV_REL, REL_WHEEL_HI_RES, notches * 120), (EV_SYN, 0, 0)]
}

/// Défilement horizontal (non utilisé par le protocole actuel, gardé pour symétrie et tests).
#[allow(dead_code)]
pub fn hscroll_events(delta: i32) -> Vec<Ev> {
    vec![(EV_REL, REL_HWHEEL, delta.signum()), (EV_SYN, 0, 0)]
}

/// Disposition « américaine ou inconnue » : seule situation où les touches uinput ASCII sont sûres.
pub fn layout_is_us(layout: Option<&str>) -> bool {
    match layout.map(str::trim) {
        None | Some("") => true,
        Some(l) => l.split(',').next().map(|f| f.trim() == "us").unwrap_or(true),
    }
}

/// Extrait `layout:` de la sortie de `setxkbmap -query`.
pub fn parse_setxkbmap(out: &str) -> Option<String> {
    out.lines().find_map(|l| l.strip_prefix("layout:")).map(|v| v.trim().to_string())
}

// ---- périphériques ----

struct Device {
    file: File,
}

impl Device {
    fn open(kind: &str) -> Result<Device, &'static str> {
        let file = OpenOptions::new().write(true).open("/dev/uinput").map_err(|_| "uinput_denied")?;
        let fd = file.as_raw_fd();
        // SAFETY : ioctl uinput sur un descripteur valide ; les arguments sont des entiers ou une structure `repr(C)`.
        unsafe {
            let ioctl = |req: u32, arg: libc::c_int| libc::ioctl(fd, req as _, arg) >= 0;
            let mut ok = true;
            if kind == "keyboard" {
                ok &= ioctl(UI_SET_EVBIT, i32::from(EV_KEY));
                for code in 1..=248 {
                    ok &= ioctl(UI_SET_KEYBIT, code);
                }
            } else {
                ok &= ioctl(UI_SET_EVBIT, i32::from(EV_KEY));
                ok &= ioctl(UI_SET_KEYBIT, i32::from(BTN_LEFT));
                ok &= ioctl(UI_SET_KEYBIT, i32::from(BTN_RIGHT));
                ok &= ioctl(UI_SET_EVBIT, i32::from(EV_REL));
                for rel in [REL_X, REL_Y, REL_HWHEEL, REL_WHEEL, REL_WHEEL_HI_RES] {
                    ok &= ioctl(UI_SET_RELBIT, i32::from(rel));
                }
            }
            let mut name = [0u8; 80];
            let label = format!("RusTV virtual {kind}");
            name[..label.len()].copy_from_slice(label.as_bytes());
            let setup = UinputSetup { bustype: BUS_USB, vendor: 0x1209, product: 0x5254, version: 1, name, ff_effects_max: 0 };
            ok &= libc::ioctl(fd, UI_DEV_SETUP as _, &setup as *const UinputSetup) >= 0;
            ok &= libc::ioctl(fd, UI_DEV_CREATE as _) >= 0;
            if !ok {
                return Err("uinput_denied");
            }
        }
        // Le bureau a besoin d'un instant pour voir apparaître le nouveau périphérique.
        std::thread::sleep(Duration::from_millis(250));
        Ok(Device { file })
    }

    fn send(&mut self, evs: &[Ev]) {
        let mut buf = Vec::with_capacity(evs.len() * std::mem::size_of::<InputEvent>());
        for (kind, code, value) in evs {
            let e = InputEvent { time: libc::timeval { tv_sec: 0, tv_usec: 0 }, kind: *kind, code: *code, value: *value };
            // SAFETY : `InputEvent` est `repr(C)` sans remplissage exploitable ; on copie ses octets tels quels.
            let bytes = unsafe { std::slice::from_raw_parts(&e as *const InputEvent as *const u8, std::mem::size_of::<InputEvent>()) };
            buf.extend_from_slice(bytes);
        }
        let _ = self.file.write_all(&buf);
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY : le descripteur est encore ouvert (fermé après ce Drop).
        unsafe {
            libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as _);
        }
    }
}

static KEYBOARD: Mutex<Option<Device>> = Mutex::new(None);
static MOUSE: Mutex<Option<Device>> = Mutex::new(None);
static WARNED: AtomicBool = AtomicBool::new(false);

fn warn(msg: &str) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("rustv-core: input {msg}");
    }
}

/// Exécute `f` sur le périphérique (créé à la première utilisation, nouvel essai à chaque appel en cas d'échec).
fn with(slot: &'static Mutex<Option<Device>>, kind: &str, f: impl FnOnce(&mut Device)) -> Result<(), &'static str> {
    let mut g = slot.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        match Device::open(kind) {
            Ok(d) => *g = Some(d),
            Err(e) => {
                warn(e);
                return Err(e);
            }
        }
    }
    if let Some(d) = g.as_mut() {
        f(d);
    }
    Ok(())
}

fn send_kbd(evs: &[Ev]) {
    let _ = with(&KEYBOARD, "keyboard", |d| d.send(evs));
}
fn send_mouse(evs: &[Ev]) {
    let _ = with(&MOUSE, "mouse", |d| d.send(evs));
}

/// `Ok` si `/dev/uinput` est utilisable (le clavier virtuel est créé), `Err("uinput_denied")` sinon.
pub fn check() -> Result<(), &'static str> {
    with(&KEYBOARD, "keyboard", |_| {})?;
    with(&MOUSE, "mouse", |_| {})
}

pub fn key_down(vk: VIRTUAL_KEY) {
    if let Some(c) = evdev_key(vk) {
        send_kbd(&key_events(c, true));
    }
}
pub fn key_up(vk: VIRTUAL_KEY) {
    if let Some(c) = evdev_key(vk) {
        send_kbd(&key_events(c, false));
    }
}

/// Touches média : on passe d'abord par MPRIS (agit sur le lecteur actif même sans démon de touches
/// multimédia) ; à défaut de lecteur, la vraie touche est envoyée.
fn media_action(vk: VIRTUAL_KEY) -> Option<&'static str> {
    match vk.0 {
        0xB3 => Some("PlayPause"),
        0xB0 => Some("Next"),
        0xB1 => Some("Previous"),
        0xB2 => Some("Stop"),
        _ => None,
    }
}

pub fn tap(vk: VIRTUAL_KEY) {
    if let Some(a) = media_action(vk) {
        if crate::smtc::control(a) {
            return;
        }
    }
    if let Some(c) = evdev_key(vk) {
        let mut v = key_events(c, true);
        v.extend(key_events(c, false));
        send_kbd(&v);
    }
}

pub fn chord(mods: &[VIRTUAL_KEY], vk: VIRTUAL_KEY) {
    let m: Vec<u16> = mods.iter().filter_map(|k| evdev_key(*k)).collect();
    if let Some(c) = evdev_key(vk) {
        send_kbd(&chord_events(&m, c));
    }
}

fn session_layout() -> Option<String> {
    let out = Command::new("setxkbmap").arg("-query").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    parse_setxkbmap(&String::from_utf8_lossy(&out.stdout))
}

/// Lance un outil de saisie ; vrai s'il a réussi.
fn run_typer(prog: &str, args: &[&str], text: &str) -> bool {
    let Some(path) = crate::platform::which(prog) else { return false };
    Command::new(path).args(args).arg("--").arg(text).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

pub fn type_text(text: &str) {
    if text.is_empty() {
        return;
    }
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    let x11 = std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
    if wayland && run_typer("wtype", &[], text) {
        return;
    }
    if x11 && run_typer("xdotool", &["type", "--delay", "8"], text) {
        return;
    }
    // Repli : touches uinput, valables seulement avec une disposition américaine (ou inconnue).
    let layout = if x11 { session_layout() } else { None };
    if layout_is_us(layout.as_deref()) && text.chars().all(|c| ascii_key(c).is_some()) {
        for c in text.chars() {
            if let Some((code, shift)) = ascii_key(c) {
                let mods = if shift { vec![KEY_LEFTSHIFT] } else { vec![] };
                send_kbd(&chord_events(&mods, code));
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        return;
    }
    warn("tool_missing");
}

pub fn mouse_move(dx: i32, dy: i32) {
    send_mouse(&move_events(dx, dy));
}
pub fn mouse_click(right: bool) {
    send_mouse(&click_events(right));
}
pub fn mouse_scroll(delta: i32) {
    send_mouse(&scroll_events(delta));
}

// ---- Fenêtres : non applicable (valeurs neutres) ----

pub fn foreground() -> HWND {
    HWND(std::ptr::null_mut())
}
pub fn foreground_title() -> String {
    String::new()
}
pub fn set_topmost(_hwnd: HWND, _on: bool) {}
pub fn force_foreground(_hwnd: isize) {}
pub fn is_window(_hwnd: HWND) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioctl_numbers_match_the_kernel() {
        assert_eq!(UI_SET_EVBIT, 0x4004_5564);
        assert_eq!(UI_SET_KEYBIT, 0x4004_5565);
        assert_eq!(UI_SET_RELBIT, 0x4004_5566);
        assert_eq!(UI_DEV_SETUP, 0x405c_5503);
        assert_eq!(UI_DEV_CREATE, 0x5501);
        assert_eq!(UI_DEV_DESTROY, 0x5502);
        assert_eq!(std::mem::size_of::<UinputSetup>(), 92);
        assert_eq!(std::mem::size_of::<InputEvent>(), 24);
    }

    #[test]
    fn navigation_keys_map_to_evdev() {
        assert_eq!(evdev_key(keys::VK_LEFT), Some(105));
        assert_eq!(evdev_key(keys::VK_UP), Some(103));
        assert_eq!(evdev_key(keys::VK_RIGHT), Some(106));
        assert_eq!(evdev_key(keys::VK_DOWN), Some(108));
        assert_eq!(evdev_key(keys::VK_RETURN), Some(28));
        assert_eq!(evdev_key(keys::VK_ESCAPE), Some(1));
        assert_eq!(evdev_key(keys::VK_BACK), Some(14));
        assert_eq!(evdev_key(keys::VK_TAB), Some(15));
        assert_eq!(evdev_key(keys::VK_MENU), Some(56));
        assert_eq!(evdev_key(keys::VK_APPS), Some(127));
        assert_eq!(evdev_key(keys::VK_F3), Some(61));
        assert_eq!(evdev_key(VIRTUAL_KEY(0xFF)), None);
    }

    #[test]
    fn media_and_volume_keys_map_to_evdev() {
        assert_eq!(evdev_key(keys::VK_VOLUME_MUTE), Some(113));
        assert_eq!(evdev_key(keys::VK_VOLUME_DOWN), Some(114));
        assert_eq!(evdev_key(keys::VK_VOLUME_UP), Some(115));
        assert_eq!(evdev_key(keys::VK_MEDIA_PLAY_PAUSE), Some(164));
        assert_eq!(evdev_key(keys::VK_MEDIA_NEXT_TRACK), Some(163));
        assert_eq!(evdev_key(keys::VK_MEDIA_PREV_TRACK), Some(165));
        assert_eq!(evdev_key(keys::VK_MEDIA_STOP), Some(166));
        assert_eq!(media_action(keys::VK_MEDIA_PLAY_PAUSE), Some("PlayPause"));
        assert_eq!(media_action(keys::VK_VOLUME_UP), None);
    }

    #[test]
    fn ascii_table_us_layout() {
        assert_eq!(ascii_key('q'), Some((16, false)));
        assert_eq!(ascii_key('Q'), Some((16, true)));
        assert_eq!(ascii_key('a'), Some((30, false)));
        assert_eq!(ascii_key('m'), Some((50, false)));
        assert_eq!(ascii_key('1'), Some((2, false)));
        assert_eq!(ascii_key('0'), Some((11, false)));
        assert_eq!(ascii_key('@'), Some((3, true)));
        assert_eq!(ascii_key(' '), Some((57, false)));
        assert_eq!(ascii_key('é'), None);
        // chaque lettre minuscule a une touche distincte
        let mut codes: Vec<u16> = ('a'..='z').map(|c| ascii_key(c).unwrap().0).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), 26);
    }

    #[test]
    fn event_sequences() {
        assert_eq!(key_events(30, true), vec![(EV_KEY, 30, 1), (EV_SYN, 0, 0)]);
        let c = chord_events(&[56], 15);
        assert_eq!(c.iter().filter(|e| e.0 == EV_KEY).map(|e| (e.1, e.2)).collect::<Vec<_>>(), vec![(56, 1), (15, 1), (15, 0), (56, 0)]);
        assert_eq!(move_events(5, 0), vec![(EV_REL, REL_X, 5), (EV_SYN, 0, 0)]);
        assert_eq!(click_events(true)[0], (EV_KEY, BTN_RIGHT, 1));
        assert_eq!(click_events(false)[2], (EV_KEY, BTN_LEFT, 0));
        assert_eq!(scroll_events(120)[0], (EV_REL, REL_WHEEL, 1));
        assert_eq!(scroll_events(-240)[0], (EV_REL, REL_WHEEL, -2));
        assert_eq!(scroll_events(-10)[0], (EV_REL, REL_WHEEL, -1));
        assert!(scroll_events(0).is_empty());
    }

    /// Manuel : sans /dev/uinput accessible, `check()` doit renvoyer `uinput_denied` (jamais de faux succès).
    #[test]
    #[ignore]
    fn live_check_reports_denied_or_ok() {
        match check() {
            Ok(()) => println!("uinput ok"),
            Err(e) => assert_eq!(e, "uinput_denied"),
        }
    }

    #[test]
    fn layout_detection() {
        assert!(layout_is_us(None));
        assert!(layout_is_us(Some("us")));
        assert!(layout_is_us(Some("us,fr")));
        assert!(!layout_is_us(Some("fr")));
        assert!(!layout_is_us(Some("fr,us")));
        assert_eq!(parse_setxkbmap("rules:      evdev\nmodel:      pc105\nlayout:     fr\n"), Some("fr".into()));
        assert_eq!(parse_setxkbmap("nothing"), None);
    }
}
