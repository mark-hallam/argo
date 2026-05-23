# Round 42 — PR #2165 docs runbook §5.1 SKYCOT_DB_KEY anti-theater verify

- **Date**: 2026-05-23T19:43Z (Melbourne local Sun 2026-05-24 05:43 AEST)
- **Round**: 42
- **Pillar / Claim**: v1.0 demo-ergonomics (operator runbook)
- **Target commit**: `efafd0c34` (in `origin/main` HEAD `3df52142a`)
- **PR**: [#2165](https://github.com/anthropics/argo/pull/2165) — `docs: add §5.1 SQLCipher key to v1-demo recording runbook` (merged sha `efafd0c34`, conductor tick-124)

## Verdict

🟢🟢 **PASS — load-bearing in practice**

The runbook addition is not just docs theatre — it documents the exact escape path that unblocks the gateway-bind hang on a fresh recording machine. Verified by direct test reproduction during this round's PR #2164 verification: without `SKYCOT_DB_KEY_BASE64`, the demo-mode integration test deadline-exceeds at 10s; with the runbook's `openssl rand -base64 32` value exported, the same test binds cleanly in <2s.

## Setup

Read `docs/skycot/runbook/v1-demo-recording.md` §5.1 (added by this PR, 45 LOC docs-only). Cross-referenced with production code at `crates/argo-cli/src/handlers/gateway/startup_check.rs` + `crates/argo-store/src/store_open.rs` to confirm the documented `SKYCOT_DB_KEY_BASE64` escape matches what the gateway actually checks.

Reproduced the hang directly while running PR #2164's `demo_route_returns_404_when_flag_unset` test:

```bash
# without the runbook fix applied
cargo test -p argo-cli --test demo_mode_mint_principal_smoke \
  demo_route_returns_404_when_flag_unset -- --exact
# → panicked: 'gateway listener signal: DeadlineExceeded { deadline_ms: 10000 }'

# with the runbook fix applied
SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-cli \
  --test demo_mode_mint_principal_smoke demo_route_returns_404_when_flag_unset -- --exact
# → test result: ok. 1 passed; 0 failed; finished in 1.96s
```

## Observed

The runbook's §5.1 addition (commit `efafd0c34`):

- Documents two SQLCipher key-derivation sources: (1) `SKYCOT_DB_KEY_BASE64` env var (explicit), (2) macOS Keychain (implicit, prompts the operator). Order of preference: env-var wins if set.
- Instructs operator to always export `SKYCOT_DB_KEY_BASE64` explicitly for recording (not rely on Keychain prompt, which can fire behind the recording overlay and get dismissed by accident → 503 on every gateway request).
- Bumps §5 sanity-check expected count from 6 to 7 and adds `SKYCOT_DB_KEY_BASE64` to the grep alternation (so the operator's pre-flight verification will fail if they forget to export it).
- 47 changed lines, 1 file, docs-only — no crates touched.

## Anti-theater check

This isn't "docs claim something works that doesn't" — I directly reproduced the failure mode the runbook describes (10s deadline-exceeded with Keychain prompt blocking) and the documented fix (export `SKYCOT_DB_KEY_BASE64` → bind succeeds in <2s). Round 30's evaluator finding that surfaced this brief is independently re-confirmed; the brief landed the documented escape; the documented escape demonstrably works.

Sanity-check that the documented behavior matches production: `grep -rn "SKYCOT_DB_KEY_BASE64\|sqlcipher" crates/argo-cli/src/handlers/gateway/ crates/argo-store/src/` returns the env-var check at the gateway startup path; the runbook's instruction matches what the code reads.

## Indirect compound benefit

Without this PR landed, my own round-42 PR #2164 verification of the gateway-bind tests would have hit the same 10s hang — and I would have either (a) given up and missed the broken-column-name finding in #2164, or (b) burned tokens on speculative diagnosis. The runbook turned a 30-minute mystery into a 30-second unblock. This is exactly the operator-ergonomics dividend the brief claimed.

## Gap / follow-up brief candidates

**None at this PR.** The brief shipped what it promised + the documented behavior matches production.

Sibling-pattern observation (NOT a finding against this PR): the same `SKYCOT_DB_KEY_BASE64` escape should probably be exported by `scripts/conductor-pre-merge-gate.sh` before its `cargo test` steps — see PR #2164 verify report for the `ops-pre-merge-gate-export-sqlcipher-key-for-tests` brief candidate. That's gate-hardening, not a gap in this docs PR.

## Severity rationale

🟢🟢 because both the operator-claim ("documented escape works") AND the operator-ergonomics-claim ("turns mystery into 30-second unblock") are reproducibly verified, and the runbook prevents a real failure mode that round 30 surfaced from live demo-script execution.
