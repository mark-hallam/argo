# ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `52fbb0f35` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test`.

## Why

Evaluator round 97 (2026-05-27T03:55Z) flagged PR #2437 as **the
SECOND admin-merge under operator-directive in 48h where the PR body
itself admits the load-bearing test was never run**:

> *"Local cargo check was interrupted by file-lock contention from
> concurrent worker compilation"*
> — PR #2437 body, merged 2026-05-27T00:26Z

The fix shipped, was admin-merged under the `--skip-clippy` lane, and
broke its own regression test (cold cargo `seeds_independently_per_tenant`
still fails). R97 spent a full evaluator round catching that — a
finding the conductor SHOULD have caught at merge-gate time by reading
the PR body the worker wrote.

Without a process pin, this becomes a recurring pattern. The
counter-pressure (operator urgency + worker quota anxiety + cargo
contention under concurrent worker compilation) is structural; only a
mechanical check enforced at the gate prevents recurrence.

## What

Add a new gate to `scripts/conductor-pre-merge-gate.sh` that scans the
PR body (fetched via `gh pr view <N> --json body`) for known
"test-not-run" admission phrases. When detected, emit a loud
`[gate-warning]` line and FAIL the gate by default.

An explicit override flag — `--allow-anti-theater-admit` — bypasses
the fail with an even louder warning trail and a SHARED.md
appendage. This is the deliberate-acknowledgement escape hatch for
the rare case where an operator genuinely wants to merge over the
admission (e.g. emergency rollback, doc-only follow-up that admits
the original test never ran).

### Gate placement

Insert as a new **Gate 9** at the END of the existing 8-gate
sequence (after workspace clippy). Rationale: the scan is fast (~2s
GitHub API call); positioning last means it fires only after every
other gate is green — operator sees ONE summary warning, not eight.

Gates 1-8 are unchanged. The existing `--skip-clippy` and
`--skip-tests` flags are unchanged.

### CLI surface

The pre-merge gate currently takes no PR number — it operates on the
local working tree. Add an OPTIONAL `--pr <N>` flag:

- `bash scripts/conductor-pre-merge-gate.sh` (no `--pr`): gate 9
  skips with a green log line (preserves existing local-only
  invocations + the `conductor-pre-merge-gate-tests.sh` smoke
  harness).
- `bash scripts/conductor-pre-merge-gate.sh --pr 2437`: gate 9 runs.
- `bash scripts/conductor-pre-merge-gate.sh --pr 2437 --allow-anti-theater-admit`:
  gate 9 emits a 5-line warning banner + a `[anti-theater-admit]`
  line to `docs/skycot/agents/SHARED.md` documenting the override
  (timestamp, PR number, body excerpt that triggered), then exits 0
  on that gate. Other gates still enforced normally.

### Detection phrase set

The phrase set is canonical, case-insensitive substring matches.
Worker should grep the PR body for ANY of these:

1. `local cargo check was interrupted`
2. `interrupted by file-lock contention`
3. `formal pre-merge gate will run`
4. `test was not run`
5. `compilation was interrupted`
6. `skipped tests due to`
7. `did not run the test`
8. `cargo test was not executed`

Phrases 1-3 are verbatim from PR #2437 and recurring across the
"second admin-merge under operator-directive in 48h" pattern R97
named. Phrases 4-8 are the obvious paraphrases the worker should
add as belt-and-suspenders. Worker MAY add 1-2 more phrases at
authoring time if a current open PR uses different language —
declare any additions in the PR body so the operator can review.

Phrases are stored as a constant array at the top of the helper
script (NOT in the gate script itself) so future operator-driven
expansion is one-file localized.

### Helper-script split (precedent: pre-merge-web-typecheck.sh)

Create new sibling `scripts/pre-merge-pr-body-anti-theater-scan.sh`
following the pattern of `scripts/pre-merge-web-typecheck.sh` (gate
7 helper):

