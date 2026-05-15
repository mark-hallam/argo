# codex-smoke-script-target-dir-inheritance — Make smoke script inherit CARGO_TARGET_DIR for internal cargo test calls

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `65a12698` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-smoke-script-target-dir-inheritance`.
**Source**: Operator observation 2026-05-16. The 2026-05-16 v1 smoke validation produced a false "FAIL #19" because `scripts/smoke-skycot.sh` runs `cargo test` for scenarios 19-25 without inheriting/propagating `CARGO_TARGET_DIR` from the build step. When build uses one target dir but tests fall through to the workspace default, stale cache mismatches surface as fake compile errors.

## Scope

P3 reliability/UX. The smoke script at `scripts/smoke-skycot.sh:248`, `:260`, `:270`, `:277`, `:284`, `:291`, `:298` runs `cargo test` without an explicit `CARGO_TARGET_DIR`. If the operator ran the binary build under one target dir (e.g., `CARGO_TARGET_DIR=/tmp/argo-smoke-v1-target cargo build`), the smoke's internal cargo invocations don't see that env var unless the operator exports it FIRST.

Operator workaround was:
```bash
export CARGO_TARGET_DIR=/tmp/argo-smoke-v2-target
cargo build -p argo-cli
bash scripts/smoke-skycot.sh /tmp/argo-smoke-v2-target/debug/argo
```

The script should make this Just Work: either (a) propagate the binary's target dir to internal cargo calls automatically, or (b) document the requirement loudly in the script header.

Scope:
- At the top of `scripts/smoke-skycot.sh`, detect whether `CARGO_TARGET_DIR` is exported. If not, derive a sensible one (e.g., default to `target/` in the repo root, or `/tmp/argo-smoke-<random>/`) and `export` it so cargo subcommands inherit.
- Update the usage-text header comment to document the `CARGO_TARGET_DIR` behavior.
- Add a one-line "TARGET_DIR=<...>" diagnostic emitted before scenarios 19+ run, so operators can see which target dir is in use.

LOC: ~30.

## Files you may touch

- `scripts/smoke-skycot.sh` — header docs + target-dir-inheritance logic + diagnostic emission. ≤40 modified lines.

## Files you must NOT touch

- The actual test invocations' command shape — only the env around them.
- Cargo manifests, source code, anything outside the script.

## Acceptance criteria

- [ ] If `CARGO_TARGET_DIR` is exported when smoke runs: smoke uses it for ALL internal cargo subcommands.
- [ ] If `CARGO_TARGET_DIR` is NOT exported: smoke picks a sensible default + emits a diagnostic line so the operator can see what's being used.
- [ ] Header comment documents the behavior.
- [ ] Running `CARGO_TARGET_DIR=/tmp/argo-smoke-test-X bash scripts/smoke-skycot.sh <path-to-argo>` succeeds without the operator needing a separate `export` step.
- [ ] Running the same command WITHOUT `CARGO_TARGET_DIR` also succeeds (sensible default).
- [ ] Smoke still passes 25/25 scenarios on a fresh target dir.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-smoke-script-target-dir-inheritance`
- PR title: `fix(smoke): propagate CARGO_TARGET_DIR to internal cargo subcommands (codex-smoke-script-target-dir-inheritance)`

## Notes for the implementer

- The script already does `set -uo pipefail`. Be careful with unset-var checks (`[[ -z "${CARGO_TARGET_DIR:-}" ]]`).
- Don't `unset CARGO_TARGET_DIR` if the operator set it — that would defeat the purpose.
- The diagnostic should print to stderr (or via the existing `heading` helper) so it shows in the run log.
- If you derive a default target dir, prefer `${SCRIPT_DIR}/../target` (the repo's standard) over a tempdir — the standard is what most cargo invocations expect and the test executable cache survives across runs.
