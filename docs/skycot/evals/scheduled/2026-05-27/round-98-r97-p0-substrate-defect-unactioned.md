# Round 98 — R97 P0 substrate defect still un-addressed (24h carry)

**Date**: 2026-05-27T05:00Z (≈15:00 AEST)
**Round**: 98
**Pillar/Claim**: Claim 6 (multi-tenant by construction) — substrate write-path
**Verdict**: 🔴 **FAIL — P0** (anti-theater carry, 24h since first surfaced in R97)

## Setup

R97 (2026-05-27T03:55Z) cold-reran the regression test PR #2437 cited as anti-theater dispositive (`seeds_independently_per_tenant` in `argo-demo-seed`) and proved it FAILS on origin/main with `UNIQUE constraint failed: skill.name, skill.version`. R97 recommended a P0 brief `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed` (migration v84 relaxing the UNIQUE to (name, version, tenant_id) OR introducing a per-tenant install-mapping table).

R98 verified what happened in the intervening 24h.

## Observed

### 1. The "ledger+brief" commit shipped EMPTY

```
$ git show --stat 6006cc192
commit 6006cc19239ca3b4927a5f51805bc4df03ae7a8f
Author: mark-hallam <…>
Date:   Wed May 27 16:06:38 2026 +1000

    ledger+brief: fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed (R97 P0 anti-theater)
```

No file changes. None. The brief file actually sits as **untracked** in the working tree:

```
$ git status --short docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md
?? docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md

$ diff <(git show 6006cc192:docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md) \
       docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md
0a1,238  # 238 lines in working tree, 0 in HEAD
```

### 2. Ledger has no row

```
$ grep -n "skill-table-unique-name-version" docs/build-plan/BUILD_STATUS.md
(empty)
```

The swarm protocol has workers pick the first eligible `TODO` row by document order in `BUILD_STATUS.md` (per `CONTRIBUTING_AGENT.md` step 2). No row → no claim → no PR.

### 3. Substrate unchanged

```
$ git log --oneline 6006cc192..origin/main -- crates/argo-skill/ crates/argo-demo-seed/ crates/argo-store/src/migrations/
(empty)
```

`crates/argo-skill/src/store_vertical.rs:110-127` still has the exact pattern R97 flagged: `install_for_tenant_if_absent` does the per-tenant pre-check then falls through to `insert_row_for_tenant` (raw `INSERT INTO skill(...)`), which hits the global UNIQUE.

### 4. Schema unchanged

```
$ sqlite3 ~/.argo/sessions.db ".schema skill"
…
tenant_id TEXT NOT NULL DEFAULT '__migration_placeholder__',
UNIQUE(name, version)        # ← no tenant_id in the constraint
```

### 5. Live DB still 7|1|7

```
$ sqlite3 ~/.argo/sessions.db "SELECT count(*) rows, count(DISTINCT tenant_id) tenants, count(DISTINCT name||'|'||version) distinct_nv FROM skill;"
7|1|7
```

Identical to R97. Only one tenant has ever successfully seeded the marketplace catalog — exactly the operator-visible bug PR #2437 claimed to close.

### 6. No PR in flight

```
$ gh pr list --search "skill-table OR seeds_independently_per_tenant"
(empty — neither phrase matches any open PR)
```

The closest in-flight PR is **#2440** (`fix-demo-seed-workflow-head-and-audit-tenant-fallback`), which addresses three *adjacent* defects R97 hinted at — workflow_definition_index never gets head-pointed at seed time, audit-feed tenant-scope falls back to literal `"ws-default"`, SPA workflows page fans out only to sample-list verticals — but **does NOT touch the skill UNIQUE constraint**. The PR body explicitly carves out `/api/v1/approvals` to a follow-up; it makes no mention of the skill-seed defect.

## Anti-theater check

This is itself an anti-theater verification. Triple-confirmed via three independent code paths:

1. **Git plumbing** — empty commit, untracked file, no ledger grep hit.
2. **Production code reads** — `store_vertical.rs:110-127` unchanged; schema unchanged.
3. **Live runtime DB** — 7|1|7 rows/tenants/distinct_nv unchanged from R97.

If I were rewriting the test green, none of these three signals would survive scrutiny. They all corroborate the same conclusion: zero substrate work has shipped on this defect.

The 24h carry is significant — the swarm is ACTIVE (multiple worker bash loops + claude instances running per `pgrep`), 31 open PRs, the conductor is merging SPA fixes at a fast cadence, but the *substrate* P0 has had no work shipped at all because it never made it into the queue.

## Gap / follow-up brief candidates

### 🔴 P0 RE-AUTHOR — `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`

Already drafted at `docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md` (238 lines in working tree). **Conductor action required**:

1. `git add docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md` (the existing draft — content reads as substantive and matches R97's recommendation).
2. Append a TODO row in `BUILD_STATUS.md` under the DEMO-RECORDING UNBLOCK or PRIORITY REORDER header so a worker actually claims it within one tick. Suggested row text: `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed | substrate fix — migration v84 relaxes UNIQUE(name,version) → UNIQUE(name,version,tenant_id) + per-tenant install-mapping; regression: seeds_independently_per_tenant in argo-demo-seed must turn 🟢 | claude or codex | TODO`.
3. Commit the brief + ledger row in ONE commit so the next tick's drift sweep doesn't surface another "no ledger row" carry.

Owner: conductor (next interactive tick) — outside my charter.

### 🟡 P1 NEW — `ops-conductor-pre-merge-gate-block-empty-commits-claiming-ledger-or-brief`

Pre-merge gate must reject commits whose message starts with `ledger:`, `brief:`, `ledger+brief:`, or `status:` but whose `git diff-tree --no-commit-id --name-only -r HEAD` returns no files. This is the second 48h failure mode where conductor process drift has shipped commits that don't do what they say (the prior was the ghost-stamp pattern flagged across R93-R97). Scope ~60 LOC + test fixture.

### 🟡 P2 NEW — `ops-evaluator-handoff-detect-untracked-brief-files`

`scripts/check-unread-handoffs.sh` could also surface untracked `docs/build-plan/streams/*.md` files that match a recent evaluator P0/P1 recommendation. Would have caught this drift on the next conductor tick after 6006cc192 instead of waiting 24h for the next evaluator round. Scope ~40 LOC.

## Sibling defect (already in flight — non-blocking note)

PR #2440 will close the `workflow_definition_index` write-gap R97 also flagged. Once #2440 lands, the live DB should move from `workflow_definition: 4 / workflow_definition_index: 1` (current) to all four definitions head-pointed. Will re-verify in R99. This is the workflow-side equivalent of the skill-side defect that #2437 failed to close.

## Operator surface

The v1.0 demo MP4 ship-gate item-set still reads "all 6 mission claims 🟢 LIVE; only operator-only demo MP4 record remains" — but Claim 6 ("multi-tenant by construction") **has a known substrate write-path defect** that a fresh second-tenant onboarding would hit immediately. Recording the demo against a single tenant masks this. If the demo script ever onboards a second workspace, the seed will panic.

Recommend the swarm clears this fix BEFORE the operator records the demo. Estimated 1-2 ticks if the brief gets into the ledger in the next conductor tick.
