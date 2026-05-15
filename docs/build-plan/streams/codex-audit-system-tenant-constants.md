# codex-audit-system-tenant-constants — Add SYSTEM/ANONYMOUS audit tenant constants + migrate background callers (F-005a)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f2a9f991` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-system-tenant-constants`.
**Source**: F-005a of `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. First half of the F-005 split (foundation); F-005b will do bulk migration + flip `AuditSink::new` to `cfg(test)`.

## Scope

P1 multi-tenancy. `AuditSink::new` defaults to `DEFAULT_AUDIT_TENANT_ID = "ws-default"` (`crates/argo-audit/src/sink.rs:52`), which leaks into production audit rows. The operator-decided fix is to flip `AuditSink::new` to `#[cfg(test)]` and migrate all 45 production callers to `new_with_tenant`.

Pre-flight revealed not every callsite has session/JWT tenant in scope:
- **Session/JWT callers** (~35): tenant already in local scope (route params or session). Trivial migration.
- **Background/system callers** (~10): periodic sweepers, system jobs, pre-auth login flows — DO NOT have a specific tenant. Examples: `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` (sweeps ALL tenants), `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` (pre-auth OIDC login), `crates/argo-cli/src/handlers/judge.rs` (admin CLI).

This brief is the FOUNDATION. F-005b depends on it MERGED.

Scope:
- Add `pub const SYSTEM_AUDIT_TENANT_ID: &str = "system"` to `crates/argo-audit/src/sink.rs` with rustdoc explaining when to use it (background sweepers, periodic system jobs, admin operations not bound to a tenant).
- Add `pub const ANONYMOUS_AUDIT_TENANT_ID: &str = "anonymous"` with rustdoc explaining when to use it (pre-auth flows where principal is not yet known: login/SSO/registration).
- Re-export both from `argo-audit` lib.
- Migrate ~10 background/system/pre-auth callers to use the new constants via `new_with_tenant`.
- DO NOT flip `AuditSink::new` to `cfg(test)` in this brief — that happens in F-005b after all callers are migrated.

LOC: ~280 (includes splitting sink.rs which is currently 520 LOC — over the 500 ceiling per F-012; can't add to it).

**Prerequisite split:** `crates/argo-audit/src/sink.rs` is currently 520 LOC. Adding 2 constants + their rustdoc pushes it further over the invariant. This brief MUST split sink.rs first into:
- `crates/argo-audit/src/sink.rs` (target ~300 LOC) — keep `AuditSink` struct + core write API
- `crates/argo-audit/src/sink_tenant.rs` (NEW, ~80 LOC) — `DEFAULT_AUDIT_TENANT_ID`, new `SYSTEM_AUDIT_TENANT_ID`, new `ANONYMOUS_AUDIT_TENANT_ID`, tenant-related helpers
- `crates/argo-audit/src/sink_broadcast.rs` (NEW IF NEEDED, only if sink.rs has a broadcast section that's ≥80 LOC and naturally cleaves) — narrowly: real-time-audit-stream broadcast helpers
- Keep all 3 modules behind unchanged `argo_audit::AuditSink` API (re-export from sink_tenant.rs and sink_broadcast.rs through sink.rs or lib.rs)

Pre-flight your split BEFORE adding the new constants — otherwise the LOC ceiling is the blocker.

## Files you may touch

- `crates/argo-audit/src/sink.rs` — refactor: extract tenant helpers + (optionally) broadcast helpers into siblings; keep `AuditSink` API stable. ≤120 modified lines.
- `crates/argo-audit/src/sink_tenant.rs` (NEW) — `DEFAULT_AUDIT_TENANT_ID` + `SYSTEM_AUDIT_TENANT_ID` + `ANONYMOUS_AUDIT_TENANT_ID` + rustdoc. ≤100 LOC.
- `crates/argo-audit/src/sink_broadcast.rs` (NEW, optional based on cleavage) — broadcast helpers from sink.rs. ≤120 LOC.
- `crates/argo-audit/src/lib.rs` — `pub mod sink_tenant;` + re-export the 2 new constants alongside the existing `DEFAULT_AUDIT_TENANT_ID`. ≤15 modified lines.
- Migrate these specific background/system/pre-auth callers (use `SYSTEM_AUDIT_TENANT_ID` or `ANONYMOUS_AUDIT_TENANT_ID` as appropriate per the rustdoc):
  - `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` — periodic sweeper across tenants → SYSTEM. ≤10 modified lines.
  - `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` — pre-auth OIDC login → ANONYMOUS. **NOTE: file is at 487 LOC (close to 500 ceiling); make the migration minimal.** ≤10 modified lines.
  - `crates/argo-cli/src/handlers/gateway/sso_saml_replay_store.rs` — pre-auth SAML → ANONYMOUS. ≤10 modified lines.
  - `crates/argo-cli/src/handlers/judge.rs` — admin operation → SYSTEM (or session tenant if available). ≤15 modified lines.
  - `crates/argo-audit/src/watchdog.rs` — internal audit watchdog → SYSTEM. ≤10 modified lines.
  - `crates/argo-audit/src/export.rs` — admin export → SYSTEM. ≤10 modified lines.
  - Any other callsite in `crates/argo-cli/src/handlers/` that does NOT have a tenant in local scope (use grep `rg -B 5 'AuditSink::new\(' crates/argo-cli/src/handlers/ --type rust` to identify). ≤30 modified lines combined for any additional background callers found.
- Test sibling `crates/argo-audit/src/sink_tenant_tests.rs` (NEW) — prove the 2 constants are stable and the migrated callers compile + emit the expected tenant. ≤80 LOC.

## Files you must NOT touch

- Session/JWT callers that already have tenant in scope — those are F-005b's scope (don't pre-empt).
- `AuditSink::new` itself — keep the function intact and PUBLIC. F-005b flips it to `#[cfg(test)]` after all production migration completes.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `SYSTEM_AUDIT_TENANT_ID` and `ANONYMOUS_AUDIT_TENANT_ID` constants exist in argo-audit + are re-exported.
- [ ] Rustdoc on each constant clearly states when to use it.
- [ ] The 6+ listed background/system/pre-auth callers all use `new_with_tenant` with the appropriate new constant.
- [ ] `AuditSink::new` remains public (not yet cfg-test — F-005b's job).
- [ ] `cargo test -p argo-audit -p argo-cli` passes.
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-audit-system-tenant-constants`
- PR title: `feat(audit): SYSTEM_AUDIT_TENANT_ID + ANONYMOUS_AUDIT_TENANT_ID + migrate background callers (codex-audit-system-tenant-constants)`

## Notes for the implementer

- Use the rustdoc to make the choice explicit at each callsite: SYSTEM for "operating across tenants or as the system", ANONYMOUS for "pre-authentication, principal not yet known". Future callers should grep for the constant they need.
- If you find a background caller that genuinely has a specific tenant available (e.g., the sweeper's per-row loop), prefer the actual tenant over SYSTEM. SYSTEM is the fallback when the operation is cross-tenant or tenant-less.
- F-005b will catch any callsites you miss — it deletes `AuditSink::new` from production. So don't worry about being exhaustive here; cover the obvious background/pre-auth ones.
