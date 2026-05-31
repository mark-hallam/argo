//! Tests for the Genesis-invocation branch of
//! `routes_onboarding::first_hire_response`.
//!
//! Split out of `routes_onboarding_tests.rs` to keep both files under
//! the 500-LOC budget. The fixture mirrors the sibling file
//! (`:memory:` sqlite, tempdir `ARGO_HOME`, workspace verticals dir,
//! the deterministic workspace root key from `mvp-credentials-cross-tenant`).

#![allow(clippy::too_many_lines)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use crate::handlers::gateway::routes_genesis_gate::test_overrides;
use super::first_hire_response;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OwnedMutexGuard;

/// Serialize against the GATEWAY-WIDE env lock (not a private mutex): the
/// Genesis hire gate reads `ARGO_VERTICALS_DIR` + the test-only forced-verdict
/// override, both process-global, so first-hire tests must serialize against
/// the genesis-stream + workers-hire test modules too — otherwise a
/// concurrent genesis test could consume a forced verdict this module set.
async fn env_lock() -> OwnedMutexGuard<()> {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    // Defensive: clear any forced verdict a prior test forgot to reset while
    // we hold the exclusive gateway-wide lock.
    test_overrides::clear_forced_verdict();
    guard
}

/// Force the next Genesis hire gate (in `first_hire_response`) to return a
/// clean `Pass`, so a *gate-cleared* hire can be driven end-to-end to assert
/// charter/duty/persona persistence. CI has no live safety-judge provider, and
/// these tests are not testing the gate (the `first_hire_blocks_*` suite is).
/// Held under the gateway-wide lock acquired by [`env_lock`]; one-shot.
fn force_safety_allow() {
    test_overrides::force_pass();
}

/// Force the next Genesis hire gate (in `first_hire_response`) to return a HARD
/// `Blocked` verdict whose eval floor genuinely FAILED (4/10 < 50%) — the
/// in-memory analogue of `routes_genesis_gate_tests::
/// disk_gate_hard_blocks_a_teammate_that_fails_its_evals`. A real first-hire
/// plan is in-memory only and always synthesises a manifest whose own seed
/// keywords appear in its contract, so no classifiable brief can ever score
/// below the floor through this seam — the only way to exercise the
/// failed-eval-floor → HARD-block routing at the first-hire seam is to inject
/// the degenerate verdict. Held under the gateway-wide lock acquired by
/// [`env_lock`]; one-shot.
fn force_eval_floor_block() {
    test_overrides::force_eval_floor_block();
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
    std::env::set_var(
        argo_credentials::WORKSPACE_ROOT_KEY_ENV,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(path: &str, body: &str, principal: Option<&str>) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        head.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
    let body = json!({ "workspace_name": "Acme", "company_name": "Acme Co" }).to_string();
    let r = req(
        "/api/v1/onboarding/provision",
        &body,
        Some(&format!("human:{email}")),
    );
    let (status, body) = provision_response(&r, Some(store)).await;
    assert!(
        status.starts_with("201") || status.starts_with("200"),
        "pre-provision failed: {status} {body}"
    );
}

#[tokio::test]
async fn first_hire_invokes_genesis_and_persists_synthesised_charter() {
    // F5 closure: the first-hire endpoint must call `argo_genesis::plan`
    // and persist a synthesised charter shape — not the pre-Genesis
    // "First hire for this workspace..." preamble. The investor-demo
    // talk-track for Scene 3 depends on this code path actually running.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // Gate-cleared hire: the deterministic eval floor runs for real on the
    // rich brief; force the safety judge to Allow (CI has no live provider)
    // so the hire reaches the persistence path this test asserts.
    force_safety_allow();
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls and qualify leads",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["genesis_invoked"], true, "genesis must run: {resp}");
    assert_eq!(v["genesis_fallback"], false);

    let charter: String = sqlx::query_scalar(
        "SELECT charter_md FROM worker WHERE vertical_id = 'gtm-engineer'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(
        !charter.starts_with("First hire for this workspace"),
        "charter must NOT be the pre-Genesis templated preamble: {charter}",
    );
    assert!(
        charter.contains("synthesised by Genesis"),
        "charter must narrate Genesis synthesis: {charter}",
    );

    // F9 closure: the audit row's endpoint flips to the
    // `genesis_invoked` variant + carries plan_id + charter_hash.
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_invoked%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "expected one genesis_invoked audit row");
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(payload["endpoint"], "onboarding.first_hire.genesis_invoked");
    assert_eq!(payload["genesis_invoked"], true);
    assert_eq!(payload["genesis_fallback"], false);
    assert!(
        payload["plan_id"].as_str().is_some_and(|s| !s.is_empty()),
        "plan_id must be present: {payload}",
    );
    assert!(
        payload["charter_hash"]
            .as_str()
            .is_some_and(|s| s.len() == 64),
        "charter_hash must be a 64-char hex digest: {payload}",
    );
}

