# argo-runtime crate surface audit

Date: 2026-05-24
Brief: `eval-argo-runtime-crate-surface`
Verdict: PASS — live shared runtime substrate, not deprecated.

## What is now pinned

- `crates/argo-runtime/src/lib.rs` public module and re-export inventory is asserted from source.
- Dispatch gating is pinned through the public `HeuristicGate`/`DecomposeGate` contract for empty, explicit multi-step, and long-goal cases.
- Context compression is pinned for thresholding, older-turn summarization, recent-turn preservation, and audit-payload shape.
- Context truncation is pinned for dropping unprotected old turns before protected system/last-assistant/current-user context, plus audit-payload shape.
- Planner behavior is pinned through `TemplatePlanner::shipped_defaults()` for a worker-bound marketing launch DAG.
- Worker resource cage behavior is pinned for deterministic slice paths and disabled-config non-destructive `NotSupported`.

## Production consumer evidence

Static grep shows direct workspace consumers:

- `crates/argo-agent-runner/Cargo.toml:20` depends on `argo-runtime`; `crates/argo-agent-runner/src/turn_planner.rs:18` uses planner/gate/subtask traits, and `crates/argo-agent-runner/src/turn_helpers_context.rs:13-66` uses context compression/truncation in the turn path.
- `crates/argo-cli/Cargo.toml:35` depends on `argo-runtime`; `crates/argo-cli/src/handlers/plan_generate.rs:11-16` uses planner and planner-bias APIs, `crates/argo-cli/src/handlers/worker_actions_dispatch.rs:28` uses action throttling, and `crates/argo-cli/src/handlers/worker_elasticity.rs:14` uses pool elasticity.
- `crates/argo-orchestrator/Cargo.toml:14` depends on `argo-runtime`; `crates/argo-orchestrator/src/worker_supervisor.rs:62-66` wires OOM restart and worker resource cage into the production supervisor.
- `crates/argo-store/Cargo.toml:12` depends on `argo-runtime`; `crates/argo-store/src/subtask.rs:273-319` implements `SubtaskSink`, and `crates/argo-store/src/session_store.rs:14` stores `PromptCacheAffinity`.

## Gaps

No new follow-up brief is recommended. The audit resolves the prior "relationship to argo-agent-runner unclear" note: `argo-runtime` is live substrate consumed by the turn loop, CLI operator surfaces, orchestrator supervisor, and store adapters. Some modules remain pure-policy seams by design, but they are wired through those consumers rather than being a deprecated wrapper.

## Verification

- `bash scripts/check-brief-freshness.sh eval-argo-runtime-crate-surface` (authored-SHA distance warning only)
- `cargo fmt -p argo-runtime`
- `CARGO_TARGET_DIR=/tmp/eval-argo-runtime-crate-surface-target cargo test -p argo-runtime --tests`
