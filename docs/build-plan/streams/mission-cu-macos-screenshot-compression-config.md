# mission-cu-macos-screenshot-compression-config

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a9675bad2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-macos-screenshot-compression-config`.

P2 mission claim 5 hardening. `docs/skycot/COMPUTER_USE_NATIVE_MACOS.md`
calls out native macOS screenshot storage cost as an open follow-up:
Retina PNG captures can be 5-10x larger than E2B browser screenshots.
Add a conservative compression knob before long native trajectories
become routine.

## Scope

Add optional screenshot compression settings to
`MacosCuConfig` and thread them into the native macOS screenshot path.
The default behavior must remain unchanged: PNG capture, no downscale,
no lossy conversion. When configured, the executor should downscale to
a max edge and/or convert the persisted audit image to JPEG using
macOS built-ins (`sips` is acceptable) without adding a new image
processing crate.

Keep the change local to the macOS executor. Do not alter the E2B
computer-use executor or gateway dispatch.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use-macos/src/config.rs` - add optional compression fields + validation + unit tests.
- `crates/argo-executor-computer-use-macos/src/screenshot.rs` - add helper(s) for post-capture transform command construction/execution.
- `crates/argo-executor-computer-use-macos/src/executor.rs` - thread config into screenshot capture/read path if needed.
- `crates/argo-executor-computer-use-macos/src/executor_loop.rs` - only if the loop owns the screenshot path that must change.
- `crates/argo-executor-computer-use-macos/src/lib.rs` - only for a narrow re-export if a new public config type is introduced.

## Files you must NOT touch

- `crates/argo-executor-computer-use/**` - E2B/browser executor out of scope.
- `crates/argo-cli/**` - dispatcher behavior out of scope.
- `docs/skycot/COMPUTER_USE_NATIVE_MACOS.md` - source doc only; no doc rewrite in this implementation brief.
- `Cargo.toml` / `Cargo.lock` - no new dependencies for this brief.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `MacosCuConfig::for_notes_app()` preserves today's PNG/no-compression behavior.
- [ ] Config validation rejects impossible compression settings (for example zero max edge or JPEG quality outside the chosen range).
- [ ] A worker can opt into a max-edge downscale and JPEG output through explicit config fields.
- [ ] Unit tests cover default behavior, invalid settings, and command/extension selection without requiring macOS permissions.
- [ ] Off-macOS tests still pass; permission-gated native capture tests remain gated.
- [ ] No new crate dependency is added.
- [ ] `cargo test -p argo-executor-computer-use-macos` passes.
- [ ] `cargo clippy -p argo-executor-computer-use-macos --all-targets -- -D warnings` passes.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` or `scripts/workspace-clippy-fast-fail.sh` result is claimed in the PR body.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-mission-cu-macos-screenshot-compression-config`
- PR title: `cu(macos): configurable screenshot compression (mission-cu-macos-screenshot-compression-config)`

## Notes for the implementer

- Prefer a small config enum/struct over several loosely related booleans if that reads better locally.
- `sips` is available on macOS and keeps this dependency-free; tests can assert command construction rather than invoking it.
- Keep persisted audit filenames predictable so existing audit walkers do not lose the image.
