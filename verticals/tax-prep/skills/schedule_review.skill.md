---
name: schedule_review
version: "0.1.0"
description: Propose 3 client-review meeting slots factoring preparer availability, appointment-type duration, and time-zone. Autonomous — preparer's own calendar; no external invite from this skill.
provenance:
  source: vertical/tax-prep
  hash: sha256-pending
---

# When to use

After `draft_return` produces a working paper. Books a private
review meeting on the preparer's calendar so the client can walk
through the draft before sign-off. No external invite from this
step; the client invite is a separate operator-approved action.

# Inputs you expect

- `client_id`, `return_year`.
- `preparer_availability` — `{tz, busy: [iso-intervals],
  preferred_windows: [iso-intervals]}`.
- `client_tz` — IANA zone (e.g. `America/Los_Angeles`); never guess.
- `appointment_type` — `quick-review` (30 min), `full-walkthrough`
  (60 min), or `amendment-review` (45 min).
- `draft_path` — optional pointer to the markdown working paper.

# Output shape

```json
{
  "client_id": "synthetic-001",
  "return_year": 2025,
  "appointment_type": "full-walkthrough",
  "duration_minutes": 60,
  "proposed_slots": [
    {
      "start_utc": "2026-03-10T17:00:00Z",
      "end_utc":   "2026-03-10T18:00:00Z",
      "preparer_tz": "America/New_York",
      "preparer_wall_clock": "2026-03-10 13:00 ET",
      "client_tz": "America/Los_Angeles",
      "client_wall_clock":   "2026-03-10 10:00 PT"
    }
  ],
  "next_action": { "name": "book_review_slot", "requires_approval": false },
  "flags": []
}
```

# Approval posture

**Autonomous.** No client-facing invite; only a private event on the
preparer's own calendar. Booking appends to the `review_log` memory
entry.

# Hard rules

- **Three distinct days** so one conflict doesn't wipe all options.
- **No same-day; earliest is next business day from now.**
- **No weekend slots** unless the client hinted weekend preference.
- **TZ on every slot** — preparer + client TZ + wall-clock present.
- **Missing `client_tz` → stop:** `flags: ["missing_client_tz"]`.
- **No double-book** against `preparer_availability.busy`.

# Examples

Synthetic — `synthetic-001`, client in `America/Los_Angeles`,
preparer in `America/New_York`. Three distinct weekday slots in the
next 10 business days; operator picks one and the booking writes
to gcal + `review_log`.
