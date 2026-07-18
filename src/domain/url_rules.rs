use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::IpNet;
use url::Url;

use super::error::{DomainError, DomainResult};

const MAX_URL_LEN: usize = 2048;

/// Parse and validate a user-supplied crawl URL (syntax + SSRF policy).
/// Network reachability is checked by the crawler adapter.
pub fn parse_crawl_url(raw: &str) -> DomainResult<Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::InvalidUrl("URL is empty".into()));
    }
    if trimmed.len() > MAX_URL_LEN {
        return Err(DomainError::InvalidUrl("URL is too long".into()));
    }

    let url = Url::parse(trimmed).map_err(|e| DomainError::InvalidUrl(e.to_string()))?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(DomainError::InvalidUrl(format!(
                "unsupported scheme `{other}` (use http or https)"
            )));
        }
    }

    let host = url
        .host_str()
        .ok_or_else(|| DomainError::InvalidUrl("URL must include a host".into()))?;

    if host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("localhost.")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
    {
        return Err(DomainError::SsrfBlocked(format!(
            "host `{host}` is not allowed"
        )));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        assert_public_ip(ip)?;
    }

    Ok(url)
}

pub fn assert_public_ip(ip: IpAddr) -> DomainResult<()> {
    if is_blocked_ip(ip) {
        return Err(DomainError::SsrfBlocked(format!(
            "address {ip} is private, loopback, link-local, or otherwise blocked"
        )));
    }
    Ok(())
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

fn is_blocked_v4(ip: Ipv4Addr) -> bool {
    if ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
    {
        return true;
    }

    // CGNAT 100.64.0.0/10
    let octets = ip.octets();
    if octets[0] == 100 && (octets[1] & 0b1100_0000) == 64 {
        return true;
    }

    // Cloud metadata commonly 169.254.169.254 (already link-local) and 0.0.0.0
    // AWS IMDS v2 still link-local.

    false
}

fn is_blocked_v6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return true;
    }
    if ip.is_unique_local() || ip.is_unicast_link_local() {
        return true;
    }
    // IPv4-mapped
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_v4(v4);
    }
    // Documentation 2001:db8::/32
    let segments = ip.segments();
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return true;
    }
    false
}

/// Optional extra nets from config can be merged later; keep helper ready.
pub fn ip_in_nets(ip: IpAddr, nets: &[IpNet]) -> bool {
    nets.iter().any(|n| n.contains(&ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_public_https() {
        let u = parse_crawl_url("https://example.com/path").unwrap();
        assert_eq!(u.host_str(), Some("example.com"));
    }

    #[test]
    fn blocks_localhost() {
        assert!(matches!(
            parse_crawl_url("http://localhost/admin"),
            Err(DomainError::SsrfBlocked(_))
        ));
    }

    #[test]
    fn blocks_private_ip() {
        assert!(matches!(
            parse_crawl_url("http://192.168.1.1/"),
            Err(DomainError::SsrfBlocked(_))
        ));
        assert!(matches!(
            parse_crawl_url("http://10.0.0.5/"),
            Err(DomainError::SsrfBlocked(_))
        ));
    }

    #[test]
    fn blocks_metadata() {
        assert!(matches!(
            parse_crawl_url("http://169.254.169.254/latest/meta-data"),
            Err(DomainError::SsrfBlocked(_))
        ));
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(matches!(
            parse_crawl_url("file:///etc/passwd"),
            Err(DomainError::InvalidUrl(_))
        ));
    }
}
