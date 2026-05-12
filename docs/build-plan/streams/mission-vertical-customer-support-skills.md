# Mission load-bearing wire — Customer Support vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/customer-support/` charter stub (vertical.toml + prompts/ + memory/schema.toml + eval/seed.jsonl).
**Authored against**: origin/main `e2fdf0a3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-customer-support-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137 ("Add 5+ skills for each charter-stub vertical"), scoped to customer-support. Today the vertical ships with a single skill (`draft_reply.md`). MISSION claim 7 ("any role across 10+ verticals") expects each charter-stub vertical to clear parity with gtm-engineer (8 skills). This brief brings customer-support to 6 skills (existing + 5 new) — the floor that closes #137 for this vertical.

## Scope

5 NEW skill files under `verticals/customer-support/skills/`, each following the `.skill.md` frontmatter shape used by `verticals/gtm-engineer/skills/research_company.skill.md`. Skills cover the dominant customer-support flow not already covered by `draft_reply`:

1. **`triage_ticket.skill.md`** — categorize an inbound ticket: severity (P0/P1/P2/P3), category (billing / outage / feature-request / how-to / abuse), and assign-to (self / human-tier-1 / human-tier-2 / abuse-review). Outputs JSON.
2. **`search_kb.skill.md`** — search the knowledge base via `web_search` tool, rank by recency + cite source. Used as input to `draft_reply`. Returns top-3 snippets with citation handles `[kb:<id>]`.
3. **`escalate_to_human.skill.md`** — when stuck, package the ticket + draft + reasoning for a human handoff. Outputs an escalation envelope: who-to-page, summary, attempted-steps, recommended-next-action.
4. **`update_ticket_status.skill.md`** — track ticket state transitions: pending → in-progress → waiting-on-customer → resolved → closed. Persists last state + reasoning to `memory.ticket_history[]` so a future skill invocation can audit the trail without re-reading the whole thread.
5. **`summarize_thread.skill.md`** — collapse a long support thread into a 5-bullet summary for handoff or post-incident review. Cites every fact (no fabrication). Used by retro tooling.

Each skill file is ~40-80 LOC of markdown. Total: ~300-450 LOC across 5 files.

Eval expansion (out of scope: `mission-vertical-customer-support-evals` is a separate brief): the skills carry their own example I/O in `# Examples` sections so the eval fixtures can use them as goldens later.

This is NOT in scope: changing `vertical.toml`; modifying `prompts/system.md` or `prompts/persona.md`; expanding `eval/seed.jsonl`; touching shared skill registry code; per-skill tests in `crates/` (skills are markdown artifacts, not Rust); wiring a regression test (separate brief #139).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/customer-support/skills/triage_ticket.skill.md` (NEW, ≤ 90 LOC)
- `verticals/customer-support/skills/search_kb.skill.md` (NEW, ≤ 80 LOC)
- `verticals/customer-support/skills/escalate_to_human.skill.md` (NEW, ≤ 80 LOC)
- `verticals/customer-support/skills/update_ticket_status.skill.md` (NEW, ≤ 80 LOC)
- `verticals/customer-support/skills/summarize_thread.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- `verticals/customer-support/skills/draft_reply.md` — existing skill stable, keep as-is
- `verticals/customer-support/vertical.toml` — schema is fine, skills auto-discovered by path
- `verticals/customer-support/prompts/**` — persona stable
- `verticals/customer-support/memory/schema.toml` — schema stable; if skills reference new memory keys, document them as future schema additions in their own `# Memory schema notes` section
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files in `verticals/customer-support/skills/`
- [ ] Each file has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/customer-support, hash: sha256-pending }`
- [ ] Each skill has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] Output shapes are deterministic JSON where applicable (triage, escalate, summarize)
- [ ] `escalate_to_human` references the brief's defined envelope (who-to-page, summary, attempted-steps, recommended-next-action)
- [ ] `update_ticket_status` documents the 5-state transition vocabulary
- [ ] No skill silently fabricates facts — every claim cites a KB id (`[kb:<id>]`) or marks itself "inferred / low-confidence"
- [ ] No file > 100 LOC
- [ ] `find verticals/customer-support/skills -name '*.skill.md' | wc -l` returns 5 (the brief's NEW additions; `draft_reply.md` keeps its `.md` extension)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-customer-support-skills`
- PR title: `verticals: 5 skills for customer-support (mission-vertical-customer-support-skills)`

## Notes for the implementer

- Model after `verticals/gtm-engineer/skills/research_company.skill.md` — the most mature skill file in the repo. Match its section structure, frontmatter, and tone (operator-readable, not LLM-prompt-y).
- The `provenance.hash` field is set to `sha256-pending` literally — the build pipeline computes the real hash at bundle time. Don't compute it yourself.
- KB-citation format `[kb:<id>]` is load-bearing — the SPA's audit drill-in parses these and renders them as cross-system jump-links. Don't invent a parallel citation syntax.
- For `summarize_thread`, the 5-bullet ceiling is intentional — longer summaries lose operator attention. If a thread genuinely has >5 distinct facts, the skill should output the top 5 by importance and emit a `truncated: true` marker.
- The brief's MAY-TOUCH excludes `vertical.toml` because skills are auto-discovered by directory walk — the vertical doesn't need a manifest entry per skill.
- Demo readiness: a working customer-support demo runs through `triage_ticket` → `search_kb` → `draft_reply` → (operator approval) → `update_ticket_status`. The 5 new skills + the 1 existing skill cover that chain end-to-end.
