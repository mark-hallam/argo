# codex-prod-tenant-filter-audit-retros-replay

**Authored against**: `origin/main` `fa16fa94c` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-prod-tenant-filter-audit-retros-replay`.

**Fast-mode eligible**: yes — known callsite list, mechanical fix.

## Summary

**🚨 P1 — cross-tenant data exposure bug.** #1983 (`codex-test-coverage-cross-tenant-negative-cases`) surfaced 3 production handlers that lookup by id WITHOUT tenant filter, allowing tenant B to read tenant A's data:

1. `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs::fetch_row` — gateway `GET /audit/<id>` returns the row by id only.
2. `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs` — replay-audit lookup unscoped.
3. `crates/argo-store/src/retro_store.rs` `RetroStore::{update_sections, finalize, list_for_worker}` — retros operations unscoped.

This is the bug-class mission claim 6 (multi-tenant isolation) hinges on. Procurement reviewers will catch this.

## Acceptance

- [ ] Each of the 3 handlers gates lookups on the caller's `tenant_id` so a cross-tenant request returns `404 Not Found` (do NOT return `403` — that leaks existence; mirror the canonical pattern from #1702 / #1901).
- [ ] Add 5 cross-tenant negative tests deferred by #1983: one per handler (3 tests) + one for each retro operation that needed it (2 tests).
- [ ] Audit kind emitted for the rejected lookup? Optional — if `RbacDeny` is the canonical place, emit it. Otherwise skip (audit chain doesn't need cross-tenant noise).
- [ ] No semantic change for in-tenant requests (existing tests stay green).
- [ ] Pre-existing flakes declared per HARD RULE 2.

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs`
- `crates/argo-store/src/retro_store.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail_tests.rs` (extend)
- `crates/argo-cli/src/handlers/gateway/routes_audit_replay_tests.rs` (extend)
- `crates/argo-store/src/retro_store_tests.rs` (extend)

## Must NOT touch

- The audit-feed listing path (`routes_audit_feed.rs`) — already tenant-scoped per #1901.
- The audit memory row schema.
- Non-tenant-scoped surfaces (health, metrics).

## Notes

- Source: #1983 PR body — codex left these as PROD-GAPs with full diagnostics rather than silently patching.
- Pattern reference: `routes_workers_hire.rs` cross-tenant test from #1702; `routes_approvals_json.rs` from #1956.
- Sizing: 150–300 LOC. 3 surgical fixes + 5 tests.
- v1.0 launch hygiene + procurement-grade leverage; cross-tenant isolation is non-negotiable for enterprise sale.
