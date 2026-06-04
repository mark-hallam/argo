# sprint-ws1-tenant-resolve-from-session-jwt-not-cwd

**Owner**: claude or codex.
**Depends on**: none. **KEYSTONE — unblocks WS1-2/3/4, WS2-1, and the whole "fresh private workspace" goal.**
**Authored against**: origin/main `35d9225d2` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-tenant-resolve-from-session-jwt-not-cwd`.

## Why

Day-0 gap-audit (`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md`, WS1)
found the single biggest blocker to a public self-serve product: **every
signed-in user is resolved to the gateway process's working-directory tenant
(the seeded demo workspace), not their own.**

Two parallel tenant-resolution systems exist and don't connect (the in-code
"ghost worker" bug, commented at `routes_onboarding_helpers.rs:19-40` +
`routes_onboarding.rs:286-294`):

1. **User-bound tenant** — `argo_identity::tenant_provision::provision_for_user`
   correctly creates a per-user `tenant` + `workspace` + `user_tenant(role=owner)`
   with `root_path = tenant://<id>/<ws>`. The JWT mint path
   (`session_jwt.rs`) already threads this `tenant_id` into the `__Host-session`
   cookie.
2. **CWD tenant** — `ensure_workspace(std::env::current_dir())`. ~59 gateway
   route files resolve tenant this way via a `resolve_workspace_tenant` helper
   duplicated 10+ times (`usage_endpoint.rs:64`, `routes_team_list.rs`,
   `routes_cohorts.rs`, `routes_worker_*.rs`, …).

Net effect: every read surface ignores the signed-in user's JWT tenant and
queries the ONE cwd tenant (the demo workspace). A stranger who signs up lands
in shared demo data instead of an empty private workspace. This is the spine of
the paying-ready goal — without it there is no real multi-tenant product.

## What

Introduce ONE canonical resolver that reads the user-bound tenant from the
`__Host-session` JWT, falling back to the cwd/anonymous tenant **only when there
is no session** (demo-bypass + truly-anonymous public endpoints keep working).
Then migrate the **core read routes** to it. The long tail of ~59 call sites is
explicitly a follow-up (WS1-1b), so this PR stays focused and under the LOC
ceiling.

### Step 1 — the canonical resolver (the unlock)
- Add `resolve_tenant_for_request(req, store) -> WorkspaceTenant` in a single new
  shared module (sibling to the existing `rate_limit::extract_tenant` pattern —
  re-verify the exact home at claim time with
  `rg 'fn resolve_workspace_tenant' crates/argo-cli/src/handlers/gateway/`).
  Order of precedence:
  1. If the request carries a valid `__Host-session` JWT with a `tenant_id`
     claim → resolve THAT tenant's workspace row. **This is the new behavior.**
  2. Else explicit `?tenant=` / `X-Tenant-Id` / `X-Argo-Tenant-Id` header
     (preserve the existing audit-feed contract from #2589 — do not regress it).
  3. Else the cwd/config tenant (`ensure_workspace(current_dir)`) — the
     demo/anonymous fallback, unchanged.
- **Reconcile the `root_path` convention (folds in WS1-5).** `provision_for_user`
  writes `root_path = tenant://<id>/<ws>` which `ensure_workspace` (built for
  filesystem paths) can't match. The resolver MUST look up the provisioned
  workspace row by `tenant_id` directly (the `user_tenant`/`workspace` join),
  NOT by trying to `ensure_workspace` the virtual `tenant://` path. Confirm the
  exact lookup at claim time by reading `tenant_provision.rs` +
  `argo-store` workspace queries. If reads and writes disagree on which
  `workspace` row a tenant owns, the fix is incomplete.

### Step 2 — migrate the CORE read routes only
Migrate these (the camera-visible, paying-ready-critical reads) from
`resolve_workspace_tenant`/cwd to `resolve_tenant_for_request`:
- `/api/v1/team` (`routes_team_list.rs`)
- `/api/v1/workers` + `/workers/{id}/*` overview (`routes_worker_*.rs`)
- `/api/v1/audit` feed + detail (re-verify it already honors the header chain)
- `/api/v1/skills`
- `/api/v1/approvals`
- `/api/v1/tenant/usage` (`usage_endpoint.rs`)

Leave the remaining ~50 call sites for **WS1-1b** (BLOCKED follow-up row). List
in the PR body exactly which routes you migrated and which you deferred.

## May-touch
- NEW shared resolver module under `crates/argo-cli/src/handlers/gateway/` (name per claim-time grep; do not create a second one if a near-named helper already exists — extend it).
- The 6 core read-route files listed above.
- `crates/argo-cli/tests/` — a new integration test (see acceptance).
- `docs/build-plan/BUILD_STATUS.md` (mandatory ledger flip on PR open) + the WS1-1b follow-up row flip TODO if you fully scope it.

Do NOT touch: write routes (`first-hire`, install-vertical, provider-key) — that
is WS1-2. Do NOT migrate the long-tail read routes — that is WS1-1b.

## Acceptance criteria
- [ ] New resolver returns the **JWT-claim tenant** when a valid `__Host-session`
      is present; returns the cwd tenant when no session (demo/anon path
      unchanged). Unit test pins both branches.
- [ ] Integration test (new, in `crates/argo-cli/tests/`): provision two users
      (A, B) → each gets their own tenant → a request bearing A's session JWT to
      `/api/v1/team` returns A's roster (empty for a fresh A), NOT the demo
      roster, and never B's. Mirror the seeding style of existing cross-tenant
      tests.
- [ ] The 6 core read routes resolve via the new helper (grep shows no residual
      `resolve_workspace_tenant`/cwd call in those 6 files).
- [ ] Demo-live + anonymous paths still work (no session → cwd tenant → demo
      data still renders for the demo walkthrough). Confirm with a no-cookie
      curl against `/api/v1/team` returning the demo roster.
- [ ] `cargo clippy -p argo-cli` clean; `cargo test -p argo-cli` for the touched
      route + the new integration test GREEN (report exact pass lines + wall-clock).
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.
- [ ] PR body lists migrated-vs-deferred routes and confirms the `root_path`
      reconciliation approach taken (with the file/line evidence).

## Out of scope (do NOT widen)
- The ~50 long-tail read-route call sites → **WS1-1b** follow-up.
- Write routes (first-hire/install/provider-key) → **WS1-2**.
- Auto-provisioning timing (verify vs forced onboarding) → **WS1-3**.
- The fresh-tenant baseline seed → **WS2-1**.

## Parallel-surface check
- `rg 'fn resolve_workspace_tenant' crates/` — enumerate the duplicate copies at
  claim time; the brief extends/replaces with ONE canonical helper (do not add a
  parallel resolver alongside them).
- `rg 'fn resolve_tenant_for_request' crates/` MUST return 0 at claim time (no
  in-flight parallel resolver). If it returns a hit, another worker is mid-flight
  — coordinate, don't duplicate.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS1). Playbook:
`docs/skycot/SPRINT-paying-ready-2026-06-04.md`.
