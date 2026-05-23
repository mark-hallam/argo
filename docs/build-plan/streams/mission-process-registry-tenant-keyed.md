# mission-process-registry-tenant-keyed

**Owner**: claude.
**Depends on**: none. (Order BEFORE `mission-agent-runner-wire-process-registry-per-session`.)
**Authored against**: origin/main `a3a881fa3` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-process-registry-tenant-keyed`.

🟡 **P2 CLAIM-5 PRE-WIRE.** `ProcessRegistry` (in `argo-tool`) has NO tenant scope at the data model. Today contained by F1 (registry never set in production); if F1 is fixed naively (just wire the registry), it becomes an instant cross-tenant leak.

## Why

PR #2117 (eval-argo-tool-surface-beyond-15-in-core, MERGED) finding F2 explicitly recommends ordering this fix BEFORE wiring the registry into production. Reasoning: making the registry tenant-keyed first means `mission-agent-runner-wire-process-registry-per-session` becomes a safe wire-up.

## Scope

- Add `tenant_id: TenantId` field to `ProcessRegistry` entries.
- Tenant-key the lookup maps (one-per-tenant or composite-key).
- Tenant-scope all public methods: `spawn`, `kill`, `list`, `cleanup`.
- Cross-tenant negative test: tenant_a spawns process P; tenant_b's `list` / `kill(P)` returns Empty / NotFound.
- Existing `#[ignore]`'d pin test `process_registry_kill_is_tenant_blind_today` (added by #2117) un-ignored after fix.
- ~200-300 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-tool/src/process_registry.rs` (or wherever — grep at claim-time)
- `crates/argo-tool/tests/argo_tool_substrate_pin_test.rs` (un-ignore the FINDINGS-ONLY test)
- New cross-tenant test file under `crates/argo-tool/tests/`
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production callers of the registry — that's a separate brief.
- Other crates.

## Acceptance criteria

- [ ] `ProcessRegistry` entries carry tenant_id.
- [ ] All public methods enforce tenant scope.
- [ ] Cross-tenant negative test passing.
- [ ] FINDINGS-ONLY pin test from #2117 un-ignored + passing.
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
