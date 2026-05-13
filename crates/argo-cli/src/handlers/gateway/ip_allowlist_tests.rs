//! Unit tests for [`super`] — the gateway-edge CIDR allowlist gate.
//!
//! Eight cases, one per acceptance bullet in
//! `mission-ip-allowlist-deployment-mode.md`:
//!
//! 1. allow — matching IPv4 inside a configured CIDR
//! 2. deny — non-matching IPv4
//! 3. XFF trust — `trust_x_forwarded_for_hops > 0` picks the right
//!    chain entry; peer IP is ignored
//! 4. IPv6 — matching `/64` block + family-mismatch non-match
//! 5. malformed-CIDR rejection at construction time
//! 6. multiple CIDRs — any-match semantics
//! 7. deny event shape — tracing emission carries `ip` + path/method
//!    (asserted via the emitted `Deny` outcome + helper)
//! 8. fail-closed when `enabled = true && cidrs = []`

use super::*;
use argo_config::IpAllowlistConfig;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn cfg(enabled: bool, cidrs: &[&str], hops: u8) -> IpAllowlistConfig {
    IpAllowlistConfig {
        enabled,
        cidrs: cidrs.iter().map(|s| (*s).to_string()).collect(),
        trust_x_forwarded_for_hops: hops,
    }
}

fn peer_v4(s: &str) -> SocketAddr {
    SocketAddr::new(s.parse::<Ipv4Addr>().unwrap().into(), 1234)
}

fn peer_v6(s: &str) -> SocketAddr {
    SocketAddr::new(s.parse::<Ipv6Addr>().unwrap().into(), 1234)
}

#[test]
fn allow_matching_ipv4_in_cidr() {
    let gate = IpAllowlist::from_config(&cfg(true, &["10.0.0.0/8"], 0)).unwrap();
    let result = gate.check("GET / HTTP/1.1\r\n\r\n", peer_v4("10.5.5.5"));
    assert_eq!(result, AllowResult::Allow);
}

#[test]
fn deny_non_matching_ipv4() {
    let gate = IpAllowlist::from_config(&cfg(true, &["10.0.0.0/8"], 0)).unwrap();
    let result = gate.check("GET / HTTP/1.1\r\n\r\n", peer_v4("192.168.1.5"));
    assert_eq!(result, AllowResult::Deny);
}

#[test]
fn xff_trust_picks_rightmost_minus_hops() {
    // Two proxies in front: hops=2 → take the entry 2-from-the-right
    // i.e. the IP injected by the *first* (left-most) proxy, which is
    // the original client. hops=1 would pick the rightmost (the last
    // proxy's claim of who connected to it).
    let gate = IpAllowlist::from_config(&cfg(true, &["10.0.0.0/8"], 2)).unwrap();
    let req = "GET / HTTP/1.1\r\n\
               X-Forwarded-For: 10.1.1.1, 203.0.113.7, 198.51.100.9\r\n\r\n";
    // Peer IP is the last proxy; not in the allowlist, but the gate
    // should ignore it and trust XFF[len-2] = 203.0.113.7. That's NOT
    // in 10.0.0.0/8, so this is a deny — i.e. XFF is honoured, not the
    // permissive peer claim.
    let peer = peer_v4("198.51.100.9");
    assert_eq!(gate.check(req, peer), AllowResult::Deny);
    // Same request, but the XFF chain places a 10.x at the
    // "first untrusted peer" slot (len-2): allow.
    let req2 = "GET / HTTP/1.1\r\n\
                X-Forwarded-For: 198.51.100.9, 10.1.1.1, 203.0.113.7\r\n\r\n";
    assert_eq!(gate.check(req2, peer), AllowResult::Allow);
}

