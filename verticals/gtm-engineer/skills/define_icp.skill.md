---
name: define_icp
version: "0.1.0"
description: "Defines (or refines) an Ideal Customer Profile from a product description, target market, known customers, and constraints. Always separates verified facts from assumptions and surfaces evidence gaps so the next research cycle is targeted."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call this skill **first** when the operator asks "who should we
sell to" or supplies fresh constraints (a new product launch,
geographic pivot, vertical entry). Output becomes the source of
truth for `find_target_accounts`, `research_company`, and
`generate_outreach`.

# Inputs you expect

- product description (one paragraph minimum)
- target market hypothesis (broad → "enterprise SaaS", narrow →
  "Series A fintech in EMEA")
- known customers OR explicit assumptions (label which is which)
- constraints (regions to exclude, pricing floor, compliance
  requirements)

# Output shape

A single fenced JSON block with these fields:

```json
{
  "industry": "<one or more verticals>",
  "company_size": "<headcount or revenue band>",
  "buyer_roles": ["<title 1>", "<title 2>"],
  "pain_points": ["<pain 1>", "<pain 2>"],
  "trigger_signals": ["<signal 1>", "<signal 2>"],
  "exclusions": ["<negative criterion 1>", "<negative criterion 2>"],
  "why_now": "<one sentence on timing>",
  "confidence": "low|medium|high",
  "evidence": {
    "verified": ["<sourced fact 1>", "<sourced fact 2>"],
    "assumed": ["<unsourced inference 1>"]
  },
  "evidence_gaps": ["<what we'd need to verify next>"]
}
```

# Rules

- **Always include exclusions.** A "negative ICP" sharpens
  targeting more than another positive criterion. ("Not: pre-seed
  consumer apps" beats yet another bullet on enterprise.)
- **Separate verified from assumed.** Anything in `verified` must
  cite (a customer name, a public report, a quote). Everything
  else goes in `assumed`.
- **Avoid generic ICPs.** "Mid-market companies that want to
  grow" is a non-ICP — fail loud and ask the operator for one
  more constraint.
- **Surface evidence gaps.** The whole point of a low-confidence
  ICP is to drive the next research cycle.
- **`why_now` matters.** A great ICP without a timing signal
  ("budget-cycle change", "new regulation", "recent funding") is
  half an ICP.

# Examples

**Strong (high confidence, focused):**

```json
{
  "industry": "B2B SaaS observability tooling",
  "company_size": "100–500 engineers",
  "buyer_roles": ["VP Engineering", "Head of SRE"],
  "pain_points": ["alert fatigue", "tool sprawl across teams"],
  "trigger_signals": ["recent SRE hire", "platform-team formation post-Series-B"],
  "exclusions": ["pre-Series-A startups", "single-team engineering orgs"],
  "why_now": "platform-team formation is the funded-org moment when alert-routing becomes a hire-vs-tool decision",
  "confidence": "high",
  "evidence": {
    "verified": ["3 reference customers all hired SRE leads within 6 weeks of buying"],
    "assumed": ["Series B is the typical platform-team-formation stage"]
  },
  "evidence_gaps": ["validate the 6-week hiring-cadence pattern beyond reference customers"]
}
```

**Weak (low confidence, abstain-quality):**

```json
{
  "industry": "B2B SaaS",
  "company_size": "any",
  "buyer_roles": ["VP Engineering"],
  "pain_points": ["want to grow"],
  "trigger_signals": [],
  "exclusions": [],
  "why_now": "general AI tailwind",
  "confidence": "low",
  "evidence": {"verified": [], "assumed": ["entire ICP"]},
  "evidence_gaps": ["everything — recommend operator supplies 2-3 reference customers + one constraint before next pass"]
}
```

# Memory

Append the output to `memory.icp[]`. Existing rows are not
overwritten — each refinement is a new row so the operator can
diff successive passes.
