//! Linux : espace disque libre (`statvfs`).
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Octets disponibles pour un utilisateur ordinaire sur le volume de `dir` (à défaut de dossier existant, on
/// interroge le premier ancêtre existant).
pub fn free_space(dir: &Path) -> Option<u64> {
    let mut p = dir;
    while !p.exists() {
        p = p.parent()?;
    }
    let c = CString::new(p.as_os_str().as_bytes()).ok()?;
    // SAFETY : `c` est terminé par NUL ; `st` est une structure de sortie valide initialisée à zéro.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
    (rc == 0).then(|| (st.f_bavail as u64).saturating_mul(st.f_frsize as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dir_has_free_space_and_missing_dir_uses_parent() {
        let t = std::env::temp_dir();
        assert!(free_space(&t).is_some_and(|b| b > 0));
        assert!(free_space(&t.join("rustv-does-not-exist").join("sub")).is_some());
    }
}
