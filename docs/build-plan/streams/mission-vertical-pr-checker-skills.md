# Mission load-bearing wire — PR-Checker vertical: 3+ skills to reach 5

**Owner**: claude. **Depends on**: existing `verticals/pr-checker/` charter stub (2 existing skills).
**Authored against**: origin/main `b825b1d5` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-pr-checker-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to pr-checker. Today the vertical ships with 2 skills (identify_invariant_violations / post_review_summary). The vertical is narrower than the gtm-engineer template ("read-only by manifest declaration" per charter) so the 8-skill target doesn't apply — bring to 5 instead with 3 new skills covering the core review flow.

## Scope

3 NEW skill files under `verticals/pr-checker/skills/`:

1. **`scan_diff_for_patterns.skill.md`** — given a PR diff, scan for known anti-patterns: large file growth (>200 LOC added in one file), `unwrap()`/`expect()` outside `#[cfg(test)]`, new `unsafe` blocks, new global mutable state, missing rustdoc on new public APIs, > 5 commits without squash-prep. Output JSON: `[{pattern, severity (info/advisory/blocker), location, suggestion}]`. Cites CLAUDE.md invariants for each. Read-only.
2. **`compare_against_brief.skill.md`** — when the PR header references a brief id (e.g. `(mission-foo-bar)`), pull the brief from `docs/build-plan/streams/<brief-id>.md` and check: files touched within may-touch list, files touched outside flagged, LOC caps respected, acceptance criteria covered by diff, sensible-deviations section present if any may-touch escapes. Output: alignment report Markdown. Read-only.
3. **`suggest_review_priorities.skill.md`** — given diff size + risk signals (migration files, gateway routes, channels, executor primitives), output a 3-tier review priority: Tier 1 (high-stakes — migration / security / auth / new public API), Tier 2 (medium — multi-crate / new endpoint / new test infrastructure), Tier 3 (low — docs / typo / single-crate refactor). Cites reasoning per tier assignment. Read-only.

Each skill ~50-90 LOC. Total: ~200-270 LOC.

This is NOT in scope: actually posting GitHub PR review comments (charter says "posts an advisory summary" — that's the existing `post_review_summary` skill); merging/closing PRs (manifest is read-only); rewriting code; running tests.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/pr-checker/skills/scan_diff_for_patterns.skill.md` (NEW, ≤ 90 LOC)
- `verticals/pr-checker/skills/compare_against_brief.skill.md` (NEW, ≤ 90 LOC)
- `verticals/pr-checker/skills/suggest_review_priorities.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Existing skill files `identify_invariant_violations.skill.md` / `post_review_summary.skill.md`
- `verticals/pr-checker/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 3 new `.skill.md` files with frontmatter + sections (When to use / Inputs / Output shape / Examples)
- [ ] `scan_diff_for_patterns` 3-severity vocab (info / advisory / blocker) + cites CLAUDE.md invariants per pattern
- [ ] `compare_against_brief` reads from `docs/build-plan/streams/<brief-id>.md` (canonical brief location)
- [ ] `suggest_review_priorities` 3-tier vocab matching conductor's actual Tier 1/2/3 review burden in SWARM_PROTOCOL.md
- [ ] All skills are read-only — no recommendation that requires writing to the repo or PR
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-pr-checker-skills`
- PR title: `verticals: 3 skills for pr-checker (mission-vertical-pr-checker-skills)`

## Notes for the implementer

- Read SWARM_PROTOCOL.md HARD RULES and PROTOCOL.md "review tiers" section BEFORE writing `suggest_review_priorities` — the Tier 1/2/3 vocabulary already exists in protocol docs. Match it; don't invent a parallel scheme.
- `scan_diff_for_patterns`'s patterns should map 1:1 to the CI-rejection invariants in CLAUDE.md (no file >500 LOC, no fn >80 LOC, no `unwrap` outside tests, etc.) — that's the load-bearing list.
- `compare_against_brief` is the highest-leverage skill — it catches "PR doesn't match its own brief" drift which is the #1 cause of conductor rejections in this swarm's history.
- The pr-checker vertical is `read-only by manifest declaration` — every skill's output should be a recommendation/report, never an action.
- Demo readiness: a working pr-checker run is `identify_invariant_violations` (existing) + `scan_diff_for_patterns` (new) + `compare_against_brief` (new) + `suggest_review_priorities` (new) → `post_review_summary` (existing). End-to-end review pipeline.
