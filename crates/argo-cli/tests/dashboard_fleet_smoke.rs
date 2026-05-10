//! HTTP smoke test for the fleet view
//! (`claude-dashboard-fleet-view-html`).
//!
//! Boots a real `argo gateway` child, hits `GET /dashboard/fleet`,
//! and asserts the response shape (200, `text/html`, page chrome
//! present, inline auto-refresh emitted, no external CDN). The
//! gateway's session store starts empty, so the page renders the
//! "no workers" state — exercises the empty-fleet code path.
mod common;

use common::{body_of, boot, http_get, pick_free_port};

#[test]
fn dashboard_fleet_renders_html_with_status_chrome() {
    let port = pick_free_port();
    // boot pre-warms with /sessions; we hit /dashboard/fleet.
    let (mut child, _drain) = boot(port, 2);

    let resp = http_get(port, "/dashboard/fleet");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "fleet status: {resp}"
    );
    assert!(
        resp.contains("Content-Type: text/html; charset=utf-8"),
        "fleet content-type: {resp}"
    );

    let body = body_of(&resp);
    assert!(body.contains("<h1>Fleet</h1>"), "fleet body: {body}");
    assert!(
        body.contains("setTimeout"),
        "auto-refresh present: {body}"
    );
    assert!(
        body.contains("location.reload"),
        "auto-refresh reloads: {body}"
    );
    assert!(
        !body.contains("https://"),
        "no external CDN refs: {body}"
    );
    // Empty-fleet message is the seeded baseline state.
    assert!(
        body.contains("No workers have reported activity yet."),
        "empty fleet rendered cleanly: {body}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
