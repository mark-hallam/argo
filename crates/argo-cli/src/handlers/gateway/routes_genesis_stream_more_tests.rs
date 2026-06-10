//! Genesis SSE collision / overwrite / hire-gate regression tests,
//! split out of `routes_genesis_stream_tests.rs` to keep that file
//! under the 500-LOC budget. Loaded as a child mod via `#[path]` from
//! the parent test file; the loopback harness helpers (`drive_handler`,
//! `build_request`, `mem_store`, `EnvGuard`) are reused from there.

use super::super::PROVIDER_KEY_ENV;
use super::{build_request, drive_handler, mem_store, EnvGuard};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn second_attempt_gets_its_own_unique_vertical_no_collision() {
    // NON-DETERMINISM ROBUSTNESS (`fix-genesis-nondeterminism-robustness`):
    // each generation now stamps a UNIQUE suffix on its `vertical_id`, so a
    // second attempt for the SAME tenant+role writes a NEW
    // `.genesis/<tenant>/<vertical_id>/` directory instead of colliding with
    // the first (the old `OutputDirNotEmpty` SSE error) or clobbering it. Both
    // attempts succeed and land in distinct dirs, leaving a manifest the
    // operator may already be reviewing untouched.
    //
    // This SUPERSEDES the prior `second_attempt_without_overwrite_emits_error_event`
    // assertion: the collision it locked in was the very mechanism behind the
    // regeneration-clobber 422, so the corrected contract is "no collision".
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();
    let tenant = "33333333-3333-3333-3333-333333333333";
    let raw1 = drive_handler(build_request(&body, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw1.starts_with("HTTP/1.1 200 OK"));
    assert!(raw1.contains("event: vertical_ready"));
    let id1 = parse_vertical_ready(&raw1)["vertical_id"]
        .as_str()
        .expect("vertical_id 1")
        .to_string();

    let raw2 = drive_handler(build_request(&body, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"), "still streams; got: {raw2}");
    assert!(
        raw2.contains("event: vertical_ready"),
        "second attempt must SUCCEED with its own unique id; got: {raw2}"
    );
    assert!(
        !raw2.contains("event: error"),
        "second attempt must NOT collide / emit an error now; got: {raw2}"
    );
    let id2 = parse_vertical_ready(&raw2)["vertical_id"]
        .as_str()
        .expect("vertical_id 2")
        .to_string();
    assert_ne!(id1, id2, "the two generations must get distinct unique ids");
    // Both share the same stable base (`<archetype>-<persona>`) — only the
    // per-generation suffix differs — so the persona is still legible in the id.
    let base = "marketing-intern-maya";
    assert!(id1.starts_with(base), "id1 keeps the role base: {id1}");
    assert!(id2.starts_with(base), "id2 keeps the role base: {id2}");
}

#[tokio::test]
async fn overwrite_true_lets_second_attempt_succeed() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let first = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();
    let retry = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
        "overwrite": true,
    })
    .to_string();
    let tenant = "44444444-4444-4444-4444-444444444444";
    let raw1 = drive_handler(build_request(&first, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw1.contains("event: vertical_ready"));
    let raw2 = drive_handler(build_request(&retry, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"));
    assert!(raw2.contains("event: vertical_ready"), "expected success on retry; got: {raw2}");
    assert!(!raw2.contains("event: error"), "must not emit error event; got: {raw2}");
}

/// Extract the JSON object on the `data:` line of the `vertical_ready` SSE
/// event from a raw chunked response. Returns the parsed payload the SPA reads
/// (carrying `vertical_id` + `contract_digest`).
fn parse_vertical_ready(raw: &str) -> serde_json::Value {
    let ready_idx = raw.find("event: vertical_ready").expect("vertical_ready event present");
    let after = &raw[ready_idx..];
    let data_line = after
        .lines()
        .find(|l| l.starts_with("data: "))
        .expect("data: line after vertical_ready");
    let json = data_line.trim_start_matches("data: ");
    serde_json::from_str(json).expect("vertical_ready data parses as JSON")
}

/// THE REGRESSION TEST for the tenant-segment mismatch (the proven
/// demo-blocking Genesis bug).
///
/// PROVES write-segment == hire-read-segment: a Genesis vertical the SSE writes
/// under a fresh/unique `X-Argo-Tenant-Id` HEADER (the exact shape the SPA
/// sends) is found by the hire gate in the SAME identity-resolved segment, so
/// the operator's confirmed digest matches and the hire clears the contract
/// axis instead of flipping 201 → 422.
///
/// Identity-first (pt2): both the SSE write tenant
/// (`routes_genesis_stream_tenant::resolve_write_tenant`) and the hire read
/// tenant (`routes_workers_hire::resolve_tenant_principal`) resolve
/// IDENTITY-FIRST via the shared `resolve_tenant::resolve_tenant_for_request`
/// — session-JWT tenant, then explicit `X-Argo-Tenant-Id` override, then cwd.
/// With a header present and NO JWT, that resolved tenant is the HEADER tenant,
/// so:
///
///   1. the vertical is written under the **header tenant** segment, and
///   2. the hire-gate resolver (`genesis_vertical_dir_for_tenant`) FINDS it in
///      that SAME header segment, and
///   3. when the hire confirms with the EXACT `contract_digest` the SSE
///      surfaced, the re-derived contract MATCHES (`contract_confirmation ==
///      "confirmed"`, NOT `"mismatch"`) — precisely the axis that flipped
///      201 → 422 before the fix.
///
/// (This SUPERSEDES the v1 single-tenant-per-cwd (DEC-025 #9) form of this
/// test, which asserted the cwd segment; under identity-first an explicit
/// header drives the segment for BOTH write and read, so write == read still
/// holds — just on the header tenant, not the cwd one.)
///
/// (The composed gate may still hold at `needs_review` in CI because the
/// safety judge has no live provider and degrades fail-safe — that is the
/// SEPARATE, correct safety behaviour, not the tenant bug. This test asserts
/// the contract axis confirms + the segment matches, which is exactly what the
/// fix changes; it does not assert a full `pass`, which would require a live
/// provider.)
#[tokio::test]
async fn sse_written_vertical_is_resolved_by_hire_gate_in_same_identity_segment() {
    let tmp = TempDir::new().unwrap();
    // Force the safety judge to degrade deterministically (no live provider)
    // so the test is provider-independent; the contract axis we assert on is
    // computed from disk, not the provider.
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
        ("ARGO_GENESIS_LLM_PROVIDER", "definitely-not-a-real-provider-xyz"),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500",
        "persona": "Casey",
    })
    .to_string();

    // A FRESH session header + NO JWT. This is the SPA-shaped request; under
    // identity-first the explicit header IS the resolved tenant (write == read).
    let fresh_header = "deadbeef-0000-4000-8000-feedfacecafe";
    let raw = drive_handler(build_request(&body, Some(fresh_header)), Arc::clone(&store)).await;
    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");
    let payload = parse_vertical_ready(&raw);
    let vertical_id = payload["vertical_id"].as_str().expect("vertical_id present");
    let sse_digest = payload["contract_digest"]
        .as_str()
        .expect("contract_digest present")
        .to_string();
    assert!(!sse_digest.is_empty(), "SSE must surface a non-empty contract digest");

    // The hire read tenant is the SAME identity-resolved tenant the SSE wrote
    // under — the header tenant, since there is no JWT. The hire endpoint
    // resolves it via `resolve_tenant_for_request`, which returns the explicit
    // `X-Argo-Tenant-Id` override on the no-session path.
    let hire_read_tenant = fresh_header;

    // (1) + (2): the hire-gate resolver finds the SSE-written vertical under
    // the identity (header) tenant segment — the SAME
    // `genesis_vertical_dir_for_tenant` call the hire fence uses. Pre-fix the
    // SSE wrote under the header slug while the hire searched the cwd slug → 422.
    let genesis_dir = super::super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
        tmp.path(),
        hire_read_tenant,
        vertical_id,
    )
    .expect(
        "hire gate MUST resolve the SSE-written vertical under the identity \
         (header) tenant segment (write-segment == hire-read-segment)",
    );
    assert!(genesis_dir.join("vertical.toml").is_file());

    // It must NOT be discoverable under the cwd segment — the explicit header
    // won the identity-first resolution for BOTH the write and the read, so the
    // cwd tenant never receives this vertical.
    let cwd = std::env::current_dir().unwrap();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (cwd_tenant, _ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .unwrap();
    assert!(
        super::super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
            tmp.path(),
            &cwd_tenant.to_string(),
            vertical_id,
        )
        .is_none(),
        "vertical must NOT be written under the cwd segment when a header drives identity"
    );

    // (3): the hire fence re-runs the gate against that SAME on-disk dir and
    // confirms with the digest the SSE surfaced. The re-derived contract MUST
    // match — `contract_confirmation == "confirmed"`, NOT `"mismatch"` — which
    // is the axis that flipped 201 → 422 before the fix.
    let gate = super::super::super::routes_genesis_gate::run_hire_gate_from_dir_with_confirmation(
        &genesis_dir,
        Some(&sse_digest),
    )
    .await;
    assert_eq!(
        gate.contract_confirmation, "confirmed",
        "the operator's confirmation of the SSE-surfaced digest must MATCH the \
         re-derived contract (write-segment == read-segment); got reasons: {:?}",
        gate.reasons
    );
    assert_eq!(
        gate.contract_digest, sse_digest,
        "the hire-gate re-derives the IDENTICAL contract digest the SSE surfaced"
    );
    // And it is NOT a hard contract mismatch block — the failure mode the bug
    // produced. (A degraded safety judge may still hold the composed state at
    // needs_review in CI; that is the separate, correct safety behaviour.)
    assert!(
        !gate.reasons.iter().any(|r| r.contains("role-contract confirmation mismatch")),
        "must not be a contract-mismatch block; got reasons: {:?}",
        gate.reasons
    );
}

