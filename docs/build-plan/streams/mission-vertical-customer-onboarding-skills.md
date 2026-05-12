# Mission load-bearing wire — Customer-Onboarding vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/customer-onboarding/` charter stub (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-customer-onboarding-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. customer-onboarding ships with 4 skills (draft_value_check_in / draft_welcome_email / identify_churn_signals / setup_health_check). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`schedule_kickoff.skill.md`** — given a new customer + the operator's calendar availability, propose 3 kickoff-call slots (60min) within first 5 business days. Output JSON. Approval-gated (calendar write side-effect downstream).
2. **`map_success_criteria.skill.md`** — given the customer's stated goals + product capability surface, draft a 5-bullet success-criteria document (measurable / time-bound / responsible-party-named per bullet). Output Markdown. Approval-gated for customer-facing sharing.

## Files you may touch

- `verticals/customer-onboarding/skills/schedule_kickoff.skill.md` (NEW, ≤ 80 LOC)
- `verticals/customer-onboarding/skills/map_success_criteria.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts; any other vertical's skill directory.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] Both approval-gated explicitly
- [ ] `schedule_kickoff` constrains to 5-business-day window
- [ ] `map_success_criteria` enforces the 5-bullet measurable/time-bound/owner pattern
- [ ] No file > 100 LOC; BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-customer-onboarding-skills`
- PR title: `verticals: 2 skills for customer-onboarding (mission-vertical-customer-onboarding-skills)`

## Notes

- Match the existing skill files' tone (operator-readable, non-LLM-prompt-y). Inspired by sibling `sales-follow-up` for cadence-style phrasing.
- The 5-bullet cap on `map_success_criteria` is operator-respect — onboarding docs longer than 5 bullets get ignored.
