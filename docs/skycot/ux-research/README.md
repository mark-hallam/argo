# SKYCOT UX research

The product-side companion to `docs/skycot/MISSION.md`. Where MISSION
states *what we believe*, this directory states *who we believe it
for* and *what their day looks like.*

Every design decision in the frontend program (the future Track C —
five-surface SPA) should trace back to one of the personas or
journey maps here. If a proposed feature doesn't make any of these
personas' day measurably better, push back.

## Personas

The platform has six distinct human roles. All are now written in
full as of 2026-05-11; tags below indicate which are primary (used
for every design decision) vs which are secondary (used for
specific surfaces).

Primary — read these before any frontend design decision:

- [delegator.md](delegator.md) — the SMB owner / dept head who
  "hires" AI teammates. Primary daily user. Primary buyer in SMB.
- [approver.md](approver.md) — usually the delegator on mobile,
  mid-meeting, with 10 seconds to decide. The single highest-
  frequency touchpoint.
- [operator.md](operator.md) — the technical admin (RBAC, billing,
  key rotation, compliance evidence). The B2B buyer in mid-market+.

Secondary — read these when designing the surface they own:

- [end-user.md](end-user.md) — the external counterparty (candidate,
  buyer, customer, patient). Doesn't use SKYCOT directly; receives
  its outputs. The reputation we are defending. Disclosure policy
  + voice consistency lives here.
- [auditor.md](auditor.md) — internal compliance + external SOC2 /
  GDPR / HIPAA audit. The persona that determines whether SKYCOT
  is deployable in regulated industries.
- [worker.md](worker.md) — the AI worker itself as a "user" of
  internal APIs (memory, skills, escalation chain). Not human, but
  its needs constrain every human-facing design choice.

## Journey maps

Five core flows. All five are now written in full as of 2026-05-11.

Daily-touchpoint journeys (highest leverage):

- [journey-hire-a-teammate.md](journey-hire-a-teammate.md) — the
  conversational onboarding flow. The first 10 minutes of the
  product. Closes mission claim #3.
- [journey-approve-on-mobile.md](journey-approve-on-mobile.md) —
  the 10-second decision on a phone, between meetings. Closes the
  daily UX for mission claim #6.
- [journey-delegate-ongoing-work.md](journey-delegate-ongoing-work.md)
  — the "give the worker a new responsibility" flow after
  onboarding. Closes recurring delegation for mission claim #2.

Lower-frequency but stakes-heavy journeys:

- [journey-audit-replay.md](journey-audit-replay.md) — the "show
  me what happened" forensic flow. Compliance, customer-
  complaint investigation, retrain-prep. Backbone of the
  auditor persona.
- [journey-retrain-a-worker.md](journey-retrain-a-worker.md) — the
  "she keeps doing X wrong, fix it" loop. The journey that makes
  the worker feel trainable, not replaceable.

## How to use this directory

- **Designing a new surface?** Read the personas it serves before
  opening Figma.
- **Reviewing a PR that touches user-facing copy or flow?** Check
  the relevant journey map for the moment that copy lives inside.
- **Disagree with something here?** Edit it. These are living
  documents, not signed-off artifacts. Note the change in the PR
  body so the design conversation has a paper trail.

## What this directory is NOT

- Not a marketing brief. We are not selling to ourselves; we are
  describing reality.
- Not a roadmap. The roadmap lives in `docs/build-plan/`.
- Not a brand or design-system spec. That comes after this — see
  the upcoming Track B (design language) deliverable.
- Not fiction. If a persona claim isn't grounded in a real user
  conversation, mark it `(assumed — needs validation)`. We will
  pressure-test these against real users before shipping.
