# Mission load-bearing wire — Social-media-drafter vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/executive-assistant/` and `verticals/research-analyst/` as canonical content-only-vertical patterns just merged in #1336 + #1337).
**Authored against**: origin/main `69d3e28e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-social-media-drafter`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #148 ("Social-media-drafter vertical"). Mirrors the just-merged executive-assistant (#1336) / research-analyst (#1337) shape — content-only vertical, no Rust, no migrations. Drafts only; every external publish action gates behind human approval.

## Scope

NEW content-only `verticals/social-media-drafter/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 110, persona ≤ 35, skills 4×≤60, schema ≤ 30, auto_approval ≤ 25, inventory ≤ 50, vertical.toml ≤ 50, eval/seed.jsonl ≥ 7 fixtures):

- `vertical.toml` — id=`social-media-drafter`, version=`0.1.0`, optional buffer/hootsuite MCP, `[escalation].human_approval_required = ["publish_post", "schedule_post", "reply_to_mention"]`
- `prompts/system.md` — voice + tone consistency rules, platform-specific length rules (X ≤ 280 / LinkedIn ≤ 3000 / IG caption ≤ 2200), claim-substantiation requirement (no factual claims without a memory-stored source), escalation triggers (regulated industry / executive-quote / customer-PII / crisis-comms / political-content)
- `prompts/persona.md` — Sam, voice-of-brand drafter; consensus-of-tone over individual cleverness; pulls from `brand_voice` memory
- 4 skills:
  - `draft_post` — channel + topic in; markdown draft + hashtag suggestions + image-prompt out
  - `repurpose_long_form` — long article URL in; 5-platform thread/post pack out
  - `respond_to_mention` — mention thread in; draft response gated on `reply_to_mention` approval out
  - `tone_check` — draft in; voice-deviation score + flagged phrases out
- `memory/schema.toml` — `brand_voice` (do/don't say), `recent_posts` (last 50 by channel), `campaign_themes`, `crisis_keywords` (auto-escalate on match)
- `eval/seed.jsonl` — 7+ fixtures: 2 happy-path drafts (X + LinkedIn), 1 repurpose, 1 tone-check, 3 escalation (regulated-claim / executive-quote / crisis-keyword)
- `auto_approval.toml` — wake list includes draft_post + repurpose_long_form + tone_check only; publish/schedule/reply intentionally absent
- `inventory.json` — sample `brand_voice` (do-say / don't-say / signature-emoji), sample `campaign_themes`, sample `crisis_keywords` (5-10 entries)

NO Rust. NO migrations. NO crate changes. Pattern is identical to the just-merged executive-assistant and research-analyst — re-use the file shapes 1:1.

This is NOT in scope: actually integrating with Buffer / Hootsuite / native platform APIs (those are MCP-connector briefs that come later); image-generation (that's already a separate brief — mission-image-generation-mcp-server #163); analytics ingestion (separate brief); A/B testing the drafts (separate); engagement-routing rules.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/social-media-drafter/vertical.toml` (NEW, ≤ 50 LOC)
- `verticals/social-media-drafter/prompts/system.md` (NEW, ≤ 110 LOC)
- `verticals/social-media-drafter/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/social-media-drafter/skills/draft_post.md` (NEW, ≤ 60 LOC)
- `verticals/social-media-drafter/skills/repurpose_long_form.md` (NEW, ≤ 60 LOC)
- `verticals/social-media-drafter/skills/respond_to_mention.md` (NEW, ≤ 60 LOC)
- `verticals/social-media-drafter/skills/tone_check.md` (NEW, ≤ 60 LOC)
- `verticals/social-media-drafter/memory/schema.toml` (NEW, ≤ 30 LOC)
- `verticals/social-media-drafter/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/social-media-drafter/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/social-media-drafter/inventory.json` (NEW, ≤ 50 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- Any frontend / web crate

## Acceptance criteria

- [ ] `argo vertical list` includes `social-media-drafter 0.1.0`
- [ ] `argo vertical lint social-media-drafter` exits 0 errors (warns of the same shape as executive-assistant / research-analyst baseline are acceptable — vertical-specific approval-gated action names are not built-in tool names, which is the documented action-vocabulary convention)
- [ ] `argo vertical show social-media-drafter --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 7 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 7 fixtures, ≥ 3 of which are escalation cases (regulated / executive-quote / crisis-keyword)
- [ ] `auto_approval.toml` wake list contains only draft / tone-check / repurpose; publish/schedule/reply gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-social-media-drafter`
- PR title: `vertical: social-media-drafter (mission-vertical-social-media-drafter)`

## Notes for the implementer

- Treat the just-merged `verticals/executive-assistant/` and `verticals/research-analyst/` as 1:1 templates — same file shape, same YAML frontmatter, same `[escalation]` block style.
- `crisis_keywords` memory is load-bearing: a draft that mentions any of these strings (case-insensitive substring match) should auto-route to escalation, not draft. Document this in the system prompt.
- Voice consistency matters more than cleverness — the persona should default to mimicking the brand_voice memory, not free-styling.
- Platform-specific length rules are HARD limits, not suggestions. A 281-char X draft is a defect.
- One eval fixture should explicitly cover the "image prompt accompanies the draft" output shape — drafters generate a parallel image prompt for the design system, they don't generate the image themselves.
