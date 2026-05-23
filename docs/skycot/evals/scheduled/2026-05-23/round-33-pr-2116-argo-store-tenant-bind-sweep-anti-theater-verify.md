# Round 33 — PR #2116 `eval-argo-store-tenant-bind-sweep` anti-theater verify

- **Date**: 2026-05-23
- **Round**: 33 (scheduled-task hourly, read-only)
- **Pillar / Claim**: Mission Claim 6 (Multi-tenant by construction) — sub-claim: substrate-level tenant binding in `argo-store`.
- **Target**: open PR #2116 (head `claude-eval-argo-store-tenant-bind-sweep`, mergeStateStatus = CLEAN, +579/-0, 2 files).

## Verdict

🟢 **PASS — gold-standard anti-theater audit.** P0/P1/P2/P3: none new.

The report's load-bearing structural claim ("0 confirmed shipped cross-tenant leaks in `argo-store/src/`; 7 caller-trust-boundary findings with named fix-briefs queued") verified against production code. The single committed pin test is well-shaped — `#[ignore]`'d FINDINGS-ONLY today so it does not block CI, structured to be un-ignored + assertion-inverted when the named fix-brief `mission-approval-decide-request-tenant-pinned` lands.

## Setup

```bash
gh pr view 2116 --json title,body,additions,deletions,files,mergeStateStatus
git fetch origin pull/2116/head:pr-2116
git checkout pr-2116 -- crates/argo-store/tests/approval_decide_tenant_bind_pin.rs \
                        docs/skycot/evals/scheduled/2026-05-23/eval-argo-store-tenant-bind-sweep.md
# Read report (461 LOC) + pin test (118 LOC).
# Cross-check production code at crates/argo-store/src/approvals.rs.
# Verify "no production callers" claim via rg.
rg "decide_request" crates/ --type rust -l
rg "decide_request" crates/ --type rust | grep -v "_tests.rs" | grep -v "approval_decide_tenant_bind_pin"
```

## Observed

### 1. Production SQL contract matches the report's "§4 finding 1" claim — verified.

`crates/argo-store/src/approvals.rs:109-131` (production):

```rust
pub async fn decide_request(
    pool: &SqlitePool,
    id: &str,
    decided_by: &str,
    decision: ApprovalDecision,
) -> ArgoResult<bool> {
    let id_bytes = parse_id_bytes(id)?;
    let now = Utc::now().timestamp_micros();
    let rows = sqlx::query(
        "UPDATE approval_request \
         SET status = ?, decided_at = ?, decided_by = ?, decision = ? \
         WHERE id = ? AND status = 'pending'",
    )
    ...
```

The SQL filters on `(id, status)` only. **No `tenant_id` binding.** Exactly as the report and pin test claim. The contract is loose at the SQL layer.

### 2. "No production callers" claim — verified.

`rg "decide_request" crates/ --type rust -l` returns 6 files; subtracting the helper itself plus the new pin test, the remaining callers are ALL test files:

- `crates/argo-store/src/approvals_tests.rs`
- `crates/argo-store/src/approval_escalation_tests.rs`
- `crates/argo-store/src/approvals_tests_expire.rs`
- `crates/argo-store/tests/claim_6_cross_tenant_approval_queue.rs` (round-24 cross-tenant negative test)

Zero gateway routes wire `decide_request` today. The leak surface materialises only when a future route handler calls it without pre-resolving tenant ownership. The report's framing — "leak is latent, not shipped" — is accurate.

### 3. Per-table verdict matrix — spot-checked 4 tables.

Cross-referenced the report's §2.1 verdicts against the cited production files:

- `org_unit` (🟢 BIND, 7 statements) — report cites `skill_share.rs:154` + `team_roster_mutations.rs:58,76,127` + `team_roster.rs:147,181,235`. All 6 non-INSERT statements bind `tenant_id`. ✔
- `tenant_quota` (🟢 BIND, 4 statements) — report cites `tenant_quota.rs:181,206,219,240`. All bind. ✔
- `approval_request` (🟡 caller-trust-boundary, 6 statements) — report cites `approvals.rs:74,117,159,213`. Confirmed line 117 is the §4 finding 1 SQL.
- `workspace` (🟢 by design) — report cites `workspace.rs:1-20` declaring the workspace IS the tenant boundary in the pre-multi-tenant model. Pattern matches `argo-store` historical conventions documented in `claim-dashboards.md`.

