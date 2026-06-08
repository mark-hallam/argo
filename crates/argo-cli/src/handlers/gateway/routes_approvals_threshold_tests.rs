//! Tests for N-of-M threshold approvals (Stream
//! `n-of-m-multi-step-approval`).
//!
//! Coverage maps to the brief's acceptance list:
//!   * threshold:2 — one approve HELD, a second distinct approve
//!     crosses + finalizes, with a per-vote audit row for each;
//!   * the SAME approver voting twice is idempotent (UNIQUE) and does
//!     NOT count twice toward the threshold;
//!   * votes are tenant-isolated (tenant A's votes never count for B);
//!   * the documented reject rule (any reject denies).
//!
//! Back-compat (no threshold rule = single-approver settle) is pinned
//! by `ThresholdRule::from_payload` returning `None`, covered by the
//! argo-identity doctest + unit tests; the settle path's single-approver
//! branch is left untouched.

use super::{
    decide_outcome, distinct_approvals, record_threshold_vote, record_vote, ThresholdOutcome,
};
use crate::handlers::worker_actions_transitions::LocatedAction;
use argo_audit::AuditSink;
use argo_identity::auto_approval::ThresholdRule;
use argo_store::SessionStore;
use std::sync::Arc;

use super::super::routes_channels::{verify_token, Decision};

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn rule(threshold: u32, approvers: &[&str]) -> ThresholdRule {
    ThresholdRule {
        threshold,
        approvers: approvers.iter().map(|s| (*s).to_string()).collect(),
        requester: None,
    }
}

fn rule_with_requester(threshold: u32, approvers: &[&str], requester: &str) -> ThresholdRule {
    ThresholdRule {
        threshold,
        approvers: approvers.iter().map(|s| (*s).to_string()).collect(),
        requester: Some(requester.to_string()),
    }
}

/// Mint a 5-field approver-bound token the gateway's `verify_token`
/// accepts, signed with the same HMAC the settle path uses. Mirrors
/// `argo_channels::token::sign_approval_token_for` for the gateway's
/// inline verifier.
fn bound_token(approval_id: &str, decision: &str, approver: &str, secret: &str) -> String {
    let exp = chrono::Utc::now().timestamp() + 300;
    let payload = format!("{approval_id}|{decision}|{exp}|{approver}");
    let sig = super::super::routes_channels::hmac_hex(payload.as_bytes(), secret.as_bytes());
    format!(
        "{approval_id}.{decision}.{exp}.{}.{sig}",
        hex::encode(approver.as_bytes())
    )
}

fn located(action_id: &str) -> LocatedAction {
    let record = serde_json::from_value(serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "job-1",
        "action_type": "send_email",
        "risk_level": "medium",
        "requires_approval": true,
        "state": "pending",
        "payload": {},
        "created_at": "2026-06-08T00:00:00Z",
    }))
    .expect("build ActionRecord");
    LocatedAction {
        record,
        vertical_id: "support".to_string(),
    }
}

#[test]
fn decide_outcome_unauthorized_records_nothing() {
    let r = rule(2, &["a", "b"]);
    assert_eq!(
        decide_outcome(&r, Decision::Approve, false, 5),
        ThresholdOutcome::Unauthorized
    );
}

#[test]
fn decide_outcome_holds_until_threshold_then_finalizes() {
    let r = rule(2, &["a", "b", "c"]);
    // One approval is below the threshold → HELD.
    assert_eq!(
        decide_outcome(&r, Decision::Approve, true, 1),
        ThresholdOutcome::Held {
            distinct: 1,
            threshold: 2,
        }
    );
    // Two distinct approvals reach the threshold → finalize.
    assert_eq!(
        decide_outcome(&r, Decision::Approve, true, 2),
        ThresholdOutcome::FinalizeApprove
    );
}

#[test]
fn decide_outcome_reject_denies_regardless_of_count() {
    let r = rule(3, &["a", "b", "c"]);
    // A reject denies even with zero approvals counted.
    assert_eq!(
        decide_outcome(&r, Decision::Reject, true, 0),
        ThresholdOutcome::FinalizeReject
    );
    // …and even when approvals already met a hypothetical threshold.
    assert_eq!(
        decide_outcome(&r, Decision::Reject, true, 9),
        ThresholdOutcome::FinalizeReject
    );
}

