---
name: track_rsvps
description: Given an attendee list and operator-pasted RSVP responses, produce the canonical 7-field RSVP envelope plus a 24h-before-event Markdown status summary. Caps confirmed count at venue capacity and flags overflow for operator review.
inputs:
  - event_id
  - attendees
  - rsvp_responses
  - venue_capacity
outputs:
  - rsvp_envelope
  - status_summary_md
  - escalation_flags
---

# Skill: Track RSVPs

Attendee list + raw RSVP responses in; canonical RSVP envelope and a
status-email-ready Markdown summary out. The envelope is the load-bearing
input for downstream `run_of_show` and `build_runbook` skills — emit all
seven fields exactly, even when empty, so consumers never re-parse.

1. **Bucket every attendee.** Each `attendees` entry resolves to exactly
   one of `confirmed` / `declined` / `pending` / `no_response`. `pending`
   means the attendee replied but did not commit (e.g. "maybe", "trying
   to make it"); `no_response` means no reply at all. Never double-count.
2. **Dietary + accessibility extraction.** Pull `dietary_constraints[]`
   and `accessibility_requests[]` from confirmed and pending attendees
   only. Deduplicate and preserve operator-supplied wording verbatim —
   do NOT canonicalize (e.g. keep both `gluten-free` and `celiac` if
   both appear).
3. **Plus-ones.** `plus_ones_count` sums declared plus-ones across
   confirmed attendees only. Pending plus-ones do not count toward the
   capacity calculation.
4. **Capacity check.** If `venue_capacity` is provided and
   `confirmed.length + plus_ones_count > venue_capacity`, flag
   `requires_immediate_operator_review` with the overflow count. Do NOT
   silently truncate the confirmed list; emit the full bucket and let
   the operator decide who to waitlist.
5. **Status summary.** Emit a Markdown block suitable for a 24h-before
   status email: headline counts, capacity status (under / at / over),
   named dietary + accessibility callouts, and a one-line "operator
   action" if any escalation flag fired.

Output shape:

```json
{
  "rsvp_envelope": {
    "confirmed": 87,
    "declined": 12,
    "pending": 5,
    "no_response": 8,
    "dietary_constraints": ["gluten-free", "vegan", "nut-free"],
    "accessibility_requests": ["wheelchair access", "live captioning"],
    "plus_ones_count": 14
  },
  "status_summary_md": "## RSVP status — 24h check\n- Confirmed: 87 (+14 plus-ones) | Declined: 12 | Pending: 5 | No response: 8\n- Capacity: 101 / 120 (under)\n- Dietary: gluten-free, vegan, nut-free\n- Accessibility: wheelchair access, live captioning\n",
  "escalation_flags": []
}
```

If capacity is exceeded, `escalation_flags` includes
`requires_immediate_operator_review` and the status summary names the
overflow count in its "operator action" line.
