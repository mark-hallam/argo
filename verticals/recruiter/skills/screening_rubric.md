# Skill: screening rubric — how Morgan scores resumes

The **only authority** for scoring an inbound resume. Every candidate
is scored against the same rubric with the same weights.

## When to apply

Apply on every `screen_candidate`. Output: per-criterion score
(0-3), one-line justification per criterion, overall recommendation
(`advance` / `hold` / `pass`), confidence band (`low` / `medium` /
`high`).

## Forbidden signals — DO NOT SCORE ON

Resumes will surface these. The rubric scores on rubric criteria
only. Do **not** discount, boost, or even mention the signal in the
rubric card.

- **Age** — DOB, graduation year, length-of-career as a proxy.
- **School prestige** — Ivy / Stanford / FAANG-school weighting.
  GPA. School name as positive or negative.
- **Gender / gender presentation** — pronouns, name as gender proxy,
  photo, references to "fit with the team" by gender.
- **Race / ethnicity / national origin** — name as ethnicity proxy,
  photo, references to "communication style".
- **Marital / family status / pregnancy** — references to family,
  childcare availability, travel as a family-status proxy.
- **Religion / political affiliation** — affiliations on the resume.
- **Disability / health status** — accommodations, medical gaps.
- **Sexual orientation** — affiliations, partner references.
- **Criminal history** — out of scope; refuse and escalate.
- **Salary history** — illegal in many jurisdictions as offer input.
- **Visa status** — only "authorized to work: Y/N" is allowed.

**`Culture fit` is forbidden phrasing** — a legally-fraught EEOC
proxy. Score on rubric criteria only.

## Scoring criteria — weighted (sum = 100)

Each criterion 0-3 (`0 absent`, `1 developing`, `2 proficient`,
`3 strong`).

1. **Domain skill match** (30) — demonstrated work with the role's
   primary stack. Shipped systems, OSS, depth of role descriptions.
2. **Seniority signal** (20) — scope of past projects, ownership,
   leading a workstream end-to-end. Years of experience is a *weak*
   proxy — prefer scope.
3. **Cross-functional collaboration** (15) — working across product,
   design, ops, security. Stated impact in shared-context language.
4. **Communication signal** (15) — clarity of resume self-description,
   public talks, writing, mentorship. Exclude any forbidden signal.
5. **Recency / activity** (10) — shipping in the last 18 months. Gaps
   with no portfolio activity feed `flag_red_flag`, not a low score.
6. **Role-specific bonus** (10) — defined per requisition; default 0.

## Recommendations

- **Advance** — score ≥ 70, no red flags. Move to send_screening_questions.
- **Hold** — score 50-69 OR strong primary signal with weak secondary.
  Recruiter reads the card and decides.
- **Pass** — score < 50 with no exceptional case. Recruiter still
  owns the no — the worker NEVER drafts rejection comms.

## Red-flag patterns — STOP and route to recruiter

Routing-only — emit `flag_red_flag` instead of continuing the rubric
flow. The recruiter decides next steps.

- **Employment gap** (severity = medium) — multi-year unexplained
  gap with no portfolio / public commits / community signal. Do
  **not** assume a reason; route to recruiter.
- **Fabrication signal** (severity = high) — date conflicts between
  LinkedIn / resume / portfolio. Stated employer that doesn't
  publicly exist.
- **Inconsistency** (severity = medium) — claimed seniority doesn't
  match described scope; skills listed but absent in any project.
- **Portfolio plagiarism** (severity = high) — wholesale public-template
  copy as personal work; tutorial code presented as production.

The summary in the action payload **never references a forbidden
signal**, even to deny one. Stick to: `<flag_type> at <source>;
recruiter decides`.

## What NOT to do

- Do not "intuit" cultural compatibility. The rubric does not ask you
  to. **`Culture fit`** is forbidden.
- Do not flag a resume as red because it is "different" from the
  funnel. Difference is not a flag.
- Do not infer protected-class signals from names, photos, or
  geographies and adjust the rubric.
- Do not draft rejection comms. Pass is a recruiter call.
- Do not extend an offer in any candidate-facing draft. `submit_offer`
  is always recruiter-delivered.
