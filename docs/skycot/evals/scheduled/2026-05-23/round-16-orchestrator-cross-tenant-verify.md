# Round 16 — Verify #2091 orchestrator cross-tenant tests close round-12 gap

- **Date**: 2026-05-23
- **Round**: 16
- **Pillar / Claim**: Mission claim 6 — multi-tenant by construction — argo-orchestrator surface.
- **Verdict**: 🟢 PASS; 🟡 P2 follow-up surfaced (DutyStore mutators take `id: &str` only).

## Why this scenario

Round 12 (2026-05-23T12:00Z) audited argo-orchestrator (9127 LOC) and
found the SQL was tenant-scoped at every WHERE clause but the crate had
**zero** cross-tenant negative tests. Same shape as the round-3
audit-export pattern that shipped a bug (#2046 → #2051): SQL was right,
no test pinned it, a refactor silently dropped the predicate. The
conductor authored `eval-orchestrator-cross-tenant-handoff-negative-test`
(P1), and #2091 MERGED at `2026-05-23T02:09:25Z`.

## Setup

```
cargo test -p argo-orchestrator --test cross_tenant_handoff_test
```

Plus full read of [crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs](crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs).

## Observed

13 / 13 tests PASS in 5m 29s (cold cargo compile, then test in 1.11s):

```
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured
```

Coverage across 5 public surfaces:

- **Handoff** (5 tests): find / insert / list_pending_for_worker /
  list_for_session / update_status / cancel.
- **RetryPolicy** (2 tests): get / list_for_skill.
- **Duty** (1 test): list_for_tenant.
- **SLA** (4 tests): get / list / update / delete.

The load-bearing pattern is **byte-identical `Debug` repr** between the
cross-tenant attempt and a genuine nonexistent-id call:

```rust
let cross_tenant = f.repo.find_handoff(f.tenant_b, &h.id).await;
let nonexistent = f.repo.find_handoff(f.tenant_b, &HandoffId::new()).await;
assert_eq!(format!("{cross_tenant:?}"), format!("{nonexistent:?}"));
```

That is stronger than "returns 404." It pins the existence-leak class
fixed in #2068 (round-4) — a caller cannot distinguish "wrong tenant"
from "no such row," so an attacker cannot probe for `worker_id ×
tenant_id` pairs by watching response shape.

## Anti-theater check

- **Tests use `OrchestratorStore::new(Arc<SessionStore>)` against a
  real `:memory:` SQLite**, not stubs. The SQL paths under test are
  the production paths.
- **The fixture seeds BOTH tenants** with a real session each. An
  empty cross-tenant query against an empty tenant is unfalsifiable
  ("returns empty because table is empty"). The fixture forces the
  cross-tenant query to filter out an actually-present sibling row.
- **Two independent assertion shapes** — empty-result tests use
  `.is_empty()`, single-row tests use byte-identical Debug repr. A
  refactor that broke either would flip a test.

## 🟡 P2 follow-up surfaced (by the brief itself, not yet authored)

The brief's own docstring (cross_tenant_handoff_test.rs lines 22-34)
flags an honest finding:

> `DutyStore::get`, `DutyStore::mark_fired`, `DutyStore::set_paused`,
> `DutyStore::link_fire`, and `DutyStore::sessions_for_duty` do NOT
> take a `TenantId` argument. They are id-scoped lookups; tenant
> scoping depends entirely on the caller pre-filtering via
> `list_for_tenant`. Production today calls them only from
> `DutyScheduler::tick`, which seeds them from `list_active`, so the
> seam holds — but the surface is not self-defending.

Confirmed at [crates/argo-orchestrator/src/duty_scheduler_store.rs](crates/argo-orchestrator/src/duty_scheduler_store.rs):

- `get` — line 74 — `pub async fn get(&self, id: &str)`
- `set_paused` — line 123 — `pub async fn set_paused(&self, id: &str, …)`
- `mark_fired` — line 135 — `pub async fn mark_fired(&self, id: &str, …)`
- `link_fire` — line 147 — same shape
- `sessions_for_duty` — line 170 — same shape

I confirmed no brief by that name exists at `docs/build-plan/streams/`.

### Brief candidate

- **Name**: `mission-orchestrator-duty-store-tenant-args`
- **Priority**: P2 (not a live leak — production seam holds — but
  exactly the kind of surface that becomes a live leak in a future
  refactor; this is the round-3 audit-export precedent).
- **Scope**: Add `tenant_id: TenantId` as the first argument to every
  `DutyStore` mutator + lookup in
  `crates/argo-orchestrator/src/duty_scheduler_store.rs`; bind it in
  every SQL WHERE clause; mirror the shape of `OrchestratorStore`
  (which does take TenantId on every method). Update the sole
  caller (`DutyScheduler::tick`) to thread the duty record's
  `tenant_id` through. Add 2 cross-tenant negatives to this same
  test file. Estimated 200-400 LOC, single crate.

## Verdict

🟢 PASS on the original gap. Round-12 finding now closed: argo-orchestrator
has real regression guards on 5 of its 6 main surfaces. **One P2
recommendation surfaced**: the DutyStore sub-surface is not
self-defending and warrants the brief named above.

## Mission-vs-code manifest update

Claim 6 / argo-orchestrator row flips from 🟡 (round 12 finding) to 🟢
with footnote about DutyStore. Manifest update inline below.
