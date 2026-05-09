# Screening question bank

The `send_screening_questions` action pulls its body from this file.
Each question set is a fixed list of 5 questions; the worker picks
the set that matches the requisition's question_set tag and slots
the candidate's first name into the greeting. The recruiter approves
the rendered body before send.

## Greeting template

> Hi {{ first_name }},
>
> Thanks for taking the time to apply for the {{ role_title }} role.
> Below are five questions we ask every candidate at this loop stage —
> they help us tailor the rest of the loop to where you are. There
> are no trick questions and no "right" answers; concrete examples
> beat polished prose.
>
> Reply when convenient — typically within a week. Your recruiter
> will follow up after we've read your responses.

## backend_screen — `JOB-BE-*`

1. Walk me through the most production-impactful backend system
   you've shipped in the last 18 months. Scope, your role, the
   reliability story.
2. Describe a time you debugged a non-obvious production issue —
   the symptom, your hypothesis, what was actually wrong.
3. What's a backend abstraction you used to like and now actively
   avoid? What changed?
4. How do you decide between adding to a service vs. introducing
   a new one? Concrete recent example.
5. What about this role looks most interesting to you, given your
   trajectory?

## frontend_screen — `JOB-FE-*`

1. Walk me through the largest frontend system you've owned. State
   model, render strategy, the part that broke under scale.
2. What's a frontend pattern you used to reach for and now avoid?
   What replaced it?
3. Describe a non-trivial accessibility issue you fixed end-to-end.
   How did you find it, what was the underlying cause?
4. How do you split work between the design system and feature
   surfaces? Concrete recent decision.
5. What about this role looks most interesting to you, given your
   trajectory?

## infra_screen — `JOB-INF-*`

1. Walk me through an incident you led the response on. Detection,
   triage, the recovery, what you changed afterwards.
2. What's an infra abstraction you used to defend and now actively
   migrate off of? What's the replacement?
3. How do you decide where to draw the line between dev-ops-owned
   tooling and platform-owned tooling? Concrete recent example.
4. Describe a security finding you owned end-to-end. The shape of
   the issue, your remediation, the verification.
5. What about this role looks most interesting to you, given your
   trajectory?

## What never appears in screening sends

- Compensation specifics (level, base, equity ranges).
- Names of interviewers on the panel.
- Rubric scores or any rubric-card content.
- Another candidate's status, name, or feedback.
- Anything from the forbidden-signals list in `skills/screening_rubric.md`.
