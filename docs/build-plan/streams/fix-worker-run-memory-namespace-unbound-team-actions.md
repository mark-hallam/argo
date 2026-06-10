# fix-worker-run-memory-namespace-unbound-team-actions

**Owner**: claude or codex.
**Depends on**: none (relates to `codex-memory-tool-namespace-wiring` #1737, already merged).
**Authored against**: origin/main `6e8149f19` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-worker-run-memory-namespace-unbound-team-actions

## Goal

A hired teammate's live run must **bind its per-worker memory namespace** so the
`memory` tool's team / `query_scored` actions work (or degrade gracefully)
instead of raising a hard tool error that leaks into the customer transcript.

## Why now (live e2e finding, 2026-06-10)

Casey (a `customer-support` template teammate) was run via gateway "Run now".
The run transcript
(`.skycot/outputs/019eaf2f-8509-7e13-bb31-baf027901ad4/transcript.md`, msg
[06]) shows a real backend error when the worker tried team-memory actions:

> `dispatch 'memory' failed: tool error: memory: memory tool not bound to a
> worker namespace — team actions require Memory::with_namespace(store, ws,
> vertical_id, ns) instead of Memory::new(store, ws, vertical_id).`

The bare `memory({})` self-scope call **succeeded**; only the
namespace-requiring (team / `query_scored`) actions failed. A hired teammate
that probes its own team memory and gets a hard tool error in its transcript is
a customer-visible defect, and the unbound namespace also means the per-worker
memory scope that **isolates one teammate's learned memory from another** is not
active in the run path (which compounds the isolation finding —
`fix-agent-runtime-memory-tool-tenant-and-vertical-scope`).

## Root cause (binding point VERIFIED; exact loss-of-worker_id site to localize)

- `memory_namespace` is derived in `build_turn_dispatch_ctx`
  (`crates/argo-agent-runner/src/turn_helpers_run.rs:127-128`) as
  `worker_id.map(|wid| MemoryNs::new(session.tenant_id, wid,
  MemoryScope::WorkerSelf))`. When `worker_id` is `None` → `memory_namespace`
  is `None` → team / `query_scored` actions raise the "not bound" error
  (`crates/argo-tool/src/tools/memory_team.rs` `resolve_namespace`, ~26-32;
  `memory_query.rs`, ~79-85).
- The gateway run **does** stamp `worker_id`:
  `routes_worker_run_exec.rs` passes `Some(worker.id)` → `process_job` →
  `work_dispatch_phases.rs` `create_session_with_worker(SessionCreate{ worker_id
  })` (lines 59-66). The `worker_id` column exists (migration 38, nullable) and
  session-persistence tests pass.
- **Yet the live transcript proves `memory_namespace == None` at dispatch** →
  `worker_id` is lost between the persisted `session.worker_id` and
  `build_turn_dispatch_ctx` **in the work-dispatch run path**. The threading
  infrastructure exists for the interactive path (`turn_run_loop.rs:140,164`
  pass `session.worker_id`), so the bug is **path-specific**: the work-dispatch
  turn loop (`work_steps.rs::run_live_loop` → `chat::run_turn` →
  `run_turn[_with_events]` → `build_turn_dispatch_ctx`) likely does not thread
  `session.worker_id` the way `turn_run_loop` does (or re-reads a session view
  with `worker_id` `None`). **Localize with a one-shot trace / assertion before
  writing the fix.**
- Prior art: `codex-memory-tool-namespace-wiring` (#1737, merged 2026-05-16)
  built the binding + `memory_namespace_runtime_tests.rs`, but those exercise
  the binding in isolation; the gateway worker-run path was never covered
  end-to-end.

## Fix (two parts)

**A. Wiring (root).** Ensure the work-dispatch run path threads
`session.worker_id` into `build_turn_dispatch_ctx` so `memory_namespace` binds.
Trace `process_job` → `work_dispatch` → `work_steps::run_live_loop` →
`chat::run_turn` → `run_turn[_with_events]` → `build_turn_dispatch_ctx`, and
pass `worker_id` (from `session.worker_id`) through, mirroring `turn_run_loop`.

**B. Graceful degradation (defense-in-depth).** Even when `memory_namespace` is
legitimately `None` (a bare operator session with no worker), the team /
`query_scored` actions should return an empty / "no team memory available"
result, NOT a hard `ArgoError::Tool` that surfaces into a transcript. Fix in
`memory_team.rs` / `memory_query.rs`.

## May touch

- `crates/argo-agent-runner/src/turn_helpers_run.rs` (derivation/threading),
  `turn_run_loop.rs` / the `run_turn` entry used by work-dispatch
- `crates/argo-cli/src/handlers/work_steps.rs`, `work_dispatch*.rs` (thread
  `worker_id` through the work-dispatch turn)
- `crates/argo-tool/src/tools/memory_team.rs`, `memory_query.rs` (graceful
  degrade)
- their `_tests` siblings

## Acceptance / verifier

1. New end-to-end (or dispatch-ctx unit) test: a worker-stamped session in the
   **work-dispatch path** yields `memory_namespace.is_some()`, and a team /
   `query_scored` action is reachable (returns a normal, possibly-empty result)
   and does NOT raise "not bound to a worker namespace". (Current
   `routes_worker_run_tests.rs` use the mock provider and don't exercise memory
   actions — add explicit coverage.)
2. A solo/operator session (no `worker_id`) → team action degrades gracefully
   (no hard tool error).
3. Per-worker isolation preserved (worker A's namespace ≠ worker B's) — keep
   `memory_namespace_runtime_tests.rs` green.
4. `cargo test -p argo-agent-runner -p argo-cli -p argo-tool` green; clippy clean.

## Size / owner / priority

~40-120 LOC incl. tests. **🟠 P2** — customer-visible confusing error, plus the
per-worker memory namespace that ISOLATES a teammate's learned memory isn't
binding in the run path (weakens the isolation in the sibling finding). Related:
`codex-memory-tool-namespace-wiring` (#1737),
`eval-memory-per-worker-scope-honored`, and
`fix-agent-runtime-memory-tool-tenant-and-vertical-scope` (sibling defect in the
same `build_turn_dispatch_ctx`).
