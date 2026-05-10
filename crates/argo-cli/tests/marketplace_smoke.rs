//! HTTP smoke tests for the skill marketplace MVP
//! (`claude-skill-marketplace-mvp`).
//!
//! Boots a real `argo gateway` child process, exercises the three
//! marketplace routes (`/marketplace`, `/marketplace/<id>`, and
//! `POST /marketplace/<id>/install`), and asserts the response shape
//! including the tenant-isolation 403.
//!
//! Co-located with the dashboard smoke under `argo-cli/tests/`; the
//! brief mentioned `argo-e2e/tests/`, but that crate does not exist
//! and the rest of the gateway smoke suite is here.
mod common;

use common::{body_of, boot, http_get, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;

/// Raw HTTP POST with arbitrary headers. The shared `http_post`
/// helper only sets `Content-Type: application/json`, so the smoke
/// test rolls its own to inject `X-Tenant-Id` for the install +
/// tenant-mismatch cases.
fn http_post_with_headers(port: u16, path: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut extra = String::new();
    for (k, v) in headers {
        extra.push_str(&format!("{k}: {v}\r\n"));
    }
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn marketplace_browse_detail_install_smoke() {
    let port = pick_free_port();
    // budget: boot's /sessions pre-warm + list + detail + 3 install posts = 6
    let (mut child, _stderr) = boot(port, 8);

    // --- list view ---
    let list = http_get(port, "/marketplace");
    assert!(
        list.contains("HTTP/1.1 200 OK"),
        "list status: {list}"
    );
    assert!(
        list.contains("Content-Type: text/html"),
        "list Content-Type: {list}"
    );
    let list_body = body_of(&list);
    assert!(
        list_body.contains("Skill marketplace"),
        "list body header: {list_body}"
    );
    assert!(
        list_body.contains("marketplace-listing-draft"),
        "list shows seed skill: {list_body}"
    );
    // Search bar + filter dropdowns are rendered.
    assert!(
        list_body.contains("name=\"q\""),
        "list has search input: {list_body}"
    );
    assert!(
        list_body.contains("name=\"vertical\""),
        "list has vertical filter: {list_body}"
    );
    assert!(
        list_body.contains("name=\"executor\""),
        "list has executor filter: {list_body}"
    );

    // --- detail view ---
    let detail = http_get(port, "/marketplace/marketplace-listing-draft");
    assert!(
        detail.contains("HTTP/1.1 200 OK"),
        "detail status: {detail}"
    );
    let detail_body = body_of(&detail);
    assert!(
        detail_body.contains("marketplace-listing-draft"),
        "detail name: {detail_body}"
    );
    // Not-yet-installed: install button should appear (active form).
    assert!(
        detail_body.contains("type=\"submit\""),
        "detail install form: {detail_body}"
    );
    assert!(
        detail_body.contains(">Install<"),
        "detail install button text: {detail_body}"
    );

    // --- install: missing tenant → 401 ---
    let no_auth = http_post_with_headers(
        port,
        "/marketplace/marketplace-listing-draft/install",
        "",
        &[],
    );
    assert!(
        no_auth.contains("HTTP/1.1 401"),
        "install without tenant header is 401: {no_auth}"
    );

    // --- install: tenant mismatch → 403 ---
    let cross_tenant = http_post_with_headers(
        port,
        "/marketplace/marketplace-listing-draft/install?tenant=beta",
        "",
        &[("X-Tenant-Id", "alpha")],
    );
    assert!(
        cross_tenant.contains("HTTP/1.1 403"),
        "tenant-mismatch install is 403: {cross_tenant}"
    );
    assert!(
        body_of(&cross_tenant).contains("tenant mismatch"),
        "403 body explains mismatch: {cross_tenant}"
    );

    // --- install: authorized → 200 + redirect link ---
    let ok = http_post_with_headers(
        port,
        "/marketplace/marketplace-listing-draft/install",
        "",
        &[("X-Tenant-Id", "alpha")],
    );
    assert!(
        ok.contains("HTTP/1.1 200 OK"),
        "happy-path install status: {ok}"
    );
    let ok_body = body_of(&ok);
    assert!(
        ok_body.contains("marketplace-listing-draft"),
        "install body names skill: {ok_body}"
    );
    assert!(
        ok_body.contains("/marketplace/marketplace-listing-draft?installed=1"),
        "install body redirects to detail: {ok_body}"
    );

    child.kill().ok();
    let _ = child.wait();
}

#[test]
fn marketplace_unknown_skill_returns_404() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 4);
    let resp = http_get(port, "/marketplace/does-not-exist");
    assert!(
        resp.contains("HTTP/1.1 404"),
        "unknown skill detail is 404: {resp}"
    );
    let body = body_of(&resp);
    assert!(
        body.contains("No skill named"),
        "404 body explains: {body}"
    );
    child.kill().ok();
    let _ = child.wait();
}
