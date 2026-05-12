---
name: qualify_lead
version: "0.1.0"
description: "Score a single lead against the operator-supplied ICP. Emits an explicit disqualify path with a reason so low-confidence leads don't pollute the pipeline."
provenance:
  source: vertical/sales-sdr
  hash: sha256-pending
---

# When to use

Call after `research_prospect` produces a fact-pack on a single
prospect, or whenever the operator drops a raw lead into the SDR.
Output gates the next step — only `high` and `medium` leads should
proceed to `draft_outreach`. `low` is parked for review;
`disqualify` is closed out and never re-surfaced without a new
signal.

# Inputs you expect

- prospect fact-pack (from `research_prospect` or operator paste)
- the ICP rubric in scope (`memory.icp_rubric`)
- optional: prior-touch history on this lead (informs re-qualification)

# Output shape

A single fenced JSON block:

```json
{
  "qualification": "high|medium|low|disqualify",
  "signals_match": [
    {"signal": "<ICP attribute>", "evidence": "<from fact-pack>"}
  ],
  "signals_missing": ["<ICP attribute not present in fact-pack>"],
  "reasoning": "<two or three sentences, naming the load-bearing signals>",
  "recommended_next_step": "draft_outreach|park|disqualify",
  "disqualify_reason": "<required iff qualification==disqualify; null otherwise>"
}
```

# Rules

- **Prefer `disqualify` with a reason over a forced `low`.** A
  prospect outside the ICP (wrong industry, wrong company size,
  do-not-contact list, jurisdiction blocked) is `disqualify` —
  not `low`. Operators see fewer disqualified leads in the queue,
  which is the point.
- **No bucket without evidence.** Every `signals_match` row cites
  the fact-pack field it came from. Empty `signals_match` → never
  `high`; cap at `medium`.
- **Missing-signal honesty.** List the ICP attributes you could
  not verify in `signals_missing`. Operators decide whether the
  gaps are tolerable — don't paper over them.
- **Recommended next step is one of three.** `draft_outreach`
  for high/medium, `park` for low (operator review), `disqualify`
  for terminal exits. Never recommend `book_meeting` from here —
  that requires explicit interest signal first.

# Examples

**Disqualify (jurisdiction):** prospect at a US sanctions-listed
entity. `qualification: disqualify`, `disqualify_reason:
"sanctioned entity per memory.icp_rubric.jurisdiction_blocklist"`,
`recommended_next_step: disqualify`.

**Medium (partial match):** SaaS, 200-employee, Series B — ICP
asks for B2B SaaS 100-500 employees post-Series-A.
`signals_match: [industry, headcount, funding_stage]`,
`signals_missing: [decision_maker_in_role]`,
`qualification: medium`, `recommended_next_step: draft_outreach`.