#[tokio::test]
async fn same_approver_voting_twice_is_idempotent() {
    let store = fresh_store().await;
    let pool = store.pool();
    let newly_first = record_vote(pool, "tenant-A", "act-1", "alice", Decision::Approve)
        .await
        .unwrap();
    let newly_second = record_vote(pool, "tenant-A", "act-1", "alice", Decision::Approve)
        .await
        .unwrap();
    assert!(newly_first, "first vote is a new row");
    assert!(!newly_second, "re-vote by same approver is a no-op");
    // One distinct approver, regardless of the double vote.
    assert_eq!(
        distinct_approvals(pool, "tenant-A", "act-1").await.unwrap(),
        1
    );
}

#[tokio::test]
async fn threshold_crosses_on_second_distinct_approver() {
    let store = fresh_store().await;
    let pool = store.pool();
    let r = rule(2, &["alice", "bob", "carol"]);

    record_vote(pool, "tenant-A", "act-1", "alice", Decision::Approve)
        .await
        .unwrap();
    let after_one = distinct_approvals(pool, "tenant-A", "act-1").await.unwrap();
    assert_eq!(after_one, 1);
    assert_eq!(
        decide_outcome(&r, Decision::Approve, true, after_one),
        ThresholdOutcome::Held {
            distinct: 1,
            threshold: 2,
        }
    );

    record_vote(pool, "tenant-A", "act-1", "bob", Decision::Approve)
        .await
        .unwrap();
    let after_two = distinct_approvals(pool, "tenant-A", "act-1").await.unwrap();
    assert_eq!(after_two, 2);
    assert_eq!(
        decide_outcome(&r, Decision::Approve, true, after_two),
        ThresholdOutcome::FinalizeApprove
    );
}

#[tokio::test]
async fn votes_are_tenant_isolated() {
    let store = fresh_store().await;
    let pool = store.pool();
    // Same approval_request_id + approver, two different tenants.
    record_vote(pool, "tenant-A", "act-1", "alice", Decision::Approve)
        .await
        .unwrap();
    record_vote(pool, "tenant-B", "act-1", "alice", Decision::Approve)
        .await
        .unwrap();
    // Each tenant tallies its OWN single vote — neither leaks into the
    // other's count.
    assert_eq!(
        distinct_approvals(pool, "tenant-A", "act-1").await.unwrap(),
        1
    );
    assert_eq!(
        distinct_approvals(pool, "tenant-B", "act-1").await.unwrap(),
        1
    );
    // A tenant with no votes for this approval sees zero.
    assert_eq!(
        distinct_approvals(pool, "tenant-C", "act-1").await.unwrap(),
        0
    );
}

#[tokio::test]
async fn reject_vote_is_not_counted_as_an_approval() {
    let store = fresh_store().await;
    let pool = store.pool();
    record_vote(pool, "tenant-A", "act-1", "alice", Decision::Reject)
        .await
        .unwrap();
    // A reject row exists but the approve tally stays zero.
    assert_eq!(
        distinct_approvals(pool, "tenant-A", "act-1").await.unwrap(),
        0
    );
}

#[tokio::test]
async fn record_threshold_vote_emits_audit_holds_then_finalizes() {
    let store = fresh_store().await;
    let audit = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-A".to_string());
    let loc = located("act-1");
    let r = rule(2, &["alice", "bob", "carol"]);

    // First authorized approve → HELD, one audit row.
    let first = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "alice",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(
        first,
        ThresholdOutcome::Held {
            distinct: 1,
            threshold: 2,
        }
    );

    // Second distinct approver → finalize.
    let second = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "bob",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(second, ThresholdOutcome::FinalizeApprove);

    audit.flush_pending().await.unwrap();
    let votes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'approval.vote_recorded'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(votes, 2, "one audit row per distinct vote");
}

#[tokio::test]
async fn record_threshold_vote_rejects_unauthorized_voter() {
    let store = fresh_store().await;
    let audit = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-A".to_string());
    let loc = located("act-1");
    let r = rule(2, &["alice", "bob"]);

    let outcome = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "mallory",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(outcome, ThresholdOutcome::Unauthorized);
    // No vote was recorded for the unauthorized actor.
    assert_eq!(
        distinct_approvals(store.pool(), "tenant-A", "act-1")
            .await
            .unwrap(),
        0
    );
}

// ── Adversarial proofs (brief
// `n-of-m-bind-approver-identity-and-self-approval-exclusion`) ──────────
// These close the conductor review-HIGH on #2747: the exploit must now
// fail to finalize.

