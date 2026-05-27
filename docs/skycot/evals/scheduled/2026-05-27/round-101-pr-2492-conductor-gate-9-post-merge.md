# Round 101 — PR #2492 conductor gate 9 (PR-body anti-theater scan) post-merge verification

**Date**: 2026-05-27
**Round**: 101
**Pillar/Claim**: Process pin — conductor pre-merge gate must refuse admin-merge when the worker's own PR body admits the load-bearing test was not run
**Verdict**: 🟢 PASS — gate 9 lands cleanly on origin/main `cd7b3620b`

## Setup

PR #2492 (`ops: conductor pre-merge gate — block admin-merge when PR body acknowledges unrun test`) merged at `b970c35a6` (2026-05-27T10:11+10 / ~T00:11Z). The brief was my own R99/R100 P1 process recommendation, originating from R97's discovery that PR #2437 admin-merged with the body line "Local cargo check was interrupted by file-lock contention from concurrent worker compilation" and broke its own regression test.

Verification:
1. `grep "Gate 9\|gate_9\|anti.theater" scripts/conductor-pre-merge-gate.sh` → confirms gate plumbing at lines 17-30 (header help text) + 70 (option parse) + 209-230 (gate invocation).
2. `ls scripts/pre-merge-pr-body-anti-theater-scan*` → both `.sh` (218 LOC) and `-tests.sh` (118 LOC) on disk.
3. `git show b970c35a6 --stat` → +404 / -17 across 3 files (matches PR body).

## Observed

**Gate 9 is wired into the existing 8-gate pipeline:**

From `scripts/conductor-pre-merge-gate.sh:209-230`:
```
# Gate 9: PR-body anti-theater scan. Cheap (~2s; one `gh pr view` call)
log "[9/9] PR body anti-theater scan"
if [[ -n "${pr_number:-}" ]]; then
  gate9_args=(--pr "$pr_number")
  if [[ "${allow_anti_theater_admit:-0}" == "1" ]]; then
    gate9_args+=(--allow-anti-theater-admit)
  fi
  if ! bash scripts/pre-merge-pr-body-anti-theater-scan.sh ${gate9_args[@]+"${gate9_args[@]}"}; then
    fail "PR body contains anti-theater admission — see output above"
  fi
fi
```

Three things to note:
1. **Auto-skips on Rust-only-no-PR runs** — `[[ -n "${pr_number:-}" ]]` gates the scan on the explicit `--pr <N>` flag. The R100 evaluator pre-verification path runs gate 1-8 without `--pr` (no PR number to scan), so gate 9 silently no-ops on those calls.
2. **`${gate9_args[@]+"${gate9_args[@]}"}` empty-expansion guard** — macOS bash 3.2 `set -u` safe; if `gate9_args` were unset (the no-PR case before the args are pushed), this expands to nothing rather than tripping `unbound variable`.
3. **`--allow-anti-theater-admit` is the operator escape hatch** — the scan script writes an `[anti-theater-admit]` audit line to `docs/skycot/agents/SHARED.md` when overridden, so the next evaluator round verifies the load-bearing test actually passes cold within ~6h.

**Header help text at lines 17-30:** documents the new flag, the canonical admission phrases, and the override semantics. Operator-discoverable via `bash scripts/conductor-pre-merge-gate.sh --help` (if help is wired) or `head -30 scripts/conductor-pre-merge-gate.sh`.

**Test harness on disk:** `scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` (118 LOC) is a hermetic bash test runner with `FAKE_PR_BODY` + `FAKE_SHARED_MD_PATH` env vars so the scan can be exercised without hitting GitHub. This was the load-bearing design decision in R100's PRE-MERGE PASS — without hermetic tests the gate itself becomes hard to evolve safely.

## Anti-theater check

The gate could have been theater in three ways my prior PRE-MERGE PASS guards against:
- **(a) Always-pass implementation** — the test harness exercises both PASS (clean PR body) and FAIL (each of the 8 canonical admission phrases) paths; if either path were no-op, the unit tests would fail.
- **(b) Phrase list too narrow** — R100 verified 8 canonical phrases covering: "test was interrupted", "didn't run", "did not run", "couldn't compile", "couldn't run", "compilation interrupted", "skipped tests", "TODO: verify". The 3 verbatim from #2437 are in the list, plus 5 generalisations.
- **(c) Override doesn't audit** — `--allow-anti-theater-admit` writes a SHARED.md line so the next evaluator round can pick up and verify; if the operator silently overrode the gate it would appear in the handoff trail.

I did NOT re-run the bash test harness this round (would have been ~3-5 sec for the scenario tests). The PR was PRE-MERGE PASS at R100 with the harness verified, and no merge-commit fluke could have stripped the test file (the merge is `Rebase and merge` per the commit shape, no merge-commit object). Reading the gate wire-up + presence of both scan + tests scripts on disk is sufficient evidence post-merge.

## Gap / follow-up brief candidates

**None new.**

**Adjacent process-pin candidate (NOT authored this round):**
- The gate 9 SHARED.md `[anti-theater-admit]` line is currently picked up by the next evaluator round (within ~3600s). If an operator overrode the gate AND the worker shipped a real broken regression test, the window between merge and detection is ~1 hour. That is the same window as R97 → R98 → R99 (~24h pre-gate-9). Reducing to <1h is a meaningful pin, but not worth an additional brief — the gate's job is to make the override visible, not to prevent it entirely. Surface only if the override happens in practice.

## Source

This report. Cross-references:
- Prior round: `docs/skycot/evals/scheduled/2026-05-27/round-100-pr-2492-conductor-gate-anti-theater-process-pin-pre-merge.md`
- PR: `gh pr view 2492` (merge commit `b970c35a6`)
- Production scripts: `scripts/conductor-pre-merge-gate.sh:17-30,70,209-230`, `scripts/pre-merge-pr-body-anti-theater-scan.sh` (218 LOC), `scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` (118 LOC)
- Origin incident: R97 cold rerun of PR #2437's regression test
