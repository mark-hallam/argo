---
name: schedule_callback
version: "0.1.0"
description: "When the operator is unavailable and the caller needs a follow-up, capture preferred callback window + reason + operator availability and produce a confirmation draft. Approval-gated only for after-hours scheduling against the operator's quiet-hours setting."
provenance:
  source: vertical/ai-receptionist
  hash: sha256-pending
---

# When to use

Call when (a) the operator cannot pick up live AND (b) the
caller has asked for or needs a follow-up call. Output drives
a confirmation message back to the caller in plain language
and a structured envelope for operator review. Does not
write to the calendar — the calendar write is a sibling brief.

# Inputs you expect

- `caller`: `{name, callback_phone, identity_confidence}` (from
  `verify_caller_identity`)
- `requested_window`: one of
  `next_available` / `specific_window` / `urgent`, plus a free-text
  range if `specific_window` (e.g. "Tue afternoon")
- `reason`: caller's stated reason for the callback (≤ 120 chars)
- `operator_availability`: cached free-busy windows over the next
  3 business days (read from `business_hours` + calendar cache)
- `quiet_hours`: operator's per-weekday quiet-hours window, e.g.
  `{mon = {start = '19:00', end = '08:00'}, ...}`, IANA timezone

# Output shape

```json
{
  "proposed_callback_at": "2026-05-14T15:30:00-07:00",
  "window_basis": "operator_first_free_slot",
  "requested_window": "next_available",
  "caller_facing_message": "Thanks — we will call you back tomorrow at 3:30pm Pacific. If anything changes, we will let you know.",
  "operator_envelope": {
    "caller_name": "Jordan Lee",
    "callback_phone": "+15551234567",
    "reason": "follow-up on demo question",
    "identity_confidence": 0.82
  },
  "next_action": "log_callback_request",
  "requires_approval": false,
  "flags": []
}
```

`next_action` is always `log_callback_request`.
`requires_approval` flips to `true` only when the proposed
slot falls inside `quiet_hours` (see Approval posture).

# Examples

- Caller asks for the next available slot Tuesday morning →
  pick the operator's first free 15-minute block on Tuesday
  ≥ 09:00 local. Confirmation message names the slot in the
  caller's timezone if extractable, else operator's tz with
  abbreviation. `requires_approval: false`.
- Caller flags `urgent` and operator has a free slot today
  ≥ 30 minutes out → propose today's first viable slot, flag
  `caller_marked_urgent`, keep `requires_approval: false`.
- Caller's only available window is Friday 8:00pm but the
  operator's quiet-hours start at 19:00 → propose the slot
  but set `requires_approval: true`, flag `after_hours_request`.
  The caller-facing message says "we will confirm and reach
  back out" — never asserts the after-hours callback as booked.
- `operator_availability` empty for the next 3 business days
  → flag `no_availability` and escalate. The caller-facing
  message says "we are tight on time this week; we will
  reach back out with options" — never promises a window.

# Approval posture

Autonomous **unless** the proposed slot falls inside the
operator's `quiet_hours` — then `requires_approval = true`
and the action gates through `[escalation].human_approval_required`
in `vertical.toml`. Hard rules:

- **Never schedule outside `quiet_hours` without approval.**
  The quiet-hours guard is the safety belt for caller-driven
  late-night / weekend asks. Apply even if the caller
  insists; the operator decides.
- **Never claim the callback is booked** in the caller-facing
  message. Use "we will call you back" — not "your callback
  is scheduled" or "calendar updated". The actual calendar
  write happens in a sibling action.
- **PII-soft envelope.** `reason` is captured verbatim
  ≤ 120 chars; redact obvious sensitive markers (account
  numbers, payment details, health terms) and flag
  `pii_soft_redaction`.
- **Holiday awareness.** If the proposed slot falls on a
  declared holiday in `business_hours.holidays`, advance to
  the next open day and re-confirm with the caller — never
  quote a closed-for-holiday window.
