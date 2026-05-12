---
name: follow_up_lead
version: "0.1.0"
description: "Drafts a templated follow-up message for a lead based on their state (browsing / qualified / under-contract / lost) and time since last contact. Output Markdown. Autonomous per the charter — but the skill never auto-sends; the runner enforces the send gate."
provenance:
  source: vertical/real-estate-agent
  hash: sha256-pending
---

# When to use

Call when the operator (or a scheduled cron) signals a lead
needs a follow-up — 24h or 72h post-viewing, or N days into
a cold-lead-revive cycle. Output is the draft; the runner
sends via the `follow_up_lead` action (auto-approved because
its body is template-only).

# Inputs you expect

- `lead_id`, `lead_name`, `lead_email`
- `lead_state`: `browsing` / `qualified` / `under_contract` / `lost`
- `last_contact_at_iso` (ISO date)
- `template_id`: `post_viewing_24h` / `post_viewing_72h` /
  `cold_lead_revive` (must match a section in
  `inventory/follow_up_templates.md`)
- `listing_id` if relevant; `agent_name` for sign-off

# Output shape

A Markdown body — no JSON metadata; the runner has
`template_id` already and this body is what gets sent:

````markdown
**Subject:** <one short line>

Hi <lead_name>,

<2–4 sentences from the named template, with merge fields
substituted exactly. Never improvise.>

— Jamie, on behalf of <agent_name>
````

# Rules

- **Template-only body.** The body MUST come from a named
  section in `inventory/follow_up_templates.md`; substitute
  `lead_name`, `listing_address`, `agent_name` exactly. No
  improvisation — the auto-approval safety case rests on
  bodies being template-only.
- **State-to-template map** (default):
  - `browsing` + cold → `cold_lead_revive`
  - `qualified` + 24h post-viewing → `post_viewing_24h`
  - `qualified` + 72h post-viewing → `post_viewing_72h`
  - `under_contract` → route to operator; don't auto-send
  - `lost` → don't follow-up; mark `last_touch_at` and stop
- **Never auto-send.** This skill produces a draft only.
- **Sign as Jamie, on behalf of `<agent_name>`.**
- **No price quotes, no inspection content.** Those route
  through `send_listing_packet` / `flag_inspection_finding`.
- **Stop on missing template.** If `template_id` doesn't
  match a section, return an empty body and ask the operator.

# Approval posture

Autonomous. `requires_approval = false` per `vertical.toml`.
The auto-approval safety case rests on bodies being
template-only; custom messages route through `send_listing_packet`.

# Examples

```markdown
**Subject:** Following up on 118 Cedar Lane

Hi Sam,

Thanks for viewing 118 Cedar Lane on Thursday. Want to see
anything else from the shortlist this weekend?

— Jamie, on behalf of Alex Agent
```

`under_contract` / `lost` leads return an empty body plus
`"OPERATOR: state=<state> — skipping auto-follow-up."`.
