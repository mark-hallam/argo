# Worker→Worker handoff audit row

**Owner**: claude. **Depends on**: —

## Why

When Worker A escalates to Worker B today, the existing handoff path
records a `Handoff` row for routing, but doesn't write a uniform audit
event tying both workers, the reason, and the linking session. Mission
claim 3 ("workers cooperate; cooperation is observable") needs every
delegation to land in the audit timeline alongside everything else.

## Scope

- New audit kind `worker_handoff`.
- On every successful handoff transition into `Acknowledged`, write a
  single audit row carrying: from_worker, to_worker, session_id, reason,
  payload_hash.
- The row goes through the SAME audit writer as every other event —
  no separate table, no parallel pipeline.
- Idempotent: re-acknowledging a settled handoff does NOT write a
  second row.

## Approach (concrete)

1. **Modify** `crates/argo-orchestrator/src/persist.rs` — narrowly:
   in the function that performs the Pending→Acknowledged transition
   (search for `HandoffStatus::Pending`-related state machine code),
   after the SQL succeeds and BEFORE returning, call a new helper
   `audit_worker_handoff(store, &handoff).await?`.
2. **NEW** `crates/argo-orchestrator/src/persist_audit.rs` (~120 LOC):
   - `pub async fn audit_worker_handoff(store, handoff) -> ArgoResult<()>`
     — uses the existing audit row writer (do NOT inline-INSERT — find
     and re-use; if no public audit-write helper exists, surface to
     conductor; this brief does not invent one).
   - 5 unit tests covering: happy path, idempotent re-ack, missing
     to-worker (Worker enum branch only), payload_hash stability,
     audit kind exact-string match.
3. **Modify** any audit-kind allowlist enum or constants table —
   add `"worker_handoff"`. EXHAUSTIVE-MATCH note: search for
   `match.*audit_kind`, `match.*Kind::` in audit_query / audit_export
   / audit_report and add arms wherever the compiler complains.
4. NO schema migration needed — the existing `audit_log` table already
   accepts `kind TEXT`. Verify the `principal` and `payload` columns
   accept JSON before assuming.

## Files you may touch

- `crates/argo-orchestrator/src/persist.rs` (narrow — 1 call site near the Acknowledged transition)
- `crates/argo-orchestrator/src/persist_audit.rs` (NEW, ≤ 200 LOC)
- `crates/argo-orchestrator/src/lib.rs` (1 pub mod line)
- Any 1-2 audit-kind enum / allowlist / dispatch sites the compiler forces (EXHAUSTIVE-MATCH)
- `crates/argo-orchestrator/tests/handoff_audit_smoke.rs` (NEW, ≤ 200 LOC) — round-trip integration

## Files you must NOT touch

- Any new schema migration — re-use existing audit_log table; if the
  table is missing a column you need, STOP and surface
- The handoff state machine itself — only the audit-write call site
- `argo-cli` — the audit row will surface naturally through
  `argo audit by-kind worker_handoff`; no new CLI in this brief

## Acceptance criteria

- [ ] After a Pending→Acknowledged transition, exactly 1 audit row of kind `worker_handoff` exists
- [ ] Re-acknowledging the same handoff (idempotent SQL path) → still exactly 1 row
- [ ] `argo audit by-kind worker_handoff` lists the row with from/to worker IDs visible
- [ ] payload_hash stable across the same input (deterministic)
- [ ] 5 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-orchestrator` clean
- [ ] Every modified file < 500 LOC
- [ ] Pre-existing `argo audit by-kind` flake is documented if it surfaces; does not block (per tick 60 amendment)

## Branch + PR

- Branch: `claude-worker-handoff-record`
- PR title: `orchestrator: audit Worker→Worker handoff transitions (claude-worker-handoff-record)`

## Notes for the implementer

- EXHAUSTIVE-MATCH note: any `match` over audit kinds across the
  workspace must include the new arm or the compiler will fail.
  `cargo build` is your friend; trust it.
- Do not invent a payload_hash algorithm. Use whatever the rest of the
  audit pipeline already uses (search for `payload_hash =`).
- This brief is deliberately thin. If you find yourself >5 files in,
  stop and re-scope.
