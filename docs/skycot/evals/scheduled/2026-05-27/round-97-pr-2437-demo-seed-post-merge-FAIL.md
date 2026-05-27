# Round 97 — PR #2437 demo-seed per-tenant idempotency — POST-MERGE FAIL

**Date**: 2026-05-27
**Round**: 97
**Pillar/Claim**: Claim 4 (Skills shared + learned) — substrate path for the v1.0 demo MP4 Scene 4 marketplace.
**Verdict**: 🔴 **FAIL** — **P0 ANTI-THEATER + functional regression.**

## Setup

- Commit under review: `72c005978` — `fix(demo-seed): per-tenant idempotency for marketplace seed (was global, silently skipped second-tenant) (#2437)`, merged 2026-05-27T00:26Z (=10:26 +1000) on operator-directive "do the 3 T0519MEL P0s now yourself".
- Closes brief `streams/fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`.
- The PR body itself notes: *"Local cargo check was interrupted by file-lock contention from concurrent worker compilation"* — i.e. the fix shipped without the new test ever passing in CI. Admin-merged under the operator-directive lane.

Reproduction run:

```bash
cd ~/code/argo
export CARGO_TARGET_DIR=$HOME/.cache/argo-evaluator-target
cargo test -p argo-demo-seed --lib seeds_independently_per_tenant
```

## Observed

Cold cargo build (1m 38s), then test exits FAILED:

```
running 1 test
test marketplace::tests::seeds_independently_per_tenant ... FAILED

failures:
---- marketplace::tests::seeds_independently_per_tenant stdout ----
thread 'marketplace::tests::seeds_independently_per_tenant' panicked at
crates/argo-demo-seed/src/marketplace.rs:205:14:
called `Result::unwrap()` on an `Err` value:
  Argo(Provider("skill install: error returned from database:
  (code: 2067) UNIQUE constraint failed: skill.name, skill.version"))

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out
```

The failure is on the **second tenant's** seed call (line 205 of `marketplace.rs` = the `seed_demo_marketplace_skills(&store, tenant_b)` invocation per the test source on origin/main).

## Anti-theater check

Three independent confirmations that this is the substrate defect — not a flaky test, not a test-only mistake:

**1. Schema** (`sqlite3 ~/.argo/sessions.db ".schema skill"`):

```sql
CREATE TABLE skill (
    id          BLOB PRIMARY KEY,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    ...
    tenant_id   TEXT NOT NULL DEFAULT '__migration_placeholder__',
    UNIQUE(name, version)            -- ← GLOBAL, no tenant_id involvement
);
```

The `UNIQUE(name, version)` constraint is global. There is no `UNIQUE(name, version, tenant_id)` compound, no partial index keyed on tenant_id. The migration that introduced `tenant_id` did not amend the uniqueness constraint.

**2. Production code** (`crates/argo-skill/src/store_vertical.rs:110-127`, origin/main):

```rust
pub async fn install_for_tenant_if_absent(...) -> ArgoResult<Option<Skill>> {
    let (frontmatter, body) = parse_skill_md(raw)?;
    if self
        .is_active_name_present_for_tenant(&frontmatter.name, tenant_id)
        .await?
    {
        return Ok(None);                              // path A: per-tenant idempotency hit
    }
    let skill = self
        .insert_row_for_tenant(frontmatter, body, learned_in, None, tenant_id)
        .await?;                                       // path B: INSERT → UNIQUE(name,version) fires
    Ok(Some(skill))
}
```

The function adds a per-tenant existence pre-check (path A). When path A doesn't hit (a NEW tenant), it falls through to `insert_row_for_tenant`, which issues a raw `INSERT INTO skill(...)`. The skill table's global `UNIQUE(name, version)` constraint rejects it — exactly as observed in the test failure.

