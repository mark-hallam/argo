# codex-audit-tenant-bulk-migrate-and-deprecate — Migrate remaining session/JWT callers + #[deprecated] AuditSink::new (F-005b)

**Owner**: codex. **Depends on**: `codex-audit-system-tenant-constants` MERGED (F-005a).
**Authored against**: origin/main `f2a9f991` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-tenant-bulk-migrate-and-deprecate`. ALSO verify F-005a is MERGED before claiming.
**Source**: F-005b of `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. Second half of F-005 split; F-005a established the SYSTEM/ANONYMOUS tenant constants + migrated background callers.

## Scope

🔴 P1 multi-tenancy — final fix of the `ws-default` audit-tenant leak. F-005a migrated background/system/pre-auth callers using new `SYSTEM_AUDIT_TENANT_ID` / `ANONYMOUS_AUDIT_TENANT_ID` constants. This brief migrates the remaining ~35 session/JWT callers (where the real tenant is in local scope) and gates `AuditSink::new` with `#[deprecated]` to make the unsafe default fail under CI's `-D warnings`.

**Mechanism revision 2026-05-16:** Operator Q3 said "rip the band-aid — cfg(test) immediately." Codex pre-claim correctly identified that `#[cfg(test)]` is Rust-semantically impossible here — it only enables the function for the SAME crate's tests, not for downstream-crate tests. 19 downstream test files across 6 crates legitimately call `AuditSink::new` as fixtures (argo-cli/tests/*, argo-agent-runner/tests/canary_routing.rs, argo-core/src/tests.rs, argo-orchestrator/src/walker_tests.rs, argo-team-headline/src/lib_tests.rs, argo-audit/benches/audit_write.rs).

The Rust-idiomatic equivalent that achieves the same outcome is `#[deprecated(note = "production must use new_with_tenant; new() retained for test fixtures only")]`:
- Production callers emit deprecation warnings → fail under CI's `-D warnings` clippy gate.
- Test callers can opt-in via `#[allow(deprecated)]` at the call site or module level (1 line per test file).
- The unsafe default is structurally still callable but loudly flagged.

This preserves operator intent (production cannot silently use the unsafe default) while respecting Rust's actual `cfg(test)` propagation semantics. Operator may swap to a stricter mechanism (e.g., move `AuditSink::new` into a `pub(crate)` test-helper module + remove from public API) in a follow-up if the deprecation warning isn't strong enough.

Scope:
- For every remaining production `AuditSink::new(store)` callsite in `crates/argo-cli/src/handlers/`: replace with `AuditSink::new_with_tenant(store, tenant)` where `tenant` is the session/JWT tenant already in local scope. Typical pattern: route handlers extract `claims.tnt` from the JWT or have a `tenant` variable from the URL/session — use that.
- For `crates/argo-agent-runner/src/turn.rs:214`: use `session.tenant_id` (already recorded at `:173` per the audit).
- For `crates/argo-agent-runner/src/turn_canary.rs` and `crates/argo-audit/src/lib.rs`: identify the appropriate tenant source per callsite.
- After ALL production migrations: add `#[deprecated(note = "...")]` to `AuditSink::new`. Production code emits warning; CI's `-D warnings` fails.
- Add `#[allow(deprecated)]` to each downstream test caller (19 files, ~1 line each) so test compile is preserved.
- Add a regression test: a grep-based CI check that `AuditSink::new(` appears in zero production paths (excluding `_tests.rs` / `/tests/` / `/benches/`). Add to `scripts/check-brief-freshness.sh` companion script or a new `scripts/lint-no-audit-sink-new.sh`.