#[tokio::test]
async fn same_token_replayed_across_channels_counts_as_one_approver() {
    // The exact #2747 exploit. One principal POSTs a SINGLE approve-token
    // to two channels. Pre-fix each channel supplied its OWN actor string
    // (push → "push-operator", email → the spoofable From), so two
    // distinct strings satisfied a threshold of 2 — one human forging a
    // quorum. Now the approver is bound INTO the token, so verifying the
    // same token on any channel resolves to the same identity.
    let secret = "test-secret";
    let token = bound_token("act-1", "approve", "alice", secret);

    // Channel A (e.g. push) and channel B (e.g. email) both receive the
    // very same token. `verify_token` derives the approver from the
    // token, NOT from any channel-supplied actor string.
    let claims_a = verify_token(&token, secret).expect("verify channel A");
    let claims_b = verify_token(&token, secret).expect("verify channel B");
    assert_eq!(claims_a.approver.as_deref(), Some("alice"));
    assert_eq!(claims_b.approver.as_deref(), Some("alice"));

    // The settle path records the vote under the token-bound approver.
    let store = fresh_store().await;
    let pool = store.pool();
    record_vote(
        pool,
        "tenant-A",
        "act-1",
        claims_a.approver.as_deref().unwrap(),
        Decision::Approve,
    )
    .await
    .unwrap();
    record_vote(
        pool,
        "tenant-A",
        "act-1",
        claims_b.approver.as_deref().unwrap(),
        Decision::Approve,
    )
    .await
    .unwrap();

    // ONE distinct approver — the threshold of 2 is NOT met, so the
    // action stays HELD. This is the bug fixed.
    let distinct = distinct_approvals(pool, "tenant-A", "act-1").await.unwrap();
    assert_eq!(
        distinct, 1,
        "one principal replaying one token = one approver"
    );
    assert!(
        !rule(2, &["alice", "bob"]).is_met(distinct),
        "threshold=2 must stay HELD on a single-principal cross-channel replay"
    );
}

#[tokio::test]
async fn requester_cannot_self_approve() {
    let store = fresh_store().await;
    let audit = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-A".to_string());
    let loc = located("act-1");
    // alice is a named approver AND the requester → her vote is excluded.
    let r = rule_with_requester(2, &["alice", "bob"], "alice");

    let outcome = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "alice",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(
        outcome,
        ThresholdOutcome::Unauthorized,
        "the requester must not approve their own action"
    );
    assert_eq!(
        distinct_approvals(store.pool(), "tenant-A", "act-1")
            .await
            .unwrap(),
        0,
        "no self-approval vote is recorded"
    );

    // bob (not the requester) votes fine.
    let outcome_bob = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "bob",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(
        outcome_bob,
        ThresholdOutcome::Held {
            distinct: 1,
            threshold: 2,
        }
    );
}

#[tokio::test]
async fn empty_allowlist_authorizes_no_threshold_voter() {
    let store = fresh_store().await;
    let audit = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-A".to_string());
    let loc = located("act-1");
    // No named approvers → the "anyone authenticated" default. For a
    // separation-of-duties control this authorizes NO ONE.
    let r = rule(2, &[]);

    let outcome = record_threshold_vote(
        &audit,
        store.pool(),
        &loc,
        "anyone",
        Decision::Approve,
        "web",
        &r,
    )
    .await
    .unwrap();
    assert_eq!(outcome, ThresholdOutcome::Unauthorized);
    assert_eq!(
        distinct_approvals(store.pool(), "tenant-A", "act-1")
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn distinct_authenticated_approvers_reach_threshold() {
    // Happy path: two DIFFERENT bound tokens (distinct, cryptographically
    // authenticated approvers) reach the quorum.
    let secret = "test-secret";
    let alice = verify_token(&bound_token("act-1", "approve", "alice", secret), secret)
        .expect("verify alice");
    let bob =
        verify_token(&bound_token("act-1", "approve", "bob", secret), secret).expect("verify bob");
    let store = fresh_store().await;
    let pool = store.pool();
    record_vote(
        pool,
        "tenant-A",
        "act-1",
        alice.approver.as_deref().unwrap(),
        Decision::Approve,
    )
    .await
    .unwrap();
    record_vote(
        pool,
        "tenant-A",
        "act-1",
        bob.approver.as_deref().unwrap(),
        Decision::Approve,
    )
    .await
    .unwrap();
    let distinct = distinct_approvals(pool, "tenant-A", "act-1").await.unwrap();
    assert_eq!(distinct, 2);
    assert!(
        rule(2, &["alice", "bob"]).is_met(distinct),
        "two distinct authenticated approvers reach the threshold"
    );
}
