---
name: research_person
version: "0.1.0"
description: "Researches a target person or buying role for outreach relevance. Falls back to role-based relevance when personal data is limited; never invents personal facts."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call when the operator names a specific person OR when
`generate_outreach` needs persona context for a role-based send.

# Inputs you expect

- person name (optional — empty means role-only research)
- role title (required when person name is missing)
- company context (the matching `account_research` row)
- current GTM hypothesis

# Output shape

```json
{
  "subject": {"person": "<name or null>", "role": "<title>"},
  "role_relevance": "<one sentence on why this role matters for the GTM hypothesis>",
  "likely_responsibilities": ["<r1>", "<r2>"],
  "likely_pains": [
    {"pain": "<what>", "basis": "verified|role-inference", "confidence": "low|medium|high"}
  ],
  "buying_influence": "decision-maker|influencer|user|gatekeeper",
  "personalization_angle": {
    "angle": "<safe, business-relevant angle>",
    "basis": "<the source — public talk, role responsibility, company signal>"
  },
  "uncertainty": ["<what you don't know but a sender should>"]
}
```

# Rules

- **Never invent personal facts.** No fabricated career history,
  no hallucinated podcast appearances, no inferred personal
  interests. Public, verifiable, business-relevant or it doesn't
  appear.
- **Avoid creepy personalization.** "I see you were at Acme from
  2014–2017" reads as creepy when the only evidence is a LinkedIn
  scrape. "I see you've spoken about platform-team formation" is
  fine when sourced to a public talk.
- **Role-based relevance is a first-class output.** When personal
  data is thin, lean on role responsibility ("VPs of Engineering
  at Series B SaaS typically own platform-team formation") rather
  than guessing about the individual.
- **Mark assumptions clearly.** Every `likely_pains` row carries
  `basis: "verified"` (sourced) or `basis: "role-inference"`
  (extrapolated from the role).
- **Populate `uncertainty`.** The whole point of this output is
  to let `generate_outreach` know what *not* to over-personalize.

# How to source

1. `web_extract` the person's public profile / company bio when
   one exists.
2. `web_search` for public talks, podcast appearances, blog posts
   under the person's name.
3. When personal data is thin, fall back to role-pattern research
   from `memory.persona_research[]` (prior runs may have built
   the role profile).

# Memory

Append role-level findings to `memory.persona_research[]`. The
key shape is `(role_title)` so multiple runs against the same
role accumulate intelligence about the role itself, not just one
person.
