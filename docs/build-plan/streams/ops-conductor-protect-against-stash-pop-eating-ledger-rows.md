# ops-conductor-protect-against-stash-pop-eating-ledger-rows

**Authored against**: origin/main `3ef2c7470` on `2026-05-27`.

**Validate before claim**: bash scripts/check-brief-freshness.sh ops-conductor-protect-against-stash-pop-eating-ledger-rows
**Severity:** P1
**Source:** docs/skycot/evals/scheduled/2026-05-27/round-99-r97-substrate-defect-ghost-stamp-resolved-substrate-still-open.md (R99 P1 NEW recommendation)
**Owner:** claude or codex (lightweight bash; either pool can ship)
**Estimated size:** ~50-80 LOC bash + ~30 LOC hermetic smoke test

## Why this brief exists

R98 (2026-05-27T05:00Z) found that commit `6006cc192` was titled
`ledger+brief: fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed (R97 P0 anti-theater)`
but `git show --stat 6006cc192` showed **zero file changes** — empty-commit "ghost-stamp."

R99 (2026-05-27T08:55Z) traced the actual root cause: it was **not** a fabricated empty commit.
The brief file (`docs/build-plan/streams/fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed.md`)
was written + staged in a worktree, the conductor ran `git stash push` to make room for a `git pull`,
the `git stash pop` after the pull silently dropped the `BUILD_STATUS.md` ledger-row INSERT because a
concurrent worker push had moved that exact region. The brief file (clean new file) popped cleanly;
the ledger-row INSERT didn't, and the auto-merge resolved by keeping the worker's version. The conductor
then commit/push'd a working tree that contained the brief file but **not** the ledger row.

This is the second time in <72h that a stash-pop has silently dropped a ledger row. The first time
took 24h of evaluator rounds to surface; the second time was caught faster but still cost a
ghost-stamped commit and required manual re-ledgering ~2h later (commit `ad1d3e6a6`).

The pattern is going to recur because:
- the conductor `git pull --rebase` flow inside an interactive tick frequently needs `stash push`
  + `stash pop` to survive concurrent worker pushes,
- workers append to `BUILD_STATUS.md` at high frequency (claims, IN_REVIEW flips, blocked notes),
- if the conductor's staged ledger-row INSERT and a worker's append overlap (or land in the
  same hunk window), `stash pop`'s 3-way merge picks the worker's text and silently drops the conductor's.

The fix is a pre-commit hook that **diffs `BUILD_STATUS.md` against `HEAD` before any conductor commit**
and refuses the commit if the diff contains **only deletions** of `| TODO |` rows that the staged copy
has lost. Surface the loss before pushing rather than the next evaluator round.

## What to build

### File 1 — `scripts/check-ledger-not-eaten-by-stash-pop.sh` (NEW, ~50-80 LOC bash)

A standalone script the conductor invokes manually + a pre-commit hook installs:

```
Usage:
  bash scripts/check-ledger-not-eaten-by-stash-pop.sh           # check working tree vs HEAD
  bash scripts/check-ledger-not-eaten-by-stash-pop.sh --staged  # check staged copy vs HEAD
  bash scripts/check-ledger-not-eaten-by-stash-pop.sh --warn    # exit 0 even on detection (warn-only)
```

Logic:
1. `git diff [--staged] HEAD -- docs/build-plan/BUILD_STATUS.md` → capture the diff.
2. Filter to lines starting with `-| ` (deletions of table rows).
3. For each deleted row, check whether the row appears (with same brief-id) elsewhere in the file
   (rename / move). If yes → benign reorder, ignore.
4. If a `| TODO |` or `| CLAIMED |` row is deleted **and not relocated**, that's the failure mode:
   refuse the commit with a clear error message pointing at the dropped brief-id + the
   `git stash list` to recover from.
5. `--warn` flag downgrades exit 1 → exit 0 + stderr warning. Useful while the gate's behavior is
   being calibrated against real conductor traffic; flip to enforce after a week of clean signal.

