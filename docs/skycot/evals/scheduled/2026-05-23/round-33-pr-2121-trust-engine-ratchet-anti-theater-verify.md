# Round 33 — PR #2121 `eval-trust-engine-ratchet-from-zero-to-grade-A` anti-theater verify

- **Date**: 2026-05-23
- **Round**: 33 (scheduled-task hourly, read-only)
- **Pillar / Claim**: Mission Claim 4 (Skills shared + learned) — sub-claim: trust-grade ratchet (5-signal writer pipeline).
- **Target**: open PR #2121 (head `claude-eval-trust-engine-ratchet-from-zero-to-grade-A`, mergeStateStatus = CLEAN, +498/-0, 1 test file).

## Verdict

🟢 **PASS — gold-standard anti-theater test.** P0/P1/P2/P3: none new.

The test is structurally honest: it drives the production `recompute_worker` writer surface (the same call site `tick_all_workers` uses on every cron tick), reads grade back from the persisted `worker.trust_grade` column between ticks, and queries the `trust_grade_changed` audit chain from raw SQL using the production payload-key contract (`from_grade` / `to_grade` formatted via `format!("{from:?}")`). The 4-rung walk + RBAC-deny pause (with counterfactual twin) + 20-tick flap-resistance is the loop-level pin existing in-crate unit tests did NOT provide.

## Setup

```bash
gh pr view 2121 --json title,body,additions,deletions,files,mergeStateStatus
git fetch origin pull/2121/head:pr-2121
git checkout pr-2121 -- crates/argo-trust-engine/tests/zero_to_grade_a_ratchet_journey.rs
# Read test (498 LOC) + cross-check production at crates/argo-trust-engine/src/recompute.rs.
```

## Observed

### 1. Production `recompute_worker` signature matches the test's call.

`crates/argo-trust-engine/src/recompute.rs:31-76`:

```rust
pub async fn recompute_worker(
    store: &SessionStore,
    sink: &AuditSink,
    tenant: &TenantId,
    worker_id: &str,
    current_grade: TrustGrade,
) -> ArgoResult<RatchetDecision> {
    let breakdown = scoring::compute_score(store, tenant, worker_id).await?;
    let decision = ratchet::ratchet_grade(current_grade, breakdown.total);
    match &decision {
        RatchetDecision::Hold => Ok(decision),  // ← short-circuit BEFORE audit emit
        RatchetDecision::Promote { from, to } | RatchetDecision::Demote { from, to } => {
            ...
            identities.set_worker_trust_grade(*tenant, parsed_worker, new_u8).await?;
            ...
            let payload = json!({
                "worker_id": worker_id,
                "tenant_id": tenant.to_string(),
                "from_grade": format!("{from:?}"),
                "to_grade": format!("{to:?}"),
                "score_total": breakdown.total,
                "score_breakdown": breakdown,
                "decision": decision_label,
            });
            sink.write_with_tenant(None, AuditKind::TrustGradeChanged, payload, None, &tenant.to_string()).await?;
```

The test's call signature `recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current_grade)` (lines 180-188, 202-204, 227-229, etc.) matches the production function 1:1. ✔

### 2. Audit-row query mirrors the production write contract.

Test lines 144-154:

```rust
let rows: Vec<(Option<String>,)> = sqlx::query_as(
    "SELECT payload_json FROM audit_record \
     WHERE tenant_id = ? AND worker_id = ? \
       AND kind = 'trust_grade_changed' \
     ORDER BY created_at ASC, seq ASC",
)
```

Then extracts `from_grade` + `to_grade` as strings (lines 158-159). The production writer (`recompute.rs:65-72`) writes exactly those keys via `json!({"from_grade": format!("{from:?}"), "to_grade": format!("{to:?}"), ...})`, tenant-stamped via `write_with_tenant(... &tenant.to_string())` so the denorm `tenant_id` column AND the `payload.tenant_id` both carry the tenant. The test's WHERE clause filters on the denorm column, matching `mission-vs-code-gap-manifest` audit-row shape contract.

### 3. The "Hold short-circuits before audit emit" claim — verified.

Test line 491 — `// Hold decisions must NOT emit trust_grade_changed` — and the production code at `recompute.rs:41`: `RatchetDecision::Hold => Ok(decision),`. The `Ok(decision)` return path skips the `sink.write_with_tenant` call entirely. So the flap-resistance test's assertion `chain.is_empty()` after 20 Hold ticks is load-bearing on the production short-circuit.

### 4. RBAC-deny counterfactual is a TRUE counterfactual.

