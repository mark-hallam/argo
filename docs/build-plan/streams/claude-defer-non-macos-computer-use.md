# claude-defer-non-macos-computer-use — Defer non-macOS computer-use executors to v1.x

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-defer-non-macos-computer-use`.

## Scope

Operator decision 2026-05-14: v1 demo runs on **macOS first** (the operator's machine is darwin). The 3 non-macOS computer-use executors stay shipped but are formally re-classified as `DEFERRED-v1.x` so we focus v1 hardening on the macOS path:

- `argo-executor-computer-use-macos` (1,945 LOC) — **v1 primary**
- `argo-executor-computer-use` (E2B Firecracker, 6,793 LOC) → **DEFERRED-v1.x** (cloud parallelism, not blocking)
- `argo-executor-computer-use-linux` (1,636 LOC) → **DEFERRED-v1.x** (server deploy path)
- `argo-executor-computer-use-windows` (2,428 LOC) → **DEFERRED-v1.x** (enterprise endpoint)

This is a documentation/scope brief; no code deletion. The deferred crates compile + ship; we just don't gate v1 on their feature parity or polish.

LOC: ~150 (mostly doc edits + 4 small Cargo.toml description tweaks + smoke-test verification).

## Files you may touch

- `crates/argo-executor-computer-use/Cargo.toml` — append ` (DEFERRED-v1.x; cloud sandbox path, not v1 demo)` to description field.
- `crates/argo-executor-computer-use-linux/Cargo.toml` — same pattern.
- `crates/argo-executor-computer-use-windows/Cargo.toml` — same pattern.
- `crates/argo-executor-computer-use-macos/Cargo.toml` — append ` (v1 primary execution path)` to description.
- NEW `crates/argo-executor-computer-use-macos/README.md` — one-page operator note: this is the v1 primary execution path.
- `docs/skycot/V1_DEFINITION.md` — §4 Claim 5 row: note macOS primary + 3 deferred.
- `docs/build-plan/PRODUCT_ROADMAP.md` — Track G row: add macOS-first note.
- `scripts/smoke-skycot.sh` — verify smoke exercises macOS executor path; add step if missing.
- `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` — annotate the 3 deferred rows.

## Files you must NOT touch

- Any Rust source in the 4 executor crates (this brief is doc-only).
- Workspace `Cargo.toml` (members stay; deferred ≠ deleted).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 4 Cargo.toml description fields updated.
- [ ] macos crate has a README declaring v1-primary status.
- [ ] V1_DEFINITION.md + PRODUCT_ROADMAP.md document the macOS-first decision.
- [ ] smoke-skycot.sh runs (or has a step that runs) the macOS executor path.
- [ ] `cargo build --workspace` still green (no Cargo.toml syntax breakage).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-defer-non-macos-computer-use`
- PR title: `scope(v1): defer non-macOS computer-use executors to v1.x (claude-defer-non-macos-computer-use)`

## Notes for the implementer

- Mission-claim alignment: claim 5 (real-world PC execution) — narrowing the v1 demo surface to macOS doesn't reduce mission coverage; it scopes the polish window.
- Operator's machine is darwin (per CLAUDE.md environment). Demo recording `docs/skycot/demos/v1-mission-mvp.mp4` will be macOS.
- This is a scope/documentation brief. No business logic changes. Drift-prevention parallel-surface rules N/A.
