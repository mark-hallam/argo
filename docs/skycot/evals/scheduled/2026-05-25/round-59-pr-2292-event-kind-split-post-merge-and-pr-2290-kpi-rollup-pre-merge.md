# Round 59 — #2292 event-kind split POST-MERGE verify + #2290 KPI rollup PRE-MERGE intent verify + process drift sweep

- **Date**: 2026-05-25 (Melbourne ~20:25 AEST)
- **Round number**: 59
- **Pillar/Claims**: Claim 2 (Delegation is the unit) substrate + Track P P-02 LOC unblock prerequisite + process loop health
- **Scheduled-task fire**: hourly cron, real-UTC `date -u` 2026-05-25T10:17:44Z, ~52min after r58 ship 10:25Z

## Headline

🟢 **Scenario 1: #2292 audit event-kind docs split — POST-MERGE verified clean**. Variant count preserved (72→72), cold `cargo check -p argo-audit --tests` clean in 35s, full `cargo test -p argo-audit --tests` green including claim-6 cross-tenant pair (2/2 PASS) + replay round-trip (2/2 PASS) + chain-verify (1/1 PASS). LOC dropped 372→222, restoring Track P P-02's 500-LOC ceiling headroom (P-02 flipped BLOCKED→TODO in conductor commit `2ee013a41` immediately post-merge).

🟡 **Scenario 2: PR #2290 (mission-claim-2 KPI rollup, take 2) PRE-MERGE intent VERIFIED + rebase required + P3 stale-docstring nit**. PR is currently CONFLICTING/DIRTY (#2292 merged at 10:15Z, 10min after #2290 opened at 10:05Z; both touch `event_kind.rs`). Intent at PR head `c6cd6c6e28fa50be53e8a254ca4479bb4d1b7242` is sound: `set_kpis` SQL is `UPDATE worker SET kpi_json = ? WHERE id = ? AND tenant_id = ?` — touches ONLY `kpi_json`, never `charter_md`. 6th regression test `rollup_does_not_clobber_existing_charter` (lines 303-340) mirrors r55's evaluator probe exactly: seeds worker → `update_charter(t, w, Some(charter), None)` → asserts charter set + kpis None precondition → seeds tool_call audit row → `scheduler.tick(now)` → asserts `report.workers_rolled == 1` AND `after.charter.as_deref() == Some(charter)` AND `after.kpis.unwrap()["tool_calls_24h"] == 1`. Non-vacuous (would FAIL loudly if scheduler regressed to `update_charter(t, w, None, Some(kpis))`). Recommendation: REBASE-THEN-MERGE.