**3. Live DB state** (anonymous probe against the running gateway's session DB):

```
$ sqlite3 ~/.argo/sessions.db "SELECT count(*) rows, count(DISTINCT tenant_id) tenants,
    count(DISTINCT name||'|'||version) distinct_nv FROM skill"
7|1|7
```

7 skill rows, 1 distinct tenant, 7 distinct (name, version) pairs. The cwd-bound demo tenant has succeeded ONCE; no second tenant has ever successfully seeded. The "0 skills · 0 shown" symptom the PR was supposed to fix would still bite the moment a SECOND tenant tries to seed against the same gateway — which is exactly the SPA `principal_resolve`-minted-tenant scenario the PR body describes.

The fix moved the failure mode from **"silently skipped, returns Ok(None)"** to **"noisy UNIQUE constraint Err"** — but did not actually enable per-tenant catalog seeding.

## Gap / follow-up brief candidates

### 🔴 P0 BRIEF — `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`

**Scope**: Either (a) add migration v84 that drops the global `UNIQUE(name, version)` and replaces it with `UNIQUE(name, version, tenant_id)`, OR (b) keep the global uniqueness as a multi-tenant marketplace publishing invariant and rework demo seeding so each tenant gets a row referencing the same skill catalog without duplicating rows (a per-tenant install-mapping table, or an `ALTER` to make the seeding INSERT-OR-IGNORE with same-id-different-tenant-id rows treated as views).

**Trade-off**: option (a) is local + low-risk for the demo path but loosens a marketplace-publishing invariant; option (b) preserves marketplace semantics but is larger scope. The brief should decide between them via operator question, not author both.

**Test pin (regression-locking)**: keep the existing `seeds_independently_per_tenant` test — it currently fails and must turn 🟢 GREEN under the fix.

**Priority justification**: P0 because (1) the PR claims to have closed a P0 demo blocker but did not; (2) any second tenant that goes through the SPA `principal_resolve` path on the live gateway will now hit a noisy DB error instead of the original silent skip — which is arguably WORSE for operator-debugging; (3) the unauthored `fix-gateway-verticals-boot-seed-workflow-definition-index` follow-up brief named in the PR body is BLOCKED on the same class of resolution.

### 🟡 P1 BRIEF — `ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test`

**Scope**: extend `conductor-pre-merge-gate.sh` to grep the PR body for known "test not run" admission phrases ("Local cargo check was interrupted", "interrupted by file-lock contention", "formal pre-merge gate will run") and either (a) hold the merge until evaluator validates, or (b) emit a `[gate-warning]` line into the conductor's merge-eligible-PR summary so the operator sees the bypass explicitly.

**Why now**: PR #2437 is the SECOND admin-merge under operator-directive in 48h where the test-the-fix-claims was never run. The first was a smaller surface (cosmetic), this one is functional substrate. Without a process pin, this becomes a recurring pattern.

### 🟢 P2 BRIEF — `fix-gateway-verticals-boot-seed-workflow-definition-index` (PR-body-named, still UNAUTHORED)

The PR body itself names this follow-up under "Out of scope". It's confirmed absent from `streams/` (only `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md` and two unrelated verticals briefs exist). Recommend conductor mechanical-author against the live-DB evidence below.

**Live evidence** (`sqlite3 ~/.argo/sessions.db`):

```
workflow_definition rows by tenant:
  019e6780-...: 1
  019e6785-...: 3
workflow_definition_index rows by tenant:
  019e6785-...: 1
```

So one tenant has 1 def-row but 0 index-rows; another has 3 def-rows but only 1 index-row. Both `/api/v1/workflows` and `/api/v1/verticals` filter on `workflow_definition_index` — they return 0 even after a successful seed. Same class of write-gap as the skills bug, just on a different table. The brief should be modelled on this round's recommended P0 above.

## Operational footnote

The gateway under probe is running at PID 53182 with **CWD = `~/code/skycot-v1-demo`** (non-canonical, operator-directed during the T0819MEL session-handoff window). The `ensure_workspace` tenant resolution depends on CWD; the seeded `019e6785` tenant becomes SPA-invisible the moment the gateway restarts from the repo root. The "make restart-live-gateway" carry from the prior 5 evaluator rounds is now **bidirectionally load-bearing**: restart in repo root → demo tenant disappears; don't restart → schema check stays FRESH but binary-vs-HEAD staleness is not detected. The previously-authored `ops-gateway-freshness-check-also-detects-binary-staleness` brief still applies.
