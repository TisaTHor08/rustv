//! Écran et son depuis l'interface TV : sortie audio par défaut, volume des sorties, résolution / fréquence
//! d'un écran (avec retour automatique au bout de 15 s sans confirmation) et HDR (« advanced color »).
//!
//! Organisation : la logique pure (filtrage des modes, choix de l'écran, machine d'état du retour arrière,
//! libellés) est testée ; les appels COM / Win32 (module `sys`) restent minces et ne sont PAS testés
//! automatiquement car ils modifient le matériel réel.
use crate::Shell;
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::State;

/// Délai pour confirmer un changement de mode avant retour automatique.
pub const REVERT_SECONDS: u32 = 15;
const MAX_MODES: usize = 400;

// ---------- logique pure : modes ----------

/// Mode tel que listé par le pilote (avant filtrage).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub bits: u32,
    pub interlaced: bool,
}

/// Mode retenu : taille et fréquence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
}

/// « 1920 x 1080 @ 60 Hz ».
#[cfg_attr(not(test), allow(dead_code))]
pub fn mode_label(m: &Mode) -> String {
    format!("{} x {} @ {} Hz", m.width, m.height, m.refresh)
}

/// Filtre, dédoublonne et trie (grande taille puis haute fréquence d'abord) : cache l'entrelacé, les tailles
/// minuscules (< 800 x 600), les fréquences < 24 Hz (0/1 = « défaut matériel ») et les profondeurs < 24 bits
/// quand une profondeur plus grande existe.
pub fn filter_modes(raw: &[RawMode]) -> Vec<Mode> {
    let best_bits = raw.iter().map(|m| m.bits).max().unwrap_or(0);
    let min_bits = if best_bits >= 24 { 24 } else { 0 };
    let mut v: Vec<Mode> = raw
        .iter()
        .filter(|m| !m.interlaced && m.width >= 800 && m.height >= 600 && m.refresh >= 24 && m.bits >= min_bits)
        .map(|m| Mode { width: m.width, height: m.height, refresh: m.refresh })
        .collect();
    v.sort_by(|a, b| (b.width, b.height, b.refresh).cmp(&(a.width, a.height, a.refresh)));
    v.dedup();
    v.truncate(MAX_MODES);
    v
}

/// Position d'un écran : l'index suit l'ordre de `tv::monitors` (gauche -> droite, puis haut -> bas).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn sort_by_position<T>(v: &mut [T], pos: impl Fn(&T) -> (i32, i32)) {
    v.sort_by_key(|m| pos(m));
}

/// Choisit l'écran d'un index (borné : un index trop grand retombe sur le dernier, `None` si aucun écran).
pub fn pick_index(count: usize, index: usize) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some(index.min(count - 1))
    }
}

/// Indique si `(w, h, r)` fait partie des modes proposés (on refuse d'appliquer autre chose).
pub fn mode_allowed(modes: &[Mode], w: u32, h: u32, r: u32) -> bool {
    modes.contains(&Mode { width: w, height: h, refresh: r })
}

// ---------- logique pure : retour arrière ----------

/// Changement en attente de confirmation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub device: String,
    /// Mode à restaurer (celui d'avant le tout premier changement non confirmé).
    pub prev: Mode,
    pub next: Mode,
    pub deadline_ms: u64,
}

/// Machine d'état « garder ou annuler ». Le temps est passé en millisecondes pour les tests.
#[derive(Default)]
pub struct RevertState {
    pending: Option<Pending>,
    next_id: u64,
}

impl RevertState {
    /// Démarre une attente. Si un changement est déjà en attente sur le même écran, on garde son mode
    /// d'origine (enchaînement : l'annulation revient à l'état connu et confirmé).
    pub fn begin(&mut self, device: &str, prev: Mode, next: Mode, now_ms: u64, secs: u32) -> Pending {
        self.next_id += 1;
        let prev = match &self.pending {
            Some(p) if p.device == device => p.prev,
            _ => prev,
        };
        let p = Pending { id: self.next_id, device: device.to_string(), prev, next, deadline_ms: now_ms + u64::from(secs) * 1000 };
        self.pending = Some(p.clone());
        p
    }

