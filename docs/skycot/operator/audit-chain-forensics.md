# Audit-Chain → Replay Forensics

Mission claim 5 is **"every action lives on a tamper-evident hash chain
that anyone can replay."** Three subsystems carry the claim:

1. `argo_audit::AuditSink::write` — every event lands as one row in
   `audit_record`, with `self_hash = sha256(parent_hash || kind || 0x00
   || canonical_payload)`.
2. `argo_audit::verify_session` (+ `argo audit verify-chain`) — recompute
   every row's hash, fail on any drift.
3. `argo_audit::rows_for_session` + `render_replay` (the `argo audit
   replay <session>` path) — re-emit the session's audit trajectory in
   `seq` order.

The smoke test
[`crates/argo-cli/tests/audit_chain_replay_smoke.rs`](../../../crates/argo-cli/tests/audit_chain_replay_smoke.rs)
exercises the full pipeline end-to-end and is the regression lock for
the claim. This runbook documents how to run it and how to debug each
waypoint when it fails.

## Running the smoke

```bash
cargo test -p argo-cli --test audit_chain_replay_smoke -- --nocapture
```

Cold run: ~3 min if the `argo` binary hasn't been built; warm: <30s.
The test spawns the CLI binary as a subprocess, so it depends on
`CARGO_BIN_EXE_argo` (Cargo sets this for `[[bin]]`-bearing crates'
integration tests automatically — no extra config).

## Waypoints

Each `#[tokio::test]` exercises a slice of the pipeline. When the
smoke fails, the failing test name points at the slice; the table
below maps each slice to the production code path it locks.

| Test                                                           | Production path                                                   |
| -------------------------------------------------------------- | ----------------------------------------------------------------- |
| `closure_smoke_session_emit_verify_render_matches_trajectory`  | full pipeline — sink → library verify → CLI verify → replay       |
| `tamper_evidence_corrupted_payload_detected_at_offset`         | `verify_session` self-hash recomputation                          |
| `cross_tenant_isolation_and_spoof_detected`                    | `verify_session` denorm sanity belt (`tenant_id` column vs JSON)  |

### Debugging a failure

- **`assert_chain_links` failed.** A row's `parent_hash` no longer
  matches the previous row's `self_hash`. Inspect the row with
  `argo audit show --session <id>` and check the chain hash columns in
  `audit_record` directly. Common cause: a write path that bypasses
  `AuditSink` and inserts straight into the table without computing
  the chain hash.

- **Library `verify_session` returned `ok=false` on the happy path.**
  Either (a) someone broke the chain-hash function in `sink.rs`, or
  (b) someone changed the canonical payload shape without re-hashing
  (e.g., added a new key after `to_string` but before the SHA-256
  call). `report.breaks[0].reason` tells you which.

- **CLI `verify-chain --json` returned non-zero or `verified=false`.**
  Library passed but CLI failed: the gap is in
  `handlers/audit_verify_chain.rs` or its `verify_all` call against the
  whole table. Run `argo audit verify-chain` (text mode) for the
  break-by-break printout.

- **`tamper_evidence` test passed but verify reported the wrong seq.**
  Either `verify_rows` is short-circuiting on the first break (it
  shouldn't — every break is appended), or the denorm sanity belt is
  flagging the row before the hash mismatch is checked. Look at
  `breaks` ordering — the hash break should come first.

- **`cross_tenant_isolation` failed.** Two failure modes:
  - **Bleed** (`bleed != 0`): rows from one tenant's session are
    landing in another tenant's session_id, i.e. session_id is not
    unique-per-tenant. This is a P0 multi-tenancy bug.
  - **Spoof not detected**: someone removed the `payload_json.tenant_id`
    vs `tenant_id` column drift check in
    [`verify.rs`](../../../crates/argo-audit/src/verify.rs).

## Operator quick-recipe

Forensic check of a single session in production:

```bash
# Chain integrity (text report — listing every break inline).
argo audit verify-chain

# Replay the session as an incident timeline.
argo audit replay <session-uuid> --format pretty

# Dump as NDJSON for downstream tools (jq, SIEM upload, …).
argo audit replay <session-uuid> --format raw > /tmp/session.ndjson
```

`argo audit verify-chain` verifies the WHOLE chain (every session in
the audit table); break rows list `(seq, kind, reason)`. There is no
`--session` flag on `verify-chain` today — to scope verification to one
session from the library side, call `argo_audit::verify_session(store,
session_id)` directly.

## When the smoke graduates the claim

When `audit_chain_replay_smoke` is green on CI, mission claim 5 is
runtime-PROVEN. Any regression in `AuditSink`, `verify_session`, the
chain-hash function, or the audit-replay rendering path that breaks
the pipeline end-to-end will surface here before it lands.

If a future change deliberately changes the chain hash inputs (e.g.,
adds a field to the canonical payload), the smoke will fail until the
fixture in
[`tests/fixtures/audit_chain_seed.rs`](../../../crates/argo-cli/tests/fixtures/audit_chain_seed.rs)
is updated to match. Treat that as a deliberate schema-version bump
and route it through `docs/skycot/operator/migrations.md`.