Test lines 404-431: twin worker in a SECOND tenant (`tenant2 = TenantId::new()`) with a SEPARATE `AuditSink`, same 5 grants + zero RBAC denies. The `scoring::compute_score` query keys on `(tenant, worker_id)` per its signature (line 38 of production), so `tenant2`'s audit rows cannot contaminate the paused worker's score. The twin is structurally isolated from the paused worker.

The Promote assertion (lines 425-429) would fail if RBAC denies stopped contributing to the score — meaning the test correctly distinguishes "RBAC denies pause promotion" from "the worker happens to be at Hold for another reason." A weight-tuning regression that made RBAC weight = 0 would surface here as a false-Promote on the original worker (and the same Promote on the twin — both promote → original test fails).

### 5. Sensible deviation — defensible.

Author calls out scaling "1 RBAC deny" (from the gap manifest) to 6 denies because a single deny (−0.5) leaves the score at +7.0, still above the 5.0 promote line — the test would pass accidentally even if RBAC denies were broken. 6 denies (−3.0) drag the score to +4.5 with a 0.5-absolute safety margin below the line. Inline docstring at lines 366-372 explains the reasoning. ✔ Defensible — pins the contract more tightly without losing intent.

### 6. The 4-rung promotion table is bit-exact against the production score formulas.

Test docstring lines 196-298 enumerate the cumulative score expectations:

- Tick 1: 2 grants → approval +2.0 + streak +1.0 = +3.0 → promote `LearnedUnreviewed` (line 2.0) → `PublishedCommunity`.
- Tick 2: 5 cumulative grants → +5.0 approval + 2.5 streak = +7.5 → promote `PublishedCommunity` (line 5.0) → `Tenant`.
- Tick 3: 3 quality=0.9 observations → +4.5 quality, total ≈ +12.0 → promote `Tenant` (line 10.0) → `PublishedVerified`.
- Tick 4: 15 cumulative grants → +27.0 → promote `PublishedVerified` (line 20.0) → `Builtin`.
- Tick 5: at Builtin, Hold (top grade).

Each tick re-reads grade from `IdentityStore::get_worker` (line 127-132) — the same path `tick_all_workers` uses (`recompute.rs:88-92`). The audit-chain assertion at lines 311-331 then verifies exactly 4 rows in created-at-order with the expected (from, to) pairs.

## Anti-theater check

How I proved this isn't theater — the same test could pass against stub `recompute_worker` if it just took (current_grade, signal_count) → grade. Three structural pins prevent that:

1. **Test uses production `IdentityStore::get_worker` for the grade re-read between ticks** (not a local Rust variable), so the persisted `worker.trust_grade` column has to actually update. Lines 127-132 + 213-217 + 238-241 + 262-265 + 286-289.
2. **Test queries `audit_record` for the `trust_grade_changed` payload using exact production key names** (`from_grade` / `to_grade` formatted via `format!("{from:?}")`). A future audit-shape refactor (e.g. renaming `from_grade` → `previous_grade`) would surface as zero rows in the test query → chain.len() == 0 → assertion failure.
3. **Test asserts the audit chain has EXACTLY 4 rows after 4 promotes** (not >= 4). An accidental double-emit (a future refactor that writes once per ratchet decision AND once per IdentityStore update) would surface as chain.len() == 8.

Could not run `cargo test` cold — concurrent conductor session reset the working tree mid-session (documented `feedback_worker_git_divergence_flake`). PR is `mergeStateStatus = CLEAN`, which proves the test passed cold-built on CI's worktree (CI cold-build = first signal that the source compiles + tests pass against origin/main + the new test). Author's own verification: `cargo test -p argo-trust-engine` green (36 tests pass: 23 lib unit + 4 claim-4 mvp + 2 cross-tenant + 3 flap-resistance + **3 new** zero-to-grade-A + 1 doctest); workspace clippy fast-fail clean (769s).

## Gap / follow-up brief candidates

**None new this round.** PR is ready to merge; the test is structurally honest and pins a loop-level contract the existing isolated unit tests did NOT cover (round 28's `claim_4_mvp_row_7_trust_grade_ratchet` covered `compute_score → ratchet_grade` composition; this round adds the *multi-tick writer pipeline* including the persisted-column re-read between ticks). With #2121 LIVE, Claim 4 MVP row 7 will progress from 🟢 substrate-only → 🟢🟢 e2e-LIVE through the full writer loop, complementing round 28's existing 🟢🟢 marker on `RatchetDecision` composition.

Re-surface for conductor visibility — already known: with #2116 + #2121 + #2120 + #2119 + #2118 + #2117 all in review and CLEAN, this is a queue of 6 evaluator deliverables ready to admin-merge in one pass.