    /// L'utilisateur garde le mode : l'attente se termine.
    pub fn confirm(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// L'utilisateur annule : l'attente se termine (l'appelant restaure `prev`).
    pub fn revert(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// Appelé par le minuteur de l'attente `id` : rend le changement à annuler si c'est toujours la bonne attente
    /// et si l'échéance est passée.
    pub fn expire(&mut self, id: u64, now_ms: u64) -> Option<Pending> {
        match &self.pending {
            Some(p) if p.id == id && now_ms >= p.deadline_ms => self.pending.take(),
            _ => None,
        }
    }

    /// Secondes restantes (arrondies au-dessus), `None` s'il n'y a pas d'attente.
    pub fn remaining(&self, now_ms: u64) -> Option<u32> {
        self.pending.as_ref().map(|p| p.deadline_ms.saturating_sub(now_ms).div_ceil(1000) as u32)
    }
}

static REVERT: Mutex<Option<RevertState>> = Mutex::new(None);
static EPOCH: Mutex<Option<Instant>> = Mutex::new(None);

fn now_ms() -> u64 {
    let mut e = EPOCH.lock().unwrap();
    e.get_or_insert_with(Instant::now).elapsed().as_millis() as u64
}

fn with_revert<R>(f: impl FnOnce(&mut RevertState) -> R) -> R {
    let mut g = REVERT.lock().unwrap();
    f(g.get_or_insert_with(RevertState::default))
}

// ---------- logique pure : audio ----------

/// Sortie audio.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub default: bool,
    pub volume: u8,
    pub muted: bool,
}

/// Index de la sortie suivante (`dir` = +1 / -1, cyclique), en partant de la sortie par défaut.
pub fn next_output(outs: &[Output], dir: i32) -> Option<usize> {
    if outs.is_empty() {
        return None;
    }
    let cur = outs.iter().position(|o| o.default).unwrap_or(0) as i32;
    Some((cur + dir).rem_euclid(outs.len() as i32) as usize)
}

/// Volume 0 à 100 depuis un niveau scalaire 0.0 à 1.0.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn level_to_percent(level: f32) -> u8 {
    (level * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Niveau scalaire 0.0 à 1.0 depuis un pourcentage (borné).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn percent_to_level(p: u32) -> f32 {
    p.min(100) as f32 / 100.0
}

/// Convertit un tampon UTF-16 terminé par 0 en `String`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn wide_to_string(w: &[u16]) -> String {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

// ---------- types exposés à l'interface ----------

#[derive(Serialize, Clone, Copy, Default)]
pub struct Hdr {
    pub supported: bool,
    pub enabled: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub device: String,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub bits_per_pixel: u32,
    pub hdr: Hdr,
    pub modes: Vec<Mode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub monitors: Vec<MonitorInfo>,
    /// Index de l'écran de la TV (mode TV actif, sinon dernier choix, sinon 0).
    pub tv_monitor: usize,
    /// Secondes avant retour automatique si un changement attend confirmation.
    pub pending_revert_seconds: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetModeResult {
    pub pending_revert_seconds: u32,
}

// ---------- appels système (non testés automatiquement) ----------

/// Windows : Core Audio, GDI, DisplayConfig. Autres OS : `platform/stub/display_sys.rs` (voir docs/PORTING.md).
#[cfg(not(windows))]
#[path = "platform/stub/display_sys.rs"]
mod sys;

#[cfg(windows)]
#[allow(non_snake_case)]
mod sys {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows::core::{interface, w, Interface, GUID, HRESULT, PCWSTR};
    use windows::Win32::Devices::Display::*;
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Foundation::{LUID, RPC_E_CHANGED_MODE};
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE, ERole};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

    fn com() {
        // Déjà initialisé sur ce thread (ou autre modèle) : sans conséquence.
        let _ = RPC_E_CHANGED_MODE;
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    // ===== Audio =====

    /// IPolicyConfig (non documentée par Microsoft), version Windows 7 et suivantes, IID f8679f50-....
    /// Disposition de la vtable, indices 0-based :
    ///  0 QueryInterface, 1 AddRef, 2 Release (IUnknown, gérés par la macro `interface`)
    ///  3 GetMixFormat, 4 GetDeviceFormat, 5 ResetDeviceFormat, 6 SetDeviceFormat,
    ///  7 GetProcessingPeriod, 8 SetProcessingPeriod, 9 GetShareMode, 10 SetShareMode,
    ///  11 GetPropertyValue, 12 SetPropertyValue, 13 SetDefaultEndpoint, 14 SetEndpointVisibility.
    /// (L'interface « PolicyConfigVista », IID 568b9108-..., n'a pas ResetDeviceFormat : SetDefaultEndpoint y
    /// est à l'indice 12. Le GUID f8679f50 correspond bien à la version avec ResetDeviceFormat.)
    /// Seule SetDefaultEndpoint est appelée ; les autres entrées ne servent qu'à garder les positions.
    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: windows::core::IUnknown {
        fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut c_void) -> HRESULT;
        fn GetDeviceFormat(&self, device: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, device: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
        fn GetProcessingPeriod(&self, device: PCWSTR, default: i32, default_period: *mut i64, min_period: *mut i64) -> HRESULT;
        fn SetProcessingPeriod(&self, device: PCWSTR, period: *mut i64) -> HRESULT;
        fn GetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn SetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn GetPropertyValue(&self, device: PCWSTR, fx_store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetPropertyValue(&self, device: PCWSTR, fx_store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> HRESULT;
    }

    /// CPolicyConfigClient.
    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    fn enumerator() -> Result<IMMDeviceEnumerator, String> {
        com();
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|_| "audio indisponible".to_string()) }
    }

    fn device_id(d: &IMMDevice) -> Option<String> {
        unsafe {
            let p = d.GetId().ok()?;
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as *const c_void));
            s
        }
    }

    fn friendly_name(d: &IMMDevice) -> String {
        unsafe {
            let Ok(store) = d.OpenPropertyStore(STGM_READ) else { return String::new() };
            let Ok(v) = store.GetValue(&PKEY_Device_FriendlyName) else { return String::new() };
            // PROPVARIANT de type VT_LPWSTR : sa représentation texte est le nom.
            v.to_string()
        }
    }

    fn volume_of(d: &IMMDevice) -> (u8, bool) {
        unsafe {
            match d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                Ok(ep) => {
                    let l = ep.GetMasterVolumeLevelScalar().map(level_to_percent).unwrap_or(0);
                    let m = ep.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    (l, m)
                }
                Err(_) => (0, false),
            }
        }
    }

