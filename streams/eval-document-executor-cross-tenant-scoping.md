# eval-document-executor-cross-tenant-scoping

**Authored against:** `origin/main` e03680c99 on 2026-05-24 (conductor tick-131).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-document-executor-cross-tenant-scoping`
**Owner:** codex
**Priority:** 🟡 P2 — CLAIM-6 substrate (multi-tenant by construction; document-executor 6 FINDINGS pinned by #2127)
**Source:** `docs/skycot/audit/2026-05-23-eval-document-executor-cross-tenant-scoping.md` (audit shipped with 6 P2 FINDINGS) — the FINDINGS exist but no fix-brief has been authored to act on them yet.

## What to do

Read the existing audit report (`docs/skycot/audit/2026-05-23-eval-document-executor-cross-tenant-scoping.md` — PR #2127 merge sha `7659f923a`). It pinned 6 wire-gaps as FINDINGS-ONLY. This brief takes ONE of them — **F-1: no `TenantId` on `DocExecutor::run` / `MsGraphExecutor::create` / `DocumentCsv::write` / `DocumentGsuite::write` / `OutputPath::resolve`** — and turns it into a contract pin (without yet shipping the fix; that's a follow-up `mission-*` brief).

1. Read the 5 functions named in F-1. Confirm their current signatures (no `TenantId` argument; or one with the tenant elided to `TenantId::default()`/empty).
2. Write `crates/argo-executor-document/tests/cross_tenant_scoping_contract.rs` (or equivalent — pick the right home crate). The test should:
   - **Pin the current (broken) shape**: assert each function's current signature so any rename or arg-add fails this test loudly. Sibling pattern: `surface_contract.rs` tests.
   - **Add a `#[ignore]`'d FINDINGS-ONLY test** that, when un-ignored, would assert two tenants writing to the same logical filename produce **distinct on-disk paths** (`<root>/tenant-a/…` vs `<root>/tenant-b/…`).
3. Update the audit report with a Δ-since-pin section if anything has changed on origin/main since 2026-05-23.

## May-touch

- `crates/argo-executor-document/tests/cross_tenant_scoping_contract.rs` (new — pick the actual crate path; `argo-executor-document` may live under a different name; consult `Cargo.toml`)
- `docs/skycot/audit/2026-05-23-eval-document-executor-cross-tenant-scoping.md` (Δ-since-pin section append-only)
- No production source touched (audit-only).

## Acceptance

- Contract-pin test passes.
- FINDINGS-ONLY `#[ignore]`'d test compiles (proves the desired post-fix shape).
- Per-crate clippy clean.
- Workspace clippy clean.
- Δ section appended to report if anything has changed.

## LOC budget

~150-300 LOC (test + report Δ).

## Follow-ups (NOT this brief)

Once this brief lands, queue `mission-document-executor-tenant-stamp` to actually ship the fix (add `TenantId` to all 5 functions, route through `OutputPath::resolve(tenant, …)`, un-ignore the FINDINGS test). That fix-brief is the load-bearing one — this brief is the surface lock that prevents future drift.
