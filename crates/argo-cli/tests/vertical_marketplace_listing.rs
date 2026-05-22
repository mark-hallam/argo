//! Gateway integration test for the
//! `mission-vertical-template-marketplace-listing` brief.
//!
//! Mirrors `vertical_introspection_smoke.rs` but exercises the
//! marketplace-listing additions: `/verticals` rows now carry
//! `headline`, `workflow_count`, `action_count`, `role_label`, and
//! `kpi_count`; `/verticals/<id>` carries an additive `marketplace`
//! envelope with `workflows[]`, `actions[]`, and `kpis[]`. These
//! fields back the brand-loud marketplace cards + detail page.

mod common;

use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

fn repo_verticals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join("verticals"))
        .expect("workspace verticals dir resolves")
}

/// Bump the listener-bind deadline so tests don't flake on a loaded
/// machine (the worker swarm runs 5+ cargo workers concurrently —
/// each gateway boot competes for CPU). Honors any operator-set
/// override; only sets a default. Called once via `Once`.
fn ensure_listener_deadline() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if std::env::var_os(common::listener_signal::DEADLINE_ENV).is_none() {
            // SAFETY: invoked once, before any test thread reads env;
            // the rest of the test process never mutates env. This
            // mirrors common dev-loop env-defaulting in other crates.
            unsafe {
                std::env::set_var(common::listener_signal::DEADLINE_ENV, "60000");
            }
        }
    });
}

/// Spawn the gateway pre-configured for this test: bundled verticals
/// dir + `--skip-startup-check` so it boots without a provider key
/// (mirrors `gateway_health_smoke.rs`). Returns the child handle +
/// the stderr drain so the gateway pipe stays open for the test's
/// lifetime.
fn spawn_gateway(
    port: u16,
    home: &Path,
) -> (std::process::Child, common::listener_signal::StderrDrain) {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    (child, drain)
}

/// `/verticals` rows ship the marketplace-listing fields: every row
/// carries `headline`, `role_label`, `workflow_count`, `action_count`,
/// `kpi_count`. The card UI relies on these to render without a
/// second per-vertical fetch.
#[test]
fn gateway_verticals_list_includes_marketplace_card_fields() {
    ensure_listener_deadline();
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _drain) = spawn_gateway(port, home.path());

    let resp = http_get(port, "/verticals");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("body parses");
    let loaded = v["loaded"].as_array().expect("loaded array present");
    assert!(
        !loaded.is_empty(),
        "marketplace must surface at least one vertical: {v}"
    );

    let finance = loaded
        .iter()
        .find(|x| x["id"] == "finance-ar")
        .unwrap_or_else(|| panic!("finance-ar missing from /verticals: {v}"));

    // Existing-shape contract (do not break the 2026-05 consumers).
    assert_eq!(finance["version"], "0.1.0");
    assert!(finance["name"].as_str().is_some());
    assert!(finance["description"].as_str().is_some());

    // Marketplace-listing additions — load-bearing for the cards.
    let headline = finance["headline"]
        .as_str()
        .expect("each row exposes a `headline`");
    assert!(
        !headline.is_empty(),
        "finance-ar carries a non-empty headline: {finance}"
    );
    assert!(
        finance["role_label"].as_str().is_some_and(|s| !s.is_empty()),
        "each row exposes a non-empty `role_label`: {finance}"
    );
    assert!(
        finance["action_count"].as_u64().is_some_and(|n| n > 0),
        "finance-ar declares actions in its manifest: {finance}"
    );
    assert!(
        finance["workflow_count"].as_u64().is_some(),
        "every row exposes `workflow_count` (0 is fine): {finance}"
    );
    assert!(
        finance["kpi_count"].as_u64().is_some(),
        "every row exposes `kpi_count` (0 is fine): {finance}"
    );

    let _ = child.wait();
}

/// `/verticals/<id>` carries the marketplace detail envelope so the
/// detail page can render the "what's in the box" panels (actions,
/// workflows, KPIs) without a second backend round-trip.
#[test]
fn gateway_vertical_show_includes_marketplace_envelope() {
    ensure_listener_deadline();
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _drain) = spawn_gateway(port, home.path());

    let resp = http_get(port, "/verticals/finance-ar");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("body parses");

    // Existing-shape contract.
    assert_eq!(v["id"], "finance-ar");
    assert_eq!(v["name"], "Finance — Accounts Receivable");

    // Additive marketplace envelope — the panels rely on this.
    let market = v
        .get("marketplace")
        .expect("show response carries `marketplace` envelope");
    assert!(
        market["role_label"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "marketplace.role_label is present: {market}"
    );
    let actions = market["actions"]
        .as_array()
        .expect("marketplace.actions is an array");
    assert!(
        !actions.is_empty(),
        "finance-ar declares >=1 action — marketplace.actions must surface them: {market}"
    );
    for action in actions {
        assert!(
            action["name"].as_str().is_some_and(|n| !n.is_empty()),
            "every action has a non-empty name: {action}"
        );
        assert!(
            matches!(action["risk"].as_str(), Some("low" | "medium" | "high")),
            "action risk is one of low|medium|high: {action}"
        );
        assert!(
            action["requires_approval"].as_bool().is_some(),
            "action carries `requires_approval` flag: {action}"
        );
    }

    // Headline is exposed at the top level too for easy consumption.
    assert!(
        v["headline"].as_str().is_some_and(|s| !s.is_empty()),
        "detail body exposes top-level `headline`: {v}"
    );

    let _ = child.wait();
}