    pub fn outputs() -> Result<Vec<Output>, String> {
        let en = enumerator()?;
        unsafe {
            let default_id = en.GetDefaultAudioEndpoint(eRender, eConsole).ok().and_then(|d| device_id(&d));
            let coll = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).map_err(|_| "audio indisponible".to_string())?;
            let n = coll.GetCount().unwrap_or(0).min(64);
            let mut v = Vec::new();
            for i in 0..n {
                let Ok(d) = coll.Item(i) else { continue };
                let Some(id) = device_id(&d) else { continue };
                let (volume, muted) = volume_of(&d);
                let default = default_id.as_deref() == Some(id.as_str());
                v.push(Output { name: friendly_name(&d), id, default, volume, muted });
            }
            Ok(v)
        }
    }

    pub fn set_default(id: &str) -> Result<(), String> {
        com();
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let pc: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL).map_err(|_| "changement de sortie indisponible".to_string())?;
            for role in [eConsole, eMultimedia, eCommunications] {
                pc.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).ok().map_err(|_| "changement de sortie refusé".to_string())?;
            }
        }
        Ok(())
    }

    pub fn set_volume(id: &str, percent: u32) -> Result<(), String> {
        let en = enumerator()?;
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let d = en.GetDevice(PCWSTR(wide.as_ptr())).map_err(|_| "sortie introuvable".to_string())?;
            let ep = d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(|_| "volume indisponible".to_string())?;
            ep.SetMasterVolumeLevelScalar(percent_to_level(percent), std::ptr::null()).map_err(|_| "volume refusé".to_string())?;
            if percent > 0 {
                let _ = ep.SetMute(false, std::ptr::null());
            }
        }
        Ok(())
    }

    // ===== Écrans =====

    pub struct Raw {
        pub device: String,
        pub name: String,
        pub primary: bool,
        pub x: i32,
        pub y: i32,
        pub width: u32,
        pub height: u32,
        pub refresh: u32,
        pub bits: u32,
    }

    fn dm_zero() -> DEVMODEW {
        DEVMODEW { dmSize: size_of::<DEVMODEW>() as u16, ..Default::default() }
    }

    fn wz(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn current(device: &str) -> Option<DEVMODEW> {
        let mut dm = dm_zero();
        let d = wz(device);
        unsafe { EnumDisplaySettingsExW(PCWSTR(d.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0)).as_bool().then_some(dm) }
    }

    /// Écrans actifs du bureau, triés comme `tv::monitors` (x puis y).
    pub fn monitors() -> Vec<Raw> {
        let mut v = Vec::new();
        for i in 0..32u32 {
            let mut dd = DISPLAY_DEVICEW { cb: size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            if unsafe { !EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0).as_bool() } {
                break;
            }
            let flags = dd.StateFlags.0;
            if flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP.0 == 0 || flags & DISPLAY_DEVICE_MIRRORING_DRIVER.0 != 0 {
                continue;
            }
            let device = wide_to_string(&dd.DeviceName);
            let Some(dm) = current(&device) else { continue };
            // Nom du moniteur : premier périphérique enfant.
            let mut mon = DISPLAY_DEVICEW { cb: size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            let dn = wz(&device);
            let child = unsafe { EnumDisplayDevicesW(PCWSTR(dn.as_ptr()), 0, &mut mon, 0).as_bool() };
            let name = if child { wide_to_string(&mon.DeviceString) } else { wide_to_string(&dd.DeviceString) };
            let pos = unsafe { dm.Anonymous1.Anonymous2.dmPosition };
            v.push(Raw {
                device,
                name,
                primary: flags & DISPLAY_DEVICE_PRIMARY_DEVICE.0 != 0,
                x: pos.x,
                y: pos.y,
                width: dm.dmPelsWidth,
                height: dm.dmPelsHeight,
                refresh: dm.dmDisplayFrequency,
                bits: dm.dmBitsPerPel,
            });
        }
        sort_by_position(&mut v, |m| (m.x, m.y));
        v
    }

    pub fn raw_modes(device: &str) -> Vec<RawMode> {
        let d = wz(device);
        let mut v = Vec::new();
        for i in 0..4000u32 {
            let mut dm = dm_zero();
            if unsafe { !EnumDisplaySettingsExW(PCWSTR(d.as_ptr()), ENUM_DISPLAY_SETTINGS_MODE(i), &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0)).as_bool() } {
                break;
            }
            let interlaced = unsafe { dm.Anonymous2.dmDisplayFlags } & DM_INTERLACED.0 != 0;
            v.push(RawMode { width: dm.dmPelsWidth, height: dm.dmPelsHeight, refresh: dm.dmDisplayFrequency, bits: dm.dmBitsPerPel, interlaced });
        }
        v
    }

    pub fn current_mode(device: &str) -> Option<Mode> {
        current(device).map(|dm| Mode { width: dm.dmPelsWidth, height: dm.dmPelsHeight, refresh: dm.dmDisplayFrequency })
    }

    /// Applique un mode. `persist` = CDS_UPDATEREGISTRY (définitif) ; sinon changement temporaire (CDS_FULLSCREEN),
    /// annulé au redémarrage de la session ou par un nouveau changement.
    pub fn apply(device: &str, m: Mode, persist: bool) -> Result<(), String> {
        let mut dm = current(device).ok_or("écran introuvable")?;
        dm.dmPelsWidth = m.width;
        dm.dmPelsHeight = m.height;
        dm.dmDisplayFrequency = m.refresh;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        let d = wz(device);
        let name = PCWSTR(d.as_ptr());
        unsafe {
            // Essai à blanc d'abord : ne change rien si le pilote refuse.
            if ChangeDisplaySettingsExW(name, Some(&dm), None, CDS_TEST, None) != DISP_CHANGE_SUCCESSFUL {
                return Err("mode refusé".into());
            }
            let flags = if persist { CDS_UPDATEREGISTRY } else { CDS_FULLSCREEN };
            if ChangeDisplaySettingsExW(name, Some(&dm), None, flags, None) != DISP_CHANGE_SUCCESSFUL {
                return Err("changement refusé".into());
            }
        }
        Ok(())
    }

    // ===== HDR (advanced color) =====

    #[derive(Clone, Copy)]
    pub struct Target {
        pub adapter: LUID,
        pub id: u32,
    }

    /// Cible de l'affichage (adaptateur + id) de l'écran dont le nom GDI est `device`.
    pub fn target_of(device: &str) -> Option<Target> {
        unsafe {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != windows::Win32::Foundation::WIN32_ERROR(0) {
                return None;
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None) != windows::Win32::Foundation::WIN32_ERROR(0) {
                return None;
            }
            for p in paths.iter().take(np as usize) {
                let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                src.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                src.header.adapterId = p.sourceInfo.adapterId;
                src.header.id = p.sourceInfo.id;
                if DisplayConfigGetDeviceInfo(&mut src.header) != 0 {
                    continue;
                }
                if wide_to_string(&src.viewGdiDeviceName).eq_ignore_ascii_case(device) {
                    return Some(Target { adapter: p.targetInfo.adapterId, id: p.targetInfo.id });
                }
            }
            None
        }
    }

    /// Lit (pris en charge, activé). Bits de la valeur : 0 supported, 1 enabled, 2 wideColorEnforced, 3 forceDisabled.
    pub fn hdr_get(t: Target) -> Hdr {
        unsafe {
            let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
            info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
            info.header.size = size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
            info.header.adapterId = t.adapter;
            info.header.id = t.id;
            if DisplayConfigGetDeviceInfo(&mut info.header) != 0 {
                return Hdr::default();
            }
            let v = info.Anonymous.value;
            Hdr { supported: v & 1 != 0, enabled: v & 2 != 0 }
        }
    }

    pub fn hdr_set(t: Target, on: bool) -> Result<(), String> {
        unsafe {
            let mut s = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE::default();
            s.header.r#type = DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE;
            s.header.size = size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32;
            s.header.adapterId = t.adapter;
            s.header.id = t.id;
            s.Anonymous.value = u32::from(on); // bit 0 : enableAdvancedColor
            if DisplayConfigSetDeviceInfo(&s.header) != 0 {
                return Err("HDR refusé".into());
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn _keep(_: &str) {
        let _ = w!("");
        let _ = <IPolicyConfig as Interface>::IID;
    }
}

// ---------- commandes ----------

fn build_monitors() -> Vec<MonitorInfo> {
    sys::monitors()
        .into_iter()
        .enumerate()
        .map(|(index, m)| {
            let hdr = sys::target_of(&m.device).map(sys::hdr_get).unwrap_or_default();
            let modes = filter_modes(&sys::raw_modes(&m.device));
            MonitorInfo { index, name: m.name, device: m.device, primary: m.primary, x: m.x, y: m.y, width: m.width, height: m.height, refresh: m.refresh, bits_per_pixel: m.bits, hdr, modes }
        })
        .collect()
}

fn device_of(index: usize) -> Result<String, String> {
    let mons = sys::monitors();
    let i = pick_index(mons.len(), index).ok_or("aucun écran")?;
    Ok(mons[i].device.clone())
}

#[tauri::command]
pub async fn audio_outputs() -> Result<Vec<Output>, String> {
    sys::outputs()
}

#[tauri::command]
pub async fn audio_set_default(id: String) -> Result<Vec<Output>, String> {
    if id.is_empty() || id.len() > 200 {
        return Err("sortie invalide".into());
    }
    // Refuse un identifiant qui n'est pas une sortie active connue.
    if !sys::outputs()?.iter().any(|o| o.id == id) {
        return Err("sortie introuvable".into());
    }
    sys::set_default(&id)?;
    sys::outputs()
}

#[tauri::command]
pub async fn audio_set_output_volume(id: String, level: u32) -> Result<(), String> {
    if id.is_empty() || id.len() > 200 {
        return Err("sortie invalide".into());
    }
    sys::set_volume(&id, level)
}

/// Passe à la sortie suivante (`dir` = 1) ou précédente (-1) et la rend par défaut.
#[tauri::command]
pub async fn audio_cycle_output(dir: i32) -> Result<Vec<Output>, String> {
    let outs = sys::outputs()?;
    let i = next_output(&outs, if dir < 0 { -1 } else { 1 }).ok_or("aucune sortie")?;
    sys::set_default(&outs[i].id)?;
    sys::outputs()
}

#[tauri::command]
pub async fn display_info(shell: State<'_, Shell>) -> Result<DisplayInfo, String> {
    let tv_monitor = {
        let t = shell.tv.lock().unwrap();
        if t.active {
            t.monitor
        } else {
            shell.settings.lock().unwrap().monitor.unwrap_or(0)
        }
    };
    let monitors = build_monitors();
    let tv_monitor = pick_index(monitors.len(), tv_monitor).unwrap_or(0);
    Ok(DisplayInfo { monitors, tv_monitor, pending_revert_seconds: with_revert(|r| r.remaining(now_ms())) })
}

#[tauri::command]
pub async fn display_set_mode(index: usize, width: u32, height: u32, refresh: u32) -> Result<SetModeResult, String> {
    let device = device_of(index)?;
    let modes = filter_modes(&sys::raw_modes(&device));
    if !mode_allowed(&modes, width, height, refresh) {
        return Err("mode non proposé".into());
    }
    let prev = sys::current_mode(&device).ok_or("écran introuvable")?;
    let next = Mode { width, height, refresh };
    if prev == next {
        return Ok(SetModeResult { pending_revert_seconds: 0 });
    }
    sys::apply(&device, next, false)?;
    let p = with_revert(|r| r.begin(&device, prev, next, now_ms(), REVERT_SECONDS));
    let id = p.id;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(u64::from(REVERT_SECONDS)) + Duration::from_millis(200));
        if let Some(p) = with_revert(|r| r.expire(id, now_ms())) {
            let _ = sys::apply(&p.device, p.prev, false);
        }
    });
    Ok(SetModeResult { pending_revert_seconds: REVERT_SECONDS })
}

