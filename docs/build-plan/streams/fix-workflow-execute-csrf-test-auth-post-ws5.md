# fix-workflow-execute-csrf-test-auth-post-ws5

**Owner**: claude or codex — gateway test / perimeter auth; conductor review before merge.
**Depends on**: none (WS5 + token-binding already shipped).
**Authored against**: origin/main `e3784d365` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-workflow-execute-csrf-test-auth-post-ws5

## Problem (proven pre-existing, 2026-06-11)

The integration test `crates/argo-cli/tests/workflow_execute_csrf.rs`
(`workflow_execute_csrf_gate`) is **RED on clean origin/main** (verified at
`e3784d365`: `test result: FAILED. 0 passed; 1 failed`). It panics at the
readiness probe with `gateway store not ready: HTTP/1.1 401 Unauthorized
{"error":"unauthorized","reason":"authentication required"}`.

Root cause: the test authenticates its setup requests (the `GET /sessions`
readiness probe + the `POST .../workflows/{name}` create + the `PUT .../head`)
with **self-asserted `X-Argo-Principal` + `X-Argo-Role: owner` headers only —
no bound token**. That worked when the test was written (#2500), but two later
landings closed that path:
- `88fa18c34` — WS5 fail-closed-by-identity perimeter (anonymous → Guest → 401
  on protected routes).
- `f8a9b72f0` — gateway derives principal+tenant from the **token binding** and
  rejects self-asserted headers that aren't backed by a verified token.

So a request with only `X-Argo-Principal` and no token is now correctly
unauthenticated → 401. The probe (and the setup requests) need a real bound
token.

NB: the gate is compile-only for tests (`cargo check --workspace --tests`), so
this red test does NOT block merges — but it is a real reliability gap (a red
test on main; priority #1 is reliability) and it leaves the CSRF route-level
integration uncovered. The path-agnostic unit coverage in
`middleware_csrf::tests` (10/10 green) still covers the double-submit logic.

## The contract to honor (do NOT mask a real regression)

The CSRF **happy path** (test lines ~260-277) already authenticates correctly:
it sends `Cookie: __Host-session={jwt}; __Host-csrf={jti}` (a real minted JWT,
`mint_jwt("owner")`, sub=`csrf-pin-user`, email=`csrf-pin@example.com`,
role=`owner`) + matching `X-CSRF-Token: {jti}` + the principal headers, and gets
200. So the bound-JWT auth path works; only the **setup** requests + the
**readiness probe** are stuck on the stale header-only auth.

The subtlety: the setup requests are **mutating** (POST/PUT). The CSRF gate
(`check_csrf_impl`) keys on the **`__Host-session` cookie** — if you authenticate
the setup requests with a session cookie, the CSRF gate will then REQUIRE a
matching `X-CSRF-Token` on them too, which is not what those steps test.

**Recommended fix:** authenticate the setup requests + the readiness probe with
`Authorization: Bearer {jwt}` (the same `mint_jwt("owner")` token), NOT a session
cookie. Bearer auth establishes identity at the perimeter without setting the
`__Host-session` cookie, so the CSRF gate stays inert on the setup steps (it only
fires for cookie-session callers). Confirm the gateway accepts the session JWT
via the `Authorization: Bearer` header for principal derivation (read
`routes_auth.rs` / the token-binding path); if it only accepts the session
JWT via cookie, instead probe a **public** readiness endpoint (e.g. `/healthz`
/ `/readyz` if one exists and is perimeter-exempt) and Bearer-auth the two
mutating setup requests.

## Acceptance

- `cargo test -p argo-cli --test workflow_execute_csrf` is GREEN cold.
- The two negative/positive CSRF assertions are unchanged in intent: cookie +
  no `X-CSRF-Token` → 403 `csrf_missing`; cookie + matching `X-CSRF-Token` → 200
  + `run_id`.
- No weakening of the perimeter or CSRF posture — the fix changes only how the
  TEST authenticates, never the gateway's auth/CSRF logic.
- The PR documents which token transport the setup requests now use and why.

## Parallel-surface check

`rg 'X-Argo-Principal|__Host-session|Authorization: Bearer'
crates/argo-cli/tests/workflow_execute_csrf.rs
crates/argo-cli/src/handlers/gateway/routes_auth.rs` — fix the test's auth
transport; do not touch the gateway auth/CSRF code.
