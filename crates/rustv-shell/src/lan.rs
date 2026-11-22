//! Détection des hôtes de réseau local (utilisée pour l'option `allow_insecure_lan` des tuiles).
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn private_v4(ip: Ipv4Addr) -> bool {
    ip.is_private() || ip.is_loopback() || ip.is_link_local()
}

fn private_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    ip.is_loopback() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80
}

/// Vrai pour une IP privée (RFC 1918, lien local, loopback, ULA IPv6) ou un nom `.local` / sans point.
/// Un nom DNS public (ex. `xxx.ugnas.com`) n'est jamais considéré comme local.
pub fn is_private_host(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
    if let Ok(ip) = h.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v) => private_v4(v),
            IpAddr::V6(v) => private_v6(v),
        };
    }
    h.ends_with(".local") || (!h.is_empty() && !h.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_hosts() {
        for h in ["192.168.1.20", "10.0.0.5", "172.16.3.4", "169.254.1.1", "127.0.0.1", "nas.local", "ugnas", "[fd12::1]", "fe80::1"] {
            assert!(is_private_host(h), "{h}");
        }
    }

    #[test]
    fn public_hosts() {
        for h in ["8.8.8.8", "172.32.0.1", "example.com", "nas.ugnas.com", "2001:db8::1", "", "evil.local.example.com"] {
            assert!(!is_private_host(h), "{h}");
        }
    }
}
