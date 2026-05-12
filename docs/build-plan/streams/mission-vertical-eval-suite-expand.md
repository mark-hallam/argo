# Mission load-bearing wire — Expand vertical eval suites to ≥7 fixtures each

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 4 #140. Charter-stub verticals today ship with 4-5 eval fixtures. The mature template (gtm-engineer) has 17. To call a vertical "shipped" we need ≥7 fixtures per vertical with ≥3 escalation-path coverage (the rare-but-load-bearing failure modes).

## Scope

For each charter-stub vertical with < 7 fixtures, add fixtures to reach 7. Coverage requirements per vertical:

- ≥3 happy-path fixtures (golden flow end-to-end)
- ≥2 edge-case fixtures (unusual but legitimate input)
- ≥2 escalation-path fixtures (input that should route to operator, NOT auto-respond)

Verticals to expand (verify per-vertical count at claim time; some may already be at 7+):
- customer-support (1 → 7+; this is the deepest gap)
- marketing-intern (1 → 7+)
- sales-sdr (1 → 7+)
- finance-ar (1 → 7+)
- healthcare-intake (~1-2 → 7+; PHI safety fixtures load-bearing)
- recruiter (1 → 7+)
- tax-prep (1 → 7+)
- product-manager (1 → 7+)
- real-estate-agent (1 → 7+)

Note: this brief assumes the skill-expansion briefs (mission-vertical-<name>-skills) have ALSO shipped, so the new skills have eval coverage. If a skill-expansion brief hasn't merged for a given vertical, write that vertical's fixtures against existing skills only and stamp a `# TODO add fixtures for <new-skills>` note.

This is NOT in scope: cross-vertical fixtures (per-vertical only); per-fixture LLM-judged scoring (separate brief — quality-judge wire is in the runtime); fixture-generation tooling (manual authoring in this brief).

## Files you may touch

- `verticals/<vertical>/eval/seed.jsonl` for each of the 9 named verticals — add fixtures to reach 7+ per vertical
- `verticals/<vertical>/eval/README.md` (NEW per vertical if absent, ≤ 80 LOC) — documents per-vertical fixture counts + the escalation-fixtures convention

## Files you must NOT touch

- `verticals/_starter-library/**` (different scope)
- `verticals/gtm-engineer/eval/**` (template, leave alone)
- Skill files (sibling skill-expand briefs)

## Acceptance criteria

- [ ] Every named charter-stub vertical's seed.jsonl has ≥7 fixtures
- [ ] ≥3 escalation-path fixtures per vertical
- [ ] No fabricated facts in fixtures (synthetic but realistic)
- [ ] Each fixture has: input description, expected behavior (1-2 sentences), expected output shape (action_type)
- [ ] `argo vertical lint <each>` passes
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-eval-suite-expand`
- PR title: `verticals: expand eval suites to ≥7 fixtures (mission-vertical-eval-suite-expand)`

## Notes

- Bias toward escalation fixtures — these catch the dangerous "looks routine but actually needs operator" failure modes. The 3-per-vertical floor is the load-bearing safety property.
- Synthetic fixtures only — never use real customer data, real PHI, real PII. Use the established placeholder patterns from `gtm-engineer/eval/seed.jsonl`.
- This brief is most valuable AFTER the per-vertical skill briefs ship (skills exist, fixtures cover them). The order matters; coordinate with sibling briefs.
