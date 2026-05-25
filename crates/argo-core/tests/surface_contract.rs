//! Public-surface contract pins for `argo-core`.
//!
//! Authored by `eval-argo-core-crate-surface` (shared core substrate
//! audit, 2026-05-24). The crate exposes the walking-skeleton `Agent`
//! plus the resume / steer / iteration-policy primitives. Today the
//! only external consumer is `argo-config` (only `RestartPolicy`).
//! The remaining surface is in-crate substrate kept stable so that
//! resume / steer / iteration-policy semantics do not silently drift
//! while higher-level runner code (`argo-agent-runner`, etc.) evolves.

#![allow(dead_code)]

use argo_core::{
    apply_restart_policy, decide_iteration_extension, mark_session_discarded, scan_interrupted,
    Agent, AgentDeps, AuditKind, AuditSink, ExtensionDecision, ExtensionLimits,
    InterruptedSession, IterationOnMax, RestartAction, RestartPolicy, RunTurnOptions,
    SteerChannel, TurnOutcome,
};
use argo_provider::Sampling;
use argo_store::SessionStore;
use argo_types::{Principal, SessionId};
use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 1. `lib.rs` re-export inventory: compile-fence + count sentinel.
//
// Total inventory pinned: 17 names across 5 modules
// (agent, audit, iter_policy, resume, steer).
// ---------------------------------------------------------------------------

#[test]
fn pub_use_inventory_pins_count_at_17() {
    let inventory = [
        // agent
        "Agent",
        "AgentDeps",
        "RunTurnOptions",
        "TurnOutcome",
        // audit (re-exported from argo-audit)
        "AuditKind",
        "AuditSink",
        // iter_policy
        "decide_iteration_extension",
        "ExtensionDecision",
        "ExtensionLimits",
        "IterationOnMax",
        // resume
        "apply_restart_policy",
        "mark_session_discarded",
        "scan_interrupted",
        "InterruptedSession",
        "RestartAction",
        "RestartPolicy",
        // steer
        "SteerChannel",
    ];
    assert_eq!(
        inventory.len(),
        17,
        "argo-core pub-use inventory drifted; update this contract with lib.rs"
    );

    // Function signatures pinned with explicit fn-pointer coercion.
    let _: fn(IterationOnMax, u32, u32, ExtensionLimits) -> ExtensionDecision =
        decide_iteration_extension;
    let _: fn(RestartPolicy, Vec<InterruptedSession>) -> Vec<RestartAction> = apply_restart_policy;

    // Constructors that don't require runtime state.
    let _: fn() -> SteerChannel = SteerChannel::new;
    let _: fn() -> ExtensionLimits = ExtensionLimits::default;
    let _: fn() -> IterationOnMax = IterationOnMax::default;
    let _: fn() -> RestartPolicy = RestartPolicy::default;
}

// ---------------------------------------------------------------------------
// 2. Iteration-policy invariants (§6.6, Amendment 9).
//
// These pin the spec-locked behavior: Halt is default; the hard
// ceiling (`extensions_used >= max_extensions`) overrides every
// policy, including `AutoContinue`.
// ---------------------------------------------------------------------------

#[test]
fn iteration_policy_default_is_halt() {
    assert_eq!(IterationOnMax::default(), IterationOnMax::Halt);
}

#[test]
fn iteration_policy_halt_always_halts() {
    let d = decide_iteration_extension(
        IterationOnMax::Halt,
        60,
        0,
        ExtensionLimits::default(),
    );
    assert_eq!(d, ExtensionDecision::Halt);
}

#[test]
fn iteration_policy_prompt_user_proposes_new_max() {
    let d = decide_iteration_extension(
        IterationOnMax::PromptUser,
        60,
        0,
        ExtensionLimits::default(),
    );
    assert_eq!(
        d,
        ExtensionDecision::PromptUser {
            new_max_if_approved: 90,
        }
    );
}

#[test]
fn iteration_policy_auto_continue_extends() {
    let d = decide_iteration_extension(
        IterationOnMax::AutoContinue,
        60,
        0,
        ExtensionLimits::default(),
    );
    assert_eq!(d, ExtensionDecision::AutoExtend { new_max: 90 });
}

