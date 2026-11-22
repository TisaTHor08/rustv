//! Icône de zone de notification, menu, fenêtre QR (Win32 pur, sans framework).
use crate::core::Core;
use crate::server;
use rustv_protocol::pairing::PairingInfo;
use rustv_protocol::TCP_PORT;
use qrcode::{Color, QrCode};
use std::sync::{Arc, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static CORE: OnceLock<Arc<Core>> = OnceLock::new();
const WM_TRAY: u32 = WM_APP + 1;
const ID_ADD: usize = 1;
const ID_AUTOSTART: usize = 2;
const ID_QUIT: usize = 3;
const ID_REVOKE_BASE: usize = 100;
const MODULE_PX: i32 = 6;
const QUIET: i32 = 3;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---- démarrage automatique (HKCU\...\Run) ----

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VAL: PCWSTR = w!("RusTV");

fn autostart_enabled() -> bool {
    unsafe {
        let mut k = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_READ, &mut k).is_err() {
            return false;
        }
        let r = RegQueryValueExW(k, RUN_VAL, None, None, None, None);
        let _ = RegCloseKey(k);
        r.is_ok()
    }
}

pub fn set_autostart(on: bool) {
    unsafe {
        let mut k = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut k, None).is_err() {
            return;
        }
        if on {
            if let Ok(exe) = std::env::current_exe() {
                let v = wide(&format!("\"{}\"", exe.display()));
                let bytes = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2);
                let _ = RegSetValueExW(k, RUN_VAL, None, REG_SZ, Some(bytes));
            }
        } else {
            let _ = RegDeleteValueW(k, RUN_VAL);
        }
        let _ = RegCloseKey(k);
    }
}

// ---- fenêtre QR ----

struct QrData {
    modules: Vec<bool>,
    width: i32,
    caption: String,
}

fn show_qr(core: &Arc<Core>) {
    let (info, name) = {
        let mut cfg = core.cfg.lock().unwrap();
        let dev = cfg.new_device();
        (
            PairingInfo {
                v: rustv_protocol::PROTOCOL_VERSION,
                id: cfg.pc_id.clone(),
                name: cfg.pc_name.clone(),
                device: dev.id,
                key: dev.key,
                ips: server::local_ips(),
                port: TCP_PORT,
                bt: core.bt_addr.lock().unwrap().clone(),
            },
            dev.name,
        )
    };
    let Ok(qr) = QrCode::new(info.to_qr_string().as_bytes()) else { return };
    let width = qr.width() as i32;
    let modules = qr.to_colors().into_iter().map(|c| c == Color::Dark).collect();
    let data = Box::new(QrData { modules, width, caption: format!("Scannez avec l'appli RusTV ({name})") });

    unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let side = (width + QUIET * 2) * MODULE_PX;
        let (cw, ch) = (side.max(360), side + 40);
        let mut rc = RECT { left: 0, top: 0, right: cw, bottom: ch };
        let style = WS_CAPTION | WS_SYSMENU;
        let _ = AdjustWindowRect(&mut rc, style, false);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
        let _ = CreateWindowExW(
            WS_EX_TOPMOST,
            w!("RusTVQr"),
            w!("Appairer un téléphone"),
            style | WS_VISIBLE,
            (sw - ww) / 2,
            (sh - wh) / 2,
            ww,
            wh,
            None,
            None,
            Some(hinst.into()),
            Some(Box::into_raw(data) as *const _),
        );
    }
}

unsafe extern "system" fn qr_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const QrData;
            if !p.is_null() {
                let d = &*p;
                let mut client = RECT::default();
                let _ = GetClientRect(hwnd, &mut client);
                let white = CreateSolidBrush(COLORREF(0x00FFFFFF));
                let black = CreateSolidBrush(COLORREF(0));
                FillRect(hdc, &client, white);
                let side = (d.width + QUIET * 2) * MODULE_PX;
                let ox = (client.right - side) / 2 + QUIET * MODULE_PX;
                let oy = QUIET * MODULE_PX;
                for y in 0..d.width {
                    for x in 0..d.width {
                        if d.modules[(y * d.width + x) as usize] {
                            let r = RECT { left: ox + x * MODULE_PX, top: oy + y * MODULE_PX, right: ox + (x + 1) * MODULE_PX, bottom: oy + (y + 1) * MODULE_PX };
                            FillRect(hdc, &r, black);
                        }
                    }
                }
                let mut cap = wide(&d.caption);
                let mut tr = RECT { left: 0, top: client.bottom - 34, right: client.right, bottom: client.bottom };
                SetBkMode(hdc, TRANSPARENT);
                DrawTextW(hdc, &mut cap[..], &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                let _ = DeleteObject(white.into());
                let _ = DeleteObject(black.into());
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let p = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut QrData;
            if !p.is_null() {
                drop(Box::from_raw(p));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ---- icône de notification ----

fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() };
    for (i, c) in "RusTV".encode_utf16().enumerate() {
        nid.szTip[i] = c;
    }
    nid
}

fn show_menu(hwnd: HWND) {
    let core = CORE.get().unwrap();
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        let n = core.conn_count();
        let status = wide(&format!("{} téléphone(s) connecté(s)", n));
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(status.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let add = wide("Ajouter un téléphone (QR code)…");
        let _ = AppendMenuW(menu, MF_STRING, ID_ADD, PCWSTR(add.as_ptr()));
        let devices = core.cfg.lock().unwrap().devices.clone();
        let labels: Vec<Vec<u16>> = devices.iter().map(|d| wide(&format!("Révoquer : {} ({})", d.name, &d.id[..4.min(d.id.len())]))).collect();
        for (i, l) in labels.iter().enumerate() {
            let _ = AppendMenuW(menu, MF_STRING, ID_REVOKE_BASE + i, PCWSTR(l.as_ptr()));
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let auto = wide("Lancer au démarrage de Windows");
        let flags = MF_STRING | if autostart_enabled() { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(menu, flags, ID_AUTOSTART, PCWSTR(auto.as_ptr()));
        let quit = wide("Quitter");
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, PCWSTR(quit.as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd); // requis pour que le menu se ferme au clic extérieur
        let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), hwnd, None).0 as usize;
        let _ = DestroyMenu(menu);

        match cmd {
            ID_ADD => show_qr(core),
            ID_AUTOSTART => set_autostart(!autostart_enabled()),
            ID_QUIT => {
                let _ = DestroyWindow(hwnd);
            }
            c if c >= ID_REVOKE_BASE => {
                if let Some(d) = devices.get(c - ID_REVOKE_BASE) {
                    core.cfg.lock().unwrap().revoke(&d.id);
                }
            }
            _ => {}
        }
    }
}

unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let ev = (lp.0 & 0xFFFF) as u32;
            if ev == WM_LBUTTONUP || ev == WM_RBUTTONUP {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let nid = tray_data(hwnd);
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

pub fn run(core: Arc<Core>) {
    let _ = CORE.set(core);
    unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).unwrap_or_default().into();
        for (name, proc) in [(w!("RusTVMain"), Some(main_proc as _)), (w!("RusTVQr"), Some(qr_proc as _))] {
            let wc = WNDCLASSW { lpfnWndProc: proc, hInstance: hinst, lpszClassName: name, hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), ..Default::default() };
            RegisterClassW(&wc);
        }
        let hwnd = CreateWindowExW(WINDOW_EX_STYLE(0), w!("RusTVMain"), w!("RusTV"), WINDOW_STYLE(0), 0, 0, 0, 0, None, None, Some(hinst), None).unwrap();
        let nid = tray_data(hwnd);
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);

        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}
