# codex-audit-tenant-bulk-migrate-and-deprecate — Migrate remaining session/JWT callers + flip AuditSink::new to cfg(test) (F-005b)

**Owner**: codex. **Depends on**: `codex-audit-system-tenant-constants` MERGED (F-005a).
**Authored against**: origin/main `f2a9f991` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-tenant-bulk-migrate-and-deprecate`. ALSO verify F-005a is MERGED before claiming.
**Source**: F-005b of `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. Second half of F-005 split; F-005a established the SYSTEM/ANONYMOUS tenant constants + migrated background callers.

## Scope

🔴 P1 multi-tenancy — final fix of the `ws-default` audit-tenant leak. F-005a migrated background/system/pre-auth callers using new `SYSTEM_AUDIT_TENANT_ID` / `ANONYMOUS_AUDIT_TENANT_ID` constants. This brief migrates the remaining ~35 session/JWT callers (where the real tenant is in local scope) and flips `AuditSink::new` to `#[cfg(test)]` to make the unsafe default unconstructable in production.

Operator decision Q3 2026-05-15: "rip the band-aid — `#[cfg(test)]` immediately."

Scope:
- For every remaining production `AuditSink::new(store)` callsite in `crates/argo-cli/src/handlers/`: replace with `AuditSink::new_with_tenant(store, tenant)` where `tenant` is the session/JWT tenant already in local scope. Typical pattern: route handlers extract `claims.tnt` from the JWT or have a `tenant` variable from the URL/session — use that.
- For `crates/argo-agent-runner/src/turn.rs:214`: use `session.tenant_id` (already recorded at `:173` per the audit).
- For `crates/argo-agent-runner/src/turn_canary.rs` and `crates/argo-audit/src/lib.rs`: identify the appropriate tenant source per callsite.
- After ALL production migrations: add `#[cfg(test)]` to `AuditSink::new`. Production code no longer compiles if it calls the bare `new`.
- Add a regression test: confirm `AuditSink::new` is unreachable from non-test code (compile-time check via doctest or similar pattern).

LOC: ~320.

## Files you may touch

- `crates/argo-audit/src/sink.rs` — add `#[cfg(test)]` to `AuditSink::new`; keep `new_with_tenant` public. **NOTE: F-005a will have split sink.rs (was 520 LOC, target ~300); confirm the split landed before claiming.** ≤20 modified lines.
- `crates/argo-agent-runner/src/turn.rs` — `:214` callsite → use `session.tenant_id`. ≤10 modified lines.
- `crates/argo-agent-runner/src/turn_canary.rs` — migrate. ≤10 modified lines.
- `crates/argo-cli/src/handlers/` (all gateway route + handler files that call `AuditSink::new`): migrate to `new_with_tenant` using the tenant already in local scope. Run `rg -l 'AuditSink::new\(' crates/argo-cli/src/handlers/ --type rust | grep -v '_tests.rs'` to identify all remaining sites after F-005a. **Each file ≤15 modified lines on average** (the change per callsite is typically 1-2 lines: just append the tenant arg).
- `crates/argo-audit/src/lib.rs` — narrowly: handle any internal `AuditSink::new` callsites or doc-examples. ≤15 modified lines.
- New regression test in `crates/argo-audit/src/sink_cfg_test_only_tests.rs` (NEW) — compile-time proof that production code can't reach `AuditSink::new`. ≤40 LOC.

## Files you must NOT touch

- `crates/argo-audit/src/watchdog.rs`, `export.rs`, and other files migrated by F-005a — leave their migrations alone.
- The background/system/pre-auth callers already handled by F-005a.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `AuditSink::new` has `#[cfg(test)]` attribute.
- [ ] `rg 'AuditSink::new\(' crates --type rust | grep -v '_tests.rs\|/tests/\|#\[cfg(test)\]'` returns ZERO hits.
- [ ] Every former production caller now passes an explicit tenant via `new_with_tenant`.
- [ ] Regression test proves `AuditSink::new` is not callable from production.
- [ ] No new use of `DEFAULT_AUDIT_TENANT_ID` ("ws-default") in production code.
- [ ] `cargo test -p argo-audit -p argo-cli -p argo-agent-runner` passes.
- [ ] `cargo clippy -p argo-audit -p argo-cli -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] PR body explicitly notes this resolves F-005 P1 per the codex audit.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-audit-tenant-bulk-migrate-and-deprecate`
- PR title: `refactor(audit): migrate remaining callers to new_with_tenant; flip new() to cfg(test) (codex-audit-tenant-bulk-migrate-and-deprecate)`

## Notes for the implementer

- This is a mechanical migration but high-touch (~35 callsites). Pattern at each site is identical: find the tenant already in local scope (route param, JWT claim, session, or worker context), pass it as the 2nd arg to `new_with_tenant`.
- If you find a callsite that DOES NOT have a tenant in scope and is NOT covered by F-005a's background/anonymous constants, ESCALATE rather than guess. The fix is either: (a) plumb the missing tenant in (might be a separate F-004-shaped problem), or (b) use SYSTEM/ANONYMOUS if appropriate.
- The `#[cfg(test)]` flip is the LAST step. Don't flip until all production callers compile against `new_with_tenant`.
- After this lands, F-005 is fully resolved. The audit's "tenant rows stamped ws-default" failure mode is structurally impossible.
