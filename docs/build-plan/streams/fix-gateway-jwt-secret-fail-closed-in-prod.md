# fix-gateway-jwt-secret-fail-closed-in-prod

**Authored against:** origin/main `7f98b30cf` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-gateway-jwt-secret-fail-closed-in-prod`
**Priority:** 🔴 P1 / HIGH — silent availability defect for paying users on a fresh deploy.

## Problem (VERIFIED 2026-06-15 AWS go-live audit)

The gateway fails closed in production for the bearer token (`boot.rs:123-129`) and the DB
key (`boot.rs:130` + `enforce_enterprise_encryption_at_boot`) — but NOT for the session JWT
secret. `auth_magic_link_token.rs:74-92` `jwt_secret()`: if `SKYCOT_AUTH_JWT_SECRET` is
unset/empty it mints a **random ephemeral per-process key** via `OnceLock` + emits only a
`tracing::warn` (L85-88), then boots normally. There is no production boot gate (grep of
`boot.rs`/`boot_serve.rs` finds no JWT guard).

**Consequence:** an operator who skips that one `put-secret-value` (the runbook lists it as
a separate step) gets a GREEN boot — but the ephemeral key changes on every Fargate task
restart / `force-new-deployment`, invalidating every `__Host-session` cookie → **silent mass
logout of all paying users** on the next deploy/restart. The runbook honestly notes
"sessions die on restart (ephemeral key)" but nothing enforces it.

## Fix
Add a production boot gate mirroring the bearer-token guard: in `boot.rs` `start()` (next to
the existing bearer/DB-key checks), when `is_production_mode_for_bind(bind)` AND
`SKYCOT_AUTH_JWT_SECRET` is unset/empty → return `Err` and refuse to boot. Treat the JWT
secret as the third must-have boot secret alongside bearer + DB key. (At absolute minimum, a
single hard `ERROR` — but fail-closed is the correct posture, matching the other two.)
Consider the same for the workspace-root/credentials-master key (narrower blast radius — it
503s the provider-key-save path rather than killing sessions; lower urgency, document if not gated).

## Verifier
- New unit/integration test: prod-mode bind + unset `SKYCOT_AUTH_JWT_SECRET` → `start()`
  returns `Err` (boot refused); with the secret set → boots. Mirror the existing bearer-token
  fail-closed test.
- `cargo test -p argo-cli <the boot-gate test>` green; non-prod (loopback) bind still boots
  without the secret (dev ergonomics preserved).

## Out of scope
- Changing the JWT/session scheme itself; only adding the production presence gate.

**LOC budget:** ~30–60 (the boot gate + a test).
