# fix-demo-mode-test-column-name-payload-to-payload-json

**Authored against**: origin/main `d80fa2998` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-demo-mode-test-column-name-payload-to-payload-json

## Why this brief exists

Surfaced as round-42 P1 finding (evaluator handoff 2026-05-23T19:43:50Z) verifying tick-124 demo-ergonomics cluster.

PR #2164 (`demo-ops-add-magic-link-bypass-for-recording`, MERGED commit fb07ae50b) shipped the load-bearing safety rail correctly — `gateway_refuses_to_bind_when_demo_mode_set_without_bypass` PASSES on cold-built `origin/main`, production dispatch is correctly wired (`routes.rs:153` ahead of bearer-auth, `OnceLock`-cached at `demo_mode.rs:37`, `startup_check.rs:76` runs BEFORE the `--skip-startup-check` early-return so the demo-mode guard cannot be bypassed).

But the integration test that was supposed to pin the `demo_mode_bypass=true` audit-row forensic-record contract — `demo_route_mints_principal_and_writes_audit_when_flag_set` at `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs:79` — FAILS LIVE-RED on `origin/main` with:

```
Database(SqliteError { message: "no such column: payload" })
```

Root cause: the test queries `SELECT kind, payload FROM audit_record` but the actual column is `payload_json` (per `crates/argo-store/src/migrations/v1.rs:214`). Production code is CORRECT (`sink_chain.rs:96+167` + `sink_tx.rs:128` use `payload_json`); the test has a column-name typo.

Without this fix the `demo_mode_bypass=true` forensic-record audit-row contract is verified by NOTHING. This is the safety-net assertion that proves every demo-mode-minted principal emits an `AuthSucceeded` row carrying `demo_mode_bypass=true` so a forensic audit can identify post-hoc which sessions were minted via the bypass path. The safety rail keeps prod safe; this test keeps the safety rail honest.

PR author's gateway-bind hang on his dev box masked the typo at author-time and at pre-merge-gate time (the test never compiled-and-ran end-to-end before merge). PR #2165 (runbook §5.1) documents the `SKYCOT_DB_KEY_BASE64` escape that turns this 30-min mystery into a 30-second unblock for follow-up sessions.

## Acceptance criteria

1. Change `payload` → `payload_json` at `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs:79`.
2. `SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-cli --test demo_mode_mint_principal_smoke -- demo_route_mints_principal_and_writes_audit_when_flag_set` PASSES on the worker's machine.
3. PR body confirms: (a) the byte-exact line being changed, (b) the test now actually exercises the audit-row payload assertion, (c) the rest of `demo_mode_mint_principal_smoke.rs` is untouched.
4. No production code change. Read-only verification that the safety-rail subtest (`gateway_refuses_to_bind_when_demo_mode_set_without_bypass`) still PASSES post-change confirms no collateral.

## Parallel-surface declarations

- Test-only fix. No new pub Rust surface; no migration; no docs.
- No name collision: the only existing `audit_record` query in this test file is the broken one being repaired.

## May-touch

- `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs` (single-line typo fix)

## Boundary

- Do NOT change production code (`sink_chain.rs`, `sink_tx.rs`, `demo_mode.rs`, `routes.rs`, `startup_check.rs`) — they are correct.
- Do NOT change the schema (`v1.rs:214` is the source of truth — column is `payload_json`).
- Do NOT touch any other test in the file.
- Do NOT skip the local gate (`SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-cli --test demo_mode_mint_principal_smoke`) — its purpose is to prove this 1-line fix actually unblocks the asserted contract.

## Sizing

~1-2 LOC test fix. Fast-mode trivially admin-mergeable in one tick.

**🔴 P1 ANTI-THEATER** — closes round-42 evaluator finding. Without this fix the demo-mode forensic-record audit-row contract has zero regression coverage; any future drift to the `payload_json` column would land silently. The whole point of #2164's smoke test was to catch exactly this class of drift.