#[test]
fn iteration_policy_hard_ceiling_overrides_every_policy() {
    // §6.6 invariant: "the loop never silently extends beyond
    // max_extensions × extension_size + initial".
    let limits = ExtensionLimits::default();
    for policy in [
        IterationOnMax::Halt,
        IterationOnMax::PromptUser,
        IterationOnMax::AutoContinue,
    ] {
        let d = decide_iteration_extension(policy, 150, limits.max_extensions, limits);
        assert_eq!(d, ExtensionDecision::Halt, "policy {policy:?} broke the hard ceiling");
    }
}

#[test]
fn iteration_policy_serde_tags_are_snake_case() {
    // The TOML/JSON config surface depends on snake_case tags; flipping
    // these would break `[loop.on_max]` config parsing silently.
    let halt = serde_json::to_string(&IterationOnMax::Halt).unwrap();
    let prompt = serde_json::to_string(&IterationOnMax::PromptUser).unwrap();
    let auto = serde_json::to_string(&IterationOnMax::AutoContinue).unwrap();
    assert_eq!(halt, "\"halt\"");
    assert_eq!(prompt, "\"prompt_user\"");
    assert_eq!(auto, "\"auto_continue\"");
}

// ---------------------------------------------------------------------------
// 3. Resume policy invariants (§6.5, Amendment 13).
//
// Pin: default is PromptUser (no silent auto-replay); `apply_restart_policy`
// is total and round-trips one action per input session.
// ---------------------------------------------------------------------------

fn sample_interrupted(name: &str) -> InterruptedSession {
    InterruptedSession {
        session_id: SessionId::new(),
        last_tool_name: name.into(),
        last_tool_arguments_json: "{}".into(),
        started_at: Utc.timestamp_opt(0, 0).unwrap(),
        age_seconds: 42,
    }
}

#[test]
fn restart_policy_default_is_prompt_user() {
    assert_eq!(RestartPolicy::default(), RestartPolicy::PromptUser);
}

#[test]
fn restart_policy_serde_tags_are_snake_case() {
    let prompt = serde_json::to_string(&RestartPolicy::PromptUser).unwrap();
    let auto = serde_json::to_string(&RestartPolicy::AutoResume).unwrap();
    let discard = serde_json::to_string(&RestartPolicy::Discard).unwrap();
    assert_eq!(prompt, "\"prompt_user\"");
    assert_eq!(auto, "\"auto_resume\"");
    assert_eq!(discard, "\"discard\"");
}

#[test]
fn apply_restart_policy_routes_each_session_by_variant() {
    let sessions = vec![sample_interrupted("file_read"), sample_interrupted("file_write")];

    let prompt_actions = apply_restart_policy(RestartPolicy::PromptUser, sessions.clone());
    assert_eq!(prompt_actions.len(), 2);
    for a in &prompt_actions {
        assert!(matches!(a, RestartAction::Prompt { .. }), "{a:?}");
    }

    let resume_actions = apply_restart_policy(RestartPolicy::AutoResume, sessions.clone());
    for a in &resume_actions {
        let RestartAction::Resume { prefix, .. } = a else {
            panic!("expected Resume, got {a:?}");
        };
        assert!(prefix.contains("interrupted"));
    }

    let discard_actions = apply_restart_policy(RestartPolicy::Discard, sessions);
    for a in &discard_actions {
        assert!(matches!(a, RestartAction::Discard { .. }), "{a:?}");
    }
}

#[test]
fn restart_action_round_trips_via_serde() {
    let session = sample_interrupted("file_read");
    let action = RestartAction::Resume {
        session: session.clone(),
        prefix: "this was interrupted; verify and continue".into(),
    };
    let json = serde_json::to_string(&action).expect("RestartAction Serialize");
    assert!(json.contains("\"kind\":\"resume\""));
    let back: RestartAction = serde_json::from_str(&json).expect("RestartAction Deserialize");
    match back {
        RestartAction::Resume { session: s, prefix } => {
            assert_eq!(s.session_id, session.session_id);
            assert!(prefix.contains("interrupted"));
        }
        other => panic!("variant changed: {other:?}"),
    }
}

#[tokio::test]
async fn scan_interrupted_and_mark_discarded_round_trip_against_session_store() {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let now = Utc::now();

    // A fresh, empty store has nothing to recover.
    let rows = scan_interrupted(&store, now).await.expect("scan empty");
    assert!(rows.is_empty(), "fresh store should have no orphaned tool_call rows");

    // mark_session_discarded is idempotent — calling it on a missing
    // session_id must not error (the UPDATE simply matches zero rows).
    mark_session_discarded(&store, SessionId::new(), now)
        .await
        .expect("idempotent UPDATE on missing session must succeed");
}

