# claude-rebase-pr-1559 — recovery brief for stalled #1559 (SIEM dispatcher (Splunk/Elastic/Datadog))

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `b46b0fa2` on `2026-05-14` (REFRESHED — main moved further).
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1559`.

## Scope

Recovery for PR #1559 (`claude-mission-siem-integration`). ~29h DIRTY against current `origin/main`. Original work is fine in shape (+1573/-2 20 files); just needs a rebase + conflict resolution + force-push.

**Light gateway touch (boot.rs imports + 1 startup spawn) — NOT a gateway-zone collision risk despite the file path.** Rebase against current main (HEAD `902d9bbc`). Main conflict surface: `crates/argo-config/src/lib.rs` + `sections.rs` (additive — take both sides on SiemConfig field addition; #1566 Schema-variant rewrite may have shifted error sites in argo-config). Boot.rs touch is post-split (#1544); use `boot_spawns.rs` for the spawn helper if the brief targeted boot.rs directly.

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `902d9bbc`)** — many landings since the PR's last push (boot.rs split #1544, runbook split #1545, migrations-mod split #1550, mcp-pool #1540, structured-logging #1555, error-types Schema rewrite #1566, roster policy_override #1567, plus other cleanup PRs). May-touch zones: NEW `crates/argo-siem/` crate, `crates/argo-cli/src/handlers/gateway/boot.rs` (light: 3 imports + spawn_siem_dispatcher() call), `crates/argo-config/src/{lib,sections,sections_siem}.rs`..

2. **Resolve additive conflicts** — take both sides on Cargo.lock, BUILD_STATUS.md, lib.rs re-exports, config-section field additions.

3. **Pattern updates from session merges**:
   - If brief touched `migrations/mod.rs` registry → use `migrations/registry.rs` (post-#1550 split).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).
   - If brief appended to RUNBOOK.md → use per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split).
   - If brief raised `ArgoError::Provider(format!("invalid|missing|bad|unknown ..."))` for caller-supplied input → switch to `ArgoError::Schema(...)` (post-#1566 sweep).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1559 already touched (`gh pr diff 1559 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1559's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1559 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so and avoid v60.rs.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-mission-siem-integration`.
- [ ] PR #1559 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-siem-integration` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase depending on conflict surface.
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 batch; one of 6 recovery briefs in this wave.
