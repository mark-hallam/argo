---
name: compose_status_update
version: "0.1.0"
description: "Composes a structured status update (done / doing / next / blocked) from raw input — for standups, weekly reports, or async check-ins."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator dumps a brain-stew of recent activity and wants it
shaped into a status update for a team, manager, or stakeholder.
Use whenever the deliverable is "what should I post in the
#updates channel / weekly email / standup."

# Inputs

- `raw_notes`: the operator's unstructured input (bullets, prose,
  scattered facts).
- `audience`: who reads this — `"engineering team"`, `"executive"`,
  `"cross-functional partners"`, `"customer"`.
- `period`: time window — `"this week"`, `"yesterday"`, `"the
  sprint"`.

# Output

Four sections, in order: **Done** (shipped, finished), **In
flight** (active work), **Next** (the immediately-upcoming step),
**Blocked** (anything waiting on someone). Each section is a
bulleted list. Omit a section if empty — don't pad.

# Examples

- Raw notes about a feature ship + a stuck migration, audience
  exec → tight 4-section update; "blocked" names the person whose
  decision is needed, not just "waiting on review."
- Same notes, audience engineering → same shape but with more
  technical specificity (PR numbers, error names).

# Approval posture

Drafts only. Operator posts. Skill does not auto-send to chat,
email, or calendar.

# Notes

- "Blocked" must name a person + the unblocking action, or it's
  not blocked, it's just slow.
- Re-write each bullet to lead with a verb, past-tense for Done,
  present for In flight, future for Next.
- Audience `"executive"` → drop tooling names and PR numbers,
  keep outcomes and risks. Audience `"engineering team"` → keep
  them.
- If `raw_notes` contains no real progress, return an empty
  Done section and flag `note: "no shipped work this period"`
  rather than inventing items.
