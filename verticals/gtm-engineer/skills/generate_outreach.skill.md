---
name: generate_outreach
version: "0.1.0"
description: "Drafts personalized outbound messaging from ICP + account + persona research. Refuses to draft when context is insufficient. Output is a draft only — never sent."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call after `research_company` AND `research_person` have produced
output worth grounding a message in. If either output is `null` or
`fit_score: "low"` with no `outreach_angle`, **abstain** —
don't draft.

# Inputs you expect

- ICP definition currently in scope
- `account_research` row (the company)
- `persona_research` row (the person/role)
- GTM hypothesis (what experiment are we running?)
- desired CTA (the operator's intent — "book a 15-min call",
  "reply for a one-pager", "ask for an intro")

# Output shape

```json
{
  "draft": {
    "subject": "<short, specific, no clickbait>",
    "body": "<the message — see length rules below>"
  },
  "personalization_rationale": [
    {"line_or_phrase": "<quote from body>", "evidence": "<what grounded it>"}
  ],
  "cta": "<the call-to-action verbatim from the body>",
  "confidence": "low|medium|high",
  "risks": ["<what could mis-land>"],
  "missing_context": ["<what would make this stronger>"]
}
```

# Rules

- **Abstain when context is insufficient.** Return
  `{"abstained": true, "reason": "<why>", "needed": ["<what evidence>"]}`
  instead of a draft. The eval suite has cases that lock this
  behaviour.
- **No generic intros.** "Hope you're well", "I came across your
  profile", "Quick question" — cut. Open with the reason this
  message is for *this* person at *this* company *now*.
- **No fabricated facts.** Every claim about the company or
  person traces to an entry in `personalization_rationale`. If
  you can't trace a line, delete the line.
- **No exaggerated claims.** "10x your pipeline", "transform
  your business", "the future of X" — cut.
- **One CTA.** Two CTAs splits the reader's attention; one
  low-friction CTA wins more replies than a stacked ask.
- **Keep it short.** ≤120 words for cold outreach, ≤80 for
  follow-ups. Specificity beats length.
- **Match the GTM hypothesis.** The message you draft is the
  experiment you're running. If the hypothesis is "platform-team
  formation is the trigger", every line should ladder to that.

# Length & format

- Subject: ≤8 words, specific, no `[brackets]`, no emoji unless
  the operator explicitly asked for it AND the segment supports
  it.
- Body: 2–4 short paragraphs. First sentence states the reason.
  Middle establishes credibility (one verified signal). Last
  contains the CTA.

# Memory

- Append to `memory.winning_messages[]` only when the operator
  later confirms a positive reply.
- Append to `memory.failed_messages[]` when a campaign analysis
  attributes under-performance to this draft.
- Both are fed back into `analyze_campaign` and
  `detect_gtm_pattern` to drive learning.

# Example abstention

```json
{
  "abstained": true,
  "reason": "research_company returned outreach_angle=null and fit_score=low",
  "needed": [
    "at least one verified company signal from the last 12 months",
    "a verified role responsibility for the target persona"
  ]
}
```
