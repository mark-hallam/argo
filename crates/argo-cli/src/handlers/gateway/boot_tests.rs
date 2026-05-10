//! Tests for [`super::check_bind_loopback`]
//! (`mvp-gateway-bind-hardening`). Loopback IPs pass without the
//! flag; non-loopback IPs and unparseable hostnames are rejected
//! unless `--insecure-bind` is set.
//!
//! Loaded via `#[cfg(test)] #[path = "boot_tests.rs"] mod tests;` in
//! `boot.rs`.

use super::check_bind_loopback;

#[test]
fn bind_loopback_accepted() {
    check_bind_loopback("127.0.0.1", false).expect("ipv4 loopback must accept");
    check_bind_loopback("::1", false).expect("ipv6 loopback must accept");
    check_bind_loopback("127.0.0.5", false).expect("127/8 range must accept");
}

#[test]
fn bind_non_loopback_rejected_without_flag() {
    let err = check_bind_loopback("0.0.0.0", false).expect_err("0.0.0.0 must reject");
    let msg = format!("{err}");
    assert!(
        msg.contains("--insecure-bind"),
        "error message must mention --insecure-bind, got: {msg}"
    );

    let err = check_bind_loopback("192.168.1.10", false).expect_err("LAN IP must reject");
    let msg = format!("{err}");
    assert!(msg.contains("--insecure-bind"), "got: {msg}");
}
