# sprint-e2e-new-user-journey-smoke

**Owner**: claude or codex.
**Depends on**: none (WS1 keystone + WS1-2/3/4 already MERGED — this asserts them end-to-end).
**Authored against**: origin/main `b5a4f72ea` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-e2e-new-user-journey-smoke`.

## Why
The fresh-signup → paid path is built and unit/integration-tested in PIECES (auth, provision,
write-path tenant, WS1-4 isolation #2696, claim_N_live). What's missing is ONE chained
end-to-end smoke that walks the whole new-user journey in a single test and asserts the user
stays in THEIR OWN tenant (never the cwd/demo workspace) at every step. This is the guard that
catches a break before the operator's live stranger-signup walk (the Friday bar).

## Build — one integration test that chains the journey
A new integration test (sibling to `tests/tenant_resolve_from_session_jwt.rs`), in-process,
no gateway HTTP boot (that path is flaky on this machine per WS1-4's note). Steps, each asserted:
1. **Verify a fresh magic-link user** → first-time verify auto-provisions the user's tenant
   (WS1-3 #2694); the minted `__Host-session` JWT carries a real `tnt` (not cwd/demo).
2. **Own empty workspace** → the 6 migrated reads (team/workers/skills/usage/audit/approvals)
   resolve to the user's tenant and return EMPTY (not the seeded demo roster).
3. **First hire lands in the user's tenant** → exercise the first-hire write path
   (`resolve_tenant_for_request`, WS1-2 #2682); assert the new worker appears in the user's
   `/team` and NOT in cwd/demo, and a second tenant B does not see it.
4. **Hash-chained audit row** → the hire produces an audit row in the user's tenant, and the
   chain verifies (reuse the existing audit-verify helper). This is the moat: provable action.
5. **Billing reachable** → a checkout session can be created for the user's tenant, OR an
   honest "Stripe not provisioned" error (NO THEATER — do not fake a session).

## Guardrails
- **Reuse existing helpers** — `provision_for_user`, the JWT mint helper, the audit-verify
  helper, the seed helpers from #2696. Do NOT fork a parallel test harness. Grep first:
  `rg 'fn provision_for_user|mint.*session|verify_chain' crates/ tests/`.
- If a step can only be reached via the gateway HTTP path (not in-process), assert the
  in-process store-level equivalent and note the gap in the PR body — do NOT skip silently.
- Test-only. No production change. If it reveals a real loop break, STOP and report it
  (that's a finding, not something to paper over).

## Verify
`cargo test -p argo-cli --test <name> -- --test-threads=1` GREEN (serial). clippy clean on the
new test file (boundary-note the pre-existing `await_holding_lock` in
`routes_settings_credentials_tests.rs`, untouched).

## PR
Title: `sprint(e2e) — new-user journey smoke (verify → own tenant → hire → audit → billing)`
