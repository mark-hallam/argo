//! Audit-honesty tests for the manual "Run now"
//! (`fix-manual-run-audit-row-kind-status-and-principal`): the row must
//! land as a worker run (not `http.request`), with its REAL outcome (not a
//! `Failed`-default badge), attributed to the SESSION principal (not
//! `anonymous`). Split out of `routes_worker_run_tests.rs` to keep both
//! files under the 500-LOC ceiling; `use super::*` reuses that module's
//! fixtures (`setup_with_worker`, `mint_session`, `run_req_session`,
//! `fetch_run_row`, `canonical_outcome_status`, …).

use super::*;

// ── Defect 1 + 2: kind/title + status honesty ─────────────────────────

/// `canonical_outcome_status` maps both directions to the vocabulary the
/// feed + web readers normalize — `completed` → `succeeded` (so a TRUE
/// run never renders `Failed`), anything else → `failed`.
#[test]
fn canonical_outcome_status_maps_both_directions() {
    assert_eq!(canonical_outcome_status("completed"), "succeeded");
    assert_eq!(canonical_outcome_status("failed"), "failed");
    // Defensive: an unexpected engine word is treated as a failure, not
    // silently surfaced as a success.
    assert_eq!(canonical_outcome_status("weird"), "failed");
}

/// A successful mocked run lands a `worker.run` audit row whose
/// `outcome_status` is the canonical `succeeded` (NOT the engine's raw
/// `completed`), and whose worker-run identity (`verb`, `action_type`)
/// is NOT a system-meta verb — so the flagship row reads as a worker run
/// and is NOT hidden from the default `/audit` feed.
#[tokio::test]
async fn run_audit_row_is_succeeded_and_worker_run_titled_not_meta() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, _principal) = fetch_run_row(&env.store).await;
    assert_eq!(
        payload["outcome_status"], "succeeded",
        "a true run must stamp the canonical succeeded status"
    );
    assert_eq!(payload["action_type"], "worker.run");
    let verb = payload["verb"].as_str().expect("verb present");
    assert_eq!(verb, "ran a manual turn");
    // None of the row's verb-like dimensions may be a metadata verb, or
    // the default feed would meta-filter the flagship row out.
    for dim in ["verb", "action_type", "endpoint"] {
        let v = payload[dim].as_str().unwrap_or_default();
        assert!(
            !argo_audit::is_metadata_verb(v),
            "{dim}={v:?} must not be a metadata verb (would hide the row)"
        );
    }
}

// ── Defect 3: actor attribution from the session, not the header ───────

/// A signed-in session (`__Host-session` cookie) attributes the run to
/// the session principal — both the `principal_id` column and the
/// decision-chain `actor` read the operator's identity, never
/// `anonymous`.
#[tokio::test]
async fn run_with_session_attributes_to_session_principal() {
    let (env, worker) = setup_with_worker("Mariah").await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_JWT_SECRET);
    let jwt = mint_session("operator@example.com", &env.tenant);
    // No `X-Argo-Principal` header at all — the actor must come from the
    // session JWT, not a header.
    let req = run_req_session(&worker.id, Some("owner"), "{}", &jwt, None);
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, principal) = fetch_run_row(&env.store).await;
    assert_eq!(
        principal.as_deref(),
        Some("human:operator@example.com"),
        "principal_id column must carry the session identity"
    );
    assert_eq!(
        payload["decision_chain"][0]["actor"], "human:operator@example.com",
        "decision-chain actor must be the session principal, not anonymous"
    );
}

/// A client-forged `X-Argo-Principal` header can NOT override the
/// server-resolved session identity (mirrors the JWT-first precedence of
/// the tenant resolver). The negative pin: a stray/hostile header is
/// ignored when a valid session is present.
#[tokio::test]
async fn run_forged_principal_header_does_not_override_session() {
    let (env, worker) = setup_with_worker("Mariah").await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_JWT_SECRET);
    let jwt = mint_session("operator@example.com", &env.tenant);
    let req = run_req_session(
        &worker.id,
        Some("owner"),
        "{}",
        &jwt,
        Some("human:attacker@evil"),
    );
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, principal) = fetch_run_row(&env.store).await;
    assert_eq!(
        principal.as_deref(),
        Some("human:operator@example.com"),
        "session JWT must win over a forged X-Argo-Principal header"
    );
    assert_eq!(
        payload["decision_chain"][0]["actor"], "human:operator@example.com",
        "forged header must not appear as the actor"
    );
}

/// No session: the `X-Argo-Principal` header remains the fallback (the
/// header-only path stays attributed, unchanged from before this fix).
#[tokio::test]
async fn run_without_session_falls_back_to_principal_header() {
    let (env, worker) = setup_with_worker("Mariah").await;
    // `run_req` carries `X-Argo-Principal: human:alice@acme` and no cookie.
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, principal) = fetch_run_row(&env.store).await;
    assert_eq!(principal.as_deref(), Some("human:alice@acme"));
    assert_eq!(payload["decision_chain"][0]["actor"], "human:alice@acme");
}

/// A genuinely anonymous call (no session, no header) preserves
/// `anonymous` — the `principal_id` column lands NULL and the
/// decision-chain actor reads `anonymous`.
#[tokio::test]
async fn run_anonymous_preserved_when_no_session_and_no_header() {
    let (env, worker) = setup_with_worker("Mariah").await;
    // Hand-built request with neither a cookie nor an X-Argo-Principal.
    let req = format!(
        "POST /api/v1/workers/{id}/run HTTP/1.1\r\nHost: x\r\n\
         X-Argo-Role: owner\r\nContent-Length: 2\r\n\r\n{{}}",
        id = worker.id
    );
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, principal) = fetch_run_row(&env.store).await;
    assert_eq!(principal, None, "anonymous call → NULL principal_id");
    assert_eq!(payload["decision_chain"][0]["actor"], "anonymous");
}
