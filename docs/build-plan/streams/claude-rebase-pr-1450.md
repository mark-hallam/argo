# claude-rebase-pr-1450 — recovery brief for stalled #1450 (mission-enterprise-cross-tenant-audit)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `abc8c2ff` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1450`.

## Scope

Recovery brief for PR #1450 per the af500037 24h-stale precedent. Sibling to `claude-rebase-pr-1447`. PR #1450 (`claude-mission-enterprise-cross-tenant-audit`) has been DIRTY for 30h+ since 2026-05-12T19:17Z with no worker re-push.

The original work is +909/-24 across 19 files (new `migrations/64__audit_record_tenant_id.sql` + cross-tenant smoke + audit-sink updates + capability hooks). Two specific recoveries:

1. **Chain-rebase against current `main` (HEAD `abc8c2ff`).**
2. **Renumber the migration slot** away from contested `64`. Per operator-sequenced plan: #1494 lands first (slot 71 + schema bump 60), then this PR picks the next free slot after #1464 settles.

**Not in scope**: changing the cross-tenant isolation semantics, redesigning the `tenant_id` column shape, broadening scope.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- Whatever PR #1450 already touched (`gh pr diff 1450 --color never | head -200`).
- Rename `crates/argo-store/migrations/64__audit_record_tenant_id.sql` to the lowest free slot at claim time.
- Migration registry — `crates/argo-store/src/migrations/registry.rs` (post `claude-store-migrations-mod-split`) or `mod.rs` if that split hasn't landed yet.

## Files you must NOT touch

- The cross-tenant isolation invariant in `argo-audit::sink` — locked from #1450's original design.
- `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs`.
- Anything outside PR #1450's existing may-touch set.

## Acceptance criteria

- [ ] `gh pr checkout 1450 && git fetch origin main && git merge origin/main` (or rebase).
- [ ] Migration slot renumbered cleanly, verified against open-PR slot reservations.
- [ ] Conflicts resolved additively.
- [ ] `cargo clippy -p argo-audit -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-audit -p argo-store` passes.
- [ ] `git push --force-with-lease origin claude-mission-enterprise-cross-tenant-audit`.
- [ ] PR #1450 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-enterprise-cross-tenant-audit` (existing PR branch).
- PR title: existing.

## Notes for the implementer

- Mechanical. Same shape as `claude-rebase-pr-1447`.
- Migration slots must NOT collide with #1447's recovery. Coordinate by checking `gh pr diff 1447 --color never | grep 'migrations/'` and picking a different slot. If you claim #1450 while #1447 is still stale, pick a slot 1 higher than whatever #1447's recovery used; if your peer worker is doing #1447 in parallel, use sticky-claim wisdom (one slot apart, deterministic by PR number).
- `--force-with-lease` only; the abandoned worker is presumed dead.
- Operator authorized recovery 2026-05-13 in conductor briefing.
