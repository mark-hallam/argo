---
name: update_lead_state
version: "0.1.0"
description: "Persist a single lead state transition into memory.lead_pipeline. 8-state vocabulary covering the SDR funnel from raw lead through hand-off or disqualification."
provenance:
  source: vertical/sales-sdr
  hash: sha256-pending
---

# When to use

After any skill that changes a lead's funnel position
(`research_prospect`, `qualify_lead`, `draft_outreach`,
`book_meeting`, `handle_objection`). Internal bookkeeping — no
outbound side effect — so this skill is not approval-gated.

# Inputs you expect

- lead_id (synthetic, generated on first transition)
- prior state (or `null` for first row)
- new state (one of the 8-state vocabulary below)
- transition trigger (which skill or operator action caused it)

# Output shape

```json
{
  "lead_id": "<sdr_<slug>_<n>>",
  "prior_state": "<8-state token or null>",
  "new_state": "<8-state token>",
  "transition_trigger": "<skill name or 'operator:<note>'>",
  "last_touched_at": "<ISO-8601 UTC timestamp>",
  "memory_row": {"scope": "lead_pipeline", "operation": "append|upsert"}
}
```

# The 8-state vocabulary

In funnel order:

1. `new` — raw lead, untouched.
2. `researched` — `research_prospect` produced a fact-pack.
3. `contacted` — `draft_outreach` was approved + sent.
4. `engaged` — prospect replied (any reply, including objections).
5. `qualified` — `qualify_lead` returned `high`/`medium` AND
   prospect signaled real interest.
6. `meeting-set` — `book_meeting` approved + invite landed.
7. `handed-off` — Account Executive owns the deal now.
8. `disqualified` — terminal; opt-out, disqualify, or AE reject.

Allowed lateral moves: `engaged` → `disqualified` (mid-thread
opt-out), `meeting-set` → `engaged` (no-show reschedule). Skipping
forward states is NOT allowed (e.g. `new` → `contacted` without
`researched`).

# Memory schema notes

Writes to `memory.lead_pipeline[]`. This funnel vocabulary is
orthogonal to `sales-follow-up`'s engagement-temperature stages
(`warm | engaged | dormant`); the shared token `engaged` is
intentional (a reply is the same event) but adjacent states
differ. Re-engagement skills read `last_touched_at` to resume.

# Examples

`research_prospect` just emitted a fact-pack: `prior_state:
"new"`, `new_state: "researched"`, `transition_trigger:
"research_prospect"`. `book_meeting` got operator approval:
`prior_state: "qualified"`, `new_state: "meeting-set"`,
`transition_trigger: "book_meeting"`.
