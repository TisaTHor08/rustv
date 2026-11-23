//! Injection d'entrées via SendInput + fenêtres (épinglage).
pub use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Codes de touches logiques utilisés par le cœur (mêmes noms que Windows ; les stubs non-Windows
/// définissent les mêmes constantes, voir `platform/stub/input.rs`).
pub mod keys {
    pub use windows::Win32::UI::Input::KeyboardAndMouse::{
        VIRTUAL_KEY, VK_APPS, VK_BACK, VK_DOWN, VK_ESCAPE, VK_F3, VK_LEFT, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
        VK_MEDIA_STOP, VK_MENU, VK_RETURN, VK_RIGHT, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    };
}

/// L'injection d'entrées est disponible (toujours vrai sous Windows).
pub fn check() -> Result<(), &'static str> {
    Ok(())
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn kbd(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

/// Les flèches et quelques touches sont des touches « étendues ».
fn is_extended(vk: VIRTUAL_KEY) -> bool {
    matches!(vk, VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN | VK_HOME | VK_END | VK_PRIOR | VK_NEXT | VK_INSERT | VK_DELETE)
}

fn flags(vk: VIRTUAL_KEY, up: bool) -> KEYBD_EVENT_FLAGS {
    let mut f = KEYBD_EVENT_FLAGS(0);
    if is_extended(vk) {
        f |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        f |= KEYEVENTF_KEYUP;
    }
    f
}

pub fn key_down(vk: VIRTUAL_KEY) {
    send(&[kbd(vk, 0, flags(vk, false))]);
}
pub fn key_up(vk: VIRTUAL_KEY) {
    send(&[kbd(vk, 0, flags(vk, true))]);
}
pub fn tap(vk: VIRTUAL_KEY) {
    send(&[kbd(vk, 0, flags(vk, false)), kbd(vk, 0, flags(vk, true))]);
}

/// Combinaison : modificateur(s) maintenus pendant l'appui de `vk`.
pub fn chord(mods: &[VIRTUAL_KEY], vk: VIRTUAL_KEY) {
    let mut v: Vec<INPUT> = mods.iter().map(|m| kbd(*m, 0, flags(*m, false))).collect();
    v.push(kbd(vk, 0, flags(vk, false)));
    v.push(kbd(vk, 0, flags(vk, true)));
    v.extend(mods.iter().rev().map(|m| kbd(*m, 0, flags(*m, true))));
    send(&v);
}

/// Injecte du texte Unicode (fonctionne indépendamment de la disposition clavier).
/// Vraie touche (selon la disposition de clavier courante) pour un caractère, avec l'état de Maj.
/// `None` si le caractère exige Ctrl/Alt/AltGr ou n'existe pas sur ce clavier : on retombe alors sur l'Unicode.
fn real_key(u: u16, hkl: HKL) -> Option<(VIRTUAL_KEY, bool)> {
    let r = unsafe { VkKeyScanExW(u, hkl) };
    if r == -1 {
        return None;
    }
    let vk = (r & 0xFF) as u16;
    let state = ((r >> 8) & 0xFF) as u8;
    if vk == 0xFF || state & !1 != 0 {
        return None;
    }
    Some((VIRTUAL_KEY(vk), state & 1 != 0))
}

/// Saisit du texte. Les caractères ordinaires passent par de vraies touches : certaines applis (YouTube TV, jeux,
/// interfaces TV) ignorent les événements Unicode et n'écoutent que les codes de touches. Le reste est injecté en Unicode.
pub fn type_text(text: &str) {
    let mut v = Vec::new();
    // La disposition de clavier est propre à la fenêtre qui reçoit les touches (AZERTY, QWERTY…) : un « 7 » ou un
    // « @ » n'ont pas la même touche partout. On calcule donc avec celle de la fenêtre au premier plan.
    let hkl = unsafe {
        let tid = GetWindowThreadProcessId(GetForegroundWindow(), None);
        GetKeyboardLayout(tid)
    };
    for u in text.encode_utf16() {
        match real_key(u, hkl) {
            Some((vk, shift)) => {
                if !v.is_empty() {
                    send(&v);
                    v.clear();
                }
                let mut one = Vec::with_capacity(4);
                if shift {
                    one.push(kbd(VK_SHIFT, 0, flags(VK_SHIFT, false)));
                }
                one.push(kbd(vk, 0, flags(vk, false)));
                one.push(kbd(vk, 0, flags(vk, true)));
                if shift {
                    one.push(kbd(VK_SHIFT, 0, flags(VK_SHIFT, true)));
                }
                send(&one);
                // Les interfaces TV (YouTube…) perdent des touches envoyées en rafale : on espace un peu.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            None => {
                v.push(kbd(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE));
                v.push(kbd(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
            }
        }
    }
    if !v.is_empty() {
        send(&v);
    }
}

pub fn mouse_move(dx: i32, dy: i32) {
    mouse(MOUSEEVENTF_MOVE, dx, dy, 0);
}
pub fn mouse_click(right: bool) {
    let (d, u) = if right { (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP) } else { (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP) };
    mouse(d, 0, 0, 0);
    mouse(u, 0, 0, 0);
}
pub fn mouse_scroll(delta: i32) {
    mouse(MOUSEEVENTF_WHEEL, 0, 0, delta);
}
fn mouse(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, data: i32) {
    send(&[INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }]);
}

// ---- Fenêtres ----

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn set_topmost(hwnd: HWND, on: bool) {
    unsafe {
        let after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
        let _ = SetWindowPos(hwnd, Some(after), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Met une fenêtre au premier plan même si le process n'a pas le focus.
/// Windows n'autorise SetForegroundWindow qu'au process qui vient de recevoir une entrée :
/// une frappe F24 (sans effet dans les applis) suffit à « débloquer » l'appel.
pub fn force_foreground(hwnd: isize) {
    let h = HWND(hwnd as *mut _);
    tap(VK_F24);
    unsafe {
        let _ = SetForegroundWindow(h);
        let _ = BringWindowToTop(h);
    }
}

/// Titre de la fenêtre au premier plan (vide si inconnu).
pub fn foreground_title() -> String {
    let h = foreground();
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn is_window(hwnd: HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}
