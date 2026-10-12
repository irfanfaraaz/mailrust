use std::net::{IpAddr, SocketAddr};

use url::{Host, Url};

/// False for anything that points back inside our own machine or network.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local() // includes 169.254.169.254, the cloud metadata address
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || o[0] == 0
                || o[0] >= 240 // 240.0.0.0/4 reserved
                || (o[0] == 100 && (o[1] & 0xC0) == 64) // 100.64.0.0/10 carrier-grade NAT
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24 protocol assignments
                || (o[0] == 198 && (o[1] & 0xFE) == 18)) // 198.18.0.0/15 benchmarking
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let s = v6.segments();
            let embedded_v4 = |hi: u16, lo: u16| {
                IpAddr::V4(std::net::Ipv4Addr::new(
                    (hi >> 8) as u8,
                    hi as u8,
                    (lo >> 8) as u8,
                    lo as u8,
                ))
            };
            // NAT64 (64:ff9b::/96) and 6to4 (2002::/16) carry an IPv4 address that can be private.
            if s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0] {
                return is_public_ip(embedded_v4(s[6], s[7]));
            }
            if s[0] == 0x2002 {
                return is_public_ip(embedded_v4(s[1], s[2]));
            }
            !(s[..6] == [0, 0, 0, 0, 0, 0] // ::/96: unspecified, loopback, IPv4-compatible
                || v6.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
                || (s[0] & 0xffc0) == 0xfe80 // fe80::/10 link local
                || (s[0] & 0xffc0) == 0xfec0 // fec0::/10 site local (deprecated)
                || (s[0] == 0x2001 && s[1] == 0x0db8)) // 2001:db8::/32 documentation
        }
    }
}

/// Checks the URL and resolves it. Returns the addresses so the caller can pin the connection to them.
pub async fn check_url(url: &str, allow_private: bool) -> Result<(Url, Vec<SocketAddr>), String> {
    let u = Url::parse(url).map_err(|_| "not a valid URL".to_string())?;
    match u.scheme() {
        "https" => {}
        "http" if allow_private => {}
        _ => return Err("webhook URLs must use https".into()),
    }
    let port = u.port_or_known_default().ok_or("URL needs a port")?;
    let addrs: Vec<SocketAddr> = match u.host() {
        Some(Host::Ipv4(ip)) => vec![SocketAddr::new(IpAddr::V4(ip), port)],
        Some(Host::Ipv6(ip)) => vec![SocketAddr::new(IpAddr::V6(ip), port)],
        Some(Host::Domain(d)) => tokio::net::lookup_host((d, port))
            .await
            .map_err(|e| format!("cannot resolve {d}: {e}"))?
            .collect(),
        None => return Err("URL needs a host".into()),
    };
    if addrs.is_empty() {
        return Err("URL host has no addresses".into());
    }
    if !allow_private && addrs.iter().any(|a| !is_public_ip(a.ip())) {
        return Err("webhook address points to a private network".into());
    }
    Ok((u, addrs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ranges_are_not_public() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "0.0.0.0",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "2002:c0a8:101::1",
            "224.0.0.1",
            "ff02::1",
        ] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip} should be blocked");
        }
        for ip in [
            "1.1.1.1",
            "8.8.8.8",
            "2606:4700:4700::1111",
            "64:ff9b::101:101",
        ] {
            assert!(is_public_ip(ip.parse().unwrap()), "{ip} should be allowed");
        }
    }

    #[tokio::test]
    async fn url_rules() {
        assert!(check_url("https://1.1.1.1/hook", false).await.is_ok());
        assert!(check_url("http://1.1.1.1/hook", false).await.is_err());
        assert!(check_url("https://127.0.0.1/hook", false).await.is_err());
        assert!(
            check_url("https://169.254.169.254/latest", false)
                .await
                .is_err()
        );
        assert!(check_url("https://[::1]/hook", false).await.is_err());
        assert!(check_url("ftp://1.1.1.1/x", true).await.is_err());
        assert!(check_url("http://127.0.0.1:9/hook", true).await.is_ok());
    }
}
