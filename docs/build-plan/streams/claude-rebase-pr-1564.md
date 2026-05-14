# claude-rebase-pr-1564 — recovery brief for stalled #1564 (per-HTTP-request audit log (compliance))

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `7c2570ec` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1564`.

## Scope

Recovery for PR #1564 (`claude-mission-gateway-request-audit-log`). ~21h DIRTY against current `origin/main`. Original work is fine in shape (+1083/-20 22 files); just needs a rebase + conflict resolution + force-push.

**Gateway-zone PR — unblocked 2026-05-14 after J-series complete.** Includes a NEW raw-SQL slot 66 (`66__gateway_request_audit.sql`) — verify free at claim time per slot-64-cluster recovery pattern (slot inventory: 57-63, 64-66 free, 67-71 reserved, 72-74 taken; pick next free). Migration v63.rs filename also added — may conflict with the recently-merged #1567's v63.rs (roster policy_override). Worker may need to renumber to v64 or v65.

**Specific actions**:

1. **Chain-rebase against current `main` (HEAD `7c2570ec`)** — many landings since the PR's last push, especially the J-series (#1504 J8 + #1511 J11) + the error-types trio (#1566 Schema + #1569 Invariant + #1568 Io in flight) + structural splits (#1544 boot.rs, #1545 RUNBOOK, #1550 migrations-mod). May-touch zones: `crates/argo-cli/src/{cli_gateway_cmds,handlers/gateway/{boot,mod,request_audit},handlers/gateway_audit}.rs` + tests + `crates/argo-config/src/sections_gateway.rs` + `crates/argo-store/migrations/66__gateway_request_audit.sql` + `crates/argo-store/src/gateway_request_audit.rs`.

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

- Whatever PR #1564 already touched (`gh pr diff 1564 --color never | head -300`).
- Resolve additive conflicts as listed above.

## Files you must NOT touch

- Anything outside PR #1564's original may-touch set.
- The original design — this is rebase-only.

## Acceptance criteria

- [ ] `gh pr checkout 1564 && git fetch origin main && git merge origin/main` (or `git rebase origin/main` with `-X theirs` for #1568 mass-rename specifically).
- [ ] All conflicts resolved.
- [ ] If a migration slot needs renumbering (check `ls crates/argo-store/migrations/` for free slot), do so.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `git push --force-with-lease origin claude-mission-gateway-request-audit-log`.
- [ ] PR #1564 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-gateway-request-audit-log` (existing).
- PR title: existing.

## Notes for the implementer

- ~10-30 min mechanical rebase (longer for #1568's 253-file surface).
- `--force-with-lease` only.
- Conductor-authored 2026-05-14 wave 2 (post-J-series completion).
