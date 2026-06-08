# fix-perimeter-bypass-off-cookie-regression-test

**Owner**: claude.
**Authored against**: origin/main `0eafc89a3` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-perimeter-bypass-off-cookie-regression-test`.

## Why (belt-and-suspenders lock; #159 already live-verified)
#159 (fail-closed SPA auth) was live-verified by the conductor (bypass-OFF battery on a throwaway :7401 — no-auth/tamper/expired/header → 401, valid `__Host-session` cookie → 200 + own tenant; see `docs/skycot/SESSION-OVERNIGHT-2026-06-08-deploy-readiness.md` §"#159 — VERIFIED"). The decode path (`middleware_session_verify_tests`), bearer fail-closed (`routes_auth_tests`), cross-tenant isolation, and prod fail-closed (`gateway_fail_closed_prod_smoke`) are all unit/integration-tested. The ONE seam not directly asserted is the **composition**: `require_identity` ALLOWS a valid `__Host-session` cookie with **bypass OFF + a bearer configured** (the SPA's exact auth path). Lock it so a future change can't silently reopen the perimeter. Security-critical: a regression here = a cross-tenant data leak.

## Where (race-free — extend the existing isolated-subprocess smoke)
`crates/argo-cli/tests/gateway_fail_closed_prod_smoke.rs` already spawns an isolated gateway (own `ARGO_HOME` tempdir + own port + `gateway_lock_guard`) with `ARGO_GATEWAY_BEARER_TOKEN=prod-token` and bypass OFF (no `ARGO_GATEWAY_AUTH_BYPASS`). Add the cookie cases THERE (isolated subprocess ⇒ no process-global env races — do NOT add an env-touching unit test to `routes_auth_tests`, which has known parallel-env-race flakiness).

## Grep-before-write
- `rg -n 'fn spawn_gateway|SMOKE_DB_KEY|fn http\(|status_line|wait_with_timeout' crates/argo-cli/tests/gateway_fail_closed_prod_smoke.rs` — reuse these helpers.
- `rg -n 'fn mint' crates/argo-cli/src/handlers/gateway/middleware_session_verify_tests.rs` — mirror the HS256 claim shape (sub/email/tnt/iat/exp).
- `jsonwebtoken` is a NORMAL dep but NOT a dev-dep; integration tests need it re-declared. `base64`/`serde_json` are already re-declared dev-deps (see the comment in `crates/argo-cli/Cargo.toml [dev-dependencies]`).

## Files you may touch
- `crates/argo-cli/Cargo.toml` — add to `[dev-dependencies]`: `jsonwebtoken = { version = "9", default-features = false }` (with a one-line comment, matching the existing base64/serde_json re-declaration note).
- `crates/argo-cli/tests/gateway_fail_closed_prod_smoke.rs` — add `.env("SKYCOT_AUTH_JWT_SECRET", <known 32+ byte const>)` to `spawn_gateway` (harmless to the existing wizard test); add a `mint_session_cookie(exp_offset_secs)` helper that signs a `serde_json::json!({sub,email,tnt:"00000000-0000-0000-0000-0000000000aa",iat,exp})` with `jsonwebtoken` + the known secret (use `std::time` for epoch secs); add the test below.

## The test (assert the AUTH outcome, not a specific 200 body)
`session_cookie_authenticates_with_bypass_off_and_fails_closed`: spawn the gateway (max_requests = exactly your request count so it exits clean, mirroring `wizard_requires_bootstrap_token_and_returns_gone_after_done`), then:
1. `GET /api/v1/workers` no auth → status line contains `401`.
2. valid cookie (`exp = now+3600`) via `Cookie: __Host-session=<jwt>` → status line does NOT contain `401` (authenticated — the SPA path).
3. tampered (`<jwt>xx`) → `401`.
4. expired (`exp = now-3600`, beyond the 60s jwt leeway) → `401`.
Then `wait_with_timeout` + assert clean exit.

## Acceptance (truth-bounded)
- `cargo test -p argo-cli --test gateway_fail_closed_prod_smoke` → all green (paste the tail), including the new test.
- `cargo check -p argo-cli` exit 0. No new clippy errors in touched files (boundary-note the pre-existing `await_holding_lock` in `routes_settings_credentials_tests.rs`).
- Do NOT change flag defaults (`ARGO_DUTY_AUTOFIRE`/`ARGO_EXECUTION_GRADUATION`) or any production code — test-only + a dev-dep.

## On ship (same commit)
Note in the PR it locks deploy-readiness #159 (fail-closed SPA cookie auth).

## Effort
S (one integration test + a dev-dep + a 2-line spawn_gateway env add). Isolated-subprocess ⇒ no env-race risk.