- Takes `--pr <N>` (required when run standalone)
- Takes `--allow-anti-theater-admit` (matches the gate's flag)
- Exits 0 when: no PR number given (skip), OR PR body has no
  admission phrase, OR `--allow-anti-theater-admit` was passed.
- Exits 1 when: PR body has an admission phrase AND no override
  flag.
- Quiet mode (`--quiet`) matches the web-typecheck helper for
  smoke-test reuse.

The gate script (`conductor-pre-merge-gate.sh`) just shells out:

```bash
# Gate 9: PR body anti-theater scan (skips when no --pr given)
log "[9/9] PR body anti-theater scan"
if ! bash scripts/pre-merge-pr-body-anti-theater-scan.sh \
     ${PR_NUMBER:+--pr "$PR_NUMBER"} \
     ${ALLOW_ANTI_THEATER:+--allow-anti-theater-admit}; then
  fail "PR body contains anti-theater admission — see output above"
fi
```

The renumbering "8 gates → 9 gates" should also update lines 14, 18,
26, 167 of the gate script (literal `8` references that need to flip
to `9`, plus the "8 → 9" total mentioned in usage docstring).

### SHARED.md trail (when override used)

When `--allow-anti-theater-admit` is set AND a phrase was detected,
append ONE line to `docs/skycot/agents/SHARED.md`:

```
2026-05-27T05:30Z [anti-theater-admit] PR #2437 admin-merge proceeded over admission phrase 'Local cargo check was interrupted' (operator override --allow-anti-theater-admit). Phrase context: "<140-char excerpt>". Gate 9 expects evaluator round to confirm test actually passes within ~6h.
```

This creates an audit trail. Evaluators reading SHARED.md within 6h
of the override see "this PR was merged over admission, verify the
load-bearing test actually passes cold." That's the safety net.

### Smoke test (mirrors pre-merge-web-typecheck pattern)

Add `scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` with
4 scenarios using a `FAKE_PR_BODY` env-var hook (matching the
existing `FAKE_*_BODY` test hooks used by
`scripts/check-demo-roster.sh`):

1. **GREEN — clean body** — body with no admission phrase exits 0.
2. **RED — admission phrase, no override** — body containing
   "Local cargo check was interrupted" exits 1.
3. **GREEN — admission phrase + override** — same body + 
   `--allow-anti-theater-admit` exits 0 AND appends the SHARED.md
   line (use a `FAKE_SHARED_MD_PATH` env hook to direct the trail
   to a tempfile for the smoke).
4. **GREEN — no PR number** — exits 0 cleanly (skip case).

Smoke harness must be hermetic — no real `gh` calls, no real
SHARED.md writes. Both `FAKE_PR_BODY` (already a precedent in
`scripts/check-demo-roster.sh`) and `FAKE_SHARED_MD_PATH` (new)
are accepted at the top of the helper script.

## May-touch

- `scripts/pre-merge-pr-body-anti-theater-scan.sh` (NEW)
- `scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` (NEW)
- `scripts/conductor-pre-merge-gate.sh` (extend with `--pr` +
  `--allow-anti-theater-admit` flags, add gate 9 invocation, bump
  "8 gates" → "9 gates" in usage docstring)
- `docs/build-plan/BUILD_STATUS.md` (mandatory ledger flip on PR open)

Do NOT touch:
- The 8 existing gate sequences (1-8) — additive only.
- `scripts/conductor-pre-merge-gate-tests.sh` — that harness
  smokes gate 6 (test-compile) only; extending it for gate 9 is
  scope creep. Gate 9's own
  `scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` covers it.
- `scripts/pre-merge-web-typecheck.sh` — pattern reference only; do
  not modify.

## Acceptance criteria

- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy --skip-tests`
      (no `--pr`) exits 0 with a green "gate 9 skipped (no --pr)"
      log line. Preserves existing local-only invocations.
- [ ] `bash scripts/pre-merge-pr-body-anti-theater-scan-tests.sh`
      passes 4/4 scenarios.
- [ ] A live test against PR #2437 (which is now MERGED but the body
      is still queryable via `gh pr view 2437 --json body`):
      `bash scripts/pre-merge-pr-body-anti-theater-scan.sh --pr 2437`
      exits **1** with a `[gate-warning]` line citing the phrase
      "Local cargo check was interrupted" and a body excerpt around
      it. Same probe + `--allow-anti-theater-admit` exits 0 with the
      SHARED.md trail line emitted to the FAKE path.
- [ ] A live test against a CLEAN current PR (worker picks any of
      #2440, #2439, #2435, #2433 — those PR bodies have no admission
      phrases): `bash scripts/pre-merge-pr-body-anti-theater-scan.sh --pr <N>`
      exits 0.
- [ ] `shellcheck scripts/pre-merge-pr-body-anti-theater-scan.sh
      scripts/pre-merge-pr-body-anti-theater-scan-tests.sh` clean.
- [ ] `bash -n` on both new scripts clean.
- [ ] PR body documents the phrase set inline + the override-flag
      semantics, so a future operator/conductor reading the script
      knows what triggers and what bypasses.

## Out of scope (do NOT widen)

- **Adding the scan to evaluator pre-verification.** Evaluators
  already run cold-cargo against the PR head — they catch the
  failure directly. The scan is specifically a conductor-side gate
  for the admin-merge fast lane. (R97's recommendation is explicit
  about this; do not add an evaluator-side scan.)
- **Auto-failing the gate when ANY worker's PR body mentions
  "interrupt".** The phrase set is canonical; expanding to
  substring-match noisy words ("interrupt", "skipped") will produce
  false positives on legitimate PR bodies. Keep the phrases
  specific.
- **Reading worker stdout / commit messages.** PR body only —
  that's the surface the worker writes deliberately as evidence.
  Commit messages drift; PR bodies are the authoritative artifact.
- **Adding `--skip-anti-theater` as a generic skip.** The whole
  point is the gate cannot be skipped without explicit
  acknowledgement. Override via `--allow-anti-theater-admit` is the
  only path, and it MUST emit the SHARED.md trail.

## Parallel-surface check

Per `docs/skycot/DRIFT_PREVENTION.md` "grep before write":

- `ls docs/build-plan/streams/ | grep pre-merge-gate` returned:
  - `ops-conductor-pre-merge-gate-web-typecheck-pin.md` (MERGED, gate 7)
  - `ops-pre-merge-gate-export-sqlcipher-key-for-tests.md` (MERGED, gate 1)
  - `ops-pre-merge-gate-fix-test-failures-not-just-clippy.md` (MERGED, gate 6)
  - No parallel brief on anti-theater PR body scanning.
- `rg 'anti-theater' scripts/` returned 0 hits — no parallel
  implementation in flight.
- `rg 'pre-merge-pr-body' scripts/` returned 0 hits — helper-script
  filename is genuinely new.
- The `FAKE_*_BODY` env-hook pattern is already established in
  `scripts/check-demo-roster.sh` — follow it; do not invent a
  parallel test-hook style.

## Source

- Evaluator round 97: `docs/skycot/evals/scheduled/2026-05-27/round-97-pr-2437-demo-seed-post-merge-FAIL.md`, recommendation "🟡 P1 BRIEF" section.
- The PR body that triggered the recommendation:
  `gh pr view 2437 --json body` — contains the exact phrase the
  scan must detect.
- Precedent script: `scripts/pre-merge-web-typecheck.sh` + its
  caller integration in `conductor-pre-merge-gate.sh:163-170`.
- 104th anti-theater verification this week (per R97 footer).

## Plain-English summary for the operator

A worker that admits in its own PR description that it never ran
the test the fix is supposed to pass is, by definition, asking the
conductor to merge unverified code. PR #2437 did exactly that, the
conductor merged it under the `--skip-clippy` lane, and the
"fix" was a no-op that broke its own test. This brief adds a
9th gate that **reads the PR body** before the merge and refuses to
proceed when the worker has written one of the recognized "I didn't
run the test" admissions. An explicit `--allow-anti-theater-admit`
flag preserves the operator's authority to merge anyway in emergencies,
but the override is itself logged to `SHARED.md` so the next
evaluator round catches it.
