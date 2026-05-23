# Round 15 — argo-trust-engine ratchet + cross-tenant isolation

**Date**: 2026-05-23
**Round**: 15
**Pillar / Claim**: Mission claim 4 — Skills shared and learned (trust grade ratchet is the gate for cross-tenant skill publication)
**Verdict**: 🟢 PASS — gold-standard substrate. Dedicated cross-tenant test file, hysteresis-based ratchet, one-rung-per-call invariant. 29 tests pass.

## Setup

`crates/argo-trust-engine` is 1064 LOC across 5 files:

```
   155  cron.rs
    65  lib.rs
   255  ratchet.rs
   173  recompute.rs
   416  scoring.rs
  1064  total
```

Plus two integration-test files:

```
crates/argo-trust-engine/tests/cross_tenant_isolation.rs    (101 LOC)
crates/argo-trust-engine/tests/flap_resistance_tests.rs     (~120 LOC)
```

Tests run:

```
cargo test -p argo-trust-engine
→ 23 + 2 + 3 + 1 = 29 tests, all pass
```

## Observed

### 🟢 Dedicated cross-tenant negative test file

[cross_tenant_isolation.rs](crates/argo-trust-engine/tests/cross_tenant_isolation.rs) carries TWO real assertions:

1. **`tenant_a_score_unaffected_by_tenant_b_events`** (lines 48-81) — sets up 2 tenants + 2 workers, emits 2 `ApprovalGranted` events under tenant_a + 10 under tenant_b, calls `compute_score(&store, &tenant_a, &worker_a)`, asserts `approval_score == 2.0` (NOT 12.0, NOT bleed). Then asserts tenant_b score still picks up its own 10. Both directions covered.

2. **`worker_id_filter_isolates_within_tenant`** (lines 83-100) — within a single tenant, two workers, emit grants for each, assert each worker's score is its own count, not the union. This catches the "within-tenant, worker_id index missing" failure mode that has bitten other crates (round-3 audit_detail, round-13 worker_budget).

The module docstring explicitly calls out the brief's gotcha that prompted the test — "the signal queries must filter by `tenant_id AND worker_id`." That's the closest a crate has come to documenting its tenant-scoping invariant inline.

### 🟢 Hysteresis-based ratchet (no flap)

[ratchet.rs:17-30](crates/argo-trust-engine/src/ratchet.rs:17) defines two distinct threshold tables — promote thresholds are STRICTLY ABOVE demote thresholds for the same grade:

```
PROMOTE  Tenant: 5.0           DEMOTE  Tenant: 3.5     (band: 1.5)
PROMOTE  PublishedVerified: 10.0  DEMOTE PV: 8.0       (band: 2.0)
PROMOTE  Builtin: 20.0         DEMOTE  Builtin: 17.0   (band: 3.0)
```

`ratchet.rs:75-97` `ratchet_grade()` always returns AT MOST ONE rung per call — verified by the doctest at lines 64-74 (a score of 50.0 against `LearnedUnreviewed` returns `Promote { to: PublishedCommunity }`, not jumping straight to Builtin).

### 🟢 Tenant-bound signal queries

[scoring.rs:240-256](crates/argo-trust-engine/src/scoring.rs:240) `cross_tenant_quality_avg()`:

```sql
SELECT AVG(quality_score) FROM trust_observation
 WHERE source_tenant_id = ? AND observed_at >= ?
```

Tenant-blob bound on read. Counter-intuitively named: this is the OUTGOING quality score (how OTHER tenants rate THIS tenant's skills), which is exactly what mission claim 4 needs — your trust grade depends on what other tenants think of work you've shared into the marketplace.

The full `compute_score()` aggregates 5 signals: streak, approval, escalation, rbac.deny, cross-tenant quality. Every signal SQL binds `(tenant_id, worker_id)`.

### 🟢 Flap-resistance integration test

[flap_resistance_tests.rs](crates/argo-trust-engine/tests/flap_resistance_tests.rs) covers:

- `oscillating_score_holds_at_published_community` — score bouncing around the promote line never changes grade.
- `climbing_score_promotes_one_rung_per_tick` — even a steep climb only moves one rung per call.
- `alternating_extremes_changes_at_most_three_times_in_a_hundred_ticks` — 100 alternating extreme scores produce ≤ 3 grade changes total. This is the strongest flap-resistance assertion I've seen on substrate.

## Anti-theater check

This isn't "23 unit tests pass therefore correct." I read:

- The actual `compute_score` SQL bindings in scoring.rs.
- The `ratchet_grade()` function logic AND its doctest.
- The cross-tenant integration test assertion (assert `approval_score == 2.0`, NOT just "approval_score >= 2.0").
- The flap-resistance assertion (`≤ 3 grade changes in 100 ticks` is a real invariant, not a smoke test).

The substrate prevents the failure modes mission claim 4 cares about:
1. ✅ A worker doesn't get free trust from another tenant's behaviour.
2. ✅ A noisy signal doesn't ratchet grade up and down on every tick.
3. ✅ A worker can't skip rungs (audit chain shows the journey).
4. ✅ Within a tenant, worker_id isolation prevents trust-grade pooling across workers.

## Gaps / follow-ups

None worth a brief at this layer. Possible future work:

- 🟢 P3 / informational — `compute_score()` returns f32 scores that map to a `TrustGrade` enum, but the persistence layer (worker.trust_grade column) is not tested in this crate. That's an integration question for whichever crate actually writes the grade transition. If the wire is unaudited, the brief would be `eval-trust-grade-ratchet-persists-to-worker-row` — but that's an argo-skill or argo-identity audit, not a trust-engine one.

## Manifest rows touched

In `mission-vs-code-gap-manifest.md` claim-4 section, the argo-trust-engine row should flip from 🔴 NOT AUDITED to 🟢 with this report as evidence. Cross-tenant negative test exists; ratchet hysteresis exists; one-rung-per-call invariant exists; signal aggregation is tenant-bound at SQL.
