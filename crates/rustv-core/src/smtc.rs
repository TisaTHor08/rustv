//! Média en cours de lecture sur le PC, via les contrôles média de Windows (SMTC) : fonctionne avec
//! n'importe quelle appli qui s'y déclare (navigateurs, Spotify, VLC…).
use crate::core::{Core, NowPlaying};
use std::sync::Arc;
use std::time::Duration;
use windows::Media::Control::{GlobalSystemMediaTransportControlsSessionManager, GlobalSystemMediaTransportControlsSessionPlaybackStatus};

fn read() -> Option<NowPlaying> {
    let mgr = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.join().ok()?;
    let s = mgr.GetCurrentSession().ok()?;
    let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let title = props.Title().ok()?.to_string();
    if title.is_empty() {
        return None;
    }
    let artist = props.Artist().map(|a| a.to_string()).unwrap_or_default();
    let playing = s.GetPlaybackInfo().ok().and_then(|i| i.PlaybackStatus().ok()) == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
    let (mut position, mut duration) = (0.0, 0.0);
    if let Ok(t) = s.GetTimelineProperties() {
        // TimeSpan : intervalles de 100 ns
        position = t.Position().map(|p| p.Duration as f64 / 1e7).unwrap_or(0.0);
        duration = t.EndTime().map(|p| p.Duration as f64 / 1e7).unwrap_or(0.0);
    }
    let source = s.SourceAppUserModelId().map(|a| a.to_string()).unwrap_or_default();
    Some(NowPlaying { title, artist, playing, position, duration, source })
}

/// Sonde toutes les 2 s tant qu'un téléphone est connecté.
pub fn run(core: Arc<Core>) {
    let mut last: Option<NowPlaying> = None;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        if core.conn_count() == 0 {
            continue;
        }
        let cur = read();
        let changed = match (&last, &cur) {
            (Some(a), Some(b)) => a.title != b.title || a.artist != b.artist || a.playing != b.playing || (a.position - b.position).abs() >= 5.0,
            (None, None) => false,
            _ => true,
        };
        if changed {
            core.set_smtc(cur.clone());
            last = cur;
        }
    }
}
