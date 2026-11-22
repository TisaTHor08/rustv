//! Injection d'entrées via SendInput + fenêtres (épinglage).
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

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
pub fn type_text(text: &str) {
    let mut v = Vec::new();
    for u in text.encode_utf16() {
        v.push(kbd(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE));
        v.push(kbd(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
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

pub fn is_window(hwnd: HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}
