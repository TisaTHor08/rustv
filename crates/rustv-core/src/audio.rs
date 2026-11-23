//! Volume principal de Windows (Core Audio) : lecture, réglage et surveillance, pour synchroniser le téléphone.
use crate::core::Core;
use std::sync::Arc;
use std::time::Duration;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

fn endpoint() -> Option<IAudioEndpointVolume> {
    unsafe {
        // Déjà initialisé sur ce thread : l'erreur est sans conséquence.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let dev = en.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
        dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
    }
}

/// Volume (0 à 100) et muet du périphérique de sortie par défaut.
pub fn get() -> Option<(u8, bool)> {
    let ep = endpoint()?;
    unsafe {
        let level = ep.GetMasterVolumeLevelScalar().ok()?;
        let muted = ep.GetMute().ok()?.as_bool();
        Some(((level * 100.0).round().clamp(0.0, 100.0) as u8, muted))
    }
}

/// Règle le volume (0 à 100) ; lève le muet si on monte le son.
pub fn set(level: u8) {
    if let Some(ep) = endpoint() {
        unsafe {
            let _ = ep.SetMasterVolumeLevelScalar(f32::from(level.min(100)) / 100.0, std::ptr::null());
            if level > 0 {
                let _ = ep.SetMute(false, std::ptr::null());
            }
        }
    }
}

/// Surveille le volume (touches du clavier, mixeur Windows…) et le diffuse aux téléphones quand il change.
pub fn run(core: Arc<Core>) {
    loop {
        if let Some((level, muted)) = get() {
            core.set_volume(level, muted);
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manuel (touche au volume réel) : `cargo test -p rustv-core audio -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn set_and_get_roundtrip() {
        let (orig, muted) = get().expect("pas de périphérique audio");
        set(37);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let (now, _) = get().unwrap();
        set(orig);
        let _ = muted;
        assert!((i16::from(now) - 37).abs() <= 1, "volume lu: {now}");
    }
}
