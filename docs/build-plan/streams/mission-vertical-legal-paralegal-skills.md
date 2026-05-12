# Mission load-bearing wire — Legal-Paralegal vertical: 1+ skill to reach 6

**Owner**: claude. **Depends on**: `verticals/legal-paralegal/` (5 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-legal-paralegal-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. legal-paralegal ships with 5 skills (cite_check / clause_review / draft_memo / redline_contract / summarize_deposition). Brief brings to 6.

## Scope

1 NEW skill file:

1. **`flag_privilege_waiver_risk.skill.md`** — given an email/doc/communication thread being shared externally, scan for attorney-client privilege markers + identify segments where forwarding would waive privilege. Output JSON: `{ privilege_segments: [{location, party_count_in_chain, basis_for_concern, severity: low|medium|high}], recommended_action: send_as_is | redact_segment_N | hold_for_attorney_review }`. **Always routes to operator** — never auto-decides waiver questions.

## Files you may touch

- `verticals/legal-paralegal/skills/flag_privilege_waiver_risk.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 5 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 1 new `.skill.md` file with full frontmatter + sections
- [ ] 3-severity vocab (low/medium/high) + 3-action vocab (send/redact/hold)
- [ ] Hard rule documented: "this skill NEVER makes a privilege determination — only flags for review"
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-legal-paralegal-skills`
- PR title: `verticals: 1 skill for legal-paralegal (mission-vertical-legal-paralegal-skills)`

## Notes

- Privilege determination is a legal-judgment call attorneys make, not paralegals — and definitely not LLMs. The skill's role is pattern-spotting + escalation, never adjudication.
- Document the disclaimer prominently in `# Hard rules`: "This skill flags potential waiver risks for attorney review; it is not legal advice."
