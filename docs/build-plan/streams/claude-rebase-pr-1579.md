# claude-rebase-pr-1579 — recovery brief for stalled #1579 (AnomalyDispatch + wire 4 anomaly classes)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `b46b0fa2` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1579`.

## Scope

Recovery for PR #1579 (`claude-mission-anomaly-alerting-notifychannel-v3`). ~13h DIRTY against current `origin/main`. Original work is fine in shape (+783/-29 17 files); just needs a rebase + conflict resolution + force-push.

Cluster D #5 v3 rewrite. PR was CLEAN at first merge attempt but re-conflicted in the merge wave. Clean rebase against current main.

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `b46b0fa2`)** — many landings this session: J-series (#1504+#1511), error-types trio (Schema #1566 + Invariant #1569 + Io #1568), structural splits (#1544 boot.rs, #1545 RUNBOOK, #1550 migrations-mod, #1592 sections_context full split, #1593 testing.rs split, #1587 wizard_onboard split), big cleanups (#1575 17-connectors, #1580 argo-gateway crate, #1585 image-gen dup, #1582 dead audit surface). May-touch zones: `crates/argo-channels/**` + `crates/argo-orchestrator/**`.

2. **Resolve additive conflicts** — take both sides on Cargo.lock, BUILD_STATUS.md, lib.rs re-exports, config-section field additions.

3. **Pattern updates from session merges**:
   - If brief touched `migrations/mod.rs` registry → use `migrations/registry.rs` (post-#1550 split).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).
   - If brief touched `sections_context.rs` → use the new per-config siblings (post-#1592 split).
   - If brief appended to RUNBOOK.md → use per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split).
   - If brief raised `ArgoError::Provider(format!("invalid|missing|bad|unknown ..."))` for caller-supplied input → switch to `ArgoError::Schema(...)` (post-#1566 sweep).
   - If brief raised `ArgoError::Provider(...)` for I/O failure → switch to `ArgoError::Io(...)` (post-#1568 sweep).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1579 already touched (`gh pr diff 1579 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1579's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1579 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-mission-anomaly-alerting-notifychannel-v3`.
- [ ] PR #1579 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-anomaly-alerting-notifychannel-v3` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase.
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 wave 3 (post-massive-merge-cascade).
