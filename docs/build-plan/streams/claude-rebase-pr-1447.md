# claude-rebase-pr-1447 — recovery brief for stalled #1447 (mission-enterprise-audit-full-text-search)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `b46b0fa2` on `2026-05-14` (REFRESHED — main moved further).
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1447`.

## Scope

Recovery brief for PR #1447 per the af500037 24h-stale precedent. PR #1447 (`claude-mission-enterprise-audit-full-text-search`) has been DIRTY for 30h+ since 2026-05-12T19:02Z with no worker re-push, blocking the audit-filter-columns shipment **plus** the wider slot-64 / v60.rs migration-slot collision (also tying up #1450 and #1464).

The original work in PR #1447 is fine in shape (+1132/-45 across 18 files; new `argo-audit::filter_dims` + `routes_audit_feed_indexed.rs` + bench). Two specific bits need recovery:

1. **Chain-rebase against current `main` (HEAD `abc8c2ff`).** ~17 commits have landed since the PR's last push.
2. **Renumber the migration slot** from the contested `64` → whatever's clean at claim time. Per operator-sequenced plan: land [#1494](https://github.com/mark-hallam/Argo-Agent/pull/1494) FIRST (schema-bump 59→60 with slot 71), then this PR picks slot 65 or 66 depending on whether #1450 or #1464 gets to 65 first.

**Not in scope**: changing the brief's substantive design (audit filter columns + indexed routes), introducing new tests, broadening scope.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- Whatever PR #1447 already touched. Reference the diff with `gh pr diff 1447 --color never | head -200` to see the current may-touch set.
- Plus: rename `crates/argo-store/migrations/64__audit_record_filter_columns.sql` to whichever slot is free (likely 65/66 — verify by running `ls crates/argo-store/migrations/` and `gh pr list --state open --json files` to see slot 64 collisions).
- Update `crates/argo-store/src/migrations/registry.rs` (the new home for `all_migrations()` post `claude-store-migrations-mod-split`) — wait, that split is still IN_REVIEW. If `migrations/mod.rs` still holds the registry at claim time, target that instead.

## Files you must NOT touch

- The `argo-audit::filter_dims` module shape — the test surface from #1447 is correct; don't redesign.
- `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs` — unchanged (locked from #1494 which lands first).
- Anything outside the may-touch set already in PR #1447.

## Acceptance criteria

- [ ] Worker checked out PR #1447's branch (`gh pr checkout 1447`), rebased against current `origin/main` (`git fetch origin main && git rebase origin/main` or `git merge origin/main` — equivalent for a single-author branch).
- [ ] Migration slot renumbered from `64` to the lowest free slot at claim time. Slot number verified clean against `gh pr list --state open --json files`.
- [ ] All conflicts resolved (most are likely additive — take both sides on `Cargo.lock`, `BUILD_STATUS.md`, etc.).
- [ ] `cargo clippy -p argo-audit -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-audit -p argo-store` passes (verify migration_reconciliation_tests still green after schema-version-fixture-refresh lands, if it has).
- [ ] `git push --force-with-lease origin claude-mission-enterprise-audit-full-text-search` — force-with-lease is OK here, this is the original branch.
- [ ] PR #1447 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-enterprise-audit-full-text-search` (re-use the existing PR branch — do NOT open a new PR).
- PR title: existing one stays.

## Notes for the implementer

- This is a mechanical recovery. The brief's intent is unchanged.
- If the migration slot collision can't be resolved cleanly (e.g. 4 PRs all need slots 64-67 simultaneously), BLOCK with a one-line reason and let the conductor coordinate operator sequencing.
- The pre-rebase worker is presumed dead (30h+ silence). `--force-with-lease` is the safe push (refuses if remote moved unexpectedly).
- Operator authorized recovery 2026-05-13 in conductor briefing.
