# sprint-ws1-1c — fail-closed boot-guard for SKYCOT_AUTH_JWT_SECRET

**Owner**: claude or codex.
**Depends on**: none (independent of WS1-1b; touches gateway startup, not route handlers).
**Authored against**: origin/main `e63c84bca` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-1c-jwt-secret-fail-closed-boot-guard`.

## Why (verified 2026-06-05 — real gap)
`crates/argo-cli/src/handlers/gateway/middleware_session_verify.rs`:
- `jwt_secret()` (:213) returns an **empty vec** when `SKYCOT_AUTH_JWT_SECRET` is unset.
- `decode_session_cookie` (:161-167) treats an empty secret as `DecodeOutcome::NoCookie` — i.e.
  **every session cookie is silently ignored** — with the comment *"Production callers detect the
  missing secret upstream."*

**That upstream guard does not exist.** `rg 'SKYCOT_AUTH_JWT_SECRET' crates/` finds no startup
panic / bail / required check. So if the gateway boots in production with the secret unset (or empty),
session-cookie auth is silently disabled for every browser user — a fail-open misconfiguration on a
fully-public launch, with no error and no log. The keystone made tenant isolation depend on the
session JWT; this makes that dependency safe to deploy.

## Task — add a fail-closed boot-guard
1. Find the gateway boot/serve entry point (where config is loaded and the HTTP listener binds).
   `rg 'fn serve|fn run|listener|TcpListener|bind\(' crates/argo-cli/src` and trace the gateway
   subcommand. **Extend any existing startup config-validation** rather than adding a parallel one (grep first).
2. **In production mode only**, if `SKYCOT_AUTH_JWT_SECRET` is unset or empty → refuse to start with a
   clear fatal error (e.g. `"SKYCOT_AUTH_JWT_SECRET must be set in production"`), non-zero exit.
   "Production mode" = NOT dev/demo/auth-bypass. **Reuse the EXISTING bypass/demo detection** (grep for
   `bypass`/`demo_mode`/`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` equivalents) — the guard must NOT fire in
   dev/demo/bypass, so the documented dev-without-secret recovery path (the :164 comment) still works.
3. Also enforce a minimum secret length (≥ 32 bytes) for HS256 strength — reject a too-short secret with
   the same fatal error.

## Verify
- Test: production-mode boot with unset / empty / too-short secret → fatal error (non-zero / `Err`);
  with a valid ≥32-byte secret → proceeds; dev/bypass mode with no secret → still proceeds (unchanged).
- `cargo test -p argo-cli -- --test-threads=1` GREEN; clippy clean on touched files (boundary-note the
  pre-existing `await_holding_lock` in `routes_settings_credentials_tests.rs`, untouched).

## Guardrails
- Do NOT break dev/demo/bypass runs without the secret (the :161-167 recovery path).
- If a boot-guard already exists somewhere (e.g. a config validator), EXTEND it; don't add a parallel one.
- WS1/security-perimeter change → conductor will adversarial-review before merge. Do NOT self-merge.

## PR
Title: `sprint(ws1-1c) — security — fail-closed boot-guard for missing/weak SKYCOT_AUTH_JWT_SECRET`
