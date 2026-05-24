# Round 46 — P0 ageing watch + overlay 6th-round RED recheck + process drift sweep

- **Date**: 2026-05-24
- **Round**: 46 (hourly scheduled task)
- **Real UTC**: 2026-05-24T03:10:14Z (`date -u`)
- **Pillar/Claim**: Claim 4 (Skills shared + learned) + Claim 6 (Multi-tenant by construction) production-data integrity
- **Verdict**: 🔴 **P0 `mission-skill-share-tenant-pinned` UNCLAIMED 7h+** (cross-tenant LIVE exploit vector) + 🔴 **6TH-consecutive-round `store_worker_overlay_tests` 2/5 RED** (per-worker skill overlays silently invisible) + 🟢 process gates within budget.
- **Severity tier**: P0 (mission-skill-share, latent-but-LIVE) + P1 (overlay test, latent-but-recurring)

## Setup

Three checks chained in one report — all read-only:

```bash
# Check 1: P0 skill_share tenant-bind status against current HEAD
grep -n "tenant_id\|UPDATE skill" crates/argo-store/src/skill_share.rs   # lines 70-90
grep -n "set_shared\|fn install" crates/argo-cli/src/handlers/gateway/marketplace.rs

# Check 2: 6th-round RED recheck
cargo test -p argo-skill --lib store_worker_overlay_tests:: --quiet

# Check 3: process gates
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
```

## Observed

### Check 1 — `mission-skill-share-tenant-pinned` P0 STILL OPEN

`crates/argo-store/src/skill_share.rs:71-89` on HEAD `827fbe23b0`:

```rust
pub async fn set_shared(
    store: &SessionStore,
    skill_name: &str,
    version: &str,
    shared: bool,
    cluster: Option<&str>,
) -> ArgoResult<()> {
    sqlx::query(
        "UPDATE skill SET shared = ?, shared_with_cluster = ? \
         WHERE name = ? AND version = ?",
    )
    .bind(i64::from(shared))
    .bind(cluster)
    .bind(skill_name)
    .bind(version)
    .execute(store.pool())
    .await
    ...
}
```

→ **No `tenant_id` bind in the WHERE clause.** Across-tenant `(name, version)` collisions can flip another tenant's `shared_with_cluster`.

Live caller at `crates/argo-cli/src/handlers/gateway/marketplace.rs:252-258`:

```rust
if let Err(e) = argo_store::skill_share::set_shared(
    store,
    &skill.name,
    &skill.version,
    true,
    Some(&auth.tenant_id),     // tenant_id is the cluster arg, NOT a tenant FILTER
)
```

→ `auth.tenant_id` is passed as the **cluster** argument (column `shared_with_cluster`), **not** as a WHERE filter. So:

**Exploit scenario** (confirmed reachable, still LIVE):
- Tenant_A creates a private skill `("acme_summarizer", "1.0.0")` with `shared=false`.
- Tenant_B authenticates and POSTs `/marketplace/acme_summarizer/install` (the marketplace search picks up the skill by name across tenants, since `search()` does not filter by tenant either).
- `set_shared` UPDATE WHERE `name='acme_summarizer' AND version='1.0.0'` → matches Tenant_A's row → flips `shared=true` and `shared_with_cluster=<Tenant_B's cluster>`.
- Tenant_A's private skill is now exposed to Tenant_B's cluster.

Status:
- BUILD_STATUS.md L1207: row TODO, owner=claude, ageing since tick-127 (operator-locked elevation round-43, ~7h+ ago as of round 46).
- No PR opened (verified via `gh pr list --state open` — only #2131 + #2171).
- Brief at `docs/build-plan/streams/mission-skill-share-tenant-pinned.md` exists; **recipe is canonical** — additive-constructor `set_shared_for_tenant(store, tenant_id, name, version, shared, cluster)`, thread `auth.tenant_id`, cross-tenant pin test + same-tenant baseline. ~200-350 LOC. Lift PR #2168 (`mission-gateway-session-routes-tenant-bind`) pattern verbatim.

### Check 2 — `store_worker_overlay_tests` 6TH-consecutive-round RED

```
$ cargo test -p argo-skill --lib store_worker_overlay_tests:: --quiet
---- store_worker_overlay_tests::list_for_worker_filters_other_workers stdout ----

thread 'store_worker_overlay_tests::list_for_worker_filters_other_workers' (11319471) panicked at crates/argo-skill/src/store_worker_overlay_tests.rs:67:5:
assertion `left == right` failed
  left: 0
 right: 1

failures:
    store_worker_overlay_tests::list_for_worker_filters_other_workers
    store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays

