---
name: compare_candidate_pair
description: Given two candidates being weighed for the same role, produce a structured side-by-side comparison — per-rubric scores, evidence per delta, best-fit strengths, weakness call-outs, and one of five recommended next steps. Internal-report; never sent externally.
inputs:
  - candidate_a
  - candidate_b
  - jd
  - rubric
outputs:
  - rubric_table
  - evidence_per_delta
  - best_fit_strengths
  - weakness_callouts
  - recommended_next_step
  - flags
---

# Skill: Compare Candidate Pair

Produce a head-to-head, evidence-cited comparison of two
candidates being weighed for the same opening. The output is an
internal decision-aid — never echoed to either candidate.

1. **Score each candidate on every rubric dimension.** Pull the
   dimensions from `rubric` verbatim — never invent new axes.
   Each cell = the 1-3-5 score plus one JD line + one resume
   line citation. If either candidate is missing evidence for a
   dimension, mark the cell `insufficient_evidence` and do not
   guess.
2. **Compute deltas only where evidence supports them.** For
   every dimension where A and B differ by ≥ 1, write a single
   sentence naming the specific evidence behind the gap (e.g.
   "A shipped a migration over 2B rows; B's largest cited
   migration is 50M rows"). Deltas without citations are not
   deltas.
3. **Best-fit strengths.** One bullet each: the strongest
   case for advancing A, and the strongest case for advancing
   B. Each bullet names a JD requirement + the evidence line
   that matches it. No "well-rounded" or "strong overall" — name
   the dimension.
4. **Weakness call-outs.** One bullet each: the most
   role-relevant gap for A, and for B. Frame as missing
   evidence against a specific JD requirement, not as a
   personal trait.
5. **Recommended next step.** Pick exactly one of the 5-action
   vocab:
   - `advance_both` — both clear the role's hard requirements
     with ≥ 2 evidence pieces each; no disqualifying gap.
   - `advance_a_only` — A clears hard requirements; B has
     ≥ 1 disqualifying gap or score-1 dimension.
   - `advance_b_only` — symmetric to the above.
   - `hold` — neither clears decisively AND new evidence is
     plausibly retrievable (pending references, missing work
     samples). Set `flags: ["awaiting_evidence"]`.
   - `reject_both` — neither clears hard requirements AND no
     retrievable evidence would change the call.
   Every action except `hold` routes through
   `advance_to_phone_screen` / `reject_candidate` with
   `requires_approval: true`.

Hard rules:

- **Evidence is rubric-dimension performance only.** Never
  demographic proxies (school prestige, age, region, name,
  graduation cohort). Never personal-fit-feel
  ("seemed more enthusiastic", "stronger communicator"
  without a cited artifact).
- **No protected-class signals as tiebreakers.** If A and B
  are within ±1 across every dimension, the answer is
  `hold` and `flags: ["near_tie_escalate"]` — never a
  preference inferred from non-rubric signal.
- **Cite by candidate id, not name, in the audit trail.**
  The report body may use names for readability; the
  `evidence_per_delta` machine-readable field uses ids.
- **Disability or accommodation disclosure on either side →
  stop.** Set `flags: ["disability_disclosure"]`, do not
  compare, escalate to operator.
- **No re-scoring of `score_resume` outputs.** If a cited
  per-dimension score disagrees with your read, flag
  `score_disagreement` and escalate — never silently
  overwrite the upstream score.
