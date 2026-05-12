---
name: schedule_posting_calendar
version: "0.1.0"
description: Produce a 14-day posting calendar from a content pipeline + channel mix + frequency policy, honoring operator quiet hours and channel best-time windows. Approval-gated before any scheduling.
provenance:
  source: vertical/social-media-drafter
  hash: sha256-pending
---

# When to use

Use when an operator has approved drafts in the content pipeline and
needs them laid across the next 14 days. Proposes a calendar — never
books the scheduler. Re-run after pipeline edits; the calendar is
regenerated, not patched.

# Inputs you expect

- `content_pipeline` — list of `{content_id, channel, approval_state,
  preferred_window_hint}`. Only `approval_state == "approved"` items
  are eligible.
- `channel_mix` — `{x?: n_per_week, linkedin?: n, instagram?: n,
  facebook?: n}`. Weekly targets, not hard caps.
- `frequency_policy` — `{quiet_hours: [{tz, start, end, days}],
  min_gap_minutes_per_channel, max_posts_per_day_per_channel}`.
- `channel_best_windows` — operator-configured `{channel: [{day,
  start, end, tz}]}`. Optional; falls back to conservative defaults.

# Output shape

```json
{
  "calendar_window": {"start_utc": "2026-05-13T00:00:00Z", "end_utc": "2026-05-27T00:00:00Z"},
  "slots": [
    {
      "slot_utc": "2026-05-14T13:00:00Z",
      "channel": "linkedin",
      "content_id": "approved-014",
      "wall_clock_local": "2026-05-14 09:00 ET",
      "window_match": "linkedin.weekday_morning",
      "rationale": "weekday morning best-window; respects 12h gap from prior LI post"
    }
  ],
  "unslotted": [
    {"content_id": "approved-021", "reason": "channel_mix_exhausted"}
  ],
  "next_action": {
    "name": "schedule_calendar_batch",
    "requires_approval": true,
    "auto_send_allowed": false
  },
  "flags": []
}
```

Also include a Markdown table of the slots — same columns, one row
per slot, in slot-time ascending order — for human review.

# Hard rules

- **Approval-gated.** `next_action.requires_approval: true` and
  `auto_send_allowed: false`. The skill never writes to a scheduler;
  booking happens via a Tier A connector after operator approval.
- **Quiet hours are inviolable.** No slot may fall in any
  `frequency_policy.quiet_hours` window (resolved in its declared
  `tz`). Quiet hours include operator-declared days-off.
- **Best-window-first.** Prefer `channel_best_windows` matches.
  Record `window_match` per slot; a slot outside any best window
  gets `window_match: "fallback"` plus a one-line rationale.
- **Channel gap + daily cap.** Two posts on the same channel must
  be ≥ `min_gap_minutes_per_channel` apart, and per-day per-channel
  count must not exceed `max_posts_per_day_per_channel`.
- **Only approved drafts.** Items with `approval_state != "approved"`
  go to `unslotted` with reason `not_approved`; never silently drop.
- **No auto-rebalance.** If `channel_mix` can't be hit given quiet
  hours + gaps, slot what fits and place the rest in `unslotted`
  with reason `channel_mix_exhausted`. Never relax policy to fit.
- **Synthetic IDs in examples.** `approved-001` etc.; never real IDs
  or post copy.

# Examples

Synthetic — 14-day window, `channel_mix {linkedin: 3, x: 5}`, ET
quiet hours 21:00–07:00, LinkedIn best window 08:00–10:00 ET, X
best windows include 12:00–14:00 ET. Six approved drafts in pipeline.
Output proposes 6 slots across distinct days; one X slot falls
outside best window → `window_match: "fallback"`. Operator reviews
JSON + table and approves the batch.
