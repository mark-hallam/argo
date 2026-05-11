---
name: score_resume
description: Given a JD + a resume, produce a 1-3-5 fit score, 3 specific match reasons (JD line + resume line), 3 specific gap notes, and a seniority call against the JD's anchors. Scoring is autonomous-safe — no external effect.
inputs:
  - jd
  - resume
  - seniority_anchors
outputs:
  - fit_score
  - match_reasons
  - gap_notes
  - seniority_call
  - flags
---

# Skill: Score Resume

Produce a 1-3-5 fit score for one resume against one JD.

1. **Read the JD's hard requirements.** Extract: must-have skills,
   minimum years-of-experience, role-shape (IC vs lead), and any
   jurisdiction requirement. If a hard requirement is ambiguous,
   flag `ambiguous_jd_requirement` and stop.
2. **Three match reasons, three gap notes.** Each reason cites
   one JD line + one resume line. Each gap names one specific
   missing piece. If you cannot produce three of either, the
   score is wrong — re-score lower or flag `insufficient_evidence`.
3. **Seniority call.** Place the candidate at junior / mid /
   senior / staff+ using `seniority_anchors`, not years alone.
   Insufficient ownership evidence at the JD's seniority is a gap
   note, not a re-classification of the JD.
4. **1-3-5 mapping.**
   - 5 → meets every hard requirement + ≥ 2 evidence pieces for
     the top skill + at least one seniority-anchor signal.
   - 3 → meets hard requirements but evidence thin, OR strong
     adjacent skill but one must-have gap. Flag `borderline` and
     set `next_action.requires_approval = true`.
   - 1 → misses ≥ 2 hard requirements OR has a disqualifying
     gap. Flag `reject_recommended` and gate
     `reject_candidate` behind approval — never auto-reject.

Hard rules:

- **No protected-class signals.** Age, gender, race, school
  prestige, etc. never appear as match reasons, gap notes, or
  inputs to the score. If the resume volunteers any, ignore them
  in the score and never echo them in the output.
- **No inferred attributes.** If you cannot cite a JD line + a
  resume line for a match, it is not a match reason.
- **Borderline always escalates.** A score of 3 sets
  `flags: ["borderline"]` and routes the next step through
  `advance_to_phone_screen` with `requires_approval: true`.
- **Disability disclosure → stop.** If the resume volunteers a
  disability or accommodation request, do not touch the score —
  set `flags: ["disability_disclosure"]` and escalate.
