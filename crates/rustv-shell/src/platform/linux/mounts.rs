//! Linux : volumes montés visibles par l'utilisateur, d'après `/proc/self/mountinfo`.

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mount {
    pub point: String,
    pub fstype: String,
}

/// Décode les séquences octales `\040` (espace), `\011`, `\134`… de mountinfo.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() + 0 && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
            let v = (b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0');
            out.push(v);
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Lignes `ID parent maj:min root mountpoint opts [optionnels] - fstype source superopts`.
pub fn parse_mountinfo(text: &str) -> Vec<Mount> {
    text.lines()
        .filter_map(|l| {
            let (left, right) = l.split_once(" - ")?;
            let point = left.split_whitespace().nth(4)?;
            let fstype = right.split_whitespace().next()?;
            Some(Mount { point: unescape(point), fstype: fstype.to_string() })
        })
        .collect()
}

const PSEUDO: &[&str] = &[
    "proc", "sysfs", "tmpfs", "devtmpfs", "devpts", "cgroup", "cgroup2", "overlay", "squashfs", "autofs", "fusectl", "securityfs", "debugfs", "tracefs", "mqueue",
    "hugetlbfs", "pstore", "bpf", "configfs", "binfmt_misc", "ramfs", "efivarfs",
];

/// Volumes « utilisateur » : sous `/media`, `/run/media` ou `/mnt`, hors systèmes de fichiers virtuels.
pub fn user_mounts(all: &[Mount]) -> Vec<Mount> {
    let mut v: Vec<Mount> = all
        .iter()
        .filter(|m| ["/media/", "/run/media/", "/mnt/"].iter().any(|p| m.point.starts_with(p)))
        .filter(|m| !PSEUDO.contains(&m.fstype.as_str()))
        .cloned()
        .collect();
    v.sort_by(|a, b| a.point.cmp(&b.point));
    v.dedup_by(|a, b| a.point == b.point);
    v
}

/// Nom affiché : dernier composant du point de montage.
pub fn label(point: &str) -> String {
    point.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(point).to_string()
}

pub fn read() -> Vec<Mount> {
    std::fs::read_to_string("/proc/self/mountinfo").map(|t| user_mounts(&parse_mountinfo(&t))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "22 28 0:21 / /proc rw,nosuid shared:13 - proc proc rw\n28 1 8:2 / / rw,relatime shared:1 - ext4 /dev/sda2 rw\n45 28 8:17 / /media/user/USB\\040DISK rw,nosuid shared:30 - vfat /dev/sdb1 rw\n46 28 0:50 / /run/media/user/Backup rw shared:31 - ext4 /dev/sdc1 rw\n47 28 0:51 / /mnt/nas rw - cifs //nas/share rw\n48 28 0:52 / /mnt/ram rw - tmpfs tmpfs rw\n";

    #[test]
    fn parses_and_filters() {
        let all = parse_mountinfo(SAMPLE);
        assert_eq!(all.len(), 6);
        assert_eq!(all[2], Mount { point: "/media/user/USB DISK".into(), fstype: "vfat".into() });
        let u = user_mounts(&all);
        let pts: Vec<&str> = u.iter().map(|m| m.point.as_str()).collect();
        assert_eq!(pts, vec!["/media/user/USB DISK", "/mnt/nas", "/run/media/user/Backup"]);
        assert_eq!(label("/media/user/USB DISK"), "USB DISK");
        assert_eq!(label("/mnt/nas/"), "nas");
    }

    #[test]
    fn unescape_octal() {
        assert_eq!(unescape("a\\040b\\134c"), "a b\\c");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("end\\04"), "end\\04");
    }
}
