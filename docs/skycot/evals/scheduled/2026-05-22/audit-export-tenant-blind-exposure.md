# Eval: Audit-export — tenant-blind data exposure (confirmed real)

**Date**: 2026-05-22 → 2026-05-23 (round 3)
**Pillar**: Cross-pillar (mission claim 6 — multi-tenant by construction)
**Mission claim**: 6 — Multi-tenant by construction
**Verdict**: 🔴 **P0 PRODUCTION DATA EXPOSURE** — confirmed at the SQL layer. Three regression tests authored & merged `#[ignore]`'d pending fix-brief. Fix-brief not yet authored.

## Setup

PR [#2046](https://github.com/mark-hallam/Argo-Agent/pull/2046) — `test(gateway): cross-tenant negative for POST /api/v1/audit/export (mission-cross-tenant-test-audit-export)` — merged today as **MERGED-FINDINGS-ONLY**. The PR author (a Claude worker) wrote 3 regression tests, found the underlying handler is tenant-blind, marked the tests `#[ignore]`, and shipped the discovery as documentation. Per the ledger:

> `mission-cross-tenant-test-audit-export — #2046 MERGED-FINDINGS-ONLY (3 ignored regression tests; fix-brief pending)`

This scenario verifies the finding independently.

## Action

```bash
# 1. Read the ignored test bodies.
rg "#\[ignore\b" crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs

# 2. Read the row-fetcher in argo-audit.
grep -B 1 -A 16 'fn fetch_rows_for_export' crates/argo-audit/src/export_bundle.rs
```

## Expected

If the swarm's finding is real, then `fetch_rows_for_export` takes no `tenant_id` parameter and runs a global table scan.

## Observed

```rust
pub async fn fetch_rows_for_export(
    store: &SessionStore,
    window: &ExportWindow,
) -> ArgoResult<Vec<ExportedRow>> {
    let mut out = Vec::new();
    crate::export::export_ndjson(store, |row| {
        if window.matches(row.created_at_micros) {
            out.push(row.clone());
        }
        Ok(())
    })
    .await?;
    Ok(out)
}
```

**Confirmed.** Signature is `(store, window)`. No tenant filter. The closure consumes every row in the window across all tenants.

The three `#[ignore]`'d tests pin the contract that SHOULD hold:

```rust
#[tokio::test]
#[ignore = "BLOCKED: routes_audit_export.rs does not tenant-scope fetch_rows_for_export — see PR findings"]
async fn cross_tenant_export_excludes_other_tenant_rows() {
    let store = mem_store().await;
    seed_for_tenant(&store, "tenant-a", 3).await;  // 3 rows under tenant-a
    seed_for_tenant(&store, "tenant-b", 2).await;  // 2 rows under tenant-b
    let resp = export_response(
        &post_with_tenant("/api/v1/audit/export", "tenant-a", json!({"format": "json"})),
        Some(&store),
    ).await;
    assert_eq!(body["row_count"], 3, "tenant-a saw {} rows; tenant-b leaked", body["row_count"]);
    // ... + b64 download + per-row tenant_marker check ...
}
```

If the test were not ignored, it would fail with `row_count == 5` instead of 3. The 2 leaked rows would be tenant-b's audit data downloadable by tenant-a in a signed bundle.

## Anti-theater check

This is **not** a theoretical exposure:

1. The handler is mounted in the live gateway (per `routes_dispatch_post.rs` — `POST /api/v1/audit/export` is dispatched to `export_response`).
2. The bundle returns a signed `download_url` (`/api/v1/audit/export/<bundle_id>?sig=…`) so the rows persist on disk after the request returns.
3. `seed_for_tenant` writes a tenant marker into each row's JSON payload. Without tenant scoping, every row from every tenant ends up in tenant-a's bundle, identifiable by the `tenant_marker` field.

This is the same class of bug as the audit_detail one #1992 fixed two days ago, on a different code path. The fix shape should mirror #1992: add `AND tenant_id = ?` to the underlying SQL + bind the caller's resolved tenant.

## Why the swarm shipped this without a fix

`MERGED-FINDINGS-ONLY` is a deliberate pattern — the test author shipped the discovery + the ignored tests so future merges can't accidentally regress past this point. The fix is a separate brief because:

1. The fix touches `fetch_rows_for_export` (argo-audit crate) AND requires a tenant-resolution wire-up in the export route handler.
2. The PR was authored as a test-only brief per the cross-tenant coverage report's plan; growing into a fix would have exceeded the brief's may-touch list.

The brief queue note: "(3 ignored regression tests; fix-brief pending)". The conductor hasn't yet authored the fix-brief.

## Gap / follow-up brief candidates

**🔴 P0 — `mission-audit-export-tenant-scope-fix`**: Fix `fetch_rows_for_export` + `fetch_preview` in `crates/argo-audit/src/export_bundle.rs` to take a `tenant_id` parameter, and update `export_response` + `preview_response` in `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` to resolve + bind it. Acceptance:

- The 3 `#[ignore]` attributes are removed from `routes_audit_export_tests.rs`.
- All 3 tests pass.
- A second sanity test: cross-tenant signed download URL (i.e. tenant-a creates a bundle, tenant-b tries the signed URL) → 404 even with a valid signature.

**🟡 P1 — `mission-audit-detect-merged-findings-only-with-no-fix-brief`**: Add a `scripts/check-findings-only.sh` CI step that fails when `git log --grep='FINDINGS-ONLY'` finds a commit older than 48h without a matching merged fix-PR in the same family. Currently the ledger notes "fix-brief pending" but there's no enforcement.

**🟢 INFO — Marketing**: The Friday founder-post should note **one** of these audit-cross-tenant findings (any of #1983, #1992, or #2046) but NOT all three as separate wins, because they're parts of the same systematic sweep. The whole-of-system claim is true: a coordinated cross-tenant audit found multiple SQL-level data-exposure bugs and the fix loop is in motion.

## Operator-decisional

The `MERGED-FINDINGS-ONLY` pattern is sound — it pins the regression contract. But "fix-brief pending" needs a deadline. **Recommend**: every FINDINGS-ONLY merge must include a `fix-by` date in the commit message (e.g. "fix-by 2026-05-25"), and the conductor's tick must fail-loud if a FINDINGS-ONLY merge ages past its fix-by date without a fix-PR in IN_REVIEW or MERGED.

Without that timer, this exposure could sit at `MERGED-FINDINGS-ONLY` indefinitely. It's a P0 data-exposure on the v1 demo's audit-export surface — the operator's "audit chain on every action" claim against Zapier in the demo runbook (06:30) presumes the audit chain isn't leaking cross-tenant rows.
