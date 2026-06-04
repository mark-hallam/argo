# sprint-ws1-2-wire-writes-to-session-jwt-tenant

**Owner**: claude or codex. **🔴 CRITICAL — completes the platform loop for a fresh user. The #1 sprint item after the keystone.**
**Depends on**: #2676 (MERGED — `resolve_tenant::resolve_tenant_for_request` now exists on main).
**Authored against**: origin/main `a0204a379` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-2-wire-writes-to-session-jwt-tenant`.

## Why

The keystone (#2676) migrated the 6 core **read** routes to resolve the tenant from the
`__Host-session` JWT. But the **write** paths still use `ensure_workspace(cwd)` — so a
signed-in user's mutations land in the gateway's cwd/demo tenant, NOT their own. The
adversarial review of #2676 confirmed this asymmetry as the top follow-up:

> "WRITE paths (`routes_post_writes_db.rs:124`, `routes_worker_run.rs:146`) still on
> `ensure_workspace(cwd)` → a signed-in customer's hired worker lands in the shared
> cwd/demo tenant while their now-JWT-aware /team and /workers reads show empty.
> Correctness regression and cwd-pooling."

**Net effect for a stranger who signs up:** they hire a teammate (write → cwd/demo
tenant) → open /team (read → their own empty JWT tenant) → **see nothing**. The platform
loop — the entire Friday bar — is broken until writes are JWT-scoped too. This is the
single most important sprint item right now.

## What

Migrate the write/mutation paths from `ensure_workspace(std::env::current_dir())` (and the
`cwd_write_tenant` shim in `routes_onboarding.rs`) to the **same** canonical resolver the
reads now use: `resolve_tenant::resolve_tenant_for_request` (added in #2676; read it first).

Confirmed write sites to migrate (re-verify + enumerate the full set at claim time with
`rg 'ensure_workspace|cwd_write_tenant|current_dir' crates/argo-cli/src/handlers/gateway/`):
- `routes_onboarding.rs` — **first-hire** (the `cwd_write_tenant` at ~:298) → write the hired worker to the user's provisioned tenant.
- `routes_worker_run.rs:146` — worker-run / dispatch.
- `routes_post_writes_db.rs:124` — generic POST writes.
- install-vertical + provider-key write paths (per the WS1 audit) if they still cwd-write.

The reads already key off `tenant_id` without `ensure_workspace` on the session path; mirror
that on the write side so a hire is written under the SAME tenant the user's reads resolve.
Preserve the anonymous/demo path (no session → cwd) so the demo walkthrough still works.

## May-touch
- `routes_onboarding.rs` (first-hire write), `routes_worker_run.rs`, `routes_post_writes_db.rs`, and any other cwd-write site the claim-time grep surfaces.
- `resolve_tenant.rs` ONLY if a write-oriented helper variant is genuinely needed (prefer reusing the existing fn).
- `crates/argo-cli/tests/` — the new loop-closure test (see acceptance).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT change the read routes (#2676 did those) or the resolver's precedence logic.

## Acceptance criteria
- [ ] **Loop-closure integration test (the load-bearing one):** provision user A → POST first-hire with A's session JWT → GET `/api/v1/team` with A's JWT returns the hired worker; a DIFFERENT user B's JWT does NOT see it; the cwd/demo tenant does NOT contain A's worker. Mirror the seeding style of `tenant_resolve_from_session_jwt.rs`.
- [ ] All migrated write sites resolve via `resolve_tenant_for_request` (grep shows no residual `ensure_workspace(current_dir)` / `cwd_write_tenant` on the session write path).
- [ ] Anonymous/demo write path unchanged (no session → cwd) so the demo still works.
- [ ] `cargo clippy -p argo-cli` clean; touched-route tests + the new test GREEN (report exact pass lines).
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.
- [ ] PR body lists every migrated write site + confirms the loop-closure test asserts A-sees-own-hire, B-does-not, cwd-does-not.

## Out of scope
- The ~50 unmigrated READ routes (WS1-1b long tail) — separate.
- The audit-header capability gate + JWT-secret boot guard (separate review follow-ups).

## Parallel-surface check
- `rg 'fn resolve_tenant_for_request' crates/` — reuse the canonical resolver from #2676; do NOT add a parallel write-resolver.

## Source
Adversarial review of #2676 (run `wf_02bda054-211`), follow-up #1. Audit: `docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS1-2).
