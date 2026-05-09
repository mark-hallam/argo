# Skill: run-of-show — composing event timelines

Use this template every time you emit `generate_run_of_show`. The
output is a markdown document the operator hands to the venue + lead
vendors on event day. Buffer-time heuristics keep the timeline honest;
under-buffered timelines are the leading source of day-of friction.

## Output template

```markdown
# Run of show — <event_name> (<event_id>)
**Date:** <event_date_iso>  **Venue:** <venue_name>  **Headcount:** <final_or_planned>

| Time  | Segment              | Owner            | Notes                |
|-------|----------------------|------------------|----------------------|
| HH:MM | Vendor load-in       | <category vendor> | Buffer: 30 min       |
| HH:MM | Setup window         | venue + AV       | Buffer: 60 min       |
| HH:MM | Sound check / walk   | AV               | Buffer: 30 min       |
| HH:MM | Doors open / arrival | venue            | Buffer: 15 min       |
| HH:MM | Program segments     | host             | Per program          |
| HH:MM | Service / catering   | catering         | Buffer: 15 min       |
| HH:MM | Wrap + load-out      | venue + AV       | Buffer: 45 min       |
```

Mark unconfirmed vendors with `(pending)` next to the owner column
so the operator can chase before event day.

## Buffer-time heuristics

- **Vendor load-in.** 30 min minimum per category. Catering and AV
  load-ins **stack, do not overlap** — never schedule them in the same
  30-min window unless the venue has separate loading docks.
- **Setup window.** 60 min for events <50 headcount; 90 min for
  50–150; 120 min for >150 or any event with full plated service.
- **Sound check.** 30 min for spoken-word programs, 45 min for any
  live music or multi-mic panel.
- **Service / catering reset.** 15 min between courses (plated) or
  20 min between buffet refreshes.
- **Wrap / load-out.** 45 min minimum; 60 min if AV brought a stage
  rig (Riverline-class) or florals included installations.

## Refusal rules

- If a confirmed vendor's `lead_time_days` from `inventory/sample_vendors.json`
  exceeds the days remaining to event date, mark the slot `(at risk)`
  in the Notes column and flag in the operator-facing summary above
  the table. Do NOT silently drop the segment.
- If two vendors share an owner column entry (e.g., both AV vendors
  appear), the operator double-booked — refuse to compose the
  timeline and surface the conflict.
- Never invent a vendor that is not in `vendor_status` memory.
  Pending segments use the category name (`catering (pending)`),
  not a guessed vendor.
