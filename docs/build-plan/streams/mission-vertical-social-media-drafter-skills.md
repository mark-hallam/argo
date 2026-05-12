# Mission load-bearing wire — Social-Media-Drafter vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/social-media-drafter/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-social-media-drafter-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. social-media-drafter ships with 4 skills (draft_post / repurpose_long_form / respond_to_mention / tone_check). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`schedule_posting_calendar.skill.md`** — given a content pipeline + channel mix + posting frequency policy, output a 14-day posting calendar with date/time/channel/content-id per slot. Honors operator-defined quiet hours + channel-specific best-time windows. Output JSON + Markdown table. Approval-gated for actual scheduling.
2. **`flag_brand_safety_risk.skill.md`** — given a drafted post, scan for brand-safety risks: legal claims, competitor disparagement, controversial topics, undisclosed sponsorship language, regulated-industry compliance (HIPAA/FINRA/FERPA flag if applicable to operator). Output JSON: `{ risks: [{type, severity: info|advisory|blocker, evidence_quote, suggested_revision}], overall_recommendation: send_as_is | revise_per_suggestion | hold_for_legal_review }`. Always halts on `blocker` severity.

## Files you may touch

- `verticals/social-media-drafter/skills/schedule_posting_calendar.skill.md` (NEW, ≤ 90 LOC)
- `verticals/social-media-drafter/skills/flag_brand_safety_risk.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `schedule_posting_calendar` honors quiet-hours + channel best-time windows; approval-gated
- [ ] `flag_brand_safety_risk` 3-severity vocab + 3-action vocab + `blocker` always halts
- [ ] No skill auto-posts; everything routes to operator approval
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-social-media-drafter-skills`
- PR title: `verticals: 2 skills for social-media-drafter (mission-vertical-social-media-drafter-skills)`

## Notes

- Brand-safety risks compound — a "blocker" finding stops the post regardless of other "advisory" ones. The skill should never average or weight; any blocker halts.
- For regulated industries, the operator declares which framework applies via `memory.brand_compliance_framework`; default is none (no extra checks).
