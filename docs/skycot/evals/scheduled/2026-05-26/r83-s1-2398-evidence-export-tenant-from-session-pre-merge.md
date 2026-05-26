# r83 S1 — PR #2398 codex-fix-evidence-bundle-exporter-tenant-from-session PRE-MERGE

- **Date**: 2026-05-26
- **Round**: 83
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (critical-path security: forged tenantId cannot bypass session JWT)
- **PR**: [#2398](https://github.com/mark-hallam/argo/pull/2398) (codex-authored, 416 additions / 52 deletions, 9 files)
- **Verdict**: 🟢 PASS (anti-theater dispositive on source-code reads alone — cold cargo not required to verify)

## Setup

Checked out PR branch `codex-fix-evidence-bundle-exporter-tenant-from-session` (head SHA `998ff2257`) in `/tmp/eval-r83-wt`.
Cross-read prod handler `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` (402 LOC, under 500 budget) against new cold integration test `crates/argo-cli/tests/audit_export_tenant_isolation.rs` (251 LOC). Diff scope: handler rewrites `tenant_id` resolution at 3 callsites (lines 71, 149, 199 — export / preview / download) via new helper `resolved_request_tenant_id(req)` at line 236.

## Observed

### Production change (load-bearing helper)

```rust
fn resolved_request_tenant_id(req: &str) -> String {
    session_tenant_id(req).unwrap_or_else(|| request_tenant_id(req))
}

fn session_tenant_id(req: &str) -> Option<String> {
    super::session_jwt::extract_session_claims(req)
        .and_then(|claims| claims.tnt)
        .filter(|tenant| !tenant.trim().is_empty())
}
```

Resolution order is **session JWT first, legacy `X-Argo-Tenant-Id` header second**. The legacy `request_tenant_id` path remains as fallback only when a valid session JWT is absent. Used at: `export_response` line 71, `preview_response` line 149, `download_response` `caller_tenant` line 199 (so tenant-mismatch existence-hide 404 at GET time now checks JWT-derived tenant too).

### Test forces the load-bearing assertion

The cold integration test boots a real gateway via `boot_gateway_with_home()` + `connect_encrypted_store` (SQLCipher), seeds two tenants with distinct row markers:

- tenant A: 3 audit rows tagged `tenant-a-marker`
- tenant B: 7 audit rows tagged `tenant-b-marker`

Mints a session JWT for tenant A only, then issues:

1. **own_path**: legitimate request with tenant A in query + JSON + header → `tenant_id == tenant_a` ✅
2. **no_param_body**: no tenantId param, no header, only JWT → `tenant_id == tenant_a` ✅
3. **forged_path**: forged `tenantId=B` in query, JSON, AND `X-Argo-Tenant-Id: B` header — but JWT still for tenant A. Asserts:
   - `tenant_id` field in response = tenant A (not B) ✅
   - `row_count = COUNTS_A.audit` (3, not 7) ✅
   - `download_url` signed with `tenant=A` ✅
   - Downloaded bundle bytes contain `tenant-a-marker` and do NOT contain `tenant-b-marker` ✅
4. **forged_preview_path**: same forged tenant on preview endpoint → row_count = 3, tenant_id = A ✅

## Anti-theater check

**Dispositive.** The test asserts against **data-driven markers** seeded into independent tenant rows — the strings `tenant-a-marker` and `tenant-b-marker` are baked into the bundle bytes by `seed_tenant_rows`. A test that "rewrites itself green" cannot satisfy this: the only way `ndjson.contains("tenant-a-marker") && !ndjson.contains("tenant-b-marker")` holds when the request explicitly forges tenant=B in every available channel (query, JSON body, header) is if the production handler actually pulled the JWT tenant. The negative assertion `!ndjson.contains("tenant-b-marker")` is the load-bearing one — it would fire on any partial fix where the handler reads from JSON or query.

Additionally, the test verifies `download_url.contains("tenant=A")` — the URL signing function `sign_export_url(&bundle_id, format, expires, &tenant_id)` consumes the resolved tenant, so a forged-tenant download URL would never decode against the JWT path. Belt-and-suspenders verified.

Helper `session_tenant_id` filters empty/whitespace tenants — a JWT with `tnt = ""` correctly falls back to the legacy header rather than masquerading as a missing tenant. Edge-case-correct.

## Gap / follow-up brief candidates

None for this PR — clean fix, real test, no surface drift. Recommend merge under the overnight Genesis/Track-P `--skip-clippy` window (operator-authorized 2026-05-25→26 per `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25`).

**Adjacent observation (not a finding, no new brief needed)**: the new helper duplicates a pattern that lives in many gateway routes (resolve-from-session-then-header). If a fourth tenant-binding fix lands soon, consider extracting `resolved_request_tenant_id` into the shared `session_jwt` module rather than copying it again. Not P0/P1 — only flag if pattern repeats a third time.
