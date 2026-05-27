# Round 99 — R97 substrate defect: ghost-stamp resolved, substrate still open

**Date**: 2026-05-27 (UTC ~08:55Z; Melbourne 2026-05-27 18:55 AEST)
**Round**: 99
**Pillar / Claim**: Anti-theater / Claim 4 (Skills shared + learned) / Claim 6 (Multi-tenant by construction)
**Verdict**: 🟡 PARTIAL — P0 carry, severity P0 (one half resolved, one half still open)

## Setup

R97 (2026-05-27T03:55Z) found PR #2437 `seeds_independently_per_tenant` test FAILS on origin/main:
`UNIQUE constraint failed: skill.name, skill.version`. Root cause: skill table has GLOBAL `UNIQUE(name, version)`
from migration v1.rs:118 (still in force); v80 added `tenant_id` column but did not amend the constraint.

R98 (2026-05-27T05:00Z) found that conductor commit `6006cc192` was titled
`ledger+brief: fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed (R97 P0 anti-theater)`
but `git show --stat 6006cc192` showed ZERO file changes — empty-commit ghost-stamp. Brief was UNTRACKED in working tree.
105th anti-theater verification this week.

R99 verifies what changed in the ~4h since R98.

## Observed

**Half 1 — R98 ghost-stamp resolution: ✅ ADDRESSED**

```
$ git log --oneline -1 ad1d3e6a6
ad1d3e6a6 briefs+ledger: R97 P0 (anti-theater fix) + R97 P1 (process pin)

$ git show --stat ad1d3e6a6 | head -10
 docs/build-plan/BUILD_STATUS.md                    |   2 +
 ...name-version-globally-blocks-per-tenant-seed.md | 238 +++++++++++++++++++
 ...n-merge-when-pr-body-acknowledges-unrun-test.md | 261 +++++++++++++++++++++
 3 files changed, 501 insertions(+)

$ grep -c "skill-table-unique" docs/build-plan/BUILD_STATUS.md
1

$ ls docs/build-plan/streams/fix-skill-table-unique*
docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md
```

The brief file is now tracked (238 LOC), the BUILD_STATUS.md ledger row exists under the
`DEMO-RECORDING UNBLOCK` header, and a second R97 P1 process brief
(`ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test`, 261 LOC)
also landed alongside. The R98 ghost-stamp class of failure is closed.

The R98 ledger row also documents that the row was re-ledgered after a stash-pop ate the first commit's
ledger row — `RE-LEDGERED 2026-05-27T06:30Z (the first ledger row landed in commit 6006cc192 got dropped
by a git-stash-pop auto-merge with concurrent worker pushes; brief file was preserved, ledger row was lost
— this row restores it)`. This is a more nuanced reading than R98's "pure ghost-stamp" verdict and the
operator should note that the underlying root cause for the empty commit was a stash-pop race, not a fabrication.

**Half 2 — substrate code path: 🔴 STILL UNCHANGED**

```
$ ls crates/argo-store/src/migrations/ | sort -V | tail -3
v81.rs
v82.rs
v83.rs                                          # no v84 yet

$ grep -n "UNIQUE.*name.*version" crates/argo-store/src/migrations/v1.rs
118:        UNIQUE(name, version)                # still global, not (name, version, tenant_id)

$ grep -n "install_for_tenant_if_absent\|insert_row_for_tenant" crates/argo-skill/src/store_vertical.rs | head -8
110:    pub async fn install_for_tenant_if_absent(
118:            .is_active_name_present_for_tenant(&frontmatter.name, tenant_id)
124:            .insert_row_for_tenant(frontmatter, body, learned_in, None, tenant_id)
```

`install_for_tenant_if_absent` (store_vertical.rs:110-127): pre-checks presence by `(name, tenant_id)` then
falls through to `insert_row_for_tenant` which hits the global `UNIQUE(name, version)`. Path B (new tenant
seeds skill that exists for another tenant) panics with the documented error.

**Half 2.5 — no PR in flight for the actual substrate fix**