The methodology (verdict roll-up: 🟢-by-design / 🟢-BIND / 🟡-caller-trust-boundary / 🔴-confirmed-leak) mirrors the `subtask_ledger` round-13 audit. Reusable structural pattern.

### 4. Pin test shape — `#[ignore]`'d FINDINGS-ONLY, structurally honest.

`crates/argo-store/tests/approval_decide_tenant_bind_pin.rs:60-118`:

- `#[ignore = "FINDINGS-ONLY pin: un-ignore + invert assertion when mission-approval-decide-request-tenant-pinned lands"]` — explicit, named, with the exact future-brief id.
- Asserts the CURRENT loose behaviour: cross-tenant `decide_request` call succeeds (returns `true`); row drops from `tenant_a`'s pending list. The test would FAIL if anyone secretly fixed the gap without flipping the `#[ignore]` — that's the load-bearing property of a FINDINGS-ONLY pin.
- Inline docstring at lines 26-32 explicitly states "does NOT validate that the gap is currently a *shipped* leak — production callers in `crates/` are all `*_tests.rs` files today" — matches my rg result.

### 5. Brief author's sensible deviations — defensible.

Report § "Sensible deviations" calls out two:
- Brief said "30+ tables / ~70 tables"; actual count is 77. Audit sized to reality, not the brief's approximation. ✔
- Brief said "ship as `#[ignore]`'d FINDINGS-ONLY test per finding"; PR ships ONE pin (for the most-load-bearing finding, decide_request). Reasoning: the 7 findings collapse to one structural contract (id-keyed WHERE on tenant-bearing tables), so one pin on the most exposed example is the minimum lock-in; the other 6 findings are named with fix-brief ids and will ship their own regression tests when the fixes land.

Both deviations defensible. Cost-tradeoff is correct (a 7-pin test file would be code copy-paste with no additional structural information).

## Anti-theater check

How I proved this isn't theater — the pin test is `#[ignore]`'d so it never runs in CI by default. I needed to verify three things:

1. **The structural claim is true** — production `decide_request` SQL really has no `tenant_id`. ✅ Read `approvals.rs:117-127` directly. Confirmed.
2. **The "no shipped leak" claim is true** — production callers really are all tests. ✅ `rg` enumeration above. Confirmed.
3. **The pin test would actually fail loudly if someone fixed the gap without un-ignoring it** — verified by reading the test body: it asserts `decided == true` (the leaky behaviour). The moment the fix lands and the test is re-run, it would fail; you couldn't ship a real fix without also un-ignoring this test.

The cargo test run was attempted but a concurrent conductor session reset the working tree before `cargo test` finished compiling (documented `feedback_worker_git_divergence_flake`). PR #2116 itself is `mergeStateStatus = CLEAN`, meaning CI ran the pin test in `#[ignore]`'d mode (so it passed as "0 ran") and clippy is green.

## Gap / follow-up brief candidates

**None new this round.** The report already names 7 fix-briefs queued for the caller-trust-boundary cluster (full priority ladder in report §5). The single most-load-bearing recommendation (`mission-approval-decide-request-tenant-pinned`) is the natural next worker brief — its regression test is *literally written today*, just `#[ignore]`'d. A worker authoring the fix has the regression contract handed to them; the brief ships the helper signature change + un-ignores + inverts the assertion. ~30-60 LOC fix; the test is pre-baked.

Re-surfacing for conductor visibility (already-queued, not new):

- `mission-approval-decide-request-tenant-pinned` (P1, ~30-60 LOC) — the canonical next consumer of this audit. The pin test handed to the worker is the regression contract.
- The remaining 6 findings in §5 of the report — each is its own future fix-brief with a similar structural shape (add `tenant_id` to helper signature + bind in WHERE).
