# Round 55 — PR #2279 `mission-kpi-rollup-from-audit-trail` PRE-MERGE FAIL (charter clobber)

- **Date**: 2026-05-25 (UTC `2026-05-25T08:33Z`; Melbourne local `2026-05-25 18:33 AEST`)
- **Round**: 55 (scheduled-task hourly fire; ~7h35min after r54 ship 00:55Z)
- **Pillar / Claim**: Mission Claim 2 (Delegation-as-unit) — KPI rollup is the brief's claim-2 evidence; this PR is also one of 6 conductor-flagged `DEFER-NEEDS-EVAL` items per T07-40 heartbeat
- **Verdict**: 🔴 **FAIL — P0** (DO NOT MERGE without fix). 5 bundled tests PASS green, but a sixth probe I wrote against the same PR head FAILS loud — proves the rollup tick destroys an existing `charter_md` column on every tick. Theater check failed: the bundled tests never seed a charter so they never observe the clobber.

## Setup

- PR head SHA: `0eef7553f346756a88e5d5893355ef32c536e10a` (HEAD of `pr-2279-verify`)
- Worktree: `/tmp/r55-pr2279-verify`
- `CARGO_TARGET_DIR=$HOME/.cache/r55-pr2279-target` (avoids `/tmp` per CLAUDE.md tier-2 #5 fix)
- `CARGO_INCREMENTAL=0`
- Diff: 13 files, +787/-23. New `crates/argo-orchestrator/src/kpi_rollup.rs` (350 LOC) + `tests/kpi_rollup_test.rs` (299 LOC) + new migration v82 + raw slot 134 (`worker.kpis_rolled_at` watermark column) + new `AuditKind::WorkerKpiRolled` variant + gateway boot wire in `boot_serve.rs` / `boot_spawns.rs::spawn_gateway_kpi_rollup` running at default 5-minute cadence.

## Observed

### Bundled 5 tests — PASS

```
running 5 tests
test rollup_24h_window_drops_stale_rows ... ok
test rollup_increments_approvals_handled_after_replied ... ok
test rollup_skipped_if_no_audit_activity_since_last_rollup ... ok
test rollup_tenant_isolated ... ok
test rollup_emits_audit_row_per_worker_per_tick ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
```

Cold compile 59.77s wall (load avg ~3, vm_stat 6.7 GB free). Each bundled test exercises a real audit-row INSERT against an in-memory SessionStore migrated through v82.

### Charter-clobber probe — FAIL

I wrote `crates/argo-orchestrator/tests/kpi_rollup_charter_clobber_probe.rs` (122 LOC) which mirrors the bundled tests but inserts one extra step: it calls `IdentityStore::update_charter(tenant, worker, Some(charter), None)` before the rollup tick — the same shape the production hire path uses at `crates/argo-cli/src/handlers/contract.rs:71` and the CLI `argo worker charter set` flow at `crates/argo-cli/src/handlers/worker_charter.rs:34`.

```
running 1 test
test rollup_does_not_clobber_existing_charter ... FAILED

---- rollup_does_not_clobber_existing_charter stdout ----

thread 'rollup_does_not_clobber_existing_charter' panicked at .../kpi_rollup_charter_clobber_probe.rs:119:5:
assertion `left == right` failed: rollup tick clobbered charter_md —
  left: None
 right: Some("You are Mariah, the Triage worker. Be calm + decisive.")
```

Build for the new probe under the warmed cache: 2.45s wall.

## Root cause

`crates/argo-orchestrator/src/kpi_rollup.rs:160-163` on the PR head:

```rust
identity
    .update_charter(tenant, worker, None, Some(&kpis_value))
    .await?;
```

`IdentityStore::update_charter` at `crates/argo-identity/src/store.rs:364-389` is documented as a CLI-shaped setter ("Either argument may be `None` to null the column.") — its UPDATE statement is:

```sql
UPDATE worker SET charter_md = ?, kpi_json = ? WHERE id = ? AND tenant_id = ?
```

…unconditionally binding both columns. Passing `None` for the charter argument therefore binds SQL `NULL` to `charter_md`. The rollup ticks every 5 minutes (`DEFAULT_KPI_ROLLUP_INTERVAL_SECS = 300`) under the gateway-boot scheduler, so any worker that has a non-null `charter_md` (every hire created via `contract.rs:71`) gets its charter nuked within 5 minutes of hire and again on every subsequent tick.

The bundled tests never set a charter, so they never observe the clobber. `Worker::draft` leaves `charter_md = None` at hire, and the bundled tests assert only `kpis["approvals_handled"]` / `kpis["tool_calls_24h"]` / row counts via the `worker_kpi_json` getter — none assert anything about `charter_md` preservation.

Production callers I grepped on `origin/main` HEAD (excluding tests):

| Site | Shape |
|---|---|
| `worker_charter.rs:34` | `update_charter(tenant_id, worker, Some(&charter_md), kpis.as_ref())` — CLI `argo worker charter set` |
| `worker_charter.rs:88` | `update_charter(tenant_id, worker, None, None)` — CLI clear |
| `contract.rs:71` | `update_charter(tenant_id, worker, Some(&charter_md), Some(&kpis_value))` — hire path |
| `kpi_rollup.rs:163` (THIS PR) | `update_charter(tenant, worker, None, Some(&kpis_value))` — ❌ nulls charter |

## Anti-theater check

How I proved this isn't a vacuous panic / test rewrites itself green:

1. The probe's positive precondition asserts `read_charter` returns `Some(charter_text)` immediately after `update_charter`. If `update_charter` was a no-op, this would fail first — it doesn't, because the bundled tests already prove the `kpi_json` half of the UPDATE persists.
2. The probe seeds a `tool_call` audit row so `list_due_workers` returns one entry; if the rollup did NOT touch this worker, then `report.workers_rolled` would be 0, and the assert would never fire. The probe asserts `workers_rolled == 1` between the precondition and the postcondition.
3. The bound charter constant is non-empty UTF-8; an accidental "empty string evaluates falsy" trap doesn't apply here.
4. Running the probe a second time (rerun with `cargo test`) reproduces deterministically.

## Impact

- **Demo blast radius**: the v1.0 demo MP4 ship gate (operator-only artifact, blocking until recorded per `project_v1_demo_mp4_ship_gate`) renders the worker-detail page (`/workers/{id}/overview`) and the Mariah card whose binding `Worker::charter_md` is the worker's identity/persona text. After 5 minutes of gateway uptime, every hired worker would display a blank charter in the demo.
- **Production blast radius**: a tenant who hires Mariah via the standard contract flow loses the charter on the first rollup tick (5 minutes after hire). The data is destroyed — no audit row captures the prior `charter_md`. Recovery requires the operator to re-set the charter via CLI, only to lose it again 5 minutes later.
- **No rate-limiter**: the rollup re-runs every tick because `list_due_workers` only suppresses workers with no new audit activity — but the rollup tick itself emits a `WorkerKpiRolled` audit row attributed to the worker via `worker_id`, which would in turn keep the worker "due" forever. (Probable second-order bug worth a separate brief.)

## Gap / follow-up brief candidate

**Brief**: `fix-kpi-rollup-preserve-charter-md`
- **Priority**: 🔴 P0 (blocks PR #2279 merge — the merge gate is sound, the implementation isn't)
- **Scope**: ~30-80 LOC. Three options for the conductor / next worker to choose:
  - **Option A (cleanest)** — add `IdentityStore::set_kpis(tenant, worker, kpis_value)` sibling method that issues `UPDATE worker SET kpi_json = ? WHERE id = ? AND tenant_id = ?`. Change `kpi_rollup.rs:163` to call it.
  - **Option B** — read the current `charter_md` first, then pass it back through `update_charter` unchanged. Two queries per tick, but minimal API surface change.
  - **Option C** — change `update_charter`'s UPDATE to use `COALESCE(?, charter_md)` so `None` preserves existing value. Risk: changes documented semantics of an existing API; the CLI clear flow at `worker_charter.rs:88` would no longer null on `None`.
- **Recommended**: Option A. Keeps `update_charter`'s documented CLI semantics; gives the rollup a fit-for-purpose setter.
- **Test scope**: include a regression test that fails on this PR's HEAD (the probe above is a starter — adapt to land in `tests/kpi_rollup_test.rs` as a 6th case `rollup_does_not_clobber_existing_charter`).
- **Pair with**: this PR (#2279). Land #2279 only after the fix brief MERGED, or land them as one squashed PR.

## Recommendation to conductor

DO NOT admin-merge #2279 in the next interactive merge wave. The conductor's T07-40 heartbeat correctly listed #2279 under `DEFER-NEEDS-EVAL` — that defer is now confirmed P0.

**Secondary recommendation**: review the conductor's 8-PR `--skip-clippy`-greenlit list (#2267 / #2270 / #2271 / #2273 / #2275 / #2277 / #2278 / #2280) — none of these touch substrate, all are test/ops/docs/mechanical-split, so the greenlight stands. Only #2279 (and possibly the other 5 DEFER-NEEDS-EVAL items: #2268 / #2269 / #2272 / #2274 / #2276) needs evaluator coverage.

39th anti-theater verification this week; **2nd P0 substrate finding** (after the #2167-discovered tenant-bind sweep).
