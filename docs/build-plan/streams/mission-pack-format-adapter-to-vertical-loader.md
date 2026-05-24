# mission-pack-format-adapter-to-vertical-loader

**Owner**: codex
**Priority**: P1 - claim 3/4 marketplace bridge
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pack-format-adapter-to-vertical-loader`.

## Source

The vertical-packs audit found two parallel marketplace formats: repo `packs/`
entries use `worker.toml`, while runtime `verticals/` entries use
`vertical.toml`. The pack cache is operationally invisible unless it can adapt
to the vertical loader.

## Goal

Add a narrow adapter that translates a pack manifest into an `argo_vertical`
vertical representation and prove one real pack can load without a parallel
`verticals/` entry.

## May Touch

- `crates/argo-vertical-packs/src/adapter.rs` (NEW)
- `crates/argo-vertical-packs/src/lib.rs`
- `crates/argo-vertical-packs/src/loader.rs`
- `crates/argo-vertical-packs/tests/pack_adapter_round_trip_test.rs` (NEW)
- `packs/**` only for a minimal fixture correction if required
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not rewrite `argo-vertical` loader internals.
- Do not change pack registry/network fetch behavior.
- Do not add gateway or SPA marketplace routes in this brief.

## Acceptance

- [ ] Adapter maps core pack fields to a loadable vertical shape.
- [ ] At least one existing pack round-trips through adapter plus vertical loader.
- [ ] Missing/unsupported pack fields fail with a specific error, not silent defaulting.
- [ ] `PackLoader` exposes a narrow load-as-vertical method or equivalent helper.
- [ ] Crate-scoped verification claimed in the PR body with any unrelated clean-main failures clearly separated.

## Notes

Keep this as the bridge layer only. Marketplace listing endpoint is a sibling row.
