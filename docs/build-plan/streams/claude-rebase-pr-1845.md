# claude-rebase-pr-1845 — recover #1845 mission-pm-approval-target-scope (cascading-slot recovery)

**Owner**: claude. **Recovery target**: PR [#1845](https://github.com/mark-hallam/Argo-Agent/pull/1845). **Original brief**: `streams/mission-pm-approval-target-scope.md`.
**Authored against**: origin/main `9508376e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1845`.
**Priority**: P0 — closes audit P0 #3 (per-target glob on session grants), part of the PM teammate track.

## Why this brief exists

PR #1845 was opened 2026-05-19T09:10Z by a claude worker. Conductor flagged a slot-117 collision in tick-5 (#1843 took 117 first); worker force-pushed at `2a36544` in tick-6 but did NOT actually renumber — the rebase only updated some files, leaving the migration still at slot 117. By tick-6 #1846 had also taken slot 118; conductor posted updated guidance for slot 119 + v73 + schema 73.

The worker did not respond. Two conductor fix-request comments are on the PR thread; last force-push was >24h ago. Per `PROTOCOL_LOOP_PROMPT.md` stale-IN_REVIEW boundary rule, a recovery brief is the right next step (not a demote).

**Slot situation has now drifted further.** Slots 117-122 are all taken on current main:

| slot | brief | merged |
|---|---|---|
| 117 | approval_draft_content | #1843 |
| 118 | workflow_definition | #1846 |
| 119 | worker_freeze | #1850 |
| 120 | session_routing_policy | #1847 |
| 121 | skill_reeval_history | #1851 |
| 122 | trust_streak_view | #1856 |

The next free slot is **123**.

## Acceptance: pick ONE recovery path

### Path A — rebase (strongly preferred — preserves the worker's adds)

1. `git fetch origin && git checkout claude-mission-pm-approval-target-scope`
2. `git rebase origin/main`. Expected conflicts in this order:
   - **`crates/argo-store/migrations/`** — your branch's `117__session_grant_target.sql` MUST be renamed to `123__session_grant_target.sql` (slots 117-122 all taken by other-track merges). Use `git mv` not delete+add.
   - **`crates/argo-store/src/migrations/v71.rs`** — rename to `v75.rs` (post-v74 from #1856 trust-streak). Update the version constant inside the file from 71 to 75.
   - **`crates/argo-store/src/migrations/mod.rs`** — accept main's `pub mod v71..v74;` chain, append `pub mod v75;` after.
   - **`crates/argo-store/src/migrations/registry_compliance.rs`** — set `EXPECTED_SCHEMA_VERSION = 75` (one higher than #1856's 74).
   - **`crates/argo-store/src/migrations/registry.rs`** — register slot 123 / v75 entry alongside the existing 117/118/119/120/121/122 entries.
   - **`crates/argo-store/src/grants.rs`** — likely no conflict; your branch's adds (the `target_glob` column + `add_session_grant` signature change + `find_session_grant_for_dispatch` consume-side helper) are purely additive on the file's surface.
   - **`crates/argo-cli/src/handlers/session.rs`** — may conflict because #1847 added a `SessionCmd::SetRoutingPolicy` variant that lives in the same enum file. Both adds should land side-by-side.
   - **`crates/argo-cli/src/cli_session_cmds.rs`** — same story (your `--target <glob>` flag + #1847's `set-routing-policy` subcommand).
3. Verify locally:
   - `bash scripts/check-migration-slots.sh` — should pass (slot 123 + v75 unique).
   - `cargo test -p argo-store grants migration_upgrade` — should pass.
   - `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` — should be clean.
4. `git push --force-with-lease origin claude-mission-pm-approval-target-scope`
5. Comment on #1845: "✅ rebased + force-pushed onto origin/main `<sha>`; slot renumbered 117 → 123, v71 → v75, schema 71 → 75."

### Path B — fresh start (LAST RESORT — only if Path A surface is too wide)

If Path A's rebase produces more than ~5 files of conflict outside the migration plumbing, close #1845 SUPERSEDED and open a fresh `mission-pm-approval-target-scope-v2` PR. Note: this would be the THIRD attempt at the same brief; flag the operator before going this path.

## Acceptance criteria

- [ ] PR #1845 ends MERGEABLE / non-DIRTY against current main.
- [ ] Migration is at slot `123__session_grant_target.sql` + `v75.rs` + `EXPECTED_SCHEMA_VERSION = 75`.
- [ ] `bash scripts/check-migration-slots.sh` passes.
- [ ] `cargo test -p argo-store grants migration_upgrade` passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] `argo session approve <sid> --tool <name> --target <glob>` CLI works as in original brief.
- [ ] `find_session_grant_for_dispatch` consume-side check behaves correctly per the original brief acceptance criteria.
- [ ] On merge, conductor flips both rows (mission-pm-approval-target-scope + claude-rebase-pr-1845) to MERGED.

## Notes for the implementer

- This is the THIRD slot cascade on this PR (117 → 119 was advised; the worker didn't actually renumber; now 123 is the only safe slot). Slot cascades happen because multiple branches accrete migrations in parallel. There is currently no automated "next available slot" claim — workers pick a slot at brief-author time and may collide. The cleanest mitigation here: do the rename atomically with the rebase and run `scripts/check-migration-slots.sh` BEFORE force-pushing.
- Don't be tempted to merge in just one tier of conflicts and force-push partial work. The conductor's previous fix-request was triggered exactly because a force-push happened with the rename incomplete — leading to a third hop. Land the rebase + rename + all tests passing IN ONE force-push.
- Check `crates/argo-types/src/capability.rs` for `Capability::ToolDispatch::tool_name_glob` — your `target_glob` field on `add_session_grant` uses the same `globset::Glob` library; verify that's still the import path on current main (no `argo-types` reorg has landed that affects it as of 2026-05-20).
