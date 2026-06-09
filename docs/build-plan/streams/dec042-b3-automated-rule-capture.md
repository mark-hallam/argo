# dec042-b3-automated-rule-capture

**Owner**: claude.
**Authored against**: origin/main `b15b9e27d` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh dec042-b3-automated-rule-capture`.

> DEC-042 Stage 1, upgrade **B-3** (the third build-loop hardening step, after
> B-1 #2753 + B-2 #2755). See
> [`docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md`](../../skycot/decisions/DEC-042-continuous-self-improvement-loop.md)
> §3 B-3. This is the **compounding** step — it makes the error rate trend down
> on its own. **Top Stage-1 row now.**
>
> ⚠️ **Merge policy (conductor note):** unlike B-1/B-2 (pure additive default-OFF
> ops tooling), B-3 produces artifacts that touch project *governance* (CLAUDE.md
> rules / CI lints). Per DEC-042 §7 "comprehension debt" + DEC-034 (operator stays
> the verifier on load-bearing calls), B-3 must **PROPOSE, never auto-apply** —
> it drafts a capture-brief / lint stub for conductor+operator approval; it does
> NOT auto-edit CLAUDE.md, the DEC register, or CI config. The conductor holds
> B-3's own PR for operator review rather than auto-merging it.

## Why (VERIFIED 2026-06-09, read-only)
Mistakes become *repeated* ledger boundary-notes, not captured rules, so the
defect/nit rate doesn't trend down (DEC-042 §3 B-3; the `recurring-nit count`
north-star metric in `scripts/north-star-metrics.py` is currently `null` — this
brief makes it computable). The signal is overwhelming and real on current main:

- `await_holding_lock` / `routes_settings_credentials` (one pre-existing clippy
  blocker) is **boundary-noted 41 times** across `BUILD_STATUS.md` (verified:
  `grep -ciE 'await_holding_lock|routes_settings_credentials'` = 41). It is in
  operator auto-memory as a known blocker AND was re-noted again on B-2 (#2755).
  That is a textbook "same class of issue flagged 3+ times" that should have
  become ONE tracked cleanup brief (or a lint allow-list entry) dozens of
  re-notes ago.
- "boundary-note"/"pre-existing" appears **379 times** in the ledger — a rich
  vein of recurring nits no mechanism is capturing.

Boris's exact habit (DEC-042 §1.2): every mistake becomes a *committed rule*, not
a chat correction; his pre-AI instinct was "3–4 occurrences → lint rule." B-3
automates the *detection + drafting* of that capture; a human/conductor still
approves the actual rule (the judgment step).

## Grep-before-write
- `rg -n 'capture-recurring|recurring-nit|rule-capture' scripts/` — confirm no existing capturer (none as of `b15b9e27d`; this brief creates `scripts/capture-recurring-nits.py` — do NOT author a parallel one).
- `rg -n 'boundary.?note|pre-existing|KNOWN_FLAKES|known.flake' docs/build-plan/ scripts/find-stale-claim-drift.py` — the existing nit-bearing surfaces (`BUILD_STATUS.md` rows, PR bodies, `docs/build-plan/KNOWN_FLAKES.md`) the capturer mines + where a captured rule would live.
- `ls scripts/ | rg -i 'lint|render-feature-matrix|north-star'` — `north-star-metrics.py` (whose `recurring_nit_count` null this fills) + the `lint-*` family a proposed lint stub would join.
- `rg -n 'def .*drift|emit|--quiet' scripts/find-stale-claim-drift.py` — the closest existing "mine-the-ledger + emit a report" pattern to mirror for tone/exit-contract (advisory, `--quiet`, exit-code-as-signal).

## Fix (a proposer, never an auto-applier; default-safe)
1. **`scripts/capture-recurring-nits.py`** (NEW, < 250 LOC). Mines real sources
   for recurring nits: (a) `BUILD_STATUS.md` rows + the last N merged-PR bodies
   (`git log`/`gh`) for normalized boundary-note phrases (lowercase, strip PR
   numbers/SHAs/dates, collapse whitespace, key on the stable token core — e.g.
   `await_holding_lock routes_settings_credentials`), (b) tally occurrences,
   (c) for any normalized pattern with **count ≥ THRESHOLD (default 3)** that is
   NOT already captured (no existing `capture-<slug>` brief and not in
   `KNOWN_FLAKES.md`), emit a structured finding `{pattern, count, examples,
   proposed_capture}`. Emits JSON to stdout; `--quiet` prints only the count
   (the `recurring_nit_count` metric). Exit 0 always (advisory, like the drift
   script).
2. **Drafting (proposal only).** With `--draft`, for each over-threshold finding
   write a DRAFT `docs/build-plan/streams/capture-<slug>.md` cleanup brief (using
   the standard brief skeleton + freshness header) that proposes the fix (e.g.
   "fix or allow-list `await_holding_lock` in `routes_settings_credentials_tests.rs`
   once, so it stops being boundary-noted") — **marked DRAFT / awaiting-approval**,
   NOT auto-queued as TODO. It must NOT edit `CLAUDE.md`, the DEC register, or any
   `.github`/CI config — those are operator-judgment surfaces (a comment in the
   draft names the candidate lint, but a human wires it).
3. **Wire `recurring_nit_count` into the north-star metric.** In
   `scripts/north-star-metrics.py`, replace the `recurring_nit_count: null` +
   its `not-yet-instrumented` reason with a real value from
   `capture-recurring-nits.py --quiet` (best-effort: stays `null` if the capturer
   errors — never fabricate). This closes one of the DEC-042 §6 build-loop
   metrics.

## Acceptance (truth-bounded — checkable verifier)
1. **`scripts/test-capture-recurring-nits.sh`** (NEW, < 120 LOC): runs the
   capturer against a fixture ledger containing a phrase repeated 4× + a phrase
   repeated 1×; asserts ONLY the 4× phrase is emitted (threshold honored), the
   1× is not, and `--draft` writes exactly one `capture-*.md` with a valid
   freshness header. Deterministic, no network, run 2×.
2. **Real-signal check (the proof it works on live data):** running
   `capture-recurring-nits.py` against the real `BUILD_STATUS.md` surfaces the
   `await_holding_lock`/`routes_settings_credentials` pattern (count ≥ 3) — the
   known recurring nit. (Assert count ≥ 3 for that normalized key.)
3. **Idempotence:** after a `capture-<slug>` draft exists, re-running does NOT
   re-emit that pattern (no duplicate drafts).
4. **`north-star-metrics.py` still emits** (exit 0) with `recurring_nit_count`
   now an integer (run it; the value is the live count).
5. `python3 -m py_compile` clean; `bash -n` on the test; each file < 500 LOC.

## On ship (same commit)
- `FEATURE_MATRIX.md` row for "B-3 automated rule-capture (proposer; `recurring_nit_count`
  metric live)" + regenerate `feature-matrix.html` via `render-feature-matrix.py`.
- Note it advances DEC-042 Stage 1 (B-1 ✅ → B-2 ✅ → **B-3 done** → B-4 next:
  separate build/verify agents) and fills the `recurring_nit_count` north-star.
- Do NOT auto-queue the generated `capture-*` drafts; list them in the PR body
  for conductor+operator triage.

## Effort
M (one Python miner + a metric wire-in + a deterministic fixture test; proposer-
only, no auto-apply, no Rust, no runtime flag).
