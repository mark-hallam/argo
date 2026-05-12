# Mission load-bearing wire — Receptionist vertical: 4+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/receptionist/` charter stub.
**Authored against**: origin/main `df7b9dfe` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-receptionist-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to receptionist. Today the vertical ships with 2 skills (draft_reply / identify_intent). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings receptionist to 6.

## Scope

4 NEW skill files under `verticals/receptionist/skills/`. The charter is "conversation-only by manifest — no calendar/CRM/SMS integrations" so all 4 are conversational primitives:

1. **`greet_visitor.skill.md`** — opening greeting matched to context (walk-in vs phone-in vs message). Output Markdown. Templated by time-of-day + greeting style (formal / casual matching the operator's brand voice). Autonomous.
2. **`answer_faq.skill.md`** — given an FAQ knowledge base (operator-provided) + a visitor question, find the best match + return the answer with citation (`[faq:<id>]`). If confidence < 0.7, emits `requires_human_followup: true` with a templated bridge ("Let me get a colleague who knows this better"). Autonomous.
3. **`escalate_out_of_scope.skill.md`** — when a question falls outside the operator's FAQ + the persona's allowed scope (clinical advice, legal advice, financial advice, sensitive personal queries), package the conversation for human escalation: who-to-page (per operator routing rules), conversation transcript, classified-as (clinical / legal / financial / personal / unknown / urgent), urgency level. Always routes to human; never tries to answer.
4. **`take_message.skill.md`** — when the human escalation isn't immediately available, capture a structured message: visitor identity, callback contact, topic, urgency, time. Confirms back to visitor in plain language. Output JSON envelope for downstream operator inbox.

Each skill ~50-80 LOC. Total: ~250-320 LOC.

This is NOT in scope: calendar integration (charter excludes); CRM write (charter excludes); SMS/voice (charter excludes); appointment scheduling (charter excludes — handled by `ai-receptionist` sibling for that scope); auto-callback.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/receptionist/skills/greet_visitor.skill.md` (NEW, ≤ 80 LOC)
- `verticals/receptionist/skills/answer_faq.skill.md` (NEW, ≤ 90 LOC)
- `verticals/receptionist/skills/escalate_out_of_scope.skill.md` (NEW, ≤ 80 LOC)
- `verticals/receptionist/skills/take_message.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Existing skill files `draft_reply.skill.md` / `identify_intent.skill.md`
- `verticals/receptionist/vertical.toml`, prompts — locked
- `verticals/ai-receptionist/**` — sibling with broader scope (calendar / SMS / CRM); don't drift its skills

## Acceptance criteria

- [ ] 4 new `.skill.md` files
- [ ] Each has frontmatter + sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] `answer_faq` confidence threshold = 0.7 + explicit `requires_human_followup: true` fallback
- [ ] `escalate_out_of_scope` 5-category vocabulary (clinical / legal / financial / personal / unknown), always routes to human
- [ ] `take_message` produces JSON envelope with visitor + contact + topic + urgency + timestamp
- [ ] `greet_visitor` matches operator brand voice (formal vs casual) parametrically
- [ ] No skill claims to schedule, send SMS, or write to a CRM (charter excludes — sibling `ai-receptionist` does that)
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-receptionist-skills`
- PR title: `verticals: 4 skills for receptionist (mission-vertical-receptionist-skills)`

## Notes for the implementer

- The receptionist vs ai-receptionist split: receptionist is conversation-only (this brief), ai-receptionist is the broader Avery-style intake with scheduling. Don't drift either set; read `verticals/ai-receptionist/skills/*` to see what NOT to duplicate here.
- `escalate_out_of_scope`'s clinical/legal/financial categories are the "I cannot give that advice" guardrail. The skill outputs a templated bridge sentence + the escalation envelope. NEVER attempts to answer.
- `answer_faq`'s 0.7 confidence threshold is the operator's safety knob. Documented in the skill file; operators can override via persona policy.
- `take_message` is the offline-capture skill — it's how the vertical handles "the answer requires a human and the human isn't here." The output JSON should be friendly for an operator inbox UI (Approvals page) downstream.
- For brand-voice parameterization in `greet_visitor`, the formal/casual dial comes from `persona.md`. Don't add a runtime knob; the skill reads persona at invoke-time and matches the tone.
