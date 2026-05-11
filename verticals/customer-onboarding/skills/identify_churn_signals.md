---
name: identify_churn_signals
description: Given engagement data, score churn risk and list the top 3 indicators driving the score.
inputs:
  - account_profile
  - playbook_state
  - engagement_thresholds
  - churn_weightings
outputs:
  - health_score
  - churn_indicators
  - next_action
  - flags
---

# Skill: Identify Churn Signals

Score the account's churn risk and surface the top 3 indicators.
The score is advisory; flipping the CRM churn flag (`mark_churn_risk`)
is always approval-gated.

1. **Signal sources.** Read from `playbook_state` and
   `engagement_thresholds`:
   - `logins_unique_days_last_7`
   - `features_touched_last_7`
   - `seats_activated_pct`
   - `tickets_severity_weighted_last_7`
   - `reply_sentiment` (positive / neutral / negative / churn-keyword)
   - `champion_status` (present / left / unknown)
2. **Scoring rule.** For each signal, normalize observed vs.
   threshold to a 0-1 risk score, multiply by the operator's
   `churn_weightings.<signal>` (in `inventory.json`), sum the
   weighted risks, divide by total weight. Output:
   - `green` if combined score < 0.3
   - `yellow` if 0.3 ≤ score < 0.6
   - `red` if score ≥ 0.6
3. **Top 3 indicators.** Rank by weighted risk descending. Each
   indicator is one line: signal name, observed value, threshold,
   risk contribution. No more than 3.
4. **Hard-trigger overrides.** Any one of these forces `red` and
   sets `flags`:
   - `churn-keyword in reply_history` → `flags: ["churn_keyword_in_reply"]`
   - `champion_status = left` → `flags: ["champion_change"]`
   - `tickets_severity_weighted_last_7 > 2× baseline` → `flags: ["ticket_volume_spike"]`
5. **Next action.** If score is `red`, recommend
   `mark_churn_risk` AND `trigger_executive_intervention`, both
   `requires_approval: true`. If `yellow`, recommend a day-7
   value check-in. If `green`, continue the playbook.

Hard rule: never auto-flip the CRM. Output is advisory; the
operator decides.