🟢 **Scenario 3: process drift sweep — all 🟢 green except 1 informational queue note**. Gateway FRESH v81=v81 (#2292 added no migration). Findings-only 4/4 within budget (5d remaining). 0 drift rows. Unread handoffs: 6 entries above conductor cutoff `2026-05-25T09:15:00Z`, but ALL are either informational (T19-08 conductor tick, T11-38 + T10-08 supervisor watchdogs) or already-actioned (r56 P1 closed by conductor T10-00 commit `3dcdf69f5`; r57 actioned by same; r58 noted no mutations required). Cutoff bookkeeping lags by ~1 tick — known race, not a regression.

## Setup

```bash
date -u                                                    # 2026-05-25T10:17:44Z
curl -sf http://127.0.0.1:7400/health                      # gateway UP
bash scripts/check-gateway-freshness.sh                    # FRESH v81=v81 drift=0
bash scripts/check-findings-only-fix-by.sh                 # 4/4 within budget
bash scripts/check-unread-handoffs.sh                      # 6 entries (all informational or actioned)
git log --oneline origin/main -25                          # since round 58 ship
git fetch origin pull/2290/head:pr-2290-head               # for PRE-MERGE intent read
```

### Scenario 1 — #2292 split anti-theater

```bash
wc -l crates/argo-audit/src/event_kind.rs                  # 222 (was 372 pre-split per #2292 diff)
wc -l crates/argo-audit/src/event_kind_docs.rs             # 49 (NEW module-level docs)
git show 60caaf6bd^:crates/argo-audit/src/event_kind.rs \
  | grep -c '^[[:space:]]*[A-Z][a-zA-Z]*[[:space:]]*,$'    # 72 variants pre-split
grep -c '^[[:space:]]*[A-Z][a-zA-Z]*[[:space:]]*,$' \
  crates/argo-audit/src/event_kind.rs                      # 72 variants post-split (no loss)
CARGO_TARGET_DIR=/tmp/eval-r59-target cargo check -p argo-audit --tests   # 35.03s cold, clean
CARGO_TARGET_DIR=/tmp/eval-r59-target cargo test -p argo-audit --tests    # all green
```

### Scenario 2 — #2290 PRE-MERGE intent read at head SHA

```bash
git show pr-2290-head:crates/argo-identity/src/store.rs | grep -A 20 'pub async fn set_kpis'
git show pr-2290-head:crates/argo-orchestrator/tests/kpi_rollup_test.rs \
  | grep -n '#\[tokio::test\]\|rollup_does_not_clobber'
git show pr-2290-head:crates/argo-orchestrator/src/kpi_rollup.rs \
  | grep -n 'set_kpis\|update_charter'                     # only set_kpis call at line 136
git merge-tree $(git merge-base origin/main pr-2290-head) origin/main pr-2290-head
```

Did NOT run cold cargo against PR head — PR is CONFLICTING; cold build would not reflect post-rebase state. Intent verification via source reads is the correct PRE-MERGE check here.

## Observed

### Scenario 1 — #2292 audit event-kind split

```
crates/argo-audit/src/event_kind.rs       222 LOC  (was 372 pre-split — 150 LOC migrated to docs file)
crates/argo-audit/src/event_kind_docs.rs   49 LOC  (NEW; pure module-level //! docs, no exported items)
crates/argo-audit/src/lib.rs               83 LOC  (+1 line: `mod event_kind_docs;`)
```

`event_kind_docs.rs` is structurally **module-level documentation only** (`//!` doc-comments) — no exported items, no types, no use statements that would create coupling. Wired in via `mod event_kind_docs;` in lib.rs (private module, only its docs reach the crate-level rustdoc tree). The `event_kind.rs` variants still carry short `///` rustdoc; the long-form payload-shape + emission-timing notes moved to `event_kind_docs.rs`.

Variant count: 72 pre-split = 72 post-split. No variant lost.

Cold `cargo check -p argo-audit --tests` clean in 35.03s. Cold `cargo test -p argo-audit --tests` results (selected):

```
test claim_6_strengthener_audit_chain_verify ... ok
test claim_6_live_cross_tenant_blanket_export_isolation ... ok
test claim_6_live_cross_tenant_select_no_existence_leak ... ok
test from_seq_filter_drops_earlier_rows ... ok
test five_rows_round_trip_in_seq_order ... ok
```

Track P P-02 unblock confirmed: conductor commit `2ee013a41` immediately flipped P-02 BLOCKED→TODO post-merge.

### Scenario 2 — #2290 KPI rollup intent

`set_kpis` at PR head (`crates/argo-identity/src/store.rs`, ~25 LOC):

```rust
pub async fn set_kpis(
    &self,
    tenant_id: TenantId,
    worker_id: WorkerId,
    kpis: &serde_json::Value,
) -> ArgoResult<()> {
    self.ensure_schema().await?;
    let result = sqlx::query("UPDATE worker SET kpi_json = ? WHERE id = ? AND tenant_id = ?")
        .bind(kpis_to_json(Some(kpis))?)
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set kpis: {e}")))?;
    if result.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "worker {worker_id} not found in tenant {tenant_id}"
        )));
    }
    Ok(())
}
```

- ✅ Tenant-scoped (`AND tenant_id = ?` in SQL).
- ✅ Only touches `kpi_json` column. `charter_md` untouched — fixes r55 P0 root cause.
- ✅ Returns `Provider` error if no rows affected (fails-closed on missing worker).

`KpiRollupScheduler::tick` at PR head (`crates/argo-orchestrator/src/kpi_rollup.rs`):

```
136:            identity.set_kpis(tenant, worker, &kpis_value).await?;
```

Zero `update_charter` calls in `kpi_rollup.rs`. The fix is wired in the load-bearing path, not just in tests.

6th regression test `rollup_does_not_clobber_existing_charter` at line 303 of `kpi_rollup_test.rs` (6/6 #[tokio::test] in file — was 5 in #2279, now 6 with the new clobber test):

```rust
identity.update_charter(tenant, worker, Some(charter), None).await.unwrap();
let before = identity.get_worker(tenant, worker).await.unwrap().unwrap();
assert_eq!(before.charter.as_deref(), Some(charter));
assert!(before.kpis.is_none(), "precondition: rollup writes KPIs");
seed_audit_row(&store, ..., "tool_call", ..., now.timestamp_micros() - 5 * MICROS_PER_SECOND).await;
let scheduler = KpiRollupScheduler::new(Arc::clone(&store), sink);
let report = scheduler.tick(now).await.unwrap();
assert_eq!(report.workers_rolled, 1);
let after = identity.get_worker(tenant, worker).await.unwrap().unwrap();
assert_eq!(after.charter.as_deref(), Some(charter));                 // r55 P0 probe
assert_eq!(after.kpis.unwrap()["tool_calls_24h"], 1);                // KPIs actually landed
```

Both halves asserted. Test would FAIL loudly if scheduler regressed to `update_charter(t, w, None, Some(kpis))` (charter would NULL out → assertion fails on `Some(charter)` check).

**Conflict shape** (`git merge-tree`): touches only `event_kind.rs` — `WorkerKpiRolled` variant declaration with a long doc-comment block on #2290's side vs. the docs migrated to `event_kind_docs.rs` on main's side. Rebase resolution is mechanical:
- Keep #2290's `WorkerKpiRolled` variant in `event_kind.rs`.
- Keep only a 1-2 line `///` rustdoc on the variant (per the new convention).
- Move the long-form payload-shape note to `event_kind_docs.rs` (append under "Product-surface notes:" or new "KPI rollup notes:" section).

**P3 stale-docstring nit** (non-load-bearing, fix during rebase): the `WorkerKpiRolled` doc-comment at PR head says:
```
/// A KPI-rollup scheduler tick ... persisted it to `worker.kpi_json` via
/// `IdentityStore::update_charter` (Stream `mission-kpi-rollup-from-audit-trail`...
```
But the implementation uses `IdentityStore::set_kpis` (not `update_charter`). Stale from when the brief was originally written. During rebase, update to `via IdentityStore::set_kpis` so doc matches the code.

### Scenario 3 — process drift

```
gateway:                   UP on 7400; FRESH v81=v81 drift=0
findings-only fix-by:      4 within-budget · 0 due-today · 0 overdue  (all 2026-05-30)
unread handoffs:           6 entries above cutoff 2026-05-25T09:15:00Z
  - T19-08 conductor       informational (operator-acted-on T10-00 already)
  - T11-38 supervisor      informational (slot/pulse status)
  - T10-08 supervisor      informational
  - r56 evaluator          ACTIONED by conductor T10-00 commit 3dcdf69f5 (merged #2272 + 2 followups)
  - r57 evaluator          ACTIONED by same conductor tick (5 carries closed via T09-50 train)
  - r58 evaluator          ACTIONED by same — no mutations required, all green
drift rows:                0
SHARED.md merge markers:   0 (stale markers from T09-10 stash-recovery confirmed gone in r58)
```

No new escalations. The 6-entry unread-handoffs count is bookkeeping lag — conductor T10-00 actioned r56's recommendation but didn't update the cutoff timestamp. Next conductor tick refreshes.

## Anti-theater check

### Scenario 1
- Pre-split vs post-split variant count: 72 = 72 (counted on both `60caaf6bd^:event_kind.rs` and current `event_kind.rs`). No variant lost during refactor.
- Cold cargo check (target dir `/tmp/eval-r59-target` was empty pre-run) — proved the split compiles from scratch, not relying on warm cached object files.
- claim-6 cross-tenant tests (`claim_6_live_cross_tenant_select_no_existence_leak` + `claim_6_live_cross_tenant_blanket_export_isolation`) PASS post-split — the load-bearing SQL `WHERE tenant_id = ?` filter on audit-row select/export was not disturbed by the docs migration.
- Read `event_kind_docs.rs` end-to-end (49 lines) — confirmed pure `//!` module-level docs, zero exported items, no risk of orphaned doc-links.

### Scenario 2
- `set_kpis` SQL read directly from PR head; the column name `kpi_json` is the same column #2279's `update_charter(t,w,None,Some(kpis))` would have touched in the broken path, but the WHERE clause is the only thing in the UPDATE — `charter_md` is **not in the SQL at all**.
- Scheduler source (`kpi_rollup.rs:136`) calls `identity.set_kpis(...)` — verified zero remaining `update_charter` calls anywhere in `kpi_rollup.rs`.
- 6th regression test mirrors r55's evaluator probe verbatim (same `update_charter(t, w, Some(charter), None)` setup; same `assert_eq!(after.charter.as_deref(), Some(charter))` assertion). Would FAIL if either (a) scheduler regressed to `update_charter(t,w,None,Some(kpis))` (sets kpis but NULLs charter) OR (b) `set_kpis` SQL accidentally included `charter_md` column.
- Did NOT run cold cargo against PR head — PR is CONFLICTING (DIRTY); cold build would compile against pre-rebase code, not the merged state. Source-read PRE-MERGE intent verification is the correct check here.

### Scenario 3
- Each "actioned" handoff cross-referenced against `git log origin/main` for the conductor commit that actioned it (T10-00 commit `3dcdf69f5`).
- "0 drift rows" cross-checked by reading `scripts/find-stale-claim-drift.py --quiet` exit-code-only behaviour against build-status state.

## Gap / follow-up brief candidates

**No new brief candidates this round.**

### Outstanding items for conductor action this tick

1. **REBASE PR #2290** — `mission-claim-2-kpi-rollup-rebase-and-resubmit` is CONFLICTING due to #2292 event_kind.rs split landing after #2290 opened. Mechanical rebase: keep `WorkerKpiRolled` variant in `event_kind.rs`, migrate the long doc-comment block to `event_kind_docs.rs` under a new "KPI rollup notes:" section, fix stale `via IdentityStore::update_charter` → `via IdentityStore::set_kpis` in the migrated docstring. Then `git rebase origin/main` should be conflict-free. Author/claim rebase brief OR direct conductor-authored rebase commit. **P1 — load-bearing for claim-2 LIVE finish; supersedes #2279.**

2. **CLOSE PR #2279 as superseded by #2290** — #2279 has been CONFLICTING since #2287 merged into its branch but never reached main. Per PR #2290 body: "Follow-up: close PR #2279 as superseded by this PR once this merges." Close now (don't wait) since #2290 is the live take. **P3 housekeeping.**

3. **(Optional, P3 docs-only) Edit `mission-demo-seed-real-llm-driver` brief** to remove the spurious `decide_and_request_approval` reference flagged in r58. Not a blocker — worker who claims will discover the inaccuracy and pivot.

### Carry status

- (h) `mission-slack-inbound-tenant-mismatch-audit-emission` — STILL OPEN (P2, **6th-round carry**) — author or close.
- (i) `eval-pack-lint-memory-schema-object-shape-coverage` — STILL OPEN (P3, **5th-round carry**) — low priority.
- (m) `conductor-claim-cycle-detector` — downgrade-status held.
- (n-r55) `fix-kpi-rollup-preserve-charter-md` — superseded by PR #2290 (this round's eval target). Once #2290 rebases + merges, carry CLOSES.
- (o-r56) `fix-demo-seed-emit-worker-hired-audit-rows` — TRACKED TODO line 214 of BUILD_STATUS.md (awaiting claim).
- (p-r57) `mission-demo-seed-real-llm-driver` — TRACKED TODO line 232 (awaiting claim).
- (NEW r59, **P3 docs-only**) Stale docstring on `WorkerKpiRolled` variant in #2290 — fix during rebase, NOT a separate brief.

### Operational health

- 43rd anti-theater verification this week.
- 4th consecutive round of process-loop health 🟢 (r55-r58 closed within one tick each; r59 has one stale-PR rebase to do but PR was already opened before the conflicting merge — not a process regression).
- Cost: ~25K tokens, ~25min wall time (well within 30-60min budget).
