---
name: draft_post_mortem_template
version: "0.1.0"
description: "Drafts a blame-aware post-mortem skeleton for an incident — timeline, impact, root cause, contributing factors, action items — sections only, not the writeup."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

After an incident, outage, missed deadline, or other "something
went wrong" event, the operator wants a structured skeleton to
fill in for the retrospective. Use any time the request is
"start a post-mortem for <incident>."

# Inputs

- `incident_summary`: 1-2 sentence framing — what happened, when.
- `severity`: `"sev1"`, `"sev2"`, `"sev3"`, `"sev4"`, or domain
  equivalent (`"customer-visible"`, `"internal-only"`).
- `audience`: `"internal-team"`, `"customer-facing"`,
  `"regulatory"`. Drives tone and depth of disclosure.

# Output

A markdown skeleton with these sections, in order:
- **TL;DR** (placeholder; 2-3 sentences when filled).
- **Impact** (who/what was affected, for how long, magnitude).
- **Timeline** (UTC timestamps, what happened, who acted).
- **Root cause** (the technical/process cause; not "who").
- **Contributing factors** (the conditions that allowed the
  root cause to surface).
- **What went well** (the responses that worked).
- **What didn't** (gaps in detection, response, communication).
- **Action items** (each: owner, deadline, severity-prevented).

Each section has 1-2 sentence guidance text under the header,
telling the operator what to fill in. No drafts of CONTENT — the
operator fills in facts; the skill provides the shape.

# Examples

- Sev1 outage, audience internal → full template, terse guidance.
- Sev3 missed deadline, audience customer-facing → same template
  + a "customer-facing tone" reminder under TL;DR.

# Approval posture

Drafts the template structure only. Skill never invents facts,
attributes blame, or fills in timestamps. Operator writes the
content from authoritative sources (incident logs, chat history).

# Notes

- Blameless framing throughout: "the process didn't catch this"
  is acceptable; "X failed to notice" is not. Guidance text in
  each section reinforces this — leave the framing visible.
- Action items must have owner + deadline + the specific failure
  mode they prevent. Vague action items ("improve monitoring")
  are the most common post-mortem failure mode.
- For `audience: "regulatory"`, add a guidance note under Root
  cause about reportable findings — but never name a specific
  regulator (tenant-dependent).