Error message format:
```
🛑 ledger row dropped from BUILD_STATUS.md without relocation:
   - line ~241: | fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed | claude | TODO | ...

This is almost always a `git stash pop` that ate a TODO row when a worker push
landed in the same hunk window. Recovery:

  1.  git stash list                # find the stash
  2.  git show stash@{0} -- docs/build-plan/BUILD_STATUS.md  # see what was there
  3.  manually re-add the row + amend or fresh commit

Run with `--warn` to bypass while debugging.
```

### File 2 — `.git/hooks/pre-commit` (extension, ~20 LOC)

If the repo's pre-commit hook already exists (check first with `git config core.hooksPath`),
extend it. Otherwise create a minimal one that delegates to script 1.

Trigger condition: the staged diff includes `docs/build-plan/BUILD_STATUS.md`. If yes,
invoke `bash scripts/check-ledger-not-eaten-by-stash-pop.sh --staged`. If the script exits
non-zero, the commit is refused.

### File 3 — `crates/argo-cli/tests/check_ledger_not_eaten_smoke.rs` — NOT NEEDED

Bash script gets a hermetic shell-script smoke harness instead:

### File 3 (replaces above) — `scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh` (NEW, ~30 LOC)

Mirrors `scripts/check-demo-roster.sh`'s harness pattern:

- Sets up a `mktemp -d` with a fake `BUILD_STATUS.md`.
- Scenario A: removes a TODO row from `BUILD_STATUS.md`, runs the script → expect exit 1.
- Scenario B: removes a TODO row but adds the same brief-id back elsewhere → expect exit 0
  (benign reorder).
- Scenario C: removes a MERGED or BLOCKED row → expect exit 0 (only TODO/CLAIMED loss is the
  failure class we're guarding against).
- Scenario D: no diff to `BUILD_STATUS.md` at all → expect exit 0 (no-op).

## Acceptance

- `bash scripts/check-ledger-not-eaten-by-stash-pop.sh` exits 0 on clean working tree.
- `bash scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh` → 4/4 scenarios pass.
- `git commit -m 'test'` after manually removing a TODO row from `BUILD_STATUS.md` → commit refused
  with the documented error message.
- `.git/hooks/pre-commit` runs the check when `BUILD_STATUS.md` is staged.

## Out of scope

- **Detecting all stash-pop merge conflicts** — only the specific class of "TODO/CLAIMED row silently
  dropped" is the target. Other classes (text reflow, header changes) are not the failure mode this
  brief addresses.
- **Auto-recovery** — the script surfaces the loss; recovery is operator-manual via `git stash list`.
- **Worker pre-push gate** — workers append rows; they don't delete TODO rows. The class of failure
  is conductor-specific.
- **Husky / `lint-staged` integration** — repo doesn't use those today; a plain
  `.git/hooks/pre-commit` script is the lightest path.

## May-touch

- `scripts/check-ledger-not-eaten-by-stash-pop.sh` (NEW)
- `scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh` (NEW)
- `.git/hooks/pre-commit` (NEW or extension)
- `scripts/install-brief-hooks.sh` (existing — extend to also install the new hook if
  the script already wires other hooks)
- `docs/build-plan/BUILD_STATUS.md` (ledger row update only)

## Verification

- `bash scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh` → 4/4 PASS.
- Manual: `echo '' >> docs/build-plan/BUILD_STATUS.md && git add docs/build-plan/BUILD_STATUS.md && git commit -m 'test'` → succeeds (no row dropped).
- Manual: delete a TODO row by hand from `BUILD_STATUS.md`, `git add` it, attempt commit → refused.
- `shellcheck scripts/check-ledger-not-eaten-by-stash-pop.sh` clean.

## Out-of-scope follow-up (NOT this brief)

The deeper fix is to make the conductor `pull-rebase-push` flow not need `stash push/pop` at all —
move the `git add` + `git commit` BEFORE the `git pull --rebase` so the row lands in its own commit
that rebase can replay cleanly. That's a workflow refactor across `scripts/conductor-claim-with-retry.sh`
+ all the README guidance the workers + conductor follow. Worth a separate brief once this gate
shows real signal.
