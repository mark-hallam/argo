# mission-orchestrator-duty-store-tenant-args

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-orchestrator-duty-store-tenant-args`.

🟢 **P2 DEFENCE-IN-DEPTH — 5 DutyStore methods not self-defending.** Source: #2091 internal docstring + evaluator round-16 verification 2026-05-23T14:30:00Z.

> "🟡 NEW P2: brief's own docstring (lines 22-34) calls out DutyStore mutators `get`/`mark_fired`/`set_paused`/`link_fire`/`sessions_for_duty` take `id: &str` only — production seam holds via `DutyScheduler::tick` pre-filter but surface is not self-defending (same risk class as round-3 audit-export bug)."

The orchestrator's substrate audit (#2091) confirmed 13 surfaces are cross-tenant-safe. But 5 DutyStore methods take `id: &str` only — they rely on the caller (`DutyScheduler::tick`) to pre-filter by tenant. That's the EXACT shape that caused round-3's audit-export P0 bug: "the caller currently has tenant context, so the substrate doesn't need to". A future refactor adds a new caller without tenant pre-filtering, and the bug ships.

## Scope

Thread `tenant_id: &TenantId` parameter into the 5 DutyStore methods:
- `DutyStore::get(id: &str)` → `DutyStore::get(tenant_id, id)`
- `DutyStore::mark_fired(id, ...)` → same
- `DutyStore::set_paused(id, ...)` → same
- `DutyStore::link_fire(id, ...)` → same
- `DutyStore::sessions_for_duty(id, ...)` → same

For each: add `AND tenant_id = ?` to the SQL predicate. Update the single production caller (`DutyScheduler::tick` in argo-orchestrator) to pass tenant_id from its existing context.

### Cross-tenant negative test per method

Add 5 tests (one per method) to `crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs` (the file shipped via #2091). Each: tenant_a creates a duty + fires it once; tenant_b calls the method with tenant_a's duty_id → assert byte-identical NotFound vs nonexistent-id.

This is the **same pattern as #2091** (which already pinned 13 surfaces) — just extending to 5 more.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-orchestrator/src/duty_store.rs`]

## Files you may touch

- `crates/argo-orchestrator/src/duty_store.rs` (or wherever the 5 methods live — find first).
- `crates/argo-orchestrator/src/duty_scheduler.rs` — single caller update.
- `crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs` — add 5 tests.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other DutyStore methods that already take tenant_id (they're fine).
- The SQL substrate / table — no migration needed (column already exists per round-12).
- argo-cron crate — out of scope.

## Acceptance criteria

- [ ] 5 methods thread tenant_id.
- [ ] All callers updated.
- [ ] 5 cross-tenant negative tests added (byte-identical NotFound assertion).
- [ ] Gap manifest argo-orchestrator row gets a "🟢 deepened — all DutyStore methods self-defending" note.
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `mission-orchestrator-duty-store-tenant-args`
- PR title: `fix(orchestrator): thread tenant_id into 5 DutyStore methods (mission-orchestrator-duty-store-tenant-args)`

## Notes for the implementer

- ~150-300 LOC. 5 method signatures + 1 caller update + 5 tests.
- **No new column needed** — the table already has tenant_id (per round-12). This is purely about the API surface being self-defending.
- **Cite #2091's own docstring lines 22-34** in the PR body as the source of this finding.
- Round-16 evaluator handoff is the trigger; reference verbatim.