/// THE REGRESSION TEST for the Genesis non-determinism robustness gap.
///
/// The LLM classifier+synthesizer is non-deterministic, so two `/genesis`
/// calls for the SAME role can produce different `contract_digest`s. The hire
/// re-derives the contract from the on-disk manifest and confirms the
/// operator's echoed digest against it, so a SINGLE clean flow (generate →
/// review → hire) works. The bug: the per-role `vertical_id`
/// (`<archetype-slug>-<persona>`) was STABLE, so a REGENERATION for the same
/// role between the operator's review and the hire (or two flows for the same
/// role) CLOBBERED the reviewed manifest on disk — the operator's confirmed
/// digest then no longer matched the (now-second) manifest → a spurious
/// `422 genesis_gate_failed` for a contract the operator genuinely reviewed.
///
/// This proves the fix: each generation gets a UNIQUE `vertical_id`, so:
///
///   1. Two generations for the same role yield DISTINCT `vertical_id`s →
///      DISTINCT on-disk dirs (no clobber), and BOTH succeed (the second is no
///      longer an `OutputDirNotEmpty` collision).
///   2. The FIRST generation's `(vertical_id, contract_digest)` — the one the
///      operator reviewed — is STILL resolvable on disk after the second
///      generation, and its confirmation STILL passes the gate's contract axis
///      (`contract_confirmation == "confirmed"`), i.e. it was NOT invalidated
///      by the regeneration.
///
/// (The composed gate may still hold at `needs_review` in CI because the
/// safety judge has no live provider and degrades fail-safe — the separate,
/// correct safety behaviour. This asserts the contract axis, which is the one
/// the clobber broke.)
#[tokio::test]
async fn regeneration_does_not_invalidate_the_first_reviewed_contract() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
        ("ARGO_GENESIS_LLM_PROVIDER", "definitely-not-a-real-provider-xyz"),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500",
        "persona": "Casey",
    })
    .to_string();

    // The cwd-derived workspace tenant the hire-commit endpoint resolves
    // (`routes_workers_hire::resolve_tenant_principal`) — the read segment.
    let cwd = std::env::current_dir().unwrap();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (cwd_tenant, _ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .unwrap();
    let cwd_tenant_str = cwd_tenant.to_string();

    // FIRST generation — the one the operator "reviews". Capture its EXACT
    // (vertical_id, contract_digest) — what the SPA pins from the SSE event.
    let raw1 = drive_handler(build_request(&body, None), Arc::clone(&store)).await;
    assert!(raw1.starts_with("HTTP/1.1 200 OK"), "first gen failed: {raw1}");
    let p1 = parse_vertical_ready(&raw1);
    let id1 = p1["vertical_id"].as_str().expect("vertical_id 1").to_string();
    let digest1 = p1["contract_digest"].as_str().expect("digest 1").to_string();
    assert!(!digest1.is_empty(), "first gen must surface a non-empty digest");

    // SECOND generation for the SAME role (a "regenerate" / "try again" between
    // review and hire), WITHOUT overwrite. Pre-fix this either clobbered the
    // first manifest (stable id + overwrite) or collided (`is not empty`). Now
    // it gets its OWN unique id, so it succeeds AND leaves the first untouched.
    let raw2 = drive_handler(build_request(&body, None), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"), "second gen failed: {raw2}");
    assert!(
        raw2.contains("event: vertical_ready"),
        "regeneration must SUCCEED (own unique id, no clobber/collision); got: {raw2}"
    );
    assert!(
        !raw2.contains("event: error"),
        "regeneration must NOT emit an OutputDirNotEmpty error now; got: {raw2}"
    );
    let p2 = parse_vertical_ready(&raw2);
    let id2 = p2["vertical_id"].as_str().expect("vertical_id 2").to_string();

    // (1) The two generations got DISTINCT ids → distinct on-disk dirs.
    assert_ne!(
        id1, id2,
        "each generation must get a UNIQUE vertical_id so a regen never clobbers \
         the reviewed manifest"
    );

    // (2) The FIRST reviewed (id, digest) is STILL resolvable after the second
    // generation — the regeneration did NOT clobber it.
    let dir1 = super::super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
        tmp.path(),
        &cwd_tenant_str,
        &id1,
    )
    .expect(
        "the FIRST reviewed vertical must STILL resolve on disk after a \
         regeneration for the same role (no clobber)",
    );
    assert!(dir1.join("vertical.toml").is_file(), "first manifest still on disk");
    // The second one resolves to its OWN distinct dir.
    let dir2 = super::super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
        tmp.path(),
        &cwd_tenant_str,
        &id2,
    )
    .expect("the second vertical resolves to its own dir");
    assert_ne!(dir1, dir2, "the two generations live in distinct dirs");

    // (3) The operator's confirmation of the FIRST contract STILL passes the
    // gate's contract axis — the regeneration did NOT invalidate it. This is
    // the exact axis the clobber broke (confirmed → mismatch → 422).
    let gate = super::super::super::routes_genesis_gate::run_hire_gate_from_dir_with_confirmation(
        &dir1,
        Some(&digest1),
    )
    .await;
    assert_eq!(
        gate.contract_confirmation, "confirmed",
        "the FIRST reviewed contract's confirmation must STILL MATCH after a \
         regeneration; got reasons: {:?}",
        gate.reasons
    );
    assert_eq!(
        gate.contract_digest, digest1,
        "the hire-gate re-derives the IDENTICAL digest the FIRST gen surfaced"
    );
    assert!(
        !gate.reasons.iter().any(|r| r.contains("role-contract confirmation mismatch")),
        "the reviewed contract must NOT become a mismatch after a regen; got: {:?}",
        gate.reasons
    );

    // The digest gate is NOT weakened: a genuinely WRONG/forged digest against
    // that same dir still fails the contract axis (mismatch), so robustness did
    // not become a bypass.
    let forged = super::super::super::routes_genesis_gate::run_hire_gate_from_dir_with_confirmation(
        &dir1,
        Some("0000000000000000000000000000000000000000000000000000000000000000"),
    )
    .await;
    assert_ne!(
        forged.contract_confirmation, "confirmed",
        "a forged/stale digest must still FAIL the contract axis — the gate is \
         not weakened by the unique-id fix"
    );
}

#[tokio::test]
async fn over_cap_boundaries_spec_rejected_with_400_at_gateway() {
    // WIRING tripwire: the `validate_input_bounds()` guard must reject an
    // over-cap boundaries spec with 400 at the gateway, BEFORE synthesis. The
    // unit tests in `policy_synth_bounds` cover the validator in isolation, so
    // a revert of the gateway guard call would leave THOSE green — this test is
    // what fails on such a revert.
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let topics: Vec<String> = (0..70).map(|i| i.to_string()).collect();
    let body = serde_json::json!({
        "role_description": "A clearly long enough role description for the gate.",
        "boundaries": { "escalation_topics": topics },
    })
    .to_string();
    let raw = drive_handler(build_request(&body, Some("tenant-x")), Arc::clone(&store)).await;
    assert!(raw.starts_with("HTTP/1.1 400 Bad Request"), "got: {raw}");
    assert!(raw.contains("bad_request"));
}
