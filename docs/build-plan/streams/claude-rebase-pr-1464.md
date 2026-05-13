# claude-rebase-pr-1464 — recovery brief for stalled #1464 (mission-scim-provisioning)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `d86af369` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1464`.

## Scope

Recovery for PR #1464 (`claude-mission-scim-provisioning`). DIRTY since 2026-05-12T16:32 — ~18h since worker's last update. Slot-64 / v60.rs collision-cluster member. CEO-authorized recovery (operator briefing 2026-05-13).

Original work shipped: +1774/-3 across 14 files. New `routes_scim_users.rs` + `_envelope.rs` + `_tests.rs` + `scim_auth.rs` + migration `64__scim_provisioned_tombstones.sql` + `routes_auth.rs` extension.

**Specific recoveries needed**:

1. **Chain-rebase against current `main` (HEAD `d86af369`)** — many landings since 18h ago: gateway-boot-split (#1544), runbook split (#1545), troubleshoot/error-dashboard (#1546-1547), gateway-tls (#1548), prompt-cache trio (#1549), migrations-mod-split (#1550), schema-fix (#1551), cli-subcommands-skill-split (#1552), 4 docs PRs (#1553-1556), cross-tenant audit (#1450 via recovery), telemetry context (#1555), mcp-pool (#1540 via recovery), and a stale-codex-claim cleanup. Worker should rebase + take both sides on additive conflicts.

2. **Renumber migration slot 64 → next free** — slot 64 was contested. #1450's recovery took slot 65, so #1464 should take 66 (or whatever's free at claim time; verify via `ls crates/argo-store/migrations/`). v60.rs no longer needs touching — see #1450's recovery for the pattern that avoids v60.rs entirely.

3. **Update for migrations-mod-split (#1550)** — the `all_migrations()` registry moved to `crates/argo-store/src/migrations/registry.rs`. The new `Migration` literal lands there now, not in `mod.rs`.

**Not in scope**: changing the SCIM design; redesigning the schema; introducing new endpoints.

## Files you may touch

- Whatever PR #1464 already touched (`gh pr diff 1464 --color never | head -200`).
- Rename `migrations/64__scim_provisioned_tombstones.sql` → next free slot.
- Update the `Migration` literal to land in `migrations/registry.rs` (post-#1550 split), not `mod.rs`.
- Resolve additive conflicts on `Cargo.lock`, `BUILD_STATUS.md`, etc. (take both sides).

## Files you must NOT touch

- The SCIM API design — locked from PR #1464's original work.
- `EXPECTED_SCHEMA_VERSION` — #1450's recovery bumped to 61 already; verify on rebase.
- Anything outside PR #1464's existing may-touch set.

## Acceptance criteria

- [ ] `gh pr checkout 1464 && git fetch origin main && git merge origin/main`.
- [ ] Migration slot renumbered cleanly (verify against `gh pr list --state open --json files`).
- [ ] Migration literal moved to `registry.rs`.
- [ ] All conflicts resolved additively (take both sides where additive).
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli` passes.
- [ ] `git push --force-with-lease origin claude-mission-scim-provisioning`.
- [ ] PR #1464 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-scim-provisioning` (existing PR branch).
- PR title: existing.

## Notes for the implementer

- This completes the slot-64 cluster: #1450 ✅ (recovered, slot 65), #1447 (recovery in flight), this brief (slot 66+).
- v60.rs is fully clear of collisions — don't touch it.
- `--force-with-lease` only.
- Operator authorized recovery 2026-05-13.
