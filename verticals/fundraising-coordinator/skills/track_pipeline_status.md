---
name: track_pipeline_status
description: Given investor_pipeline data, surface stalled investors (>14d no contact), at-risk diligence threads, and a next-best-action per active investor. Reporting only — does not contact investors.
inputs:
  - investor_pipeline_snapshot
  - now_iso
outputs:
  - pipeline_summary
  - stalled
  - at_risk
  - next_best_action
  - _confidence
---

# Skill: Track Pipeline Status

Produce a pipeline review the founder can scan in under 60 seconds.

1. **Group by stage.** Count investors at each stage (`intro`,
   `first-call`, `followup`, `diligence`, `term-sheet`, `passed`,
   `closed`). Surface counts as the lead line.
2. **Stalled (>14d no contact).** Any investor where
   `now - last_contact_at > 14 business days` AND stage is not
   `passed` / `closed`. Sort by stage-age descending; oldest stall
   in any pre-term-sheet stage is the most actionable.
3. **At-risk diligence.** For investors at `diligence` or
   `first-call` stage, surface any with negative signals:
   - >7d to respond to founder's last message
   - vague feedback in the last interaction ("interesting, will
     circle back" without a date)
   - downgraded interest signals (smaller potential check, fewer
     partners on the next call, reschedule pattern)
   These are noted but never editorialized. Surface the signals;
   let the founder judge.
4. **Per-investor next-best-action.** For each active investor
   (not `passed` / `closed`), one specific recommendation:
   - "send T+7 follow-up draft (already prepared)"
   - "propose Tue/Wed/Thu slots for partner meeting"
   - "queue for resurrect ping in 3 weeks"
   - "ask founder to nudge personally (>21d stalled)"
5. **Per-investor cadence respect.** If
   `investor_pipeline.cadence_pref` overrides default 2/7/14 cadence,
   compute stall thresholds against the override. Some investors
   want monthly; do not flag them as stalled at day 15.
6. **No outbound.** This skill REPORTS. Any nudge, follow-up, or
   slot proposal is a downstream skill behind an approval gate.

Output shape:

```json
{
  "pipeline_summary": {
    "intro": 4, "first-call": 6, "followup": 8, "diligence": 3,
    "term-sheet": 1, "passed": 9, "closed": 0
  },
  "stalled": [
    {"investor": "Globex Ventures", "stage": "followup", "days_since_contact": 17, "last_action": "founder sent T+7 update"},
    {"investor": "Initech Partners", "stage": "diligence", "days_since_contact": 22, "last_action": "reference call request"}
  ],
  "at_risk": [
    {"investor": "Initech Partners", "signals": ["reschedule x2", "downgraded check signal", "vague last reply"]}
  ],
  "next_best_action": [
    {"investor": "Globex Ventures", "action": "send T+14 stalled draft (already prepared)"},
    {"investor": "Acme Capital", "action": "propose 3 slots for partner meeting next week"}
  ]
}
```
