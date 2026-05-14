# mission-forge-dynamic-cost-registry — Link executor cost ceiling to dynamic price registry

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `bfe849f4` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-dynamic-cost-registry`.

## Scope

Phase B audit P2 follow-up (`docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md` §5 TODO #3). `argo-executor-computer-use/src/executor.rs:190` has a TODO: "Profile-pinned tokens-per-dollar is TODO; future: read from price registry". Currently the executor uses a hard-coded conservative cost ratio, which makes budget ceiling calculations pessimistic.

This brief wires the executor to a runtime-accessible price registry that argo-forge already maintains. The registry exposes per-model tokens-per-dollar pricing; the executor reads it on each session start and uses the model-specific ratio for budget projection.

LOC: ~250.

## Files you may touch

- `crates/argo-executor-computer-use/src/executor.rs` — narrowly: replace hard-coded ratio with registry lookup at session-start. Fall back to conservative default if registry unavailable. ≤80 modified lines.
- `crates/argo-executor-computer-use/src/executor_tests.rs` (if exists, else new sibling) — test: registry-driven ratio applied; fallback path covered. ≤100 modified lines.
- `crates/argo-forge/src/pricing.rs` (or wherever pricing already lives) — narrowly: ensure pub accessor exists for per-model rate lookup. ≤30 modified lines.
- `crates/argo-forge/Cargo.toml` — confirm no new deps needed.
- `crates/argo-executor-computer-use/Cargo.toml` — add `argo-forge = { workspace = true }` if not present.  ≤3 lines.

## Files you must NOT touch

- Anything outside the brief's may-touch set per CLAUDE.md.
- The forge stub (separate concern; that's `claude-forge-dspy-stub-warning-upgrade` already MERGED).

## Acceptance criteria

- [ ] Executor reads model-specific cost ratio from forge pricing registry at session start.
- [ ] Fallback to conservative default if registry lookup fails (no panics).
- [ ] 3 unit tests: registry hit, registry miss → fallback, registry error → fallback.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-forge-dynamic-cost-registry`
- PR title: `feat(executor): link CU cost ceiling to forge price registry (mission-forge-dynamic-cost-registry)`

## Notes for the implementer

- Mission-claim alignment: production hardening (better cost-budget projections; supports claim 4 and 5).
- This is a post-v1 polish item — not v1 ship-blocking.
- Drift-prevention: no new pub types; consumes existing forge pricing surface.
