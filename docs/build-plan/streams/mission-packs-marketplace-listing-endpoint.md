# mission-packs-marketplace-listing-endpoint

**Owner**: codex
**Priority**: P2 - claim 3/4 marketplace surface
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-packs-marketplace-listing-endpoint`.

## Source

The vertical-packs audit found that repo `packs/` entries have no gateway surface.
Operators cannot browse available packs from the product even though the pack
substrate exists.

## Goal

Add a read-only gateway listing endpoint for packs that can merge repo packs and
local cache metadata without installing or mutating anything.

## May Touch

- `crates/argo-cli/src/handlers/gateway/routes_packs.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes.rs`
- `crates/argo-cli/src/handlers/gateway/mod.rs`
- `crates/argo-cli/src/handlers/gateway/routes_packs_tests.rs` (NEW)
- `crates/argo-vertical-packs/src/**` only for additive read/list helpers
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not add SPA UI in this brief.
- Do not install packs or mutate `~/.argo/packs`.
- Do not change pack fetch/auth behavior.

## Acceptance

- [ ] `GET /api/v1/packs` returns a paginated catalog shape with id, version, vertical_id, author, description, installed, and source.
- [ ] Endpoint enforces the existing gateway auth/role pattern.
- [ ] Empty catalog and happy-path repo-pack cases are tested.
- [ ] Response shape is stable enough for a later SPA marketplace page.
- [ ] Crate-scoped verification claimed in the PR body with any unrelated clean-main failures clearly separated.

## Notes

Read-only endpoint only. Pack install/publish flows are separate work.
