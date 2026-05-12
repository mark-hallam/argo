# Mission load-bearing wire — Marketing Intern vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/marketing-intern/` charter stub.
**Authored against**: origin/main `e2fdf0a3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-marketing-intern-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to marketing-intern. Today the vertical ships with one skill (`draft_social.md`). MISSION claim 7 expects parity-with-gtm-engineer (~6+ skills). This brief brings marketing-intern to 6 skills.

## Scope

5 NEW skill files under `verticals/marketing-intern/skills/`. Skills cover the dominant marketing-intern flow not already covered by `draft_social`:

1. **`competitive_scan.skill.md`** — weekly scan of named competitors via `web_search` + `web_extract`: pricing changes, product launches, hiring posts (proxy for direction), exec moves, funding events. Output JSON: `[{competitor, signal_type, summary, source, date}]`.
2. **`draft_newsletter.skill.md`** — drafts a weekly newsletter section combining `competitive_scan` output + internal product changelog (operator-provided). Outputs Markdown with a 2-line lede, 3 bullet items, and a sign-off — every claim cited. Approval-gated downstream.
3. **`brand_mention_summary.skill.md`** — collapses brand mentions from web/social search into a sentiment-bucketed report (positive / neutral / negative / question / complaint). For each bucket, top-3 mentions with source + date. Output JSON.
4. **`recommend_post_time.skill.md`** — given a target audience + channel, recommend 3 optimal post times for the upcoming week (factoring timezone + historical engagement if available in memory; otherwise generic best-practice defaults with `assumption: generic-best-practice` marker).
5. **`tag_audience_segment.skill.md`** — categorize an inbound social engagement (reply, mention, DM) into segments: customer / prospect / press / community / spam. Routes to escalation when uncertain (confidence < 0.6 → human review).

Each skill file ~40-80 LOC. Total: ~300-400 LOC.

This is NOT in scope: changing `vertical.toml`; modifying prompts; expanding `eval/seed.jsonl`; live integration with social platforms (Tier A connectors are separate); auto-publishing skills (charter mandates approval-gating).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/marketing-intern/skills/competitive_scan.skill.md` (NEW, ≤ 90 LOC)
- `verticals/marketing-intern/skills/draft_newsletter.skill.md` (NEW, ≤ 90 LOC)
- `verticals/marketing-intern/skills/brand_mention_summary.skill.md` (NEW, ≤ 80 LOC)
- `verticals/marketing-intern/skills/recommend_post_time.skill.md` (NEW, ≤ 70 LOC)
- `verticals/marketing-intern/skills/tag_audience_segment.skill.md` (NEW, ≤ 70 LOC)

## Files you must NOT touch

- `verticals/marketing-intern/skills/draft_social.md` — existing skill stable
- `verticals/marketing-intern/vertical.toml` — schema is fine, skills auto-discovered
- `verticals/marketing-intern/prompts/**` — persona stable
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each file has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/marketing-intern, hash: sha256-pending }`
- [ ] Each skill has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] No skill auto-publishes — every output flagged for downstream approval
- [ ] `recommend_post_time` emits an `assumption` marker when falling back to generic defaults
- [ ] `tag_audience_segment` routes to human-review on confidence < 0.6 (explicit)
- [ ] No fabricated competitor signals — `competitive_scan` cites every source URL + date
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-marketing-intern-skills`
- PR title: `verticals: 5 skills for marketing-intern (mission-vertical-marketing-intern-skills)`

## Notes for the implementer

- Match the tone of `verticals/marketing-intern/prompts/persona.md` (read it first). The marketing-intern persona is junior, energetic, source-citing — skills should reinforce, not contradict.
- The `web_extract` tool is in the vertical's allowed tools list — `competitive_scan` and `brand_mention_summary` should explicitly call it out in `# Tools you may invoke`.
- Approval-gating is the marketing-intern's defining safety property. Every output that touches the public (newsletter, social post, audience response) MUST mark itself `requires_approval: true` in its output frame. This isn't enforced by the skill file alone — it's documented so the downstream runner respects it.
- The 5-bullet newsletter ceiling exists for the same reason as customer-support's summarize_thread — operators read every approval, and longer drafts get rubber-stamped. Be ruthless about brevity.
- Cite-everything is non-negotiable. A marketing skill that fabricates a competitor pricing change to fill a newsletter section is a fireable offence in real life — the skill file should call this out in `# Hard rules`.
