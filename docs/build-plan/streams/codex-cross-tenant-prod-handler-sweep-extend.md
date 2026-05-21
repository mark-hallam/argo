# codex-cross-tenant-prod-handler-sweep-extend

**Authored against**: `origin/main` `842a6e2e4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cross-tenant-prod-handler-sweep-extend`.

**Fast-mode eligible**: yes — defensive sweep extending #1983's findings.

## Summary

#1983 audited gateway integration tests and surfaced **3 confirmed cross-tenant data exposure bugs** in production code (audit_detail, audit_replay, retros). The follow-up brief `codex-prod-tenant-filter-audit-retros-replay` (TODO) fixes those 3. This brief extends the audit to ALL production handlers — find any other unscoped lookups before procurement does.

Pre-condition: **`codex-prod-tenant-filter-audit-retros-replay` MERGED** OR claimed (so the known 3 don't show up again).

## Acceptance

- [ ] `rg -n 'pub.*fn.*fetch|pub.*fn.*get_by_id|pub.*fn.*find' crates/argo-store/src/ crates/argo-cli/src/handlers/gateway/` — enumerate every lookup function.
- [ ] For each, classify:
  - **TENANT-SCOPED-OK** — function signature takes `tenant_id` as a required param AND uses it in the WHERE clause.
  - **TENANT-SCOPED-OPTIONAL** — function takes `tenant_id` as `Option<TenantId>` — verify callsites always pass `Some(t)` from a known tenant.
  - **UNSCOPED-OK** — intentionally unscoped (system tables, migration registry).
  - **UNSCOPED-BUG** — multi-tenant entity, no tenant filter. Cross-tenant data exposure.
- [ ] PR body table: crate | file:line | function | classification | note.
- [ ] For each UNSCOPED-BUG: fix in-brief if ≤3 instances (cargo clippy + integration test pattern from #1956). Larger gaps spawn follow-up briefs.
- [ ] Each fix adds a cross-tenant negative test mirroring #1983's pattern.
- [ ] No semantic change for in-tenant queries.

## May touch

- Production handler files where a fix is genuinely additive.
- New cross-tenant negative tests in `*_tests.rs` files.
- Store-layer functions where `tenant_id` needs to be a required arg.

## Must NOT touch

- The known 3 handlers already covered by the pending brief (`audit_detail`, `audit_replay`, `retros`) — those land via the other brief.
- Tests audited by #1983 (settled).
- The audit memory row schema.

## Notes

- Pre-flight: pull the latest after `codex-prod-tenant-filter-audit-retros-replay` lands so the known-3 are off the list.
- Sizing: 200-500 LOC depending on how many additional bugs surface.
- Procurement-grade leverage: comprehensive tenant-isolation coverage is what enterprise buyers ask for in security reviews.
