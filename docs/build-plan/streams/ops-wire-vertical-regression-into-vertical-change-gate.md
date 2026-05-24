# ops-wire-vertical-regression-into-vertical-change-gate

**Owner**: codex.
**Depends on**: `eval-vertical-regression-cadence` MERGED (#2197).
**Authored against**: origin/main `4312a5ca6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-wire-vertical-regression-into-vertical-change-gate`.

## Scope

`argo-vertical-regression` is now pinned as a real manual harness, but #2197
found it is not run automatically for vertical edits. Add a narrow Codex worker
pre-push hook so branches that touch `verticals/**` run:

```bash
cargo test -p argo-vertical-regression --test regression_all
```

Keep the no-vertical-change path cheap. The script already runs workspace
clippy; this brief only adds a conditional extra test for vertical content
changes. Do not edit Claude conductor instructions.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`scripts/codex-pre-push.sh`]

## Files you may touch

- `scripts/codex-pre-push.sh` — detect vertical-path changes against the branch merge-base plus staged/uncommitted changes, then run the vertical regression test only when needed.
- `scripts/tests/codex-pre-push-vertical-regression.test.sh` — NEW hermetic shell test using a mocked `cargo` and temp git repo.
- `docs/skycot/runbook/per-vertical-regression.md` — document that Codex pre-push now enforces the harness for vertical edits.
- `docs/build-plan/BUILD_STATUS.md`

## Files you must NOT touch

- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — conductor pre-merge behavior is out of scope.
- Claude scheduled-task prompts or Claude conductor instructions.
- Rust production crates; this is script/runbook wiring only.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `scripts/codex-pre-push.sh` runs the vertical regression command when branch or staged changes include `verticals/**`.
- [ ] `scripts/codex-pre-push.sh` does not run the vertical regression command when no `verticals/**` paths changed.
- [ ] The existing workspace-clippy behavior and exit handling remain intact.
- [ ] New shell test covers both the trigger and no-trigger paths using a mocked `cargo`.
- [ ] `bash -n scripts/codex-pre-push.sh` passes.
- [ ] `bash scripts/tests/codex-pre-push-vertical-regression.test.sh` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-ops-wire-vertical-regression-into-vertical-change-gate`
- PR title: `ops: gate vertical regression on vertical edits (ops-wire-vertical-regression-into-vertical-change-gate)`

## Notes for the implementer

- Pre-push usually runs after commits exist, so do not rely only on
  `git diff --cached`. Include paths changed since `git merge-base HEAD
  origin/main` and then union staged/uncommitted paths for local retries.
- Use `git diff --name-only` and exact prefix matching for `verticals/`.
- Keep the command literal in the script so future runbook drift is obvious.
