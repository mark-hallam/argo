# Mission load-bearing wire — HR-Screener vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/hr-screener/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-hr-screener-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. hr-screener ships with 4 skills (draft_policy_clarification / structure_exit_interview / summarize_workforce_survey / triage_employee_concern). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`anonymize_incident_report.skill.md`** — given an incident report containing PII (employee names, manager names, team names, dates that uniquely identify), produce an anonymized version safe for HR-leadership distribution. Preserves pattern + severity; replaces identifiers with role/seniority bands. PHI/PII-regex sniff: if real SSN/DOB patterns detected, halts and routes to operator.
2. **`recommend_policy_review.skill.md`** — given a cluster of related triaged employee concerns (≥3 in a 90-day window), recommend whether existing policy needs review/update. Output: policy_name, evidence_count, suggested_revision_categories (clarification / scope-expansion / new-policy / no-action). Approval-gated before policy-doc edits propagate.

## Files you may touch

- `verticals/hr-screener/skills/anonymize_incident_report.skill.md` (NEW, ≤ 90 LOC)
- `verticals/hr-screener/skills/recommend_policy_review.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `anonymize_incident_report` PHI/PII regex sniff documented
- [ ] `recommend_policy_review` 4-category recommendation vocab + 3-in-90-days threshold
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-hr-screener-skills`
- PR title: `verticals: 2 skills for hr-screener (mission-vertical-hr-screener-skills)`

## Notes

- `anonymize_incident_report` is the HR analog of healthcare-intake's PHI fence — every transformation must be reversible by the operator (the original stays in source). Skill outputs a paired `mapping` envelope so the operator can decode if needed.
- `recommend_policy_review` should NEVER auto-draft policy text. It surfaces the pattern; operators draft the policy.
