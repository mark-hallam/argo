# ops-gate-worktree-web-typecheck-node-modules

Authored against: origin/main cb7d4cfb23d993d758d8d61c987d3b3bd73bee71 on 2026-06-10
Validate before claim: bash scripts/check-brief-freshness.sh ops-gate-worktree-web-typecheck-node-modules

## Goal

Make gate 7 (web typecheck) of `scripts/conductor-pre-merge-gate.sh` work
when the gate runs from a git worktree that has no `web/node_modules`.

## Why now (conductor evidence 2026-06-10)

Gating PR #2765 from a worktree (`git worktree add /tmp/gate-2765 …`, the
B-1 pattern) failed gate 7 with:

```
[web-typecheck FAIL] pnpm tsc --noEmit exited 254
 ERR_PNPM_RECURSIVE_EXEC_FIRST_FAIL  Command "tsc" not found
```

`pnpm --dir web exec tsc` resolves `tsc` from `web/node_modules/.bin`, which
exists only in the main checkout (node_modules is gitignored, worktrees do
not share it). The conductor worked around it manually:
`ln -sfn $REPO/web/node_modules /tmp/gate-2765/web/node_modules` →
`tsc --noEmit` exit 0. That workaround should be the script's own behavior —
a false-FAIL on a green PR pushes conductors toward gate-overrides, which is
exactly the normalization-of-deviance the gate exists to prevent.

## Fix (in `scripts/conductor-pre-merge-gate.sh`, gate-7 block only)

Before invoking pnpm, if `web/node_modules/.bin/tsc` is absent AND the
repo-root main checkout has one, symlink it in (worktree-local, gitignored
path, idempotent):

```bash
main_nm="$(git rev-parse --path-format=absolute --git-common-dir)/../web/node_modules"
if [[ ! -x web/node_modules/.bin/tsc && -x "$main_nm/.bin/tsc" ]]; then
  ln -sfn "$main_nm" web/node_modules
  warn "web-typecheck: linked node_modules from main checkout (worktree run)"
fi
```

Fall through to the existing invocation. If neither exists, FAIL with a
message that says "run pnpm install in web/" (true failure, honest message).

## May touch

- `scripts/conductor-pre-merge-gate.sh` (gate-7 block)
- `scripts/pre-merge-web-typecheck.sh` (the gate-7 helper shipped by the
  MERGED `ops-conductor-pre-merge-gate-web-typecheck-pin` brief — put the
  link-in logic wherever the pnpm invocation actually lives; do NOT add a
  second parallel typecheck path)
- `scripts/conductor-pre-merge-gate-tests.sh` (add the worktree case if the
  harness supports it cheaply; otherwise document the manual check)

## Acceptance / verifier

1. From a fresh worktree of current main touching a web file, gate 7 runs
   tsc and reports PASS/FAIL on the CODE (not "tsc not found"). Paste both
   the pre-fix failure and post-fix pass in the PR body.
2. From the main checkout, gate 7 behavior byte-identical (no symlink made).
3. `bash -n` + shellcheck clean.

## Size / owner

~10 LOC bash. Owner: claude.