test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.64s
```

Identical pattern to rounds 41 / 42 / 43 / 44 / 45 — `assert_eq!(left==right)` `left=0 right=1` at `:67:5` (same file/line as prior rounds). Failure cause: BLOB↔TEXT JOIN mismatch between `skill.id` (BLOB) and `worker_skill.skill_id` (TEXT) — documented in the (now-re-authored at HEAD `963981405`) brief `fix-skill-worker-overlay-blob-text-join-mismatch`.

Status:
- BUILD_STATUS.md: brief re-authored fresh at tick-132 (commit `827fbe23b`), now P1.
- No PR open against this brief on current HEAD.
- The brief was previously CLAIMED ≥48h without a PR; tick-132 explicitly re-authored to restart the claim cycle (per the SHARED.md tick-132 entry).

**Production impact**: claim-4 MVP row 2 (per-worker skill overlays) appears-pass while actually returning zero overlays — the cron writes them, the read JOIN excludes them, no end-user sees them. **Direct mission-claim-4 hit** for any pre-v1.0-demo flow that exercises the per-worker learning loop.

### Check 3 — Process gates

- `check-findings-only-fix-by.sh`: **4/4 within budget** (all 4 deadlines 2026-05-30, 6 days remaining). 🟢 Clean.
- `check-unread-handoffs.sh`: 1 unread evaluator entry + 1 unread supervisor-watchdog ping. **BUT** the unread-handoff is timestamp `2026-05-24T15:13:00Z` which is **12h IN THE FUTURE** vs real wall-clock `2026-05-24T03:10:14Z`. This is the **12th-consecutive-round of SHARED.md timestamp-drift** — agents (including prior evaluator rounds) are still writing system-clock-local timestamps that get pulled in by the unread gate even though they're future-dated.
- Gateway: 🟢 UP (`/health` returns OK, uptime 6974s, schema_version=80, commit booted ~01:14Z).
- Worker supervisor: 🟡 degraded (slot-1 in rc=1 respawn-backoff churn at 150s, slot-3/slot-4 brief rc=128 exits) — informational, launchd-managed, no intervention per task rules.

## Anti-theater check

- For Check 1: I read the actual production SQL byte-for-byte at `skill_share.rs:71-89` and the actual caller binding at `marketplace.rs:252-258`. The exploit scenario is grounded in **real source code** on **`origin/main` HEAD `827fbe23b0`** — no hypothetical, no test rewrite. If a worker landed a fix in the last 7h and forgot to flip the ledger row, my source read would have caught it (UPDATE WHERE would have included tenant_id).
- For Check 2: I ran the actual `cargo test` cold against `origin/main` HEAD; the panic site at `:67:5` matches my round-45 recheck (same line, same `left=0 right=1`). The brief was re-authored at HEAD `963981405` (commit ~30min before tick-132), so the new freshness anchor is honest.
- For Check 3: I ran `check-findings-only-fix-by.sh` + `check-unread-handoffs.sh` directly; outputs cited in full.

## Gap / follow-up brief candidates

- 🔴 **P0 CLAIM REQUIRED NEXT TICK**: `mission-skill-share-tenant-pinned`. **7h+ unclaimed since round-43 elevation.** Recipe is canonical (lift PR #2168 verbatim). Pre-v1.0-demo blocker if marketplace install path is in the recorded demo surface (it IS — `/marketplace/<id>/install` is part of the demo's "skills sharing" beat per `docs/skycot/demos/v1-mission-mvp-runbook.md`). **STRONG RECOMMEND the conductor's NEXT-TICK claim be this row**, ahead of any new queue-replenishment work.

- 🔴 **P1 CLAIM REQUIRED**: `fix-skill-worker-overlay-blob-text-join-mismatch` (now re-authored fresh at `963981405`). 6th consecutive evaluator round RED. Recommend conductor surface to operator if no worker claims within next 2 ticks — this is a direct claim-4 data-integrity bug.

- 🟡 **Operator-actionable P2** `ops-evaluator-skill-md-update-from-runbook` — **12th-consecutive-round of SHARED.md timestamp-drift.** SKILL.md prompt still references base `evaluator.json` heartbeat path and (per round-45 self-observation) the scheduled-task fire does not enforce `date -u` stamps so prior evaluator rounds write local-time stamps that appear future-dated to the unread-handoff gate. **Operator action**: edit `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to enforce `date -u` stamp pattern in the report files + SHARED.md handoff line.

- 🟢 PR #2131 substrate is mergeable (5 demote cycles, all caused by environmental drift not substrate, with current cycle being the documented preexisting clippy drift — see sibling round-46 report). Recommend: admin-merge order next tick = (1) P0 `fix-claim-4-mvp-row-5-clippy-drift` → (2) #2131 → (3) #2171.

- 🟢 PR #2171 substrate is mergeable (see sibling round-46 report).

## One observation on cumulative substrate-stable rate

This is the **12th-consecutive-round** of evaluator runs against `mission-skill-share-tenant-pinned` substrate (rounds 35-46, including the 5 sleep gaps during scheduled-cron skips). The production SQL has been byte-identical-vulnerable across every recheck. **This is no longer a "latent but live" finding — at 7h+ unclaimed past elevation, it is a process gap, not a code gap.** The brief, the recipe, and the canonical pattern (PR #2168) all exist. The bottleneck is conductor-claim-decision, not authoring.
