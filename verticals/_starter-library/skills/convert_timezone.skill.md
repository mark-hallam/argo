---
name: convert_timezone
version: "0.1.0"
description: "Converts a timestamp or interval from one timezone to another, with explicit DST handling and ambiguity flags."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

Anywhere the operator (or another skill) needs a time expressed
in someone else's timezone — meeting confirmations, deadlines
across regions, on-call rotation hand-offs. Use any time
"<time> in <zone> A → <zone> B" is the operation.

# Inputs

- `source_time`: ISO-8601 string with offset OR `[start, end]`
  for intervals. Naive times (no offset) are rejected — see
  Notes.
- `source_zone`: IANA timezone of `source_time`. Required even
  if `source_time` already includes an offset, to handle DST
  transitions unambiguously.
- `target_zone`: IANA timezone to convert to.
- `format` (optional): `"iso"` (default), `"human"` (`"Tue 14:00
  BST"`), or `"both"`.

# Output

The converted timestamp/interval in `target_zone`, plus a
`note` field if any of these apply: DST transition crossed,
ambiguous local time (fall-back hour), non-existent local
time (spring-forward gap).

# Examples

- `2026-03-12T10:00-05:00` in `America/New_York` → `Europe/London`
  → `2026-03-12T14:00+00:00` (note: source date is before US DST,
  target is before UK DST).
- Interval `[09:00-17:00]` `Asia/Tokyo` → `America/Los_Angeles`
  → produces a "day before" span; flag explicitly.

# Approval posture

Pure computation. No side effects. Safe to call freely.

# Notes

- Reject naive timestamps. Always ask the operator for the
  source timezone explicitly — guessing is the #1 source of
  scheduling errors.
- Use IANA tz database names (`"Europe/London"`), not
  abbreviations (`"BST"`/`"GMT"`) which are ambiguous.
- Annotate DST transitions in `note` even when the conversion
  is technically unambiguous — operator may have intended the
  "other" hour.
- For recurring slots, do NOT convert once and assume it holds;
  re-convert per occurrence because DST shifts mid-recurrence.
