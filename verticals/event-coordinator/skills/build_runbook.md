---
name: build_runbook
description: Given an event spec, produce a per-hour runbook with explicit time / item / owner / dependency / risk columns drawn from the matching runbook_templates skeleton.
inputs:
  - event_id
  - event_shape
  - event_date
  - confirmed_vendors
  - confirmed_venue
outputs:
  - runbook
  - escalation_flags
---

# Skill: Build Runbook

Event spec in; per-hour runbook out. The runbook is the most
operator-visible deliverable on event day — own every dependency.

1. **Template seed.** Pick the matching skeleton from
   `runbook_templates` (e.g. `conference` or `offsite`). If none
   matches, flag `template_unmatched` and surface the closest shapes.
2. **Per-hour granularity.** One row per scheduled hour from event
   start through event end + 1 hour buffer. Setup / teardown blocks
   render as separate rows above / below the event window.
3. **Required columns** (markdown table, in this order):
   - `time` — `HH:MM` in the event's local timezone.
   - `item` — what happens this hour (single phrase, ≤ 80 chars).
   - `owner` — named vendor or person. Mark `(unconfirmed)` if not
     in `confirmed_vendors`.
   - `dependency` — what must complete first (reference a prior
     row's `time` or vendor).
   - `risk` — one-line risk callout for this row.
4. **Risk callouts.** Surface at least one risk per major transition
   (registration → opening, lunch → afternoon, close → teardown).
   "No known risk" is valid when justified.
5. **Unconfirmed vendors.** Any `owner` not in `confirmed_vendors`
   renders with `(unconfirmed)` and a `chase` risk callout naming
   days-until-event.
6. **Budget tie-in.** End the runbook with the `spend_summary`
   bracketed line citing the event's budget line.

Output shape:

```json
{
  "runbook": {
    "format": "markdown",
    "header": "| time | item | owner | dependency | risk |",
    "rows": [
      {"time": "07:00", "item": "Setup + AV check", "owner": "Lighthouse AV",
       "dependency": "venue access at 06:30", "risk": "AV smoke-test required before 08:30"}
    ],
    "spend_summary": "[budget: bl_offsite_q3_fy26 | period: Q3-FY26 | ceiling: $80000 | used: $58400 | remaining: $21600]"
  },
  "escalation_flags": []
}
```
