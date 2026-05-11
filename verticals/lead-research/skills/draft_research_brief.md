---
name: draft_research_brief
version: "0.1.0"
description: "Combines firmographic, contacts, news, and tech-stack signals into a sales-ready 1-pager with ranked talking points and overall confidence."
provenance:
  source: vertical/lead-research
  hash: sha256-pending
capabilities: ["file_read"]
---

# When to use

Call AFTER `research_company` and `identify_decision_makers` have
run for the same company in this session (or the matching `accounts`
memory row has been populated by a prior run). This skill does no
fresh research — it composes.

# Inputs you expect

- company name (required)
- the `accounts` row for this company (firmographic + contacts +
  news + tech_stack, as produced upstream)
- current ICP / GTM hypothesis the SDR is running

# Output shape

A single markdown brief, ≤ 1 page rendered, with these sections in
order:

```markdown
# <Company> — Lead Research Brief

**Confidence:** <low|medium|high> — <one sentence>

## Firmographic
- <field>: <value> (<source URL or "unknown">)

## Contacts (top 5)
- <name or "role only"> — <title> — <committee_role> — <linkedin or "no public URL">
  - Why: <one sentence>

## News (last 12 months)
- <YYYY-MM-DD> — <headline> — <source URL>
  - Relevance: <one sentence>

## Tech stack
- <signal> (<verified|inferred>) — <source>

## Talking points
1. <angle> — grounded in: <which signal above>
2. ...

## What we don't know
- <gap 1>
- <gap 2>
```

# Rules

- **No re-research.** This skill composes existing signals. If a
  required signal is missing, list it under "What we don't know"
  rather than fabricating it.
- **Talking points cite upstream.** Every ranked angle names the
  specific signal it grounds in. Angles without an upstream reference
  are dropped, not laundered.
- **3–5 talking points.** Fewer when evidence is thin. The SDR
  picks one to lead with, not eight.
- **Confidence inherits the weakest input.** If
  `identify_decision_makers` returned `confidence: low`, the brief's
  overall confidence cannot be `high`.
- **No outreach drafting.** The brief stops at "talking points".
  Outreach copy is the SDR vertical's job and requires its own
  approval gate (`send_outreach_email`).
