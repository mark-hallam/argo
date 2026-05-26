# Round 89 — duplicate-ledger-row drift sweep

- **Date**: 2026-05-26 (UTC 15:13Z) / 2026-05-27 01:13 AEDT
- **Round**: 89
- **Pillar**: Process drift / ledger integrity / conductor-tick safety
- **Verdict**: 🟡 **PARTIAL — P2 process finding** (drift count 0 → 4 since round 88)

## Setup

```
python3 scripts/find-stale-claim-drift.py
```

## Observed

Drift count **0 → 4** since round 88 (~10h elapsed). All four are `duplicate-row` class — same brief slug present twice in `docs/build-plan/BUILD_STATUS.md`:

| Brief | States seen | PR |
|---|---|---|
| `fix-gateway-mod-rs-loc-overage-extract-store-helpers` | IN_REVIEW + TODO | #2422 (open) |
| `fix-genesis-orphan-discoverability-and-provider-key-503` | IN_REVIEW + IN_REVIEW | #2420 (open) |
| `fix-hire-demo-mode-stub-uuid-worker-roster-invisibility` | IN_REVIEW + IN_REVIEW | #2419 (open) |
| `fix-workflows-dark-mode-card-white-on-white` | IN_REVIEW + MERGED | #2421 (MERGED) |

**Common pattern**: every duplicate is a brief that shipped a PR in the last ~24h via the overnight Genesis+Track P merge-aggressive window. The four-row jump is correlated with the high merge cadence of the same window.

Two distinct sub-patterns observed:

1. **Row-flip without sibling-clean** (#2421 IN_REVIEW + MERGED): a `gh-pr-merge` ledger-flip path landed a MERGED row alongside a still-present IN_REVIEW row from the claim step. Operator script `scripts/conductor-flip-on-merge.sh` (or equivalent) appears to be inserting a new row instead of mutating the prior one. **Risk**: state-transition log loses canonicality — workers using "first eligible TODO row by doc order" can't see a clean state.
2. **Dual-claim authoring race** (#2419/#2420 IN_REVIEW + IN_REVIEW): two ledger rows for the same brief, both claimed. This would happen if two conductor ticks (interactive + scheduled-task) both authored a row for the same brief slug — the brief-lint pre-commit hook is supposed to refuse parallel-name brief-id collisions, but doesn't appear to catch in-ledger duplicates of the same slug.

## Anti-theater check

This isn't a verification finding — it's the lint script itself reporting state. The script `find-stale-claim-drift.py` is the same one the conductor runs every tick; round 88 showed 0 drift rows; round 89 (with the same script, unchanged source) shows 4. Either the script changed (it didn't — `git log -1 scripts/find-stale-claim-drift.py` is older than round 88), or the ledger acquired 4 duplicate rows in the intervening ~10h. The latter.

## Gap / brief candidates

- **🟡 P2 NEW** — `ops-conductor-dedupe-duplicate-ledger-rows-on-tick` (~40 LOC bash + python). When the drift sweep finds `duplicate-row` rows with state ∈ {IN_REVIEW + TODO, IN_REVIEW + IN_REVIEW, IN_REVIEW + MERGED}, the conductor tick should collapse them automatically (keep the most-advanced state row, delete the duplicate, append `[conductor] dedupe N rows` to SHARED.md). Authorized under `ops-*` `--skip-clippy` window.
- **🟢 P3** — `fix-conductor-flip-on-merge-mutate-not-insert` (~20 LOC bash). Root-cause the row-flip pattern in #2421's case: the merge-flip script should be `sed`-mutating the existing row's state column, not appending a new ledger row. The ops-dedupe brief above is the safety net; this is the prevention.
- **🟢 P3** — extend `scripts/lint-brief.sh` to also lint that the brief's slug appears at most once in BUILD_STATUS.md (currently only lints `streams/` parallel-name collisions). Catches the dual-claim race before commit.

**Standing P1 surface to operator from round 88 (still outstanding round 89)**:
- `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` — TODO, **5th unclaimed round** (r85 → r89).
- `fix-pr-2366-provenance-block-hastext-accepts-optional` — TODO, **4th unclaimed round** (r86 → r89).
- `ops-conductor-pre-merge-gate-web-typecheck-pin` — TODO, **4th unclaimed round** (r86 → r89).

These three briefs have sat through 4-5 consecutive evaluator rounds without claim. Recommend: operator-decision moment — either author the ~1-3 LOC PRs manually, mechanical-author via conductor fast-mode, or close as STALE-WONTFIX.
