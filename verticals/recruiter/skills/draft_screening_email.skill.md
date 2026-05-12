---
name: draft_screening_email
version: "0.1.0"
description: Draft a candidate-facing screening email matching the operator's brand voice across three stages (initial outreach, follow-up, scheduling). Always approval-gated before send.
provenance:
  source: vertical/recruiter
  hash: sha256-pending
---

# When to use

Use after `score_resume` returns `advance` AND the operator has
approved the advance, OR when re-engaging a candidate who has gone
quiet, OR when scheduling a confirmed slot. Produces a draft only;
the send is always approval-gated.

# Inputs you expect

- `candidate` — `{name, contact_channel, last_touched_at, current_stage}`.
- `screening_questions` — operator-supplied list of 2-5 questions for
  the initial-outreach stage; empty for follow-up / scheduling stages.
- `stage` — one of `initial-outreach`, `follow-up`, `scheduling`.
- `operator_signature` — `{name, title, team}`. Never invented.
- `proposed_slots` — required when `stage = scheduling`; output of
  `schedule_interview`.

# Output shape

```json
{
  "stage": "initial-outreach",
  "subject": "string",
  "body": "string (≤ 150 words for initial; ≤ 100 for follow-up/scheduling)",
  "next_action": {
    "name": "send_screening_email",
    "requires_approval": true
  },
  "flags": []
}
```

# Stage templates

The three stages are explicit; do not invent a fourth.

**`initial-outreach`** — first touch. Opens with one specific
evidence line from the resume (cite by content, not "great
background"). Restates role in one line. Lists the 2-5 screening
questions verbatim from `screening_questions`. Proposes a 30-minute
call. Signs off with `operator_signature`.

**`follow-up`** — re-engagement. References the prior touch's date
(`candidate.last_touched_at`) and the role. One sentence asking
whether the candidate is still interested. No new questions, no new
commitments.

**`scheduling`** — slot confirmation. Lists the 3 `proposed_slots`
with `{date, time, candidate_tz, interviewer_tz}` on a single line
each. Asks the candidate to pick one. No marketing copy.

# Hard rules

- **No protected-class signals.** Never reference age, gender,
  school, country of origin, family status, etc.
- **No commitments not pre-approved.** Salary band, equity,
  remote-anywhere, fast-track promotion — these belong in
  `flag_offer_submission`, not screening email.
- **No false warmth.** "Loved your profile" and "your background is
  incredible" are placeholder phrases — replace with one specific
  resume-line citation or do not send.
- **Forbidden phrasing.** "Culture fit", "Rockstar", "Ninja",
  "Overqualified", "Per your request" — drop from drafts.
- **Approval-gated always.** `next_action.requires_approval: true`
  on every stage, every send.
- **Synthetic placeholders in examples.** Use `candidate@example.com`
  and synthetic names.
- **Missing fields → stop.** If `candidate.name`, `contact_channel`,
  or (for scheduling) `proposed_slots` are missing, set `flags:
  ["missing_candidate_fields"]` and do not draft.

# Examples

Initial-outreach (sketch):

```
subject: "Skycot — backend role — quick screen?"
body: "Hi Alex, I'm Morgan at Skycot Hiring. Your ingestion
pipeline rewrite (resume line: 'owned Postgres → Kafka migration')
maps closely to what this role owns... [2 screening questions
verbatim] ... 30-minute call in the next two weeks? — Morgan,
Technical Recruiter, Skycot Hiring"
```