LOC: ~380 (bumped from 320 to account for 19 #[allow(deprecated)] additions).

## Files you may touch

**Production migration (~30 callers across 2 crates):**
- `crates/argo-audit/src/sink.rs` OR `crates/argo-audit/src/sink_tenant.rs` (whichever holds `AuditSink::new` post-F-005a split) — add `#[deprecated(note = "...")]` to `AuditSink::new`; keep `new_with_tenant` public. ≤25 modified lines.
- `crates/argo-agent-runner/src/turn.rs` — `:214` callsite → use `session.tenant_id`. ≤10 modified lines.
- `crates/argo-agent-runner/src/turn_canary.rs` — migrate. ≤10 modified lines.
- `crates/argo-cli/src/handlers/` (all gateway route + handler files that call `AuditSink::new`): migrate to `new_with_tenant` using the tenant already in local scope. Run `rg -l 'AuditSink::new\(' crates/argo-cli/src/handlers/ --type rust | grep -v '_tests.rs'` to identify all remaining sites after F-005a. **Each file ≤15 modified lines on average** (the change per callsite is typically 1-2 lines: just append the tenant arg).
- `crates/argo-audit/src/lib.rs` — narrowly: handle any internal `AuditSink::new` callsites or doc-examples. ≤15 modified lines.

**Downstream test compat (~19 test files — add `#[allow(deprecated)]`):**
- `crates/argo-cli/tests/activity_tail_smoke.rs`, `eval_skill_honesty_smoke.rs`, `recover_smoke.rs`, `session_capability_gaps.rs`, `vertical_isolation_e2e.rs`, `worker_restart_smoke.rs` — add `#[allow(deprecated)]` at fn or module level. ≤3 lines each (~18 lines total).
- `crates/argo-agent-runner/src/turn_budget_tests.rs`, `turn_tests_max_rounds.rs` — `#[allow(deprecated)]`. ≤6 lines combined.
- `crates/argo-agent-runner/tests/canary_routing.rs` — `#[allow(deprecated)]`. ≤3 lines.
- `crates/argo-audit/src/archive_tests.rs`, `filter_dims_tests.rs`, `gap_tests.rs`, `tests.rs` — `#[allow(deprecated)]`. ≤12 lines combined.
- `crates/argo-audit/tests/replay_roundtrip.rs` — `#[allow(deprecated)]`. ≤3 lines.
- `crates/argo-audit/benches/audit_write.rs` — `#[allow(deprecated)]`. ≤3 lines.
- `crates/argo-cli/src/handlers/audit_by_vertical_tests.rs`, `gateway/routes_audit_detail_tests.rs`, `gateway/routes_audit_feed_tests.rs`, `gateway/routes_worker_activity_tests.rs`, `gateway/routes_worker_overview_tests.rs` — `#[allow(deprecated)]`. ≤15 lines combined.
- `crates/argo-core/src/tests.rs`, `argo-orchestrator/src/walker_tests.rs`, `argo-team-headline/src/lib_tests.rs` — `#[allow(deprecated)]`. ≤9 lines combined.

**Regression lint:**
- `scripts/lint-no-audit-sink-new-in-production.sh` (NEW) — small bash/grep script that fails if `AuditSink::new(` appears in any file NOT matching `_tests.rs|/tests/|/benches/|#\[cfg\(test\)\]`. Run-able standalone; future CI invocation TBD. ≤30 LOC.
- Optionally extend `scripts/check-brief-freshness.sh` to invoke the new lint as part of its check sweep (≤10 modified lines).

## Files you must NOT touch

- `crates/argo-audit/src/watchdog.rs`, `export.rs`, and other files migrated by F-005a — leave their migrations alone.
- The background/system/pre-auth callers already handled by F-005a.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `AuditSink::new` has `#[deprecated(note = "...")]` attribute pointing to `new_with_tenant`.
- [ ] `rg 'AuditSink::new\(' crates --type rust | grep -v '_tests.rs\|/tests/\|/benches/'` returns ZERO production hits.
- [ ] Every former production caller now passes an explicit tenant via `new_with_tenant`.
- [ ] Downstream test callers have `#[allow(deprecated)]` so they still compile.
- [ ] `scripts/lint-no-audit-sink-new-in-production.sh` exists and exits 0 on current main; would exit 1 if production code introduced a new `AuditSink::new` call.
- [ ] No new use of `DEFAULT_AUDIT_TENANT_ID` ("ws-default") in production code.
- [ ] `cargo test -p argo-audit -p argo-cli -p argo-agent-runner -p argo-core -p argo-orchestrator -p argo-team-headline` passes.
- [ ] `cargo clippy -p argo-audit -p argo-cli -p argo-agent-runner --all-targets -- -D warnings` clean (deprecation warnings turn into errors → forces production migration).
- [ ] PR body explicitly notes this resolves F-005 P1 per the codex audit AND explains the `#[cfg(test)]` → `#[deprecated]` mechanism revision.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-audit-tenant-bulk-migrate-and-deprecate`
- PR title: `refactor(audit): migrate remaining callers to new_with_tenant; flip new() to cfg(test) (codex-audit-tenant-bulk-migrate-and-deprecate)`

## Notes for the implementer

- This is a mechanical migration but high-touch (~35 callsites). Pattern at each site is identical: find the tenant already in local scope (route param, JWT claim, session, or worker context), pass it as the 2nd arg to `new_with_tenant`.
- If you find a callsite that DOES NOT have a tenant in scope and is NOT covered by F-005a's background/anonymous constants, ESCALATE rather than guess. The fix is either: (a) plumb the missing tenant in (might be a separate F-004-shaped problem), or (b) use SYSTEM/ANONYMOUS if appropriate.
- The `#[cfg(test)]` flip is the LAST step. Don't flip until all production callers compile against `new_with_tenant`.
- After this lands, F-005 is fully resolved. The audit's "tenant rows stamped ws-default" failure mode is structurally impossible.
