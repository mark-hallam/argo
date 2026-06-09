# fix-parallel-pub-types-gate-enforce

Authored against: origin/main c359fc5ca on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-parallel-pub-types-gate-enforce

## Goal

Make gate 4 (parallel-pub-types lint) of the conductor pre-merge gate
actually fail on violations, and clear/adjudicate the 6 duplicate pub types
currently on main so the flag flip doesn't create a permanently-red gate.

## Why now (architecture audit 2026-06-10, P1 — silent gate)

- `scripts/conductor-pre-merge-gate.sh:168` runs
  `python3 scripts/lint-parallel-pub-types.py` WITHOUT `--enforce`; the
  linter only exits non-zero when `--enforce` is passed. The only enforcing
  caller is `.github/workflows/ci.yml.disabled:30` — a disabled workflow.
  Verified live: the linter prints 6 violations and exits 0.
- Duplicate pub types are the drift class that previously produced the
  cross-tenant `McpConnectionPool` credential-leak P1 (see
  DRIFT_PREVENTION.md "No parallel abstractions").
- The 6 live duplicates (none allowlisted):
  - `PitchChannel` enum — `routes_pitch_ask.rs:38` vs
    `argo-eval/src/pitch_runner.rs:40` (same Track-P domain concept)
  - `CanonicalQaIndex` — `argo-eval/src/pitch_runner.rs:385` vs
    `argo-skill/src/canonical_qa.rs:111` (same concept)
  - `ListQuery`, `SourceKind`, `StepOutcome`, `StepStatus` pairs (run the
    linter for paths)

## Fix (one PR, ordered)

1. For each of the 6: canonicalize (one home, the other side imports /
   re-exports — prefer the owning domain crate; `argo-types` if genuinely
   shared) OR add an allowlist entry in
   `scripts/lint-parallel-pub-types.allowlist` with a one-line justification.
   The two Track-P pairs (`PitchChannel`, `CanonicalQaIndex`) look like true
   duplicates — canonicalize those; the generic-name pairs may be legitimately
   distinct (allowlist with justification).
2. THEN add `--enforce` to `scripts/conductor-pre-merge-gate.sh:168`.
3. Verify the gate now exits non-zero on a synthetic duplicate (add a case to
   `scripts/conductor-pre-merge-gate-tests.sh` if the harness supports it).

## May touch

- `scripts/conductor-pre-merge-gate.sh` (the one flag)
- `scripts/lint-parallel-pub-types.allowlist`
- the 6 type-definition sites + their importers (canonicalization)
- `scripts/conductor-pre-merge-gate-tests.sh`

## Acceptance / verifier

1. `python3 scripts/lint-parallel-pub-types.py --enforce` exits 0 on the PR
   head (clean tree).
2. Gate harness case proves a synthetic duplicate FAILS the gate.
3. `cargo check --workspace` green after canonicalization moves.
4. No new `pub` surfaces beyond re-exports of the canonical homes.

## Size / owner

~100-250 LOC depending on canonicalization fallout. Owner: claude.
🔴 P1 — restores a drift gate the whole protocol assumes is active
(DEC-042 verifier-spine).
