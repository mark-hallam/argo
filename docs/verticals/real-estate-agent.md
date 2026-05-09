# Vertical: real-estate-agent

V2.3 demo vertical (Stream `claude-vertical-real-estate-agent`).
Eighth demo vertical — see also `customer-support`, `sales-sdr`,
`marketing-intern`, `marketplace-seller`, `gtm-engineer`,
`pr-checker`, and `finance-ar`.

## Persona
Jamie — executive assistant for a licensed real-estate agent.
Enthusiastic, professional, and meticulous about staying inside the
bounds of what an unlicensed assistant can say. The licensed agent
("Pat the Agent" in the demo data) is the one signing offers and
giving binding price guidance; Jamie schedules, drafts, follows up,
and flags.

## Action surface
- `schedule_viewing` — books a private gcal event on the agent's
  diary. Autonomous-safe.
- `send_listing_packet` — emails a curated listing packet to a
  qualified lead via gmail. Approval-gated; operator approves the
  body.
- `draft_offer` — renders a markdown offer letter via the
  `document_creation` executor. Approval-gated; the agent reviews
  price + contingencies before signing the official document outside
  this system.
- `flag_inspection_finding` — hands a material inspection finding
  to the agent for re-negotiation judgement. Approval-gated;
  routing-only.
- `follow_up_lead` — sends a templated post-viewing follow-up
  using a fixed template from `inventory/follow_up_templates.md`.
  Auto-approval safe because the body is template-only.

The 5 actions exercise 3 different `NotifyChannel` shapes: gmail
(email), document_creation, and human handoff.

## Memory schema
- `leads` — qualification state per lead.
- `property_interests` — lead-to-listing interest map.
- `viewings` — booked viewings + gcal event ids.
- `offers` — draft offer state per buyer + listing.
- `follow_ups` — templated follow-up sends.
- `inspection_flags` — material findings handed to the agent.
- `real_estate_learnings` — reusable patterns the operator confirms.

## Fair Housing
The system prompt (`prompts/system.md`) carries a load-bearing Fair
Housing rule: Jamie refuses to steer leads by protected class, refuses
to filter listings on protected-class signals, and escalates to the
operator when a lead asks for steering. The `re_fair_housing_refusal`
eval scenario locks the refusal behaviour at run time.

## Demo
```sh
./scripts/demo_real_estate.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and prints the operator's next-step
checklist. No external API keys required for the doctor scan.

## Approval gates
| Action                    | Gate                  | Why                                |
|---------------------------|-----------------------|------------------------------------|
| `schedule_viewing`        | autonomous            | agent's own calendar               |
| `follow_up_lead`          | autonomous            | template-only body                 |
| `send_listing_packet`     | gmail.send + approval | buyer-facing copy                  |
| `draft_offer`             | document_creation + approval | binding-adjacent surface  |
| `flag_inspection_finding` | human + approval      | licensed-judgment territory        |

`auto_approval.toml` cap is `$2/day` total + `$0.05/action` —
deliberately tighter than finance-ar because real-estate buyer
contact has a higher reputation cost per misstep.

## Bootstraps via Genesis
The closest existing archetype is `CalendarCoordinator` — the
`charter_keywords` for that archetype include `schedule`, `meeting`,
`book`, `coordinate`, and `availability`, all of which match the RE
agent's clerical surface. A future `RealEstateAgent` archetype could
be added to `argo-genesis` once the demo has soaked; for now,
operators bootstrap via:

```sh
argo genesis classify --charter "schedule and coordinate property viewings for a licensed real-estate agent"
# → CalendarCoordinator
```

…and rename the resulting vertical to `real-estate-agent` after the
manifest is generated.

## Sensible deviations from brief
The brief proposed a few paths and a separate `actions/` folder
that don't match the existing vertical convention. Diverged to:
- Inline action declarations in `vertical.toml` (matches every
  shipped vertical incl. `finance-ar` #937; the loader does not
  read separate action files).
- `prompts/persona.md` + `prompts/system.md` (the `[role]`
  validator expects markdown for `system_prompt`).
- `memory/schema.toml`, `eval/seed.jsonl`, `auto_approval.toml`
  at the vertical root, `scripts/demo_real_estate.sh` (no `bin/`
  dir exists in the repo).

## Deferred follow-up work (out of scope)
- A `RealEstateAgent` archetype in `argo-genesis` so
  `genesis classify` bootstraps natively.
- A real MLS connector (demo uses synthetic listings only).
