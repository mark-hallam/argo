# claude-brief-freshness-header — add freshness header to brief template (forward-only)

**Owner**: claude. **Depends on**: scripts/check-brief-freshness.sh (already on main per tick408). **Phase A — process improvement.**

## Why

This session has produced 4 batches of "may-touch widening" fixes for briefs that were authored against an earlier snapshot of main and went stale as primitives shipped (cli.rs split, gateway split, SkillFrontmatter field additions, etc.). 3 worker tabs have self-exited citing brief-misalignment as a primary friction. Per worker reports: ~30-50% of iter time spent on premise validation, with worst case 5 of 8 evaluated briefs stale (62% rate).

The freshness validation script (`scripts/check-brief-freshness.sh`, shipped tick408) gives workers a way to detect drift before they spend context on claim-then-discover-broken. But the script needs anchor data — what SHA the brief was authored against — to reason about whether the codebase has drifted "since".

This brief adds that anchor as a standard brief-template header.

## Scope (forward-only)

**This brief does NOT retrofit existing briefs.** Mass-editing 300+ existing briefs would itself create conflict pressure. Going forward only:

1. Update `docs/build-plan/PROTOCOL_BRIEF_TEMPLATE.md` — add a 2-line header section that every NEW brief must include:
   ```
   **Authored against**: origin/main SHA <sha> on <YYYY-MM-DD>
   **Validate before claim**: `scripts/check-brief-freshness.sh <brief-id>`
   ```
2. Update `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` Step 8 (queue replenishment) — when conductor authors new briefs, capture current `git rev-parse origin/main` short SHA at top.
3. Document in CLAUDE.md / PROTOCOL_BRIEF_TEMPLATE that older briefs without the header are still valid; workers should run the freshness script regardless.
4. Optional sub-task: extend `check-brief-freshness.sh` to read the SHA header and emit "brief is N commits behind main" warning when N > 100.

## Files you may touch

- `docs/build-plan/PROTOCOL_BRIEF_TEMPLATE.md` (narrow header addition)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (Step 8 narrow extension)
- `docs/build-plan/PROTOCOL.md` (cross-link to freshness check)
- `CLAUDE.md` (narrow note that freshness header on briefs is forward-only)
- `scripts/check-brief-freshness.sh` (optional SHA-distance check ~10 LOC)

## Files you must NOT touch

- Any existing brief at `docs/build-plan/streams/*.md` — retrofit creates 300-file conflict surface
- BUILD_STATUS.md — this brief is process, not a status flip

## Acceptance criteria

- [ ] PROTOCOL_BRIEF_TEMPLATE has the 2-line header documented
- [ ] PROTOCOL_LOOP_PROMPT Step 8 instructs conductor to capture current SHA when authoring
- [ ] CLAUDE.md notes freshness header is forward-only (existing briefs unchanged)
- [ ] No existing brief is edited as part of this PR
- [ ] `cargo build` clean (pure docs change; no Rust touched)

## Branch + PR

- Branch: `claude-brief-freshness-header`
- PR title: `protocol: brief freshness header (forward-only) (claude-brief-freshness-header)`

## Notes

- This is intentionally minimal. Scope creep risk: someone might propose retrofitting all 300 briefs. Reject — the value comes from new briefs being self-validating, not from rewriting old ones. Old briefs already get the script's path-check + sibling-coverage check without a SHA anchor.
- Conductor session logged ~5 worker self-exits citing brief-misalignment friction; this brief addresses the source.
