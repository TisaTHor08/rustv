//! Linux : gestionnaire de paquets de la distribution (d'après `/etc/os-release`) pour indiquer
//! comment installer mpv, Hyperion… sans jamais télécharger un binaire.

/// Valeur d'une clé `KEY=value` / `KEY="value"` d'os-release.
fn value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')).map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
}

/// Préfixe de commande d'installation (sans le nom du paquet).
pub fn install_prefix(os_release: &str) -> &'static str {
    let mut ids: Vec<String> = Vec::new();
    ids.extend(value(os_release, "ID"));
    ids.extend(value(os_release, "ID_LIKE").map(|v| v.split_whitespace().map(str::to_string).collect::<Vec<_>>()).unwrap_or_default());
    for id in &ids {
        match id.as_str() {
            "debian" | "ubuntu" | "linuxmint" | "pop" | "raspbian" | "elementary" => return "sudo apt install",
            "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => return "sudo dnf install",
            "arch" | "manjaro" | "endeavouros" | "cachyos" => return "sudo pacman -S",
            "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" | "sles" => return "sudo zypper install",
            "alpine" => return "sudo apk add",
            "void" => return "sudo xbps-install",
            "gentoo" => return "sudo emerge",
            _ => {}
        }
    }
    "your package manager: install"
}

/// Commande complète pour `package` sur cette machine.
pub fn install_command(os_release: &str, package: &str) -> String {
    format!("{} {package}", install_prefix(os_release))
}

/// Message d'erreur stable et court renvoyé par `player::install` et `hyperion::install` sous Linux.
pub fn use_package_manager(package: &str) -> String {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    format!("use_package_manager:{}", install_command(&text, package))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_distributions() {
        assert_eq!(install_command("ID=ubuntu\nID_LIKE=debian\n", "mpv"), "sudo apt install mpv");
        assert_eq!(install_command("NAME=\"Linux Mint\"\nID=linuxmint\nID_LIKE=\"ubuntu debian\"\n", "mpv"), "sudo apt install mpv");
        assert_eq!(install_command("ID=fedora\n", "mpv"), "sudo dnf install mpv");
        assert_eq!(install_command("ID=\"arch\"\n", "mpv"), "sudo pacman -S mpv");
        assert_eq!(install_command("ID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n", "mpv"), "sudo zypper install mpv");
        assert_eq!(install_command("ID=garuda\nID_LIKE=arch\n", "mpv"), "sudo pacman -S mpv");
        assert_eq!(install_command("", "mpv"), "your package manager: install mpv");
    }
}