#[tokio::test]
async fn first_hire_synthesises_multiple_duties_from_picked_vertical_actions() {
    // F10 closure: the worker must carry duties derived from the picked
    // vertical's autonomous-safe actions, NOT a single hardcoded
    // `daily_check_in`. gtm-engineer declares three low-risk
    // no-approval actions (create_listing_draft, verify_price,
    // request_photos) so the worker should land with at least two
    // matching duties.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    force_safety_allow();
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");

    let entry_actions: Vec<String> = sqlx::query_scalar(
        "SELECT entry_action FROM duty d \
         JOIN worker w ON w.id = d.worker_id \
         WHERE w.vertical_id = 'gtm-engineer' ORDER BY entry_action",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(
        entry_actions.len() >= 2,
        "expected ≥2 duties; got: {entry_actions:?}",
    );
    assert!(
        !entry_actions.iter().any(|a| a == "daily_check_in"),
        "the hardcoded daily_check_in duty must not appear when the \
         vertical declares autonomous-safe actions: {entry_actions:?}",
    );
}

#[tokio::test]
async fn first_hire_persists_persona_bio_from_operator_brief() {
    // Closes UX-AUDIT-T0119MEL F15: pre-fix, `persona.bio` landed NULL
    // and `persona.voice_preamble` was a 35-char templated string.
    // Operator's freeform brief from step 5 of the onboarding wizard
    // is the richest signal we get about who this worker should *be*,
    // so it must propagate to the persona row that the worker-profile
    // "About" section renders.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    force_safety_allow();
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let description = "Mariah, you book outbound demo calls and qualify leads. \
                       Always confirm before drafting a public reply.";
    let body = json!({
        "description": description,
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");

    let (bio, voice_preamble): (Option<String>, String) = sqlx::query_as(
        "SELECT p.bio, p.voice_preamble \
         FROM persona p JOIN worker w ON w.id = p.worker_id \
         WHERE w.vertical_id = 'gtm-engineer'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        bio.as_deref(),
        Some(description),
        "persona.bio must round-trip the operator's freeform brief",
    );
    assert!(
        voice_preamble.starts_with("You are Mariah"),
        "voice_preamble must name the worker, got: {voice_preamble}",
    );
    assert!(
        voice_preamble.contains("book outbound demo calls"),
        "voice_preamble must fold the brief into the system prompt, got: {voice_preamble}",
    );
    assert!(
        !voice_preamble.starts_with("I'm a "),
        "voice_preamble must not be the pre-fix templated string: {voice_preamble}",
    );
}

#[tokio::test]
async fn first_hire_blocks_when_genesis_plan_fails_no_ungated_worker_activated() {
    // P0 bypass closure (commit 3ebd5ef00 follow-up): pre-fix, when the
    // operator's brief didn't classify into any archetype, Genesis returned
    // UnknownArchetype and first-hire SILENTLY activated a non-Genesis
    // templated worker that never cleared the safety gate. DEC-025's invariant
    // ("never hire a teammate that fails its own evals") means a teammate with
    // NO plan has no eval floor to clear, so it must NOT be activated. The
    // seam now fails closed with a 422 `genesis_plan_failed` and persists NO
    // worker row.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        // "Pioneer" has a capital letter so `name_from_description`
        // returns it, but the rest of the brief intentionally avoids
        // every archetype keyword so `classify_charter` returns
        // `UnknownArchetype` and we exercise the (now fail-closed) branch.
        "description": "Pioneer xyzzy plugh blorf nonsense filler",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(
        status, "422 Unprocessable Entity",
        "plan()-fail fallback must fail closed, not activate an ungated worker: {resp}"
    );
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "genesis_plan_failed", "body: {resp}");
    assert_eq!(v["gate"]["hireable"], false);
    assert_eq!(v["gate"]["fallback_reason"], "unknown_archetype");

    // No worker row may exist — the hire was refused, not committed.
    let worker_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(worker_count, 0, "no ungated worker may be persisted on plan()-fail");

    // The fail-closed decision is audited.
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_gate_fallback_blocked%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "expected one fallback-blocked audit row");
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(payload["fallback_reason"], "unknown_archetype");
}

#[tokio::test]
async fn first_hire_blocks_degraded_gate_without_a_live_provider() {
    // The core P0 guarantee at this seam: a Genesis plan that the safety judge
    // cannot clear (no live provider in CI → degraded → needs_review) is NOT
    // hireable. Without forcing the signal, the rich brief clears the
    // deterministic eval floor but the safety judge degrades, so the composed
    // verdict is needs_review and the hire is refused (422) — never a 201, and
    // never a persisted worker.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // NB: deliberately do NOT call force_safety_allow() here.
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls and qualify leads",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(
        status, "422 Unprocessable Entity",
        "degraded gate (no provider) must block the first-hire, not 201: {resp}"
    );
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "genesis_gate_failed", "body: {resp}");
    assert_eq!(v["gate"]["hireable"], false);
    // The eval floor genuinely ran (10 starter seeds) — proves the gate is
    // real, not a blanket refusal.
    assert_eq!(v["gate"]["eval_total"], 10, "real eval floor ran: {resp}");
    assert_eq!(v["gate"]["state"], "needs_review");

    let worker_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(worker_count, 0, "degraded-gate hire must not persist a worker");

    // The gate verdict is audited even on a block.
    let gate_rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_gate%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(!gate_rows.is_empty(), "gate verdict must be audited");
}

