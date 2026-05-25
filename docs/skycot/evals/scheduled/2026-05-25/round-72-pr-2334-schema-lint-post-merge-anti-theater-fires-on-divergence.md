# Round 72 · #2334 schema-version lint POST-MERGE anti-theater verify

**Date**: 2026-05-25 (Melbourne 2026-05-26 07:09 AEST)
**Round**: 72
**Pillar / Claim**: Process drift prevention (ops infrastructure; not a mission-claim axis but blocks the schema-bump operator-action recurrence pattern that caused the round-66→r70 gateway 503 carry).
**Verdict**: 🟢 **PASS** (P0 — pre-merge gate enforcement is real, not theater)

## Setup

`#2334 ops: lint expected schema version against migration tip` MERGED inline 2026-05-26 07:08:41 +1000 (UTC 2026-05-25T21:08:41Z) by operator/mark-hallam — commit `bb4781b8e`. r71 evaluated it as PRE-MERGE PASS at 20:20Z; conductor T20-08 did NOT merge it (deferred to operator); operator merged it ~48min later.

The 5-round β carry that started at r66 is **closed** by this merge — the schema-bump operator-action drift now has a CI-side enforcement layer that fires _before_ any PR ships with `EXPECTED_SCHEMA_VERSION` lagging the tip migration.

## Observed

**Merged diff** (2 files, +66 LOC, additive only):
- `scripts/check-expected-schema-version-matches-tip.sh` (+59 LOC; new) — reads `crates/argo-store/src/migrations/v*.rs` tip via `ls | sort -V | tail -1 | grep -oE 'v[0-9]+'` vs `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs`; exits 0 on match, 1 on mismatch, 2 on parse errors.
- `scripts/conductor-pre-merge-gate.sh` (+7 LOC) — invokes the lint at line 74, gates merge on rc=0.

**Anti-theater simulation** — I built a temp checkout at `/tmp/r72-schema-lint-test/` mirroring just the two files the script reads (`lib.rs` + `migrations/v79..83.rs`), then drove the script through 4 scenarios via `ARGO_SCHEMA_CHECK_REPO_ROOT=$TMPDIR`:

| Scenario | EXPECTED | tip | Predicted | Observed | RC |
|---|---|---|---|---|---|
| Baseline | 83 | v83 | exit 0 PASS | `✅ EXPECTED_SCHEMA_VERSION=83 matches tip migration v83.` | 0 |
| Lag (the v83 carry pattern) | 83 | v84 | exit 1 FAIL | `❌ EXPECTED_SCHEMA_VERSION=83 lags behind tip migration v84. Bump crates/argo-store/src/lib.rs:364 to 84.` | 1 |
| Post-bump | 84 | v84 | exit 0 PASS | `✅ EXPECTED_SCHEMA_VERSION=84 matches tip migration v84.` | 0 |
| Reverse divergence (EXPECTED ahead) | 99 | v84 | exit 1 FAIL | `❌ EXPECTED_SCHEMA_VERSION=99 lags behind tip migration v84.` | 1 |

All 4 transitions observed as predicted. RC=1 propagates through `set -euo pipefail` in `conductor-pre-merge-gate.sh:74` → gate fails → merge refused.

**Live invocation** against the current main:
```
$ bash scripts/check-expected-schema-version-matches-tip.sh
✅ EXPECTED_SCHEMA_VERSION=83 matches tip migration v83.
```

## Anti-theater check

How I proved this isn't just "test rewrites itself green":

1. **The script reads two real files** — `lib.rs` (line 364, `pub const EXPECTED_SCHEMA_VERSION: u32 = 83;`) and the `migrations/v*.rs` glob. Not a stubbed function — actual file-system inputs that change as the repo evolves.
2. **Both transitions observed** — same script, same flags, just different file contents in the temp checkout. Exit code flips 0→1→0→1 deterministically with the divergence.
3. **Wire is non-skippable** — `scripts/conductor-pre-merge-gate.sh:74` invokes the script under the `--skip-clippy` umbrella but NOT a `--skip-schema-lint` flag. Read the gate-script wrapper grep: no escape hatch.
4. **The lint detects the historic v83 failure mode** — the round-67/68/69 pattern (migration shipped without bumping `EXPECTED_SCHEMA_VERSION` const) would now exit 1 at gate-time; the diff to fix it (one-line sed on lib.rs) is named in the error message.

**Minor cosmetic finding (P3, NOT blocking):** the error message in the "reverse divergence" case (EXPECTED=99, tip=84) reads `lags behind tip migration v84.` That phrasing is wrong-direction in this edge case — EXPECTED is _ahead_, not behind. The lint correctly fires, but the suggested remediation (`Bump … to 84`) would silently regress the const. Realistically, EXPECTED leading tip would only happen via a botched rebase or revert, so impact is low.

## Gap / follow-up brief candidates

| Candidate | Priority | Scope |
|---|---|---|
| `ops-schema-lint-distinguish-leads-vs-lags-error-message` | **P3** | ~3 LOC; if `EXPECTED_N > TIP_N`, change "lags behind" → "leads tip migration"; suggest re-add the missing migration file vs bumping the const downward. Pure ergonomics, no behavior change. |

## Closure ledger

This report closes the r66→r70 β-carry pattern (`ops-add-expected-schema-version-vs-tip-migration-ci-lint` brief tracked across 5 evaluator rounds). The merge happened ~14h after r66 first surfaced it; lint is now in the gate; no further evaluator surveillance needed on this axis until a schema-bump PR rolls and we verify the lint actually blocks a real divergence (not a synthetic temp dir).

**Cost**: ~3K tokens, ~3min wall for this scenario.
