//! HTTP smoke tests for the web dashboard MVP
//! (`claude-web-dashboard-mvp`).
//!
//! Boots a real `argo gateway` child process, hits each dashboard
//! tab + the bundled assets, and asserts the response shape. The
//! crate's gateway lock is held by [`common::boot`] so these tests
//! serialize against other gateway smoke suites.
mod common;

use common::{body_of, boot, http_get, pick_free_port};

#[test]
fn dashboard_tabs_render_html_with_expected_sections() {
    let port = pick_free_port();
    // boot pre-warms with a /sessions call, then we hit 4 tabs +
    // /actions/live + style.css + htmx.min.js — budget 8 requests.
    let (mut child, _stderr) = boot(port, 8);

    let workers = http_get(port, "/dashboard/workers");
    assert!(
        workers.contains("HTTP/1.1 200 OK"),
        "workers tab status: {workers}"
    );
    assert!(
        workers.contains("Content-Type: text/html"),
        "workers tab Content-Type: {workers}"
    );
    let workers_body = body_of(&workers);
    assert!(
        workers_body.contains("<h1>Workers</h1>"),
        "workers body: {workers_body}"
    );
    assert!(
        workers_body.contains("/dashboard/assets/htmx.min.js"),
        "workers links to htmx asset: {workers_body}"
    );
    assert!(
        workers_body.contains("hx-trigger=\"load, every 5s\""),
        "live-actions panel polls every 5s: {workers_body}"
    );

    let sessions = http_get(port, "/dashboard/sessions");
    let sessions_body = body_of(&sessions);
    assert!(
        sessions_body.contains("<h1>Sessions</h1>"),
        "sessions body: {sessions_body}"
    );

    let approvals = http_get(port, "/dashboard/approvals");
    let approvals_body = body_of(&approvals);
    assert!(
        approvals_body.contains("Pending approvals"),
        "approvals body: {approvals_body}"
    );

    let spend = http_get(port, "/dashboard/spend");
    let spend_body = body_of(&spend);
    assert!(
        spend_body.contains("Spend — last 30 days"),
        "spend body: {spend_body}"
    );

    let live = http_get(port, "/dashboard/actions/live");
    assert!(
        live.contains("HTTP/1.1 200 OK"),
        "actions/live status: {live}"
    );
    let live_body = body_of(&live);
    assert!(
        live_body.contains("Live actions"),
        "actions/live body: {live_body}"
    );

    // Tab anchors marked active only on the matching tab — sanity
    // check the layout's per-tab class swap.
    assert!(
        workers_body
            .contains("href=\"/dashboard/workers\" class=\"active\""),
        "workers tab marked active: {workers_body}"
    );
    assert!(
        spend_body.contains("href=\"/dashboard/spend\" class=\"active\""),
        "spend tab marked active: {spend_body}"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn dashboard_assets_are_bundled_no_external_cdn() {
    let port = pick_free_port();
    // boot pre-warms with /sessions; we hit 2 asset URLs.
    let (mut child, _stderr) = boot(port, 3);

    let css = http_get(port, "/dashboard/assets/style.css");
    assert!(css.contains("HTTP/1.1 200 OK"), "css status: {css}");
    assert!(
        css.contains("Content-Type: text/css"),
        "css Content-Type: {css}"
    );
    let css_body = body_of(&css);
    assert!(
        css_body.contains("--accent"),
        "css bundles the design tokens: {css_body}"
    );

    let js = http_get(port, "/dashboard/assets/htmx.min.js");
    assert!(js.contains("HTTP/1.1 200 OK"), "js status: {js}");
    assert!(
        js.contains("Content-Type: application/javascript"),
        "js Content-Type: {js}"
    );
    let js_body = body_of(&js);
    assert!(
        js_body.contains("argo-htmx-shim"),
        "js bundles the htmx shim: {js_body}"
    );
    // Hard rule from the brief: no external CDN dependencies.
    assert!(
        !css_body.contains("https://"),
        "css must not pull from a CDN: {css_body}"
    );
    assert!(
        !js_body.contains("https://"),
        "js must not pull from a CDN: {js_body}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
