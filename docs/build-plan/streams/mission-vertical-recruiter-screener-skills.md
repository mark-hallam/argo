# Mission load-bearing wire — Recruiter-Screener vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/recruiter-screener/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-recruiter-screener-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. recruiter-screener ships with 4 skills (calibrate_against_pipeline / draft_outreach / find_screening_slot / score_resume). Sibling vertical `recruiter` covers broader scope. Brief brings recruiter-screener to 6.

## Scope

2 NEW skill files:

1. **`compare_candidate_pair.skill.md`** — given two candidates being weighed for the same role, output a structured comparison: rubric-dimension scores side-by-side, evidence per delta, strength of each as "best-fit", weakness call-outs, recommended-next-step (advance-both / advance-A-only / advance-B-only / hold / reject-both). Cites evidence per claim. Internal-report.
2. **`coach_interviewer.skill.md`** — given an interview type (phone-screen / technical / behavioral / panel) + candidate brief, output a 5-bullet "what to probe", 3-bullet "common pitfalls" (e.g. anchoring on first impression), and a 3-question rubric. Internal-prep doc.

## Files you may touch

- `verticals/recruiter-screener/skills/compare_candidate_pair.skill.md` (NEW, ≤ 90 LOC)
- `verticals/recruiter-screener/skills/coach_interviewer.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts; `verticals/recruiter/` (sibling).

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `compare_candidate_pair` 5-action vocab + bias-aware framing (never demographic proxies)
- [ ] `coach_interviewer` covers 4 interview-type templates explicitly
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-recruiter-screener-skills`
- PR title: `verticals: 2 skills for recruiter-screener (mission-vertical-recruiter-screener-skills)`

## Notes

- The bias-aware framing in both skills is non-negotiable — document explicitly in `# Hard rules`: "evidence is rubric-dimension performance only; never demographic proxies; never personal-fit-feel."
- `coach_interviewer`'s 3-bullet "common pitfalls" is the highest-leverage section — interviewer-anchoring + confirmation-bias drive 50% of bad hires. Bias the content toward calibration.
