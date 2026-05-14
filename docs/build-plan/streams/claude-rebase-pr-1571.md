# claude-rebase-pr-1571 — recovery brief for stalled #1571 (/openmetrics route + default_registry())

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `b46b0fa2` on `2026-05-14` (REFRESHED — main moved further).
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1571`.

## Scope

Recovery for PR #1571 (`claude-mission-prometheus-exporter-openmetrics-route`). ~13h DIRTY against current `origin/main`. Original work is fine in shape (+902/-16 15 files); just needs a rebase + conflict resolution + force-push.

**Gateway-zone PR.** Light touch on `handlers/gateway/{boot,mod,routes_auth}.rs` (route registration). Rebase against HEAD `7c2570ec`; post-J-series the routes_auth.rs additions may need to land in routes_dispatch_get.rs instead (verify gateway routing topology at claim time).

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `7c2570ec`)** — many landings since the PR's last push, especially the J-series (#1504 J8 + #1511 J11) + the error-types trio (#1566 Schema + #1569 Invariant + #1568 Io in flight) + structural splits (#1544 boot.rs, #1545 RUNBOOK, #1550 migrations-mod). May-touch zones: `crates/argo-cli/src/handlers/gateway/{boot,mod,routes_auth}.rs` + `crates/argo-audit/src/sink.rs` + Cargo.toml additions.

2. **Resolve additive conflicts** — take both sides on Cargo.lock, BUILD_STATUS.md, lib.rs re-exports, config-section field additions, gateway routes_dispatch_*.rs route-table edits.

3. **Pattern updates from session merges**:
   - If brief touched `migrations/mod.rs` registry → use `migrations/registry.rs` (post-#1550 split).
   - If brief touched boot.rs spawn helpers → use `boot_spawns.rs` (post-#1544 split).
   - If brief appended to RUNBOOK.md → use per-topic sub-file under `docs/skycot/runbook/` (post-#1545 split).
   - If brief raised `ArgoError::Provider(format!("invalid|missing|bad|unknown ..."))` for caller-supplied input → switch to `ArgoError::Schema(...)` (post-#1566 sweep).
   - If brief raised `ArgoError::Provider(...)` for I/O failure → switch to `ArgoError::Io(...)` (post-#1568 sweep landing soon).
   - If brief added a migration in `v6X.rs` Rust file → verify next free Rust version at claim time (post-#1567 v63.rs landing).

**Not in scope**: changing the original design; introducing new features; widening may-touch beyond the original PR's footprint.

## Files you may touch

- Whatever PR #1571 already touched (`gh pr diff 1571 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1571's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1571 && git fetch origin main && git merge origin/main` (or `git rebase origin/main` with `-X theirs` for #1568 mass-rename specifically).
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-mission-prometheus-exporter-openmetrics-route`.
- [ ] PR #1571 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-prometheus-exporter-openmetrics-route` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase (longer for #1568's 253-file surface).
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 wave 2 (post-J-series completion).
