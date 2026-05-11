---
name: draft_outreach
description: Given a candidate context + JD, produce a personalized outreach email (90-second read) that names the strongest evidence for the role, proposes a 30-minute screening call, and gates the send behind operator approval.
inputs:
  - candidate_context
  - jd
  - operator_signature
outputs:
  - outreach_draft
  - next_action
  - flags
---

# Skill: Draft Outreach

Produce a personalized first-touch outreach email for a candidate
whose fit score is 5 (clear-advance). Score-3 (borderline) and
score-1 (reject-needed) candidates do not get outreach — they get
escalated to the operator instead.

1. **Open with the strongest evidence.** Name the JD requirement
   and the exact resume line that maps to it. "Your work on the
   schema migration described in your resume looks closest to
   what this role owns" — not "great background!"
2. **Restate the role in one line.** Title + the 1-2 things the
   role owns. Pull both from the JD; do not invent scope.
3. **Propose one concrete next step.** A 30-minute phone screen.
   Use `find_screening_slot` to produce 3 candidate windows and
   reference them by date + time + timezone — not "let me know
   when works".
4. **Sign off.** Use `operator_signature`; never invent a name
   or a recruiter title.
5. **Read time ≤ 90 seconds.** ~150 words. Anything longer is a
   re-draft.

Hard rules:

- **No protected-class signals.** Never reference age, gender,
  school prestige, country of origin, etc. The candidate's
  *evidence for the role* is the only thing in the open.
- **No commitments not in the JD.** Salary band, equity,
  remote-anywhere, fast-track promotions — none of these belong
  in first-touch outreach unless the JD states them.
- **No false warmth.** "Loved your profile" and "your background
  is incredible" are placeholder generators — replace with one
  specific evidence line or do not send.
- **Output is `outreach_draft` only.** The `next_action` is
  always `send_outreach_email` with `requires_approval: true`.
- **If candidate_context lacks a name, contact channel, or
  resume body** — set `flags: ["missing_candidate_fields"]`,
  stop drafting, and escalate to the operator.