/// Garde le mode : l'enregistre définitivement (CDS_UPDATEREGISTRY).
#[tauri::command]
pub async fn display_confirm() -> Result<(), String> {
    match with_revert(|r| r.confirm()) {
        Some(p) => sys::apply(&p.device, p.next, true),
        None => Ok(()),
    }
}

/// Annule tout de suite le changement en attente.
#[tauri::command]
pub async fn display_revert() -> Result<(), String> {
    match with_revert(|r| r.revert()) {
        Some(p) => sys::apply(&p.device, p.prev, false),
        None => Ok(()),
    }
}

#[tauri::command]
pub async fn display_set_hdr(index: usize, on: bool) -> Result<Hdr, String> {
    let device = device_of(index)?;
    let t = sys::target_of(&device).ok_or("écran introuvable")?;
    if !sys::hdr_get(t).supported {
        return Err("HDR non pris en charge".into());
    }
    sys::hdr_set(t, on)?;
    Ok(sys::hdr_get(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(w: u32, h: u32, r: u32, bits: u32, il: bool) -> RawMode {
        RawMode { width: w, height: h, refresh: r, bits, interlaced: il }
    }

    #[test]
    fn filter_dedup_sort() {
        let v = filter_modes(&[
            raw(1920, 1080, 60, 32, false),
            raw(1920, 1080, 60, 16, false),
            raw(1920, 1080, 60, 32, false),
            raw(3840, 2160, 30, 32, false),
            raw(3840, 2160, 60, 32, false),
            raw(1920, 1080, 120, 32, false),
            raw(1920, 1080, 30, 32, true),
            raw(640, 480, 60, 32, false),
            raw(1280, 720, 1, 32, false),
            raw(1280, 720, 50, 8, false),
        ]);
        let got: Vec<_> = v.iter().map(mode_label).collect();
        assert_eq!(got, ["3840 x 2160 @ 60 Hz", "3840 x 2160 @ 30 Hz", "1920 x 1080 @ 120 Hz", "1920 x 1080 @ 60 Hz"]);
    }

    #[test]
    fn filter_keeps_low_depth_when_only_choice() {
        let v = filter_modes(&[raw(1024, 768, 60, 16, false)]);
        assert_eq!(v.len(), 1);
        assert!(filter_modes(&[]).is_empty());
    }

    #[test]
    fn label_and_allowed() {
        let m = Mode { width: 1920, height: 1080, refresh: 60 };
        assert_eq!(mode_label(&m), "1920 x 1080 @ 60 Hz");
        assert!(mode_allowed(&[m], 1920, 1080, 60));
        assert!(!mode_allowed(&[m], 1920, 1080, 59));
    }

    #[test]
    fn monitor_order_and_pick() {
        let mut v = vec![("b", 1920, 0), ("a", 0, 0), ("c", 1920, -500)];
        sort_by_position(&mut v, |m| (m.1, m.2));
        assert_eq!(v.iter().map(|m| m.0).collect::<Vec<_>>(), ["a", "c", "b"]);
        assert_eq!(pick_index(3, 1), Some(1));
        assert_eq!(pick_index(3, 9), Some(2));
        assert_eq!(pick_index(0, 0), None);
    }

    fn m(w: u32, h: u32, r: u32) -> Mode {
        Mode { width: w, height: h, refresh: r }
    }

    #[test]
    fn revert_confirm() {
        let mut s = RevertState::default();
        let p = s.begin("D1", m(1920, 1080, 60), m(3840, 2160, 30), 1000, 15);
        assert_eq!(s.remaining(1000), Some(15));
        assert_eq!(s.remaining(15_500), Some(1));
        assert_eq!(s.confirm().map(|x| x.next), Some(m(3840, 2160, 30)));
        assert_eq!(s.remaining(2000), None);
        assert!(s.expire(p.id, 99_000).is_none(), "plus rien à annuler après confirmation");
    }

    #[test]
    fn revert_timeout() {
        let mut s = RevertState::default();
        let p = s.begin("D1", m(1920, 1080, 60), m(1280, 720, 60), 0, 15);
        assert!(s.expire(p.id, 14_999).is_none(), "trop tôt");
        let e = s.expire(p.id, 15_000).expect("échéance");
        assert_eq!(e.prev, m(1920, 1080, 60));
        assert!(s.expire(p.id, 20_000).is_none(), "une seule annulation");
    }

    #[test]
    fn revert_chain_keeps_original_and_ignores_stale_timer() {
        let mut s = RevertState::default();
        let a = s.begin("D1", m(1920, 1080, 60), m(1280, 720, 60), 0, 15);
        let b = s.begin("D1", m(1280, 720, 60), m(2560, 1440, 60), 10_000, 15);
        assert_eq!(b.prev, m(1920, 1080, 60), "retour au mode confirmé d'origine");
        assert!(s.expire(a.id, 16_000).is_none(), "ancien minuteur ignoré");
        assert!(s.expire(b.id, 25_000).is_some());
        // Autre écran : ne reprend pas le mode d'origine du premier.
        s.begin("D1", m(1, 1, 1), m(2, 2, 2), 0, 15);
        let c = s.begin("D2", m(800, 600, 60), m(1024, 768, 60), 0, 15);
        assert_eq!(c.prev, m(800, 600, 60));
        assert_eq!(s.revert().map(|x| x.device), Some("D2".to_string()));
        assert!(s.revert().is_none());
    }

    fn out(id: &str, default: bool) -> Output {
        Output { id: id.into(), name: id.into(), default, volume: 0, muted: false }
    }

    #[test]
    fn next_output_cycles() {
        let o = [out("a", false), out("b", true), out("c", false)];
        assert_eq!(next_output(&o, 1), Some(2));
        assert_eq!(next_output(&[out("a", true), out("b", false)], -1), Some(1));
        assert_eq!(next_output(&[out("a", false)], 1), Some(0));
        assert_eq!(next_output(&[], 1), None);
    }

    #[test]
    fn parsing_helpers() {
        assert_eq!(level_to_percent(0.374), 37);
        assert_eq!(level_to_percent(1.7), 100);
        assert_eq!(level_to_percent(-1.0), 0);
        assert_eq!(percent_to_level(250), 1.0);
        assert!((percent_to_level(37) - 0.37).abs() < 1e-6);
        let mut buf = [0u16; 8];
        for (i, c) in "HDMI".encode_utf16().enumerate() {
            buf[i] = c;
        }
        assert_eq!(wide_to_string(&buf), "HDMI");
        assert_eq!(wide_to_string(&[0x41, 0x42]), "AB");
    }
}