```
$ gh pr list --state open --search "skill-table OR seeds_independently_per_tenant"
[{"number":2373,"title":"web — fix-skills-hero-count-pill ..."},     # unrelated web fix
 {"number":2440,"title":"fix: surface seeded workflow + verticals + audit rows on /api/v1/* endpoints"}]
```

PR #2440 IS in flight for the SIBLING tenant-fallback defects in workflow / verticals / audit endpoints,
but does NOT touch the skill UNIQUE constraint. The brief is in TODO state in BUILD_STATUS.md awaiting
worker claim. PR #2440 is OPEN with `mergeStateStatus=UNKNOWN` and `updatedAt=2026-05-27T04:44Z` — has been
sitting ~4h without merge-train progress (likely waiting for evaluator pre-verification of cold cargo + the
non-trivial substrate diff scope).

## Anti-theater check

Three independent verifications proved Half 2:

1. **Migration directory listing** — `ls crates/argo-store/src/migrations/` shows v83 as max; no v84.rs file.
2. **Migration v1.rs:118** — still has bare `UNIQUE(name, version)`; the schema-altering migration that
   would relax this constraint does not exist on origin/main.
3. **Skill store code path** — `crates/argo-skill/src/store_vertical.rs:110-124` shows the exact
   pre-check-then-raw-INSERT pattern R97 traced as the failure mode. Production path B (new tenant) still
   hits the global UNIQUE.

I did NOT re-run the cargo test (token + time budget — workers are running tests in parallel and the shared
target dir is contended). The substrate code reads alone are conclusive: if the code path is byte-for-byte
identical to the failing-state R97 documented, the test failure is the same.

## Gap / follow-up brief candidates

🔴 P0 CARRY (3rd round) — **No NEW brief authorship needed.** The brief
`fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed` is now properly ledgered as TODO under
DEMO-RECORDING UNBLOCK; conductor's responsibility next tick is to surface it for a worker claim or
mechanical-author the migration. Brief is 238 LOC of substrate work (migration v84 + SQL slot 135 table-swap
to relax constraint to `UNIQUE(name, version, tenant_id)`); ~80-120 LOC of production code change.
**Recommendation**: bump this brief to top-of-queue via a fresh `PRIORITY REORDER` header in BUILD_STATUS.md
so a worker claims it before the next demo-recording window opens.

🟡 P1 NEW — **`ops-conductor-protect-against-stash-pop-eating-ledger-rows`** ~30-50 LOC bash.
The R98 root cause is more interesting than "empty commit" suggested: it was a stash-pop auto-merge
race against concurrent worker pushes. The brief file was preserved but the ledger-row INSERT was
dropped silently. A pre-commit hook that diffs `BUILD_STATUS.md` before and after a stash-pop, refusing the
pop if a `| TODO |` row would be lost, would catch this class of failure at commit time instead of 24h later
when the next evaluator round runs. The fact that re-ledgering took the operator manually noticing R98's
report makes this a worthwhile process pin.

🟢 P3 CARRY (6th round) — `ops-conductor-tick-emit-current-utc-not-self-stamp` ~3 LOC. Still 4
ghost-stamped conductor entries detected by `check-unread-handoffs.sh`. Not load-bearing — informational
noise — but trivial to fix.

## Operator-surface

Mission claim Claim 4 (Skills shared + learned) is currently 🟢 LIVE in the status block (per #2092 + #2110
merges). That LIVE flag is **correct for the single-tenant skill-load path** that the regression test
`claim_4_live_install_invoke` actually exercises. It is **incorrect for cross-tenant skill seeding** — but
that path is not covered by any current mission-* regression test, so the claim's verifier doesn't catch it.

If the v1.0 demo MP4 script ever onboards a second workspace, the seed will hit the documented UNIQUE
panic. The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) currently scripts only a single tenant
per recording so this is not a recording-blocker for the operator's first take — but is a credibility risk if
any second workspace gets seeded during the demo.

**Recommend swarm clears this substrate fix BEFORE operator records the final demo MP4**, even though
the LIVE flag for Claim 4 won't change.

## Cost

~6K tokens for this scenario (verification reused R97/R98 context from LATEST.md + heartbeat).
