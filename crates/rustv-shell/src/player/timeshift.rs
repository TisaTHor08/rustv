//! Direct différé (timeshift) : calculs purs à partir des propriétés du cache de mpv.

/// `(secondes disponibles en arrière, retard sur le direct)`.
/// `start` : début de la plage cherchable du cache ; `end` : fin du cache (`demuxer-cache-time`).
pub fn info(pos: f64, start: Option<f64>, end: Option<f64>) -> (f64, f64) {
    let back = start.map(|s| (pos - s).max(0.0)).unwrap_or(0.0);
    let behind = end.map(|e| (e - pos).max(0.0)).unwrap_or(0.0);
    // moins de 2 s de retard : on est « en direct »
    (back, if behind < 2.0 { 0.0 } else { behind })
}

/// Début de la plage cherchable qui contient `pos` (sinon la première).
pub fn seekable_start(ranges: &[(f64, f64)], pos: f64) -> Option<f64> {
    ranges.iter().find(|(s, e)| *s <= pos + 0.5 && pos <= *e + 0.5).or_else(|| ranges.first()).map(|r| r.0)
}

/// Position absolue visée pour un saut relatif de `delta` secondes, bornée au cache.
/// Erreurs stables : `nothing_buffered`, `already_live`.
pub fn seek_target(delta: f64, pos: f64, start: Option<f64>, end: Option<f64>) -> Result<f64, String> {
    if !delta.is_finite() || delta == 0.0 {
        return Err("invalid".into());
    }
    if delta < 0.0 {
        let s = start.ok_or("nothing_buffered")?;
        if pos - s < 1.0 {
            return Err("nothing_buffered".into());
        }
        Ok((pos + delta).max(s))
    } else {
        let e = end.ok_or("already_live")?;
        if e - pos < 2.0 {
            return Err("already_live".into());
        }
        Ok((pos + delta).min(e - 1.0))
    }
}

/// « -2:34 » (ou « LIVE »).
pub fn badge(behind: f64) -> String {
    if behind < 1.0 {
        return "LIVE".into();
    }
    let s = behind.round() as u64;
    if s >= 3600 {
        format!("-{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
    } else {
        format!("-{}:{:02}", s / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_values() {
        assert_eq!(info(100.0, Some(40.0), Some(100.5)), (60.0, 0.0));
        assert_eq!(info(100.0, Some(40.0), Some(160.0)), (60.0, 60.0));
        assert_eq!(info(100.0, None, None), (0.0, 0.0));
    }

    #[test]
    fn clamp() {
        assert_eq!(seek_target(-30.0, 100.0, Some(40.0), Some(160.0)), Ok(70.0));
        assert_eq!(seek_target(-300.0, 100.0, Some(40.0), Some(160.0)), Ok(40.0));
        assert_eq!(seek_target(-10.0, 100.0, Some(100.0), Some(160.0)), Err("nothing_buffered".into()));
        assert_eq!(seek_target(-10.0, 100.0, None, None), Err("nothing_buffered".into()));
        assert_eq!(seek_target(30.0, 100.0, Some(40.0), Some(160.0)), Ok(130.0));
        assert_eq!(seek_target(300.0, 100.0, Some(40.0), Some(160.0)), Ok(159.0));
        assert_eq!(seek_target(30.0, 100.0, Some(40.0), Some(101.0)), Err("already_live".into()));
        assert!(seek_target(0.0, 1.0, None, None).is_err());
    }

    #[test]
    fn ranges_and_badge() {
        assert_eq!(seekable_start(&[(0.0, 10.0), (50.0, 90.0)], 60.0), Some(50.0));
        assert_eq!(seekable_start(&[(5.0, 10.0)], 200.0), Some(5.0));
        assert_eq!(seekable_start(&[], 1.0), None);
        assert_eq!(badge(0.2), "LIVE");
        assert_eq!(badge(154.0), "-2:34");
        assert_eq!(badge(3725.0), "-1:02:05");
    }
}