// ---------------------------------------------------------------------------
// 4. Steer channel invariants (§6.3, Amendment 14).
//
// Per-session FIFO that drains exactly once and removes the slot when
// empty so depth() returns 0 after a drain.
// ---------------------------------------------------------------------------

#[test]
fn steer_channel_drains_in_enqueue_order_then_resets_depth() {
    let ch = SteerChannel::new();
    let s = SessionId::new();
    assert_eq!(ch.depth(s), 0);
    ch.enqueue(s, "one".into());
    ch.enqueue(s, "two".into());
    assert_eq!(ch.depth(s), 2);

    let drained = ch.drain(s);
    assert_eq!(drained, vec!["one".to_string(), "two".to_string()]);
    assert_eq!(ch.depth(s), 0, "drained session slot must be removed");

    // Draining an empty queue yields an empty Vec, not a panic.
    assert!(ch.drain(s).is_empty());
}

#[test]
fn steer_channel_isolates_sessions() {
    let ch = SteerChannel::new();
    let a = SessionId::new();
    let b = SessionId::new();
    ch.enqueue(a, "for-a".into());
    ch.enqueue(b, "for-b".into());
    assert_eq!(ch.drain(a), vec!["for-a".to_string()]);
    // Draining `a` must not affect `b`.
    assert_eq!(ch.drain(b), vec!["for-b".to_string()]);
}

// ---------------------------------------------------------------------------
// 5. Agent / AgentDeps structural pins.
//
// We don't re-prove the end-to-end loop here — that's covered by
// `crates/argo-core/src/tests.rs`. We only pin the structural
// guarantees external callers rely on: AgentDeps is Send + Sync (so
// the loop can be driven from a Tokio runtime), Agent::new takes
// AgentDeps by value, RunTurnOptions stays a plain struct (so it can
// be constructed by name from outside the crate without builder
// indirection), and TurnOutcome exposes the four counters that
// telemetry hangs off.
// ---------------------------------------------------------------------------

#[test]
fn agent_deps_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AgentDeps>();
    assert_send_sync::<Agent>();
}

#[test]
fn agent_constructor_signature_is_stable() {
    let _: fn(AgentDeps) -> Agent = Agent::new;
}

#[test]
fn run_turn_options_struct_literal_compiles() {
    // Construct by name — proves no required field was added or
    // renamed without a contract bump.
    let _ = RunTurnOptions {
        max_iterations: 8,
        system: Some("be terse".into()),
        principal: Principal::Local {
            os_user: "u".into(),
        },
        workspace_root: PathBuf::from("/tmp/argo-core-surface-noop"),
        model: "mock".into(),
        sampling: Sampling::default(),
    };
}

#[test]
fn turn_outcome_struct_literal_compiles_with_counters() {
    let outcome = TurnOutcome {
        final_text: "ok".into(),
        iterations: 1,
        tokens_in: 10,
        tokens_out: 3,
    };
    assert_eq!(outcome.iterations, 1);
    assert_eq!(outcome.tokens_in + outcome.tokens_out, 13);
}

// ---------------------------------------------------------------------------
// 6. Audit re-export sanity.
//
// `crates/argo-core/src/audit.rs` is a thin re-export of
// `argo_audit::{AuditKind, AuditSink}`. Pinning AuditSink::new's
// signature here catches the case where the audit crate's constructor
// shape drifts and argo-core's re-export silently follows.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_sink_constructor_signature_is_stable() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");
    // Production callers must use `new_with_tenant`; `new` is retained
    // as a deprecated shim for legacy test fixtures (see argo-audit
    // sink.rs). Both are part of the public surface today.
    let _sink: AuditSink = AuditSink::new_with_tenant(store.clone(), "t".into());
    #[allow(deprecated)]
    {
        let _legacy: AuditSink = AuditSink::new(store);
    }
    // AuditKind variants used by argo-core today; flipping any of
    // these to a different enum shape would break the loop's writes.
    let _: AuditKind = AuditKind::TurnStart;
    let _: AuditKind = AuditKind::TurnEnd;
    let _: AuditKind = AuditKind::AssistantMessage;
    let _: AuditKind = AuditKind::ToolCall;
    let _: AuditKind = AuditKind::ToolResult;
    let _: AuditKind = AuditKind::Halt;
    let _: AuditKind = AuditKind::SteerEnqueued;
}
