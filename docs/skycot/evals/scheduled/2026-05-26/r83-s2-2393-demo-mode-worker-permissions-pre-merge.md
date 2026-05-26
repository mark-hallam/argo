# r83 S2 — PR #2393 codex/fix-gateway-demo-mode-worker-permissions-grant-all PRE-MERGE

- **Date**: 2026-05-26
- **Round**: 83
- **Pillar / Claim**: Claim 6 (multi-tenant + RBAC) — demo-mode investor-safe permission grant + new audit-kind for override visibility
- **PR**: [#2393](https://github.com/mark-hallam/argo/pull/2393) (codex-authored, 286 additions / 3 deletions, 6 files)
- **Verdict**: 🟢 PASS (anti-theater dispositive via 3 negative gates in the new cold test + audit-kind wired in all 4 required places)

## Setup

Checked out PR branch `codex/fix-gateway-demo-mode-worker-permissions-grant-all` (head SHA `738ff9a79`). Source diff scope:

- `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs` (+158/-2) — adds `compute_permissions_demo`, `demo_permission_override_allowed`, `demo_principal_for_request`, `demo_principal_from_token`, `demo_mode_enabled_for_permissions`, `emit_demo_override_audit`
- `crates/argo-cli/src/handlers/gateway/routes_worker_overview_tests_data.rs` (+119/-0) — new test `compute_permissions_demo_grants_all_only_for_demo_principal` at line 301
- `crates/argo-audit/src/event_kind.rs` (+4/-0) — new enum variant `PermissionDemoOverrideGranted`
- `crates/argo-audit/src/event.rs` (+3/-0) — label + parse arms
- `crates/argo-audit/src/event_tests.rs` (+1/-1) — count test updated

## Observed

### Production gate logic (the AND-chain)

`demo_permission_override_allowed(demo_mode_enabled, header_principal, demo_principal)` requires ALL of:

1. `ARGO_DEMO_MODE=1` env var set (`demo_mode_enabled_for_permissions`)
2. A valid demo JWT present in `Authorization: Bearer` or session cookie (`demo_principal_for_request` → `demo_principal_from_token` validates HS256 signature, requires `demo: true` claim, lowercase-canonical email)
3. EITHER no `X-Argo-Principal` header, OR header matches the demo JWT's derived `human:{email}` principal_id (line 213-215 — `header_principal.map(|p| p == demo_principal.principal_id).unwrap_or(true)`)

When ALL three hold, `compute_permissions_demo` returns the 7-field all-true `Permissions` struct + a `demo_override_granted = true` flag. Otherwise it falls through to the existing `resolve_permissions(role, is_hire_author)` role-based path.

When the override fires, `emit_demo_override_audit(store, tenant, &worker, &actor)` writes an `AuditKind::PermissionDemoOverrideGranted` row with payload `{ principal_id, tenant_id, worker_id, permissions: WORKER_OVERVIEW_PERMISSION_NAMES }`. Separate from the always-fired `worker.overview.viewed` audit — both rows land per demo override request.

### Test exercises 3 negative gates

The new test (`compute_permissions_demo_grants_all_only_for_demo_principal`) verifies:

1. **Positive case** (DEMO_MODE=1 + valid demo JWT + matching X-Argo-Principal): all 7 permissions = true ✅
2. **Audit row written**: SQL query against `audit_record WHERE kind = 'permission.demo_override_granted'` confirms `principal_id`, `worker_id`, all 7 permission-name strings present in payload ✅
3. **Negative — no demo JWT**: same principal header, but JWT absent → `can_edit_charter = false` (role-based wins) ✅
4. **Negative — mismatched X-Argo-Principal**: demo JWT for `demo@example.com` but header claims `other@example.com` → `can_edit_charter = false` ✅
5. **Negative — DEMO_MODE off**: demo JWT + matching header but `ARGO_DEMO_MODE` env var removed → `can_edit_charter = false` ✅

All 3 negative gates pin against the exact failure mode that would matter most: a worker-overview RBAC bypass leaking into production (DEMO_MODE off) or to non-demo principals.

### Audit-kind wire (all 4 required places)

Grep confirmed `PermissionDemoOverrideGranted` exists in:

- `event_kind.rs:94` — enum variant declared
- `event.rs:47` — `to_label_owned` arm → `"permission.demo_override_granted"`
- `event.rs:134` — `from_label_owned` parse arm
- `event.rs:221` — included in `ALL_KINDS` array

The existing tests `label_round_trip_for_every_kind` and `all_kind_count_tracks_declared_variants` in `event_tests.rs` will fail at compile or at test-runtime if any of those 4 sites is missing. (Codex's PR-body verification log confirmed both passed.)

## Anti-theater check

**Dispositive.** Three independent failure modes each need a distinct fix to satisfy the test:

- Removing the `header_principal == demo_principal.principal_id` check would fail case 4
- Removing the `demo_mode_enabled` check would fail case 5
- Removing the JWT requirement entirely would fail case 3

A trivially-green rewrite would have to elide all 3 negative assertions simultaneously — visible in diff review. The audit-row SQL query (`fetch_one` on a specific `kind`) is a property assertion: the production code must actually call `emit_demo_override_audit` AND the audit sink must persist the row before the test passes. Cannot be satisfied by mocking.

One adjacent risk worth noting: the PR body acknowledges `FreezeButton` is gated by the worker-layout's `canFreeze` capability (a separate path) rather than the overview permissions payload. So even with the demo override granting all permissions, freeze may still gate on a different signal. Not a regression introduced by this PR — just an observation that the override surface is narrower than "every demo action works" might suggest.

## Gap / follow-up brief candidates

None blocking. Optional **P3 preventive brief**: `fix-freeze-button-demo-mode-respect-overview-permissions` — extend `FreezeButton` gate to also accept `permissions.canFreeze` from the worker-overview payload (or equivalent), so demo-mode override is uniform across all worker actions, not just the overview-managed subset. Tiny scope (~20 LOC in `web/components/worker/FreezeButton.tsx` + test). Recommend conductor consider IF it surfaces during demo recording dress rehearsal; otherwise leave.

Recommend merge under overnight `--skip-clippy` window.
