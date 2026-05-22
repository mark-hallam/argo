# mission-audit-export-tenant-scope-fix

**Owner**: claude.
**Depends on**: `mission-cross-tenant-test-audit-export` MERGED-FINDINGS-ONLY (✅ via #2046 — provides the 3 `#[ignore]`'d regression tests this brief un-ignores).
**Authored against**: origin/main `949f7996a` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-export-tenant-scope-fix`.

🔴 **P0 CROSS-TENANT DATA EXPOSURE.** Source: evaluator round-3 (`docs/skycot/evals/scheduled/2026-05-22/audit-export-tenant-blind-exposure.md`). `POST /api/v1/audit/export` is tenant-blind on all three surfaces:

1. **`POST /api/v1/audit/export`** + **`POST .../preview`** → call `fetch_rows_for_export` / `fetch_preview` which run a global `SELECT FROM audit_record` with **no tenant filter** (`argo-audit/src/export.rs`).
2. **`GET .../export/<id>?sig=…`** → HMAC validates over `bundle_id|format|expires` only, **no per-tenant binding** in the signed payload + no caller-tenant check before reading bytes.

A caller authenticated to tenant_a can request a signed bundle covering any time window and receive **every tenant's audit rows**. This is the same shape as #1983's audit_detail bug (closed via #1992), but on the export surface that never got the cross-tenant test.

PR #2046 shipped 3 `#[ignore]`'d regression tests + findings as MERGED-FINDINGS-ONLY. This brief un-ignores them and ships the actual fix.

## Scope

Apply the same pattern #1992 used for `audit_detail` to the export surface. Three changes:

1. **`fetch_rows_for_export(store, window)` → `fetch_rows_for_export(store, tenant_id, window)`** — add `tenant_id: TenantId` parameter. Update the SQL to `SELECT … FROM audit_record WHERE tenant_id = ? AND created_at >= ? AND created_at < ?`.
2. **`fetch_preview(store, window)` → `fetch_preview(store, tenant_id, window)`** — same shape.
3. **Bind tenant into the HMAC-signed payload** + **check caller-tenant on download**. The signed payload becomes `bundle_id|format|expires|tenant_id`. The `GET .../export/<id>?sig=...` handler extracts the caller's tenant (`request_tenant_id(req)`) and verifies it matches the signed `tenant_id` before reading bytes from disk. Mismatch → 404 (not 403 — same existence-hide pattern used by `routes_worker_skills` per #2040).

Plus: **un-ignore the 3 regression tests** from #2046. They should now pass.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-audit/src/export.rs` — narrowly scoped: extend `fetch_rows_for_export` + `fetch_preview` signatures with `tenant_id`. Update the SQL. Update the signed-payload format to include `tenant_id`.
- `crates/argo-audit/src/export_tests.rs` (or co-located) — narrowly scoped: extend existing tests to cover the new signature + add a cross-tenant negative.
- `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` — narrowly scoped: pass `request_tenant_id(req)` into the new signatures + verify it on download.
- `crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs` — un-ignore the 3 `#[ignore]`'d tests from #2046; add a 4th test for cross-tenant signed-URL download attempt.

## Files you must NOT touch

- The HMAC signing infrastructure outside `routes_audit_export.rs` — adding `tenant_id` to the payload is a small format change; don't refactor the signer.
- Any other audit-related crate (`argo-audit` outside `export.rs`).
- The signed-URL format on the wire (operators may have outstanding URLs; backward-compat the unsigned format if needed, but the new format MUST include tenant binding).
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `fetch_rows_for_export` + `fetch_preview` signatures include `tenant_id: TenantId`.
- [ ] All SQL `SELECT FROM audit_record` paths in `export.rs` filter by `tenant_id`.
- [ ] HMAC-signed payload now includes `tenant_id` (format: `bundle_id|format|expires|tenant_id`).
- [ ] `GET .../export/<id>?sig=...` extracts caller tenant via `request_tenant_id(req)` + verifies it matches the signed `tenant_id` before reading bytes. Mismatch → 404 (existence-hide pattern).
- [ ] **The 3 `#[ignore]`'d regression tests from #2046 are un-ignored** + all pass:
  - cross-tenant export download attempt → 404
  - cross-tenant export preview → empty
  - cross-tenant signed-URL download → 404
- [ ] **A 4th new test**: cross-tenant signed URL with tenant_a's bundle_id + tenant_b's signature → 404 (signature validation catches the cross-tenant manipulation).
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-audit` passes.
- [ ] `cargo test -p argo-cli routes_audit_export` passes (including all 4 cross-tenant tests).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (final sanity).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-audit-export-tenant-scope-fix`
- PR title: `audit: tenant-scope export + preview + signed-URL download (mission-audit-export-tenant-scope-fix)`

## Notes for the implementer

- ~200-300 LOC expected. Mostly signature plumbing + new SQL filters + un-ignoring the existing tests.
- **The 3 #[ignore]'d tests from #2046 are the spec**. If you un-ignore them and they pass without changing your impl, you got it. If they fail, the impl needs more work.
- **The signed-URL backward-compat question**: any existing signed URLs in the wild (e.g. cached in operator browsers) will fail to validate against the new payload format. For a pre-launch system this is fine — no real users. The brief's PR body should explicitly say "no backward-compat needed; pre-launch". If you find evidence that we DO need backward-compat (e.g. signed URLs persisted in the DB), flag in the PR body and split into a 2-PR sequence (first PR accepts both old + new format; second PR removes old after a grace window).
- **The 404 vs 403 choice**: existence-hide (404) is the standard SKYCOT pattern post-#1992. #2040 also used 404 on cross-tenant skill removal. Don't deviate.
- **Process suggestion from the evaluator**: every `MERGED-FINDINGS-ONLY` commit should carry a `fix-by <date>` line. The conductor's tick should fail-loud if a FINDINGS-ONLY merge passes its fix-by without a fix-PR. This brief itself is the example — landed within 30 min of the evaluator's recommendation. Worth memorializing as a small conductor-tooling brief in a follow-up.
