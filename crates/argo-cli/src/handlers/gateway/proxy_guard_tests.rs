//! Tests for `validate_bind_address` + `extract_real_ip`. Loaded
//! from `proxy_guard.rs` via `#[path = "proxy_guard_tests.rs"]`.
//! Five cases mirror the brief's acceptance criteria — see each
//! `#[test]`'s name.

use super::{extract_real_ip, validate_bind_address, RUNBOOK_PATH};
use argo_config::GatewayBehindProxyConfig;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn default_cfg() -> GatewayBehindProxyConfig {
    GatewayBehindProxyConfig::default()
}

fn proxy_cfg(trusted: &[&str]) -> GatewayBehindProxyConfig {
    let mut cfg = GatewayBehindProxyConfig {
        expected_proxy: true,
        trusted_proxies: Vec::new(),
    };
    for ip in trusted {
        cfg.trusted_proxies
            .push(ip.parse().expect("test ip parses"));
    }
    cfg
}

#[test]
fn loopback_bind_always_accepted() {
    let cfg = default_cfg();
    validate_bind_address("127.0.0.1", &cfg, false).expect("ipv4 loopback");
    validate_bind_address("::1", &cfg, false).expect("ipv6 loopback");
    validate_bind_address("127.0.0.5", &cfg, false).expect("127/8 range");
}

#[test]
fn non_loopback_rejected_without_contract() {
    let cfg = default_cfg();
    let err = validate_bind_address("0.0.0.0", &cfg, false).expect_err("0.0.0.0 must reject");
    let msg = format!("{err}");
    assert!(
        msg.contains(RUNBOOK_PATH),
        "error must reference the runbook, got: {msg}"
    );
    assert!(
        msg.contains("expected_proxy=true") || msg.contains("expected_proxy = true"),
        "error must mention the config opt-in, got: {msg}"
    );
    // Hostname (non-IP) is treated as non-loopback — guards against
    // operators thinking `--bind localhost` is safe (it resolves to
    // loopback on most systems but the parse-then-check path doesn't
    // resolve, so this stays strict).
    let err = validate_bind_address("localhost", &cfg, false).expect_err("hostname must reject");
    assert!(format!("{err}").contains(RUNBOOK_PATH));
}

#[test]
fn non_loopback_accepted_when_proxy_expected() {
    let cfg = proxy_cfg(&["10.0.0.5"]);
    validate_bind_address("0.0.0.0", &cfg, false).expect("expected_proxy=true allows non-loopback");
    validate_bind_address("192.168.1.10", &cfg, false).expect("LAN bind allowed under contract");
    // Explicit --insecure-bind is also an escape hatch, with or
    // without the contract. The two opt-ins are not mutually
    // exclusive — both surface in the existing handler signature.
    let strict = default_cfg();
    validate_bind_address("0.0.0.0", &strict, true).expect("insecure_bind=true escape hatch");
}

#[test]
fn extract_real_ip_ignores_xff_without_contract() {
    let cfg = default_cfg();
    let peer: SocketAddr = "203.0.113.7:51000".parse().expect("peer");
    let req = "GET / HTTP/1.1\r\nHost: x\r\nX-Forwarded-For: 1.2.3.4\r\n\r\n";
    let ip = extract_real_ip(req, peer, &cfg);
    assert_eq!(
        ip,
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        "expected_proxy=false must NOT honor XFF; got {ip}"
    );
}

#[test]
fn extract_real_ip_honors_xff_from_trusted_peers() {
    let proxy = proxy_cfg(&["10.0.0.5"]);
    let req = "GET / HTTP/1.1\r\nHost: x\r\nX-Forwarded-For: 198.51.100.9, 10.0.0.5\r\n\r\n";

    // Trusted LB peer: XFF honored.
    let lb_peer: SocketAddr = "10.0.0.5:51000".parse().expect("lb peer");
    assert_eq!(
        extract_real_ip(req, lb_peer, &proxy),
        IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)),
        "trusted_proxies peer must surface XFF first hop"
    );

    // Loopback peer (same-host caddy/nginx): XFF honored without
    // listing 127.0.0.1 in trusted_proxies (default-allow loopback).
    let loop_peer: SocketAddr = "127.0.0.1:51000".parse().expect("loop peer");
    assert_eq!(
        extract_real_ip(req, loop_peer, &proxy),
        IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)),
        "loopback peer must surface XFF first hop without explicit listing"
    );

    // Untrusted peer (spoofing attempt): XFF dropped, peer IP wins.
    let attacker: SocketAddr = "203.0.113.7:51000".parse().expect("attacker peer");
    assert_eq!(
        extract_real_ip(req, attacker, &proxy),
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        "untrusted peer's XFF must be dropped; we use the TCP peer IP"
    );

    // Trusted peer but no XFF header: peer IP is the source.
    let plain = "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
    assert_eq!(
        extract_real_ip(plain, lb_peer, &proxy),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
        "missing XFF falls through to peer IP"
    );
}
