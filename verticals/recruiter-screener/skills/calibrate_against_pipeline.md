---
name: calibrate_against_pipeline
description: Given the current candidate's score + 5 prior similar candidates' outcomes from pipeline_history, flag whether the score is consistent with how the team has historically scored similar profiles for the same JD family.
inputs:
  - current_candidate_score
  - similar_candidates
  - jd_family
outputs:
  - calibration
  - flags
  - next_action
---

# Skill: Calibrate Against Pipeline

Sanity-check the current score against the team's historical
calls on similar profiles.

1. **Pull the 5 most similar candidates.** From
   `pipeline_history` for the same `jd_family` (e.g.
   `backend_senior`, `frontend_mid`). "Similar" = same seniority
   call + overlapping top skill. If history has < 3 similar
   candidates, flag `not_enough_history` and return — calibration
   is not yet load-bearing for this JD family.
2. **Compare current score to historical mode.** If 4+ of the 5
   similar candidates scored 5 and were advanced, but the current
   candidate is at 1, that is **inconsistent**. The reverse is
   also inconsistent.
3. **Three buckets:**
   - `consistent` — current score is within ±1 of the
     historical mode AND the team's outcome (advanced /
     rejected) lines up with the recommendation.
   - `inconsistent` — current score is ≥ 2 steps off the
     historical mode, OR the recommended outcome contradicts
     how the team consistently resolved similar candidates.
     Flag `calibration_mismatch` and escalate.
   - `not_enough_history` — fewer than 3 similar candidates on
     file. Return without a calibration call; do not block.
4. **Cite the similar candidates by id**, not by name. The
   audit trail must point at history rows, not personal info.

Hard rules:

- **Calibration never overrides the score.** If you find an
  inconsistency, the answer is **escalate** — not silently
  re-score. The operator decides whether the historical pattern
  or the current resume is the right call.
- **No protected-class signals in similarity.** "Similar"
  means: same seniority, overlapping top skill, similar
  scope-of-ownership. Never: same school, same region, same
  graduation cohort.
- **`next_action` is informational** when consistent, and
  `escalate_calibration_mismatch` (operator-routed, no external
  effect) when inconsistent.
