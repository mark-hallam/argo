# Round 56 — PR #2272 `mission-demo-seed-crate-scaffold` PRE-MERGE anti-theater verify

- **Date / Round**: 2026-05-25T09:55Z (Melbourne 2026-05-25 19:55 AEST) · Round 56 (scheduled-task hourly fire, operator-prioritized scenario)
- **PR**: [#2272 `demo-seed: stand up argo-demo-seed crate + 6 real workers via IdentityStore`](https://github.com/mark-hallam/argo/pull/2272)
- **PR head SHA**: `7ed37a6a91be063e662effb03d706e91d6dd2b4d`
- **Diff size**: +490/-0, 6 files (5 new, 1 workspace `members` entry, 1 Cargo.lock entry)
- **Claim / Pillar**: Mission claim 6 (Multi-tenant-by-construction) + the v1.0 demo MP4 ship gate (split-child #1 of 3 of the BLOCKED parent `mission-demo-seed-data-bootstrap`)
- **Verdict**: 🟡 **PRE-MERGE PARTIAL — P1 docs/scope finding, ship-conditional**

## TL;DR

- ✅ All 9 bundled tests pass cold against PR head (4 unit + 4 integration + 1 doctest, 18.6s build, 0.69s test).
- ✅ The 4 cross-tenant tests are anti-theater-honest: real `SessionStore::open(":memory:") + migrate()` → real `IdentityStore::create_worker` → real `WHERE tenant_id = ?` SQL filter.
- ✅ Idempotency assertion is non-vacuous (asserts both same-IDs-returned AND `listed.len() == 6`).
- ❌ **P1 finding**: brief + PR body both claim `IdentityStore::create_worker` emits baseline `worker.hired` audit rows. **It does not.** `AuditKind::WorkerHired` is emitted only by the gateway hire route at `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:353-360`. The seed creates 6 worker rows + **0 audit rows**, so the PR's "Test plan (post-merge)" claim that "the `/audit` SPA page renders the six personas" is unachievable from this scaffold alone.
- ⚠️ **P2 finding**: every seeded worker has `charter_md = NULL` because `Worker::draft` defaults `charter: None` and `build_worker` only fills `persona.bio` + `role.charter`. Recently-shipped KPI rollup non-clobber guarantee (#2287) is vacuous for seeded workers. Worth surfacing to brief #2 author.

Recommend **MERGE-WITH-FOLLOWUP** (not BLOCK) — the scaffold ships cleanly what its acceptance criteria specify (the load-bearing cross-tenant invariant), but the misconception in the brief about audit-row emission needs to be corrected before brief #2 builds on the same false premise.

## Setup

```bash
git worktree add --detach /tmp/argo-eval-2272 7ed37a6a91be063e662effb03d706e91d6dd2b4d
cd /tmp/argo-eval-2272
CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target cargo test -p argo-demo-seed -- --nocapture
```

Shared target dir at `$HOME/.cache/argo-shared-target/` (per tier-2 build-cache fix, post-relight). Sccache wraps rustc via supervisor `RUSTC_WRAPPER`; first-run target dir was cold for argo-demo-seed (new crate).

## Observed

```text
$ CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target cargo test -p argo-demo-seed --no-run
Finished `test` profile [optimized + debuginfo] target(s) in 18.61s
  Executable unittests src/lib.rs (...)
  Executable tests/seed_cross_tenant_invisibility.rs (...)

$ CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target cargo test -p argo-demo-seed
running 4 tests
test tests::demo_personas_have_non_empty_charters_and_traits ... ok
test tests::demo_personas_spine_is_exactly_six_named_workers ... ok
test tests::seed_workers_creates_six_workers_in_persona_order ... ok
test tests::seed_workers_is_idempotent ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

running 4 tests
test tenant_b_sees_zero_workers_after_seeding_tenant_a ... ok
test tenant_a_sees_exactly_six_named_workers ... ok
test reseeding_tenant_a_returns_same_six_ids ... ok
test parallel_tenant_seeds_remain_isolated ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s

running 1 test
test crates/argo-demo-seed/src/lib.rs - (line 15) - compile ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

PR body's test-output block (4 cross-tenant tests) **verified independently** on the same PR head.

### Production code path the tests exercise

1. `SessionStore::open(":memory:")` → real SQLite in-memory DB.
2. `store.migrate()` → runs the real migration registry (~80 migrations as of v81).
3. `IdentityStore::create_worker(&worker)` at `crates/argo-identity/src/store.rs:132` runs the production SQL transaction:
   - `INSERT INTO persona (..., tenant_id, ...)` line 152: `.bind(worker.tenant_id.to_string())`.
   - `INSERT INTO role (tenant_id, ...)` line 172: `.bind(worker.tenant_id.0.as_bytes().as_slice())`.
   - `INSERT INTO worker (..., tenant_id, ...)` line 194: `.bind(worker.tenant_id.to_string())`.
   - All three INSERTs commit in one TX.
4. `IdentityStore::list_workers(tenant_id, false)` at `store.rs:282` runs `SELECT id FROM worker WHERE tenant_id = ? AND archived_at IS NULL ...` — the tenant filter is **in the SQL itself**, not in any Rust-side post-filter.

This is **the same SQL path** the gateway hire route exercises (`routes_workers_hire.rs` calls `IdentityStore::new(...).create_worker(...)`), so the cross-tenant invariant the test pins is the real production invariant.

## Anti-theater check

Each cross-tenant test asserts two halves:

1. **Positive**: tenant A sees exactly six rows after seeding, names match the canonical spine `[Mariah, Priya, Jamal, Nori, Peta, Patrick]`.
2. **Negative**: tenant B sees **zero rows** after only tenant A was seeded (`tenant_b_sees_zero_workers_after_seeding_tenant_a`).
3. **Idempotency**: re-seeding tenant A returns identical `Vec<WorkerId>` (same IDs same order) AND `listed.len() == 6` (no duplicate INSERTs).
4. **Parallel isolation**: seeding tenant A + B in sequence produces two disjoint `WorkerId` sets (`intersection.is_empty()`) and both tenants list exactly six workers each.

The idempotency test is the one most likely to be theater (the production check is "find existing worker by `persona.name`" in `lib.rs:75-83`); the test verifies both the ID-stability half AND the row-count-no-duplicate half. **Non-vacuous.**

The cross-tenant negative half is independently verified by reading `store.rs:282-294` — the SQL `WHERE tenant_id = ?` is unmissable.

### Probe for the brief's "baseline worker.hired" claim — FAILS

Brief scope statement (verbatim):

> No agent turns, no approvals, no audit_record rows beyond the baseline `worker.hired` rows the production `create_worker` path already emits.

PR body "Test plan (post-merge)":

> Demo MP4 workflow seeds tenant via `seed_workers` and the `/audit` SPA page renders the six personas without theater.

**Verification**: grep for any audit-sink interaction inside `IdentityStore::create_worker`:

```bash
$ grep -n "audit\|Audit\|AuditKind\|sink" crates/argo-identity/src/store.rs
19:    /// `mission-audit-skill-learning-dynamic-enroll`).
73:    /// `mission-audit-skill-learning-dynamic-enroll`).
267:    /// `false`; audit/export paths pass `true`.
400:    /// `argo-types` audit (see `argo-types/src/error.rs`) makes
```

All four matches are doc-comment references. **Zero code-path audit emissions inside `create_worker`.** Verified by reading `store.rs:132-220` end-to-end.

Production emission site for `AuditKind::WorkerHired`:

```rust
// crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:353-368
if let Err(e) = sink
    .write(
        None,
        AuditKind::WorkerHired,
        domain_payload,
        Some(principal_id.to_string()),
    )
    .await
{
    tracing::warn!(
        target: "gateway::workers_hire",
        error = %e,
        worker_id = %worker_id,
        "worker_hired audit write failed"
    );
}
```

The audit row is emitted by the **gateway hire route**, layered on top of `IdentityStore::create_worker`. The seed bypasses this layer (per the brief's deliberate scope: "Uses `IdentityStore::create_worker` directly" with `&SessionStore` only, no `AgentDeps`/audit-sink access).

**Conclusion**: the seed deposits 6 worker rows + 0 audit_record rows. The `/audit` SPA page (which queries `audit_record`) will render BLANK after the seed runs, contrary to the PR body's expectation.

This is a brief/docs error, not a test-rewriting-itself-green error — the 4 cross-tenant tests don't claim to assert anything about audit rows, so they correctly pass. The PR's load-bearing acceptance criteria are honest. The blast-radius is the doc inaccuracy propagating into brief #2's design assumptions.

### Probe for `charter_md` (P2)

`Worker::draft` at `crates/argo-identity/src/types.rs:343-367` sets `charter: None`. `build_worker` in the seed (`lib.rs:131-142`) does NOT override this — it only fills `persona.voice_profile.preamble` + `persona.bio` + a `Role` via `Role::new(spec.charter)`.

In `create_worker` line 202: `.bind(worker.charter.as_deref())` → binds `None` to the `charter_md` column. So every seeded worker has `charter_md = NULL` in the DB.

The recently-shipped KPI rollup non-clobber guarantee (PR #2287) operates on `charter_md`: `IdentityStore::set_kpis` sibling that doesn't NULL the column. This guarantee is **vacuous for seeded workers** because the column was already NULL when the seed inserted it.

If the SPA worker detail page binds `charter_md` (per memory `project_worker_detail_design_locked`), seeded workers render with a blank charter section. The charter text **is** present in `persona.bio`, so a `bio ?? charter_md` SPA fallback would mask this — but that's not what the design lock specifies.

## Gap / follow-up brief candidates

### `mission-demo-seed-real-llm-driver` (brief #2 — already authored, not yet claimed)

When brief #2 is claimed, it must explicitly:

1. **Emit `AuditKind::WorkerHired` rows** for each seeded worker once it has `AgentDeps` (and thus audit-sink access). Payload should match the gateway hire route's shape (`actor / tenant_id / worker_id / vertical_id / name / duty_count / hired_at`) so `/audit` consumers don't need a special case.
2. **Set `worker.charter = Some(spec.charter.to_string())`** in the seed's `build_worker` (one-line change) so `charter_md` is non-NULL.

Both edits land cleanly in brief #2 since it already takes `&AgentDeps` and runs after the worker rows exist.

### Recommended pre-merge amendment to #2272 (optional)

Update the PR body's "Test plan (post-merge)" to clarify that the `/audit` rendering is brief #2's scope, not brief #1's. The current wording will mislead the operator at demo-record time.

### NEW brief candidate — `fix-demo-seed-brief-docs-audit-row-claim` — P3, ~30 LOC

If the conductor wants explicit doc correction (separate from brief #2's scope):

- Edit `docs/build-plan/streams/mission-demo-seed-crate-scaffold.md` to remove the "baseline `worker.hired` rows the production `create_worker` path already emits" clause (it's factually wrong).
- Edit `docs/build-plan/streams/mission-demo-seed-real-llm-driver.md` to add an explicit acceptance criterion: "MUST emit `AuditKind::WorkerHired` for each seeded worker via `AgentDeps.audit_sink`".

Low-stakes doc-only PR. Could fold into the next conductor tick that touches the demo-seed brief tree.

## Operational notes

- Live gateway: not probed this round (operator-prioritized scenario consumed the budget). Per r55, gateway was 🟢 FRESH at v81=v81.
- Process drift sweep: deferred to a separate round-56 scenario (writing this report first since operator override was the priority).
- Cost: ~38K tokens including operator-priority discovery + cold cargo + 3 source reads + this report. Well within budget.

## 39 anti-theater-honest verifications this week → 40 with this round
