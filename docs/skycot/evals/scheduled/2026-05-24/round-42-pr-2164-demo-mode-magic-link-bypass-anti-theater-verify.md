# Round 42 — PR #2164 demo-mode magic-link bypass anti-theater verify

- **Date**: 2026-05-23T19:43Z (Melbourne local Sun 2026-05-24 05:43 AEST)
- **Round**: 42
- **Pillar / Claim**: v1.0 demo-ergonomics + Pillar 4 (capability-based security; auth-bypass surface)
- **Target commit**: `3df52142a` (origin/main HEAD)
- **PR**: [#2164](https://github.com/anthropics/argo/pull/2164) — `demo-ops — magic-link bypass for v1.0 demo recording` (merged sha `fb07ae50b`, conductor tick-124)

## Verdict

🟡 **PARTIAL — P1**

Load-bearing safety rail PASSES on cold-built `origin/main`. Production demo-mode dispatch is wired correctly. **But the integration test that was supposed to pin the audit-row forensic-record contract is broken** — queries a column name (`payload`) that has never existed (column is `payload_json`, has been since migration v1). The test only "passes" on the author's dev-box because the gateway hangs BEFORE reaching the broken query, masking the typo. On a recording machine that follows the new §5.1 runbook (`SKYCOT_DB_KEY_BASE64`), the gateway binds cleanly and the test fails loudly.

## Setup

Cold-built `cargo build -p argo-cli --bin argo --tests` from scratch (10m01s), then exercised the 3 integration tests + 2 unit tests:

```bash
# 1. load-bearing safety rail (no gateway bind required)
cargo test -p argo-cli --test demo_mode_mint_principal_smoke \
  gateway_refuses_to_bind_when_demo_mode_set_without_bypass -- --exact

# 2. unit tests (route classifier + JWT round-trip)
cargo test -p argo-cli --bin argo handlers::gateway::demo_mode

# 3. gateway-binding tests (need SKYCOT_DB_KEY_BASE64 per #2165 runbook §5.1)
SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-cli \
  --test demo_mode_mint_principal_smoke demo_route_returns_404_when_flag_unset
SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-cli \
  --test demo_mode_mint_principal_smoke demo_route_mints_principal_and_writes_audit_when_flag_set
```

## Observed

**Safety-rail test (load-bearing)**: ✅ PASS in 1.98s. Production binary refuses to bind with `ARGO_DEMO_MODE=1` set + `ARGO_GATEWAY_AUTH_BYPASS` unset; stderr contains "ARGO_DEMO_MODE=1 cannot be used in production" and the remediation hint. Confirmed via code-read at `crates/argo-cli/src/handlers/gateway/startup_check.rs:76` — `demo_mode_compatibility_check()?` runs BEFORE the `if skip` early-return at line 77, so `--skip-startup-check` cannot bypass the demo-mode safety guard. Cached via `OnceLock` at `demo_mode.rs:37` so the flag can't be flipped mid-process by a malicious caller.

**Unit tests**: ✅ 2/2 PASS — `is_mint_principal_request_matches_both_forms` + `mint_demo_session_round_trips`.

**Flag-off 404 test**: ✅ PASS in 1.96s (with `SKYCOT_DB_KEY_BASE64` set per #2165). Without `ARGO_DEMO_MODE=1`, the route falls through to a 404 — production demo-mode dispatch at `routes.rs:153` correctly returns `None` when `is_enabled()` returns false.

**Mint+audit-row test**: 🔴 **FAIL** in 2.03s with `Database(SqliteError { code: 1, message: "no such column: payload" })` at test line 83. Query at line 79 is `SELECT kind, payload FROM audit_record WHERE kind = 'auth.succeeded' ORDER BY seq ASC`. The actual column is `payload_json` (per `crates/argo-store/src/migrations/v1.rs:214`, has been since v1). All 3 production write paths in `crates/argo-audit/src/sink_chain.rs:96+167` + `sink_tx.rs:128` correctly use `payload_json`. The typo is in the test only.

## Anti-theater check

This isn't "test rewrites itself green" — the test was always RED on any environment where the gateway actually binds. It only appeared green because:

1. PR author's dev-box hits the macOS Keychain-prompt hang documented in #2165 (no `SKYCOT_DB_KEY_BASE64` exported → gateway can't open `sessions.db` → `await_listener_signal` deadline-exceeds at 10s → test panics before reaching the broken query). Author's PR-body honestly flagged this: "gateway-spawning integration subtests hang per author dev-box env note, not a code defect."
2. The pre-merge gate doesn't export `SKYCOT_DB_KEY_BASE64` either, so the same hang masked the typo at gate time.
3. The load-bearing safety-rail subtest passes BECAUSE it specifically tests the gateway REFUSING to bind — never reaches the audit-row query.

I verified the production audit-write path independently: `demo_mode.rs:132-152` constructs the payload `{"method":"demo_mode_bypass","demo_mode_bypass":true,"email":<email>}` and writes via `AuditSink::write` → `sink_chain` SQL uses `payload_json`. The PRODUCTION code is correct; the test that was supposed to PIN it is broken.

## Gap / follow-up brief candidates

**(a) `fix-demo-mode-test-column-name-payload-to-payload-json` (P1, ~1-2 LOC)** — change `payload` → `payload_json` at `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs:79`. Without this fix, the forensic-record contract for `demo_mode_bypass=true` audit row is verified by NOTHING. Trivial; admin-mergeable in one tick.

**(b) `ops-pre-merge-gate-export-sqlcipher-key-for-tests` (P1, ~5-20 LOC, sibling of already-authored `ops-pre-merge-gate-fix-test-failures-not-just-clippy`)** — `scripts/conductor-pre-merge-gate.sh` should export `SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32)` before any `cargo test` step. Without this, every gateway-spawning integration test will hang and be silently skipped at gate time. This is the second instance this week (after the `--skip-clippy` masking pattern) of pre-merge gate ergonomics masking real test signal.

## Severity rationale

P1 not P0 because: (i) the production safety rail IS load-bearing and verified, (ii) the production audit-write path IS correct (verified via code-read across 3 sink paths), (iii) the gap is "forensic record claim is not test-pinned" not "forensic record is missing." The harm is future-looking: a refactor dropping the `demo_mode_bypass: true` field or routing through a tenant-stripped audit sink would NOT trip the existing test. The author's "operator who accidentally booted with the flag in a prod-shaped environment sees obvious noise plus a permanent forensic record" claim is currently honest but untested.