#[test]
fn ipv6_match_plus_family_mismatch() {
    let gate = IpAllowlist::from_config(&cfg(true, &["2001:db8::/32"], 0)).unwrap();
    let ok = gate.check(
        "GET / HTTP/1.1\r\n\r\n",
        peer_v6("2001:db8:1234::1"),
    );
    assert_eq!(ok, AllowResult::Allow);
    // A peer in 2001:db8::/32 is fine; a peer outside it is denied.
    let nope = gate.check(
        "GET / HTTP/1.1\r\n\r\n",
        peer_v6("2001:db9::1"),
    );
    assert_eq!(nope, AllowResult::Deny);
    // Family mismatch: a v4-only allowlist must not match a v6 peer.
    let v4_only = IpAllowlist::from_config(&cfg(true, &["10.0.0.0/8"], 0)).unwrap();
    let result = v4_only.check(
        "GET / HTTP/1.1\r\n\r\n",
        peer_v6("2001:db8::1"),
    );
    assert_eq!(result, AllowResult::Deny);
}

#[test]
fn malformed_cidr_rejected_at_construction() {
    // Missing slash, prefix-too-large, garbage addr — each should
    // fail at gate-construction time rather than silently widening.
    let cases = [
        "10.0.0.0",        // no '/'
        "10.0.0.0/33",     // IPv4 prefix > 32
        "::1/129",         // IPv6 prefix > 128
        "not-an-ip/8",     // bad addr
        "10.0.0.0/abc",    // bad prefix
    ];
    for raw in cases {
        let cfg = IpAllowlistConfig {
            enabled: true,
            cidrs: vec![raw.to_string()],
            trust_x_forwarded_for_hops: 0,
        };
        let r = IpAllowlist::from_config(&cfg);
        assert!(r.is_err(), "expected '{raw}' to reject, got {:?}", r.as_ref().ok());
    }
}

#[test]
fn multiple_cidrs_any_match_wins() {
    let gate = IpAllowlist::from_config(&cfg(
        true,
        &["10.0.0.0/8", "192.168.1.0/24", "2001:db8::/32"],
        0,
    ))
    .unwrap();
    assert_eq!(
        gate.check("GET / HTTP/1.1\r\n\r\n", peer_v4("10.5.5.5")),
        AllowResult::Allow
    );
    assert_eq!(
        gate.check("GET / HTTP/1.1\r\n\r\n", peer_v4("192.168.1.10")),
        AllowResult::Allow
    );
    assert_eq!(
        gate.check(
            "GET / HTTP/1.1\r\n\r\n",
            peer_v6("2001:db8::dead:beef")
        ),
        AllowResult::Allow
    );
    // A peer outside every block is denied — multiple-CIDRs is
    // strictly additive (union), not subtractive.
    assert_eq!(
        gate.check("GET / HTTP/1.1\r\n\r\n", peer_v4("8.8.8.8")),
        AllowResult::Deny
    );
}

#[test]
fn deny_log_helper_carries_method_and_path() {
    // `log_deny` shells out to the tracing layer; here we just
    // assert the request-parsing helper picks the right method/path
    // and strips the query-string (a logged secret would be a leak).
    let req = "POST /sessions?token=secret HTTP/1.1\r\n\
               Authorization: Bearer abc\r\n\r\n";
    let (method, path) = super::request_method_and_path(req);
    assert_eq!(method, "POST");
    assert_eq!(path, "/sessions");
    // The exporter is best-effort — log_deny itself must not panic
    // on malformed input. Empty request → empty fields.
    super::log_deny(IpAddr::from(Ipv4Addr::new(1, 2, 3, 4)), "");
}

#[test]
fn fail_closed_empty_allowlist_when_enabled() {
    let gate = IpAllowlist::from_config(&cfg(true, &[], 0)).unwrap();
    // Loopback, RFC1918, public — all denied. Empty list with
    // gate-on is deny-all by design (see brief Notes section).
    for ip in [
        peer_v4("127.0.0.1"),
        peer_v4("10.0.0.1"),
        peer_v4("8.8.8.8"),
    ] {
        assert_eq!(gate.check("GET / HTTP/1.1\r\n\r\n", ip), AllowResult::Deny);
    }
    // …but `enabled = false` short-circuits to allow regardless.
    let off = IpAllowlist::from_config(&cfg(false, &[], 0)).unwrap();
    assert_eq!(
        off.check("GET / HTTP/1.1\r\n\r\n", peer_v4("8.8.8.8")),
        AllowResult::Allow
    );
}
