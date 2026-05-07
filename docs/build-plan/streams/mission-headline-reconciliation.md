# Mission ship-blocker (deferred to last) — reconcile headline numbers

**Owner**: claude. **Depends on**: mission-reenable-ci-workflows, mission-gateway-bearer-auth, mission-otlp-exporter, mission-coverage-tooling merged. **DO NOT CLAIM EARLY** — this brief should be the last brief executed before declaring "production-ready", because the numbers it cites move every week.

> SOURCE: deep-research findings tick 198, Section 2, item #17.
> Headline numbers in README + ROADMAP + BUILD_STATUS overstate
> reality (27 vs 53 crates, 50K vs 183K LOC, "production-ready"
> claim vs CI-was-off, "zero LOC violations" vs known violations).
> A technical-DD engineer will discover this on first 10-minute
> read-through, and trust burns.

## Scope

Reconcile every public-facing claim in:

- [README.md](README.md)
- [ROADMAP.md](ROADMAP.md)
- [docs/build-plan/BUILD_STATUS.md](docs/build-plan/BUILD_STATUS.md)
  (the conductor-tick header section, NOT the per-row content)
- [docs/skycot/MISSION.md](docs/skycot/MISSION.md) (only any
  empirical claim; the mission paragraph itself is immutable)
- [docs/skycot/launch_readiness.md](docs/skycot/launch_readiness.md)
- [docs/skycot/README.md](docs/skycot/README.md)

…against ground truth as of the day this brief is claimed. The
numbers WILL have changed since this brief was authored (deep-research
captured them at 53 crates / 183,122 LOC / 7 source-LOC violations
as of tick 198); rebaseline at claim time.

For each surfaced claim:

1. Measure the current value (`ls crates/ | wc -l`,
   `find crates -name "*.rs" -not -path "*/target/*" | xargs wc -l`,
   `find ... -size +500l`, `git log --oneline | wc -l`, `gh pr list
   --state merged --limit 1000 | wc -l`, etc.).
2. Replace overstated values with measured ones, OR remove the
   claim if it is unsalvageable (e.g., "production-ready as of PR
   #522" — delete; replace with "v1 demo gate green; production
   readiness checklist in MISSION_BACKLOG.md Section 2").
3. Where a number was deliberately aspirational (e.g.
   ROADMAP-style projections), tag it `(target)` clearly.

This is a docs/comms brief, not a Rust brief. No code touches
expected. The acceptance is honesty, not coverage.

## Files you may touch

- `README.md`
- `ROADMAP.md`
- `docs/build-plan/BUILD_STATUS.md` (header section ONLY; never the
  per-row content)
- `docs/skycot/MISSION.md` (empirical claims only; mission paragraph
  immutable)
- `docs/skycot/launch_readiness.md`
- `docs/skycot/README.md`
- `docs/skycot/CHANGELOG.md` — narrowly: add an entry noting the
  reconciliation date

## Files you must NOT touch

- Any Rust source file
- Any per-row content in BUILD_STATUS.md (worker-owned)
- The mission paragraph in MISSION.md (immutable per
  CLAUDE.md / MISSION.md own statement)
- Brief files under `streams/` (those reflect their PR context)

## Acceptance criteria

- [ ] Every empirical claim in README.md verified against current
      tree state (crate count, LOC, PR count, test count, vertical
      count, channel count, connector count, MERGED-row count)
- [ ] ROADMAP.md headline numbers updated; aspirational items
      explicitly tagged `(target)` or `(planned)`
- [ ] No "production-ready" claim that isn't backed by Section 2
      production blockers being MERGED
- [ ] launch_readiness.md reflects actual production gaps (or is
      removed if subsumed by MISSION_BACKLOG.md)
- [ ] BUILD_STATUS.md header section accurate as of brief-claim date
- [ ] CHANGELOG.md entry summarizing what claims were reconciled
      and against what date's tree state
- [ ] PR body lists, in a table, every claim that was changed
      (BEFORE → AFTER + source-of-truth command)
- [ ] No code edits in this PR

## Branch + PR

- Branch: `claude-mission-headline-reconciliation`
- PR title: `docs: reconcile headline numbers across README/ROADMAP/MISSION (mission-headline-reconciliation)`

## Notes for the implementer

- Numbers move every commit. Do this brief LAST, after the
  preceding production blockers (CI re-enabled, gateway auth, OTLP,
  migrations, coverage) have all merged.
- Do not soften the language to make the gap look smaller; the
  point is to publish honest figures. A reader who finds a
  reconciled README + a MISSION_BACKLOG.md will trust the
  project more than one who finds inflated numbers.
- The DEEP_RESEARCH analysis flagged the gap honestly:
  "substrate ~75%, runtime ~45%". That framing is appropriate for
  README; consider including it.
- If you discover a claim you cannot verify (e.g., "658 PRs
  merged" — measurable; "9/9 demo gate" — measurable but the
  meaning shifts as briefs land) — surface it in the PR body as
  "could not verify" rather than guessing.
- This brief intentionally has no LOC budget — it's docs.
