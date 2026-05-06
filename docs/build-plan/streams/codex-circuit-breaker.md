# V1.1 — Worker circuit breaker

**Owner**: codex. **Depends on**: H-2 (#800),
H-spend-persistence (#809) merged.

## Scope

Auto-pause a worker when N consecutive actions fail. Operator
must explicitly resume. Prevents a misbehaving worker from
burning budget or doing damage.

- New `Worker::circuit_state: CircuitState` field
  (`Closed | Open { tripped_at, reason } | HalfOpen`)
- New table `worker_circuit_history` (audit trail)
- After every action transition to `failed` OR
  `escalation_exhausted`, increment a per-worker counter; if
  counter ≥ threshold (default 3), trip → Open
- While Open, the dispatch gate refuses every action with a
  clear `circuit_open` error
- `argo worker resume <id>` flips to HalfOpen → next action
  passing flips back to Closed; another fail re-opens

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies (CircuitState enum)
- `crates/argo-identity/src/circuit.rs` (NEW)
- `crates/argo-identity/src/lib.rs` — narrowly: re-export CircuitState
- `crates/argo-identity/src/types.rs` — narrowly: add
  `circuit_state: CircuitState` field to Worker struct
- `crates/argo-store/migrations/<next>__worker_circuit_history.sql`
  (NEW)
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs` —
  narrowly: ONE check before the auto-approval gate
- `crates/argo-cli/src/handlers/worker_actions_transitions.rs` —
  narrowly: bump counter on `fail` / `escalation_exhausted`
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` —
  narrowly: trip-check during dispatch (sibling-file pattern)
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: add
  `Resume` subcommand variant
- `crates/argo-cli/src/handlers/worker_resume.rs` (NEW)

## Acceptance criteria
- [ ] CircuitState enum + per-worker persistence
- [ ] Threshold default 3, configurable via env
      `ARGO_CIRCUIT_THRESHOLD`
- [ ] Open worker rejects actions with `circuit_open` error
- [ ] `argo worker resume <id>` flips Open → HalfOpen
- [ ] HalfOpen + 1 successful action → Closed
- [ ] 5 unit tests + 1 integration test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-circuit-breaker`
- PR title: `worker: circuit breaker for consecutive-fail auto-pause (codex-circuit-breaker)`
