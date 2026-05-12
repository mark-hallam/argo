---
name: detect_disengagement
description: Given a lead's engagement history (last_touch_at, opens / clicks pattern, response gaps), score current engagement, trend it, and recommend one action (continue-cadence / pause-30d / switch-channel / drop-from-cadence). Conservative on drop — only recommended when engagement_score < 20 AND days_since_last_response > 60. Drafting only — never mutates `lead_index` or the CRM.
inputs:
  - lead_context
  - engagement_history
  - cadence_policy
outputs:
  - engagement_score
  - trend
  - days_since_last_response
  - recommended_action
  - supporting_evidence
  - flags
---

# Skill: Detect Disengagement

Score the lead's engagement, name the trend, recommend one action.

1. **Read engagement history.** `engagement_history` from
   `memory.lead_index` carries opens, clicks, replies, and the
   timestamped touch log. If the field is missing or empty, flag
   `no_engagement_history` and stop — the vertical never infers
   disengagement from absence of data.
2. **Score 0-100.** Weight recent replies highest, then clicks,
   then opens; decay each signal's contribution by age (touches
   older than 90 days contribute zero). Express the score as an
   integer; no fractional precision the operator can't act on.
3. **Trend vocab — exactly four.** `improving` (score up >10
   over last 3 touches), `stable` (within ±10), `declining`
   (score down >10), `cold` (no engagement signal in last 45
   days). One value only; no compound trends.
4. **Action vocab — exactly four.**
   - `continue-cadence` — score ≥ 60 OR trend = improving.
   - `pause-30d` — score 20-59 AND trend ∈ {declining, stable}.
   - `switch-channel` — score 20-59 AND last 2+ touches on the
     same channel went unread (try the alternate channel from
     `cadence_policy.channels` before pausing).
   - `drop-from-cadence` — score < 20 AND `days_since_last_response`
     > 60. Both gates required; one alone never triggers drop.
5. **Supporting evidence.** 2-4 short citations from
   `engagement_history` — "last reply 78 days ago", "0 opens in
   last 4 touches", "click-through on Msg 6 but no reply since".
   The operator scans evidence before accepting the action.

# Hard rules

- **Two-gate drop threshold is load-bearing.** Never recommend
  `drop-from-cadence` unless BOTH `engagement_score < 20` AND
  `days_since_last_response > 60`. Premature drops cost
  pipeline; the operator's trust in the vertical erodes the
  first time a warm lead is dropped silently.
- **No mutation.** This skill never updates `lead_index`, never
  archives a lead, never writes to the CRM. It surfaces a
  recommendation; the operator commits it through their own
  workflow or an approval-gated `update_crm` action.
- **No engagement data → flag, don't guess.** A lead with zero
  touches is not "cold" — they are unqualified-for-this-skill.
  Flag `no_engagement_history` and let `research_lead_signals`
  re-enter from a fresh signal.
- **Channel-switch before drop.** If a lead has only been
  touched on one channel and `cadence_policy.channels` lists
  more, prefer `switch-channel` over `pause-30d` or
  `drop-from-cadence` — one channel's silence is not the lead's
  verdict.
- **`do_not_contact` short-circuits everything.** If
  `lead_context.do_not_contact` is true, return immediately with
  `recommended_action = drop-from-cadence`,
  `flags: ["do_not_contact"]`, and skip the score / trend
  computation. The operator's instruction is not negotiable.
