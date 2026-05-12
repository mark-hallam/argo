---
name: research_prospect
version: "0.1.0"
description: "Deep-dive on a single prospect (company + key contact). Verified-vs-inferred separation; abstains on personalization when evidence is weak. JSON shape compatible with gtm-engineer's research_company."
provenance:
  source: vertical/sales-sdr
  hash: sha256-pending
---

# When to use

First step of the SDR loop on a named prospect. Reads
`memory.icp_rubric` for the fit context and
`memory.account_research[]` if the company is already in cache;
otherwise sources via `web_search` + `web_extract`. Output feeds
`qualify_lead`.

# Inputs you expect

- prospect company name (+ URL when available)
- key contact (name + role) if known
- ICP rubric in scope
- optional: current outbound campaign hypothesis

# Output shape

A single fenced JSON block. The top-level shape mirrors
`gtm-engineer/skills/research_company` so a skill-router can swap
implementations without translation:

```json
{
  "company": "<name>",
  "summary": "<one paragraph, verified facts only>",
  "relevant_signals": [
    {"signal": "<what>", "evidence": "<source>", "recency": "<date or 'unknown'>"}
  ],
  "likely_pains": [
    {"pain": "<what>", "basis": "verified|inferred", "confidence": "low|medium|high"}
  ],
  "buying_triggers": ["<trigger>"],
  "fit_score": "low|medium|high",
  "outreach_angle": {
    "angle": "<one sentence>",
    "evidence": "<the verified signal this is grounded in>"
  },
  "contact": {
    "name": "<contact name>",
    "role": "<role>",
    "verified_at": "<source URL or 'unverified'>"
  }
}
```

# Rules

- **Verified vs. inferred.** Every `likely_pains` entry carries
  `basis: "verified"` (sourced) or `basis: "inferred"`
  (hypothesis from signals). Mixing the two is the dominant path
  to bad outreach.
- **Cite per signal.** A `relevant_signals` row without an
  `evidence` field is invalid — drop it.
- **Don't hallucinate a recent event.** If the most recent
  verified signal is older than 12 months, say so. Pretending
  "they just pivoted" is the failure this skill exists to prevent.
- **Abstain when evidence is weak.** When you have only the
  company name and nothing more, return `outreach_angle: null` and
  `fit_score: "low"`. `draft_outreach` will refuse to draft from
  a null angle — that's the intended hand-off.
- **Contact verification.** If the key contact's role can't be
  verified from a primary source (LinkedIn, company about page,
  press release), mark `verified_at: "unverified"` — never invent.

# Examples

Inputs: `company="Northwind Fintech"`, `contact="Priya Shah, VP
Eng"`. Output cites a verified Series-B announcement (`recency:
2026-05-04`), a verified hiring spike on platform engineering
(basis: verified), an inferred scaling pain (basis: inferred,
confidence: medium), and an outreach angle grounded in the
Series-B signal.
