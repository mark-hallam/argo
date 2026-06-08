# fix-work-audit-rows-tenant-roundtrip

**Owner**: claude.
**Authored against**: origin/main `d0ee42aee` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-work-audit-rows-tenant-roundtrip`.

## Why (evidence-grounded — live repro 2026-06-08)
The product's core promise is "every action emits a tamper-evident, READABLE audit row." Today a freshly-written work-path audit row does NOT round-trip into the tenant's audit feed, because the WRITE path and the READ path resolve the tenant differently:
- WRITE: `crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs:485` writes `PitchCanonicalQaMiss` / `PitchLlmCallEmitted` / `PitchTruthBoundaryEvaluated` via `AuditSink::new_with_tenant(store, rate_limit::extract_tenant(req))`. `extract_tenant` precedence: `?tenant=` query → `x-tenant-id`/`x-argo-tenant-id` header → fallback.
- READ: `GET /api/v1/audit` resolves scope via `routes_audit_feed_scope::resolve_tenant_scope` (honors `?tenant=` but NOT the `x-tenant-id` header, and uses a different default).
- Live proof: a `pitch/ask` call returns a real model answer (model=claude-sonnet-4-6) AND writes rows, but `GET /api/v1/audit` never surfaces them (demo-tenant feed's newest row is days old; `?tenant=<arbitrary>` returns 0). The rows are written to a scope the feed won't read back for the same caller.

## Grep-before-write
- `rg -n 'extract_tenant' crates/argo-cli/src/handlers/gateway/rate_limit.rs` (write-side resolver).
- `rg -n 'resolve_tenant_scope|row_in_tenant_scope' crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs` (read-side resolver).
- `rg -n 'resolve_tenant_for_request' crates/argo-cli/src/handlers/gateway/resolve_tenant.rs` (the CANONICAL resolver used by /workers, /team, etc. — precedence: `__Host-session` cookie `tnt` → explicit tenant header (no-cookie only) → cwd fallback).
- `rg -ln 'AuditSink::new_with_tenant\(.*extract_tenant' crates/argo-cli/src` (every work-path writer with the same mismatch — pitch, gtm draft, marketing draft, genesis gate, worker.run).

## Fix (first-principles: one tenant-resolution truth for a request)
Make audit WRITES and READS agree for the same principal/session. Prefer routing the audit write tenant through the **canonical `resolve_tenant_for_request`** (cookie `tnt` → header → cwd) so it matches what the feed reads, instead of the parallel `extract_tenant`. If `extract_tenant` must stay for rate-limiting, at minimum make `resolve_tenant_scope` and `extract_tenant` agree on precedence (including the session-JWT/cookie `tnt`). Apply consistently to ALL work-path audit writers found above. Do not weaken tenant isolation (a row must still be scoped to exactly one tenant; the goal is that the WRITE and READ pick the SAME one).

## Acceptance (truth-bounded — round-trip proof)
1. New integration/round-trip test: write a work-path audit row as tenant T (via the real handler), then `GET /api/v1/audit` resolved to T returns that exact row. Assert it for at least pitch (`PitchLlmCallEmitted`) and one other work-path writer.
2. Live (gateway :7400): a fresh `POST /api/v1/pitch/ask` followed by `GET /api/v1/audit` for the same caller surfaces the new row (paste the row).
3. Cross-tenant negative: tenant B's feed does NOT show tenant T's row.
4. `cargo test -p argo-cli` green; `cargo clippy -p argo-cli -D warnings` clean (or boundary-note the known argo-cli await-holding-lock pre-existing clippy issue).

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (audit readability/round-trip row) + regen HTML. Note it closes the conductor-flagged work-path audit round-trip gap.

## Effort
M (touches several write sites + the resolver seam + a round-trip test). High value: it's the product's core auditability promise.
