# fix-ledger-guard-pipefail-sigpipe-false-positive

**Owner**: claude.
**Authored against**: origin/main `7c81986c2` on `2026-06-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-ledger-guard-pipefail-sigpipe-false-positive`.

> Operator auto-memory item
> (`feedback_ledger_guard_pipefail_sigpipe_false_positive`): the ledger guard
> "can SPURIOUSLY refuse a valid BUILD_STATUS.md commit (e.g. routine
> CLAIMED→IN_REVIEW) ... Needs a hook-fix brief." This is that brief. The
> false refusal forces conductors into `git commit --no-verify`, which
> normalizes bypassing the guard — the exact habit the guard exists to
> prevent.

## Why (VERIFIED 2026-06-10, read-only)
`scripts/check-ledger-not-eaten-by-stash-pop.sh` (the #2634 pre-commit guard
against stash-pop eating TODO/CLAIMED ledger rows) has a SIGPIPE race that
reads as "brief-id missing" on perfectly valid commits:

- Line 35: `set -uo pipefail`.
- Line 91 (the relocation check, inside the deleted-row loop):
  `if printf '%s\n' "$current_content" | grep -qF "| $brief_id |"; then`
- `docs/build-plan/BUILD_STATUS.md` is **2,363,353 bytes** (verified
  `wc -c`) — ~36× the 64 KB pipe buffer.

Failure mechanics: on a routine in-place row edit (CLAIMED→IN_REVIEW flip),
the diff contains a deleted `-| <id> | ... | CLAIMED |` line AND the id still
exists in the file (the `+` line), so the relocation grep SHOULD match. But
`grep -qF` exits 0 at the FIRST match and closes its stdin; `printf` — still
writing the remaining megabytes — takes SIGPIPE and its pipeline segment
exits 141. Under `pipefail`, the pipeline status is 141 (the rightmost
non-zero), the `if` evaluates FALSE, the id lands in `dropped`, and the
script exits 1 refusing a valid commit. Whether it fires depends on where in
the 2.3 MB file the row sits (matches inside the final pipe-buffer window
don't SIGPIPE), which is why it's intermittent.

The existing smoke harness
(`scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh`, 5 cases) uses a
~6-row fixture ledger — far below the pipe buffer — so it can never
reproduce this. Its scenario (B) ("TODO row deleted but re-added elsewhere →
expect exit 0") is exactly the case that false-positives at real ledger
size.

## Grep-before-write
- `rg -n 'pipefail|grep -qF|printf' scripts/check-ledger-not-eaten-by-stash-pop.sh` — the one affected pipeline (line 91); no other producer-into-`grep -q` pipeline exists in the script.
- `rg -il 'sigpipe' docs/build-plan/streams/` — no existing fix brief (the only related brief is the original guard brief `ops-conductor-protect-against-stash-pop-eating-ledger-rows`, MERGED #2634).
- `ls scripts/check-ledger-not-eaten-by-stash-pop.smoke.sh` — the harness to EXTEND (do not author a parallel one).

## Fix (root fix — remove the pipe, keep semantics)
1. Replace the line-91 pipeline with a **no-pipe pure-bash substring
   check**: `if [[ "$current_content" == *"| $brief_id |"* ]]; then`.
   Identical semantics to `grep -qF "| $brief_id |"` (fixed-string
   containment over the whole content; newlines are irrelevant to a
   substring match), zero subprocesses, structurally immune to SIGPIPE, and
   faster in the loop. Brief-ids are kebab-case (no glob metacharacters),
   and the `*"..."*` pattern quotes the expansion so even a hostile id
   cannot glob.
2. Acceptable fallback if review prefers grep: write `$current_content` to a
   `mktemp` file ONCE before the loop and use
   `grep -qF -- "| $brief_id |" "$tmpfile"` (file input — early exit on a
   file descriptor SIGPIPEs nothing). Do NOT keep the producer pipe, and do
   NOT drop `set -o pipefail` script-wide (it guards the git invocations).
3. No behavior change to: exit codes (0/1/64), `--warn`, `--staged`,
   `BUILD_STATUS_PATH` test hook, or the dropped-row report format.

## Acceptance (truth-bounded — checkable verifier)
1. **Extend the existing smoke harness** with a regression case: baseline
   ledger = one `| big-brief-relocate | claude | CLAIMED |` row near the TOP
   followed by ≥ 256 KB of filler rows (e.g. ~3,000 generated MERGED rows);
   working-tree edit flips that row CLAIMED→IN_REVIEW (id still present).
   Expect exit 0. **Demonstrate in the PR body that this case exits 1 on the
   PRE-fix script** (run the new case against `git show HEAD:scripts/...`)
   and exits 0 post-fix — the reproduce-then-fix proof.
2. Full smoke harness (existing 5 cases + the new one) passes, run 2×;
   paste the tail.
3. A true row-drop still refuses: existing scenario (A) (TODO deleted, not
   relocated → exit 1) must still pass — the fix must not weaken the guard.
4. `bash -n` clean on both files; `shellcheck` clean (or no NEW findings).
5. Both files stay < 500 LOC.

## On ship (same commit)
Ops/script-only fix — no FEATURE_MATRIX row needed. PR body notes it closes
the operator auto-memory item
`feedback_ledger_guard_pipefail_sigpipe_false_positive` (the documented
`--no-verify` workaround is retired).

## Effort
S (~5-line script change + ~40-line smoke case; no Rust, no web).
