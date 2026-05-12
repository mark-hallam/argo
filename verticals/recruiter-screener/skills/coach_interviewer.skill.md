---
name: coach_interviewer
description: Given an interview type (phone-screen / technical / behavioral / panel) plus a candidate brief, produce an internal interviewer-prep doc — 5-bullet "what to probe", 3-bullet "common pitfalls" (calibration risks), and a 3-question rubric. Internal-prep; never sent to the candidate.
inputs:
  - interview_type
  - candidate_brief
  - jd
  - rubric
outputs:
  - what_to_probe
  - common_pitfalls
  - question_rubric
  - flags
---

# Skill: Coach Interviewer

Produce a tight prep doc for the interviewer before they meet
the candidate. The goal is to reduce calibration drift — what
to listen for, what to discount, and how to score consistently.

1. **Dispatch on `interview_type`.** Use exactly one of the
   four templates below. If the value is anything else, set
   `flags: ["unknown_interview_type"]` and stop.
   - `phone-screen` — probe motivation, role-shape fit,
     compensation-band alignment, hard-requirement
     confirmation, scheduling logistics.
   - `technical` — probe a single core competency in depth
     (system design / coding / debugging per the JD anchor),
     reasoning under uncertainty, and one cross-cutting
     concern (testing / observability / security).
   - `behavioral` — probe one cited resume project end-to-end,
     conflict-resolution, decision-making under ambiguity,
     ownership-vs-influence, and feedback-handling.
   - `panel` — probe consistency across already-covered
     rubric dimensions, surface any single panelist's
     dissenting read, and reserve airtime for the candidate
     to ask questions.
2. **"What to probe" — five bullets.** Each bullet names one
   specific JD requirement + one open-ended question that
   surfaces evidence for or against it. Cite the JD line.
3. **"Common pitfalls" — three bullets.** This is the
   highest-leverage section. Lead with: (1) anchoring on
   first impression in the opening 5 minutes — re-score after
   the interview, not during; (2) confirmation bias once a
   provisional read forms — every "yes" needs a probe that
   could have produced a "no"; (3) similarity-to-self bias —
   shared school, hobby, or background is not signal.
4. **"Question rubric" — three questions, each scored 1-3-5.**
   Each question maps to one rubric dimension. The scoring
   anchors describe what a 1, 3, and 5 sounds like in the
   candidate's own words — not the interviewer's adjective.

Hard rules:

- **Evidence is rubric-dimension performance only.** Never
  demographic proxies (school, age, region, accent, name).
  Never personal-fit-feel ("seemed sharp", "good vibe",
  "would grab a beer with them"). The pitfalls section names
  these by category; the question rubric anchors must be
  artifact-based.
- **Pitfalls are non-negotiable.** The 3 calibration-risk
  bullets ship on every output, even if the interviewer is
  experienced. Interviewer-anchoring + confirmation bias drive
  ~50% of bad hires; repetition is the point.
- **No leading questions in the question rubric.** "Tell me
  about a time you led a migration" is a probe; "You led
  the X migration, right?" is a leading question and
  invalidates the answer.
- **Output is interviewer-facing only.** Never echo the
  prep doc to the candidate; `next_action` is always
  `internal_review` with no external effect.
- **If `candidate_brief` is missing the resume or the
  cited JD** — set `flags: ["missing_prep_inputs"]`, stop,
  and escalate to the operator. Coaching without inputs is
  worse than no coaching.
