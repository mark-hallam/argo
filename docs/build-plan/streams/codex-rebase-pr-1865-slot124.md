# codex-rebase-pr-1865-slot124 — rebase #1865 + rename migration slot 123 → 124

**Owner**: codex. **Recovery target**: PR [#1865](https://github.com/mark-hallam/Argo-Agent/pull/1865). **Original brief**: `streams/mission-audit-principal-tenant-from-token.md`.
**Authored against**: origin/main `70fc980b` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rebase-pr-1865-slot124`.
**Priority**: P1 — closes audit P1-2 (principal+tenant from token, not header).

## Why this brief exists

PR #1865 (`mission-audit-principal-tenant-from-token`) is fix-requested with a slot collision: it adds `crates/argo-store/migrations/123__service_token_binding.sql`, but slot 123 was already claimed by #1845 (`123__session_grant_target.sql`, MERGED). The boot-time `duplicate_slot_validator::validate_no_duplicate_slots()` would reject the workspace.

The functional code (service_token_binding table + principal-binding routes) is fine as-is. Only the slot number + the migration registry needs to shift.

## Fix steps

1. `git fetch origin && git checkout claude-mission-audit-principal-tenant-from-token`
2. `git rebase origin/main` against current `origin/main` (`70fc980b` or newer).
3. Rename the migration:
   - `git mv crates/argo-store/migrations/123__service_token_binding.sql crates/argo-store/migrations/124__service_token_binding.sql`
4. Update the migration registry: any `vNN.rs` file the PR added, plus `crates/argo-store/src/migrations/mod.rs` + `crates/argo-store/src/migrations/registry_compliance.rs`. The PR likely added `v75.rs`; rename to `v76.rs` (or whichever maps to slot 124 — verify with the next-free version number after `v75`).
5. Update any `LATEST_SCHEMA_VERSION` constant if the PR set it; should now be the v76 (or equivalent) number.
6. Run: `bash scripts/check-migration-slots.sh` must report no duplicate slots.
7. Run: `cargo test -p argo-store --lib migration` + the PR's specific tests to confirm.
8. `git push --force-with-lease origin claude-mission-audit-principal-tenant-from-token`
9. Comment on #1865: "✅ rebased + slot renamed 123 → 124. Migration registry updated to v76 (or whichever)."

## Acceptance criteria

- [ ] PR #1865 ends MERGEABLE / non-DIRTY against current main.
- [ ] No slot collisions (`bash scripts/check-migration-slots.sh` clean).
- [ ] All PR-original tests pass.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Migration registry version bumped consistently.

## Out of scope

- Any change to the service_token_binding functional code — the principal-binding logic and the 3 affected routes (`routes_auth_principal`, `routes_skill_ir`, `routes_ghostwriter`) are correct as-is.
- Adding new tests — existing tests should still pass after the slot rename.