#[tokio::test]
async fn first_hire_blocks_a_thin_teammate_that_fails_its_eval_floor() {
    // Required verification (brief): a teammate whose eval floor genuinely
    // FAILS, hired via first-hire, is HARD-`blocked` — never the degraded
    // `needs_review`, and never a 201. A check that *ran* said no, and NO
    // worker row is persisted. This is the core P0 guarantee at the
    // previously-unfenced first-hire seam.
    //
    // The brief classifies to an archetype (so `plan()` succeeds and the seam
    // reaches `run_hire_gate_with_confirmation`), then we inject a degenerate
    // failed-eval-floor verdict (`force_eval_floor_block`). This is the
    // in-memory analogue of `routes_genesis_gate_tests::
    // disk_gate_hard_blocks_a_teammate_that_fails_its_evals` (which truncates
    // the on-disk seed file): the first-hire plan is in-memory only and a
    // classifiable archetype always synthesises a manifest whose own seed
    // keywords appear in its contract, so no real brief can score below the
    // floor through this seam — the failed-floor→HARD-block ROUTING is what
    // this seam test proves (the real-eval-floor hard-block itself is pinned at
    // the gate layer by the disk test). The injected verdict carries the
    // canonical "deterministic eval floor failed" reason `compose` emits, and
    // the response shape under test is the SAME `gate_blocked_response` the SSE
    // + hire-commit fences use.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // The eval floor failed (4/10) → HARD block. Force the degenerate verdict;
    // see the helper doc for why a real first-hire plan can't fail its floor.
    force_eval_floor_block();
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        // Classifies to an archetype so a plan exists and the seam runs the
        // gate (which consumes the forced failed-eval-floor verdict).
        "description": "Mariah, you book outbound demo calls",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(
        status, "422 Unprocessable Entity",
        "a thin teammate that fails its eval floor must be refused: {resp}"
    );
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "genesis_gate_failed");
    assert_eq!(
        v["gate"]["state"], "blocked",
        "eval-floor failure is a HARD block, not needs_review: {resp}"
    );
    assert_eq!(v["gate"]["hireable"], false);
    assert_eq!(
        v["gate"]["eval_total"], 10,
        "the failed-floor verdict reports the 10 starter seeds: {resp}"
    );
    assert!(
        v["gate"]["eval_pass_rate"].as_f64().unwrap() < 0.5,
        "floor must be below the 50% bar: {resp}"
    );
    assert!(
        v["gate"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.as_str().unwrap_or("").contains("eval floor failed")),
        "block reason must cite the eval floor: {resp}"
    );

    let worker_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(worker_count, 0, "blocked teammate must not be persisted");
}
