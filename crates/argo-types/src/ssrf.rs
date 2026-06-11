//! Shared Server-Side Request Forgery (SSRF) address blocklist.
//!
//! [`is_blocked_ip`] is the single source of truth for "an IP the server must
//! never fetch on a caller-supplied URL's behalf". Both the Genesis wiki-URL
//! fetcher (`argo-genesis::safe_fetch`) and the `web_extract` tool guard
//! (`argo-tool`'s web tools) call this one function, so the two paths can never
//! drift to different blocklists — a missed range in one copy would silently
//! re-open the SSRF hole the other copy closed.
//!
//! The classifier is intentionally conservative: when in doubt, block. A
//! false-positive (refusing a legitimate public address) is a recoverable
//! caller error; a false-negative (allowing an internal address) is a security
//! breach.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// `true` when `ip` falls in a range the server must never fetch on a
/// caller-supplied URL's behalf (SSRF defence): loopback, RFC-1918 private,
/// link-local (incl. the `169.254.169.254` cloud-metadata endpoint), the
/// `0.0.0.0/8` "this host" block, broadcast, CGNAT, documentation, reserved /
/// Class-E, and multicast — and their IPv6 equivalents (unique-local,
/// link-local, IPv4-mapped private).
///
/// # Examples
///
/// ```
/// use std::net::IpAddr;
/// use argo_types::is_blocked_ip;
///
/// assert!(is_blocked_ip("169.254.169.254".parse::<IpAddr>().unwrap())); // cloud metadata
/// assert!(is_blocked_ip("127.0.0.1".parse::<IpAddr>().unwrap()));       // loopback
/// assert!(is_blocked_ip("10.0.0.5".parse::<IpAddr>().unwrap()));        // RFC-1918
/// assert!(!is_blocked_ip("8.8.8.8".parse::<IpAddr>().unwrap()));        // public
/// ```
#[must_use]
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

/// Blocked IPv4 ranges: loopback, RFC-1918 private, link-local (incl. the
/// `169.254.169.254` metadata endpoint), the whole `0.0.0.0/8`
/// "this host"/"this network" block, broadcast, CGNAT, documentation,
/// reserved/Class-E (`240.0.0.0/4`), and multicast.
fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    let [a, b, _, _] = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        // 0.0.0.0/8 — "this host"/"this network". `is_unspecified()` only
        // matches 0.0.0.0, but on Linux any 0.x.x.x routes to localhost, so
        // `http://0.1.2.3/` would otherwise reach loopback-bound services
        // (the gateway binds loopback). Block the whole /8.
        || a == 0
        || v4.is_broadcast()
        || v4.is_multicast()
        || v4.is_documentation()
        // 100.64.0.0/10 — Carrier-Grade NAT shared address space.
        || (a == 100 && (64..=127).contains(&b))
        // 192.0.0.0/24 — IETF protocol assignments.
        || (a == 192 && b == 0 && v4.octets()[2] == 0)
        // 240.0.0.0/4 — reserved / Class E (never a legitimate fetch target).
        || a >= 240
}

/// Blocked IPv6 ranges: loopback, unspecified, multicast, unique-local
/// (`fc00::/7`), link-local (`fe80::/10`), and IPv4-mapped addresses whose
/// embedded v4 is itself blocked.
fn is_blocked_v6(v6: Ipv6Addr) -> bool {
    if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
        return true;
    }
    // IPv4-mapped (::ffff:a.b.c.d) — re-check the embedded v4 so a mapped
    // private address can't slip through.
    if let Some(mapped) = v6.to_ipv4_mapped() {
        return is_blocked_v4(mapped);
    }
    let first = v6.segments()[0];
    // fc00::/7 unique-local.
    let unique_local = (first & 0xfe00) == 0xfc00;
    // fe80::/10 link-local.
    let link_local = (first & 0xffc0) == 0xfe80;
    unique_local || link_local
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_ipv4_internal_ranges() {
        for ip in [
            "127.0.0.1",       // loopback
            "10.0.0.1",        // RFC-1918
            "172.16.5.4",      // RFC-1918
            "192.168.1.1",     // RFC-1918
            "169.254.169.254", // link-local cloud metadata endpoint
            "0.0.0.0",         // unspecified
            "0.1.2.3",         // 0.0.0.0/8 — routes to localhost on Linux
            "255.255.255.255", // broadcast
            "100.64.0.1",      // CGNAT
            "240.0.0.1",       // reserved / Class E (240.0.0.0/4)
        ] {
            let parsed: IpAddr = ip.parse().expect("parse v4");
            assert!(is_blocked_ip(parsed), "{ip} must be blocked");
        }
    }

    #[test]
    fn allows_public_ipv4() {
        for ip in ["8.8.8.8", "1.1.1.1", "93.184.216.34"] {
            let parsed: IpAddr = ip.parse().expect("parse v4");
            assert!(!is_blocked_ip(parsed), "{ip} must be allowed");
        }
    }

    #[test]
    fn blocks_ipv6_internal_ranges() {
        for ip in [
            "::1",                    // loopback
            "::",                     // unspecified
            "fc00::1",                // unique-local
            "fe80::1",                // link-local
            "::ffff:10.0.0.1",        // ipv4-mapped private
            "::ffff:169.254.169.254", // ipv4-mapped metadata
        ] {
            let parsed: IpAddr = ip.parse().expect("parse v6");
            assert!(is_blocked_ip(parsed), "{ip} must be blocked");
        }
    }

    #[test]
    fn allows_public_ipv6() {
        let parsed: IpAddr = "2606:4700:4700::1111".parse().expect("parse v6");
        assert!(!is_blocked_ip(parsed), "public v6 must be allowed");
    }
}
