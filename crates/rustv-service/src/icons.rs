//! Extraction de l'icône d'un raccourci (.lnk) en PNG base64.
use base64::{engine::general_purpose::STANDARD, Engine};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

/// À appeler une fois par thread qui extrait des icônes (le shell exige COM).
pub fn com_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn dib(hdc: HDC, hbm: HBITMAP, w: i32, h: i32) -> Option<Vec<u8>> {
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h, // haut vers le bas
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let n = unsafe { GetDIBits(hdc, hbm, 0, h as u32, Some(buf.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS) };
    (n != 0).then_some(buf)
}

pub fn icon_png_base64(path: &str) -> Option<String> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut info = SHFILEINFOW::default();
        let r = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if r == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let hicon = info.hIcon;
        let mut ii = ICONINFO::default();
        let ok = GetIconInfo(hicon, &mut ii).is_ok();
        let result = if ok && !ii.hbmColor.is_invalid() {
            let mut bm = BITMAP::default();
            GetObjectW(ii.hbmColor.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            let hdc = GetDC(None);
            let color = dib(hdc, ii.hbmColor, w, h);
            let mask = if ii.hbmMask.is_invalid() { None } else { dib(hdc, ii.hbmMask, w, h) };
            ReleaseDC(None, hdc);
            color.and_then(|mut px| encode(&mut px, mask.as_deref(), w as u32, h as u32))
        } else {
            None
        };
        if !ii.hbmColor.is_invalid() {
            let _ = DeleteObject(ii.hbmColor.into());
        }
        if !ii.hbmMask.is_invalid() {
            let _ = DeleteObject(ii.hbmMask.into());
        }
        let _ = DestroyIcon(hicon);
        result
    }
}

fn encode(bgra: &mut [u8], mask: Option<&[u8]>, w: u32, h: u32) -> Option<String> {
    // Icônes anciennes : pas de canal alpha → on utilise le masque (blanc = transparent).
    let has_alpha = bgra.chunks_exact(4).any(|p| p[3] != 0);
    for (i, p) in bgra.chunks_exact_mut(4).enumerate() {
        if !has_alpha {
            let transparent = mask.map(|m| m[i * 4] != 0).unwrap_or(false);
            p[3] = if transparent { 0 } else { 255 };
        }
        p.swap(0, 2); // BGRA → RGBA
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(bgra).ok()?;
    }
    Some(STANDARD.encode(out))
}
