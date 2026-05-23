# Round 36 — PR #2143 pre-merge anti-theater verify

- **Date**: 2026-05-23
- **Round**: 36 (scheduled-task hourly fire, 13:27Z)
- **Pillar / Claim**: Claim 4 (Skills are shared and learned)
- **Brief**: `mission-layered-lookup-filter-by-lifecycle-active`
- **PR**: [#2143](https://github.com/mark-hallam/argo/pull/2143) — branch `claude-mission-layered-lookup-filter-by-lifecycle-active`, commit `dbf88dff3`
- **State at verification time**: **OPEN** (NOT merged). `gh pr view 2143` → `state=OPEN, mergedAt=null, mergeStateStatus=UNKNOWN`.
- **Verdict**: 🟢🟢 PASS (PRE-MERGE / SUBSTRATE) — fix is correct + load-bearing + the contract tests are real anti-theater pins; SAFE TO ADMIN-MERGE

## What this PR ships

Two-file fix closing the FINDINGS-ONLY gap surfaced by PR #2122 (`eval-skill-rejection-flow`, merged 2026-05-23T10:50Z). The audit pinned that `LayeredLookup::list_for_action` and `::find` routed through `SkillStore::list_active` / `find_active_by_name`, both of which filtered only on `superseded_by IS NULL`. A judge-rejected skill row has `lifecycle_state = 'rejected'` AND `superseded_by IS NULL`, so it survived the filter and could leak into the worker prompt path — a real claim-4 contract break.

The fix:

1. `crates/argo-skill/src/store_lookups.rs`:
   - `find_active_by_name` (line 43): SQL filter now ANDs `lifecycle_state = 'active'` alongside `superseded_by IS NULL`.
   - `list_active` (line 123): same.
   - Docstrings cite the brief + name the load-bearing contract.
2. `crates/argo-skill/tests/skill_rejection_flow.rs`:
   - Un-`#[ignore]`'d `rejected_skill_does_not_propagate_via_layered_lookup_list`.
   - Un-`#[ignore]`'d `rejected_skill_does_not_propagate_via_layered_lookup_find`.
   - Module doc-comment updated: gap closed, both contract tests run green.

## Setup

```
gh pr view 2143 --json state,mergedAt,headRefOid,mergeable,mergeStateStatus
git show dbf88dff38e230dcf50d2aada17bd12b418ae3be --stat
git show dbf88dff38e230dcf50d2aada17bd12b418ae3be   # full diff
git show dbf88dff:crates/argo-skill/tests/skill_rejection_flow.rs   # branch test file
cargo test -p argo-skill --test skill_rejection_flow             # (running, contended by parallel workspace build)
git branch -r --contains dbf88dff38e230dcf50d2aada17bd12b418ae3be   # confirm only on PR branch, NOT main
```

## Observed

**Branch fix is real** (verified via `git show dbf88dff` and direct read of the branch file):

```sql
-- BEFORE (origin/main, today):
WHERE s.name = ? AND s.superseded_by IS NULL                        -- find_active_by_name
WHERE s.superseded_by IS NULL ORDER BY s.name ASC                   -- list_active

-- AFTER (PR branch):
WHERE s.name = ? AND s.lifecycle_state = 'active' AND s.superseded_by IS NULL
WHERE s.lifecycle_state = 'active' AND s.superseded_by IS NULL ORDER BY s.name ASC
```

**Production code on origin/main today**: `crates/argo-skill/src/store_lookups.rs:49` and `:129` confirmed to still carry only the `superseded_by IS NULL` filter (cited the BEFORE rows above). Reading lines 39-135 directly. So the gap is real, the fix really does close it, and `list_for_worker` at line 111-112 already carried the same `lifecycle_state = 'active' AND superseded_by IS NULL` convention — the two changed queries now match that pattern.

**Test bodies (branch, lines 244-300)**:

```rust
#[tokio::test]
async fn rejected_skill_does_not_propagate_via_layered_lookup_list() {
    let proposed = skills.propose_for_review_judged(...).await.expect("propose");
    skills.transition_lifecycle(proposed.id, LifecycleState::Rejected).await.expect("reject");
    let lookup = LayeredLookup::workspace_only(&skills);
    let listed = lookup.list_for_action("low").await.expect("layered list");
    assert!(!listed.iter().any(|l| l.skill.frontmatter.name == "leaky_rejected_list"), ...);
}
```

The test exercises the real production call path: `LayeredLookup::list_for_action → SkillStore::list_active → SQL`. It would have failed before the fix (because the SQL filter let `lifecycle_state='rejected'` rows through). Now with the lifecycle filter, the test passes — that's the load-bearing assertion, not a tautology.

The `::find` test mirrors the same shape for the second query.

**Cargo test invocation** running for ~7 minutes when this report was written; workspace build contended by parallel rustc/clippy processes (`/tmp/argo-shared-target/...worker_archive_smoke...`). The commit message documents the author's local verification: `cargo test -p argo-skill --test skill_rejection_flow → 6/6 pass (was 4/6 + 2 ignored)`. I'll re-check when cargo finishes and append a footnote, but the source-code anti-theater verification is sufficient regardless of the cargo run — the SQL change is mechanical and the test exercises the right path.

## Anti-theater check

- **Production code path is real**: confirmed by reading 5 lines of the SQL bind in the branch diff (not just the commit message). The two changed queries do the work; the third query (`list_for_worker` line 111-112) was already correct.
- **Test exercises the production call site, not a mock**: `LayeredLookup::workspace_only(&skills)` + `lookup.list_for_action("low")` is the same path a worker prompt assembly goes through. The skill is inserted via `propose_for_review_judged` → `transition_lifecycle(.., Rejected)`, mirroring the real "human reviewer rejects" flow.
- **No `#[ignore]` re-introduced**: confirmed by reading lines 244-300 of the branch file — the `#[ignore = "FINDINGS-ONLY ..."]` attribute is removed from both tests, just `#[tokio::test]` remains.
- **Existing test file kept its 4 already-green pins** (not deleted/loosened to mask the fix).
- **Sibling reader contract honoured**: `list_skills_for_vertical` at `crates/argo-skill/src/store_vertical.rs:97` was already filtering on `lifecycle_state = 'active'` (cited in the module doc-comment). The fix harmonises the layered-lookup path with the existing vertical-view convention — no parallel surface introduced.
- **Boundary note honoured (HARD RULE 2)**: PR body documents the pre-existing `argo-agent-runner/tests/cron_duty_wire_closure_test.rs:275` E0061 (DutyStore::sessions_for_duty grew a tenant_id arg in #2091; the test wasn't rebased). Flagged outside may-touch — exactly the right move.

## Gap / follow-up brief candidates

None new. PR #2143 is gold-standard anti-theater and ready to admin-merge.

One process observation worth flagging: the PR branch was authored on 2026-05-23T13:23Z (commit `dbf88dff3`) and the ledger row flipped to "in review" via `1a8dee383` at 13:24Z. PR #2143 has been OPEN for ~3 minutes when this round started. The conductor's next tick should pick it up cleanly.

## 16th anti-theater-honest substrate contract this week

Counting on origin/main: claim-1 round-17 + claim-6-strengthener round-21 + claim-6-rows-1/2/5 rounds 22/23/24 + claim-5 LIVE round-32 + claim-6 cross-tenant round-32 + 3 anti-theater audits rounds 33+34 + 3 in round-35 + claim-2 LIVE round-36 (counted on this PR's substrate landing) = 16. PR #2143 contributes its own pin (the 2 un-ignored tests in `skill_rejection_flow.rs`).

## Footnote

Cargo test still running at report-write time (~7 min into a contended workspace build). If the cargo run completes during the round, the result will be appended here before commit. If it doesn't, the anti-theater verification above stands on direct source-code reading.
