# pitch-agent-worker-template

**Owner**: claude.
**Authored against**: origin/main `f500fc3da` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-agent-worker-template`.
**Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) MERGED + `pitch-voice-io-provider-agnostic-crate` (P-03) MERGED + `pitch-canonical-qa-skill` (P-04) MERGED.

**Refresh decision (2026-05-25)**: Option A. Use the current pack-loader contract: `packs/pitch-agent/pack.toml` loaded with `argo_vertical_packs::PackManifest::load(root)`, plus root-level `worker.toml` and pack-local content files. This is the smallest refresh because it consumes the public `argo-vertical-packs` API already on `origin/main` and avoids adding a second vertical metadata format.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-05 of 8**

The Pitch Agent worker template is a general-purpose vertical template. Skycot is the first instance, but the template must let any tenant deploy a public-representation agent with its own truth boundary, canonical Q&A, and voice stack.

This brief assembles the Track P parts authored by P-01 through P-04 into a deployable pack: persona modes, skills bundle, memory schema, truth-boundary policy reference, voice manifest reference, tool permissions, workflows, and the cohort metadata that makes it discoverable in the UI.

## Why this brief exists

Platform invariant: vertical-specific behavior belongs in manifests, workflow definitions, prompt packs, memory schemas, tool permissions, escalation policies, evaluation cases, and UI/gateway metadata, not in runtime forks. A worker is the deployable form of those ingredients for one role.

Operator-locked scope (memory `project_pitch_agent_v1_demo.md`): "General template, Skycot is the first instance. Any tenant deploys their own Pitch Agent with their own truth boundary, canonical Q&A, and voice stack." This brief makes that concrete.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — what the agent must be able to do
- **Memory**: `project_pitch_agent_v1_demo.md`; `project_worker_detail_design_locked.md` (LinkedIn-profile UI style for worker detail pages, C2 personality register); `project_pm_cohorts_peta_patrick.md` (precedent for cohort registration)
- **Pack format**: `crates/argo-vertical-packs/src/manifest.rs` — `PackManifest::load(root)` parses root-level `pack.toml`
- **Cohort metadata**: per memory `project_cohorts_page_design_locked`, the cohorts page has 7 dept tabs; this brief adds the 8th: Sales & Investor Relations

## Scope

### 1. Pack directory

Author the in-repo pack skeleton:

```text
packs/pitch-agent/
├── pack.toml
├── worker.toml
├── personas/
│   ├── elevator-30s.md
│   ├── investor-3min.md
│   ├── demo-10min.md
│   ├── diligence-deep.md
│   └── voice-and-tone.md
├── memory/
│   └── schema.json
├── workflows/
│   ├── rehearsal.toml
│   ├── web-embed.toml
│   └── live-meeting.toml
├── policies/
│   └── truth-boundary-ref.toml
├── skills/
│   └── bundle.toml
├── tools/
│   └── permissions.toml
├── escalations/
│   └── routes.toml
├── evals/
│   └── cases-ref.toml
└── ui/
    └── cohort.toml
```

The current pack schema does not define a mode discriminator for persona packs. Flatten the four pitch modes into four sibling persona files under `personas/`; the new `pitch-mode-switcher` skill selects the active file by context tag.

### 2. Pack descriptor

```toml
# packs/pitch-agent/pack.toml
[pack]
name = "pitch-agent"
version = "0.1.0"
display_name = "Pitch Agent"
description = "Public-representation worker for investor, customer, and press conversations."

[fetcher]
base_url = "https://github.com/mark-hallam/skycot-packs/releases/download"
asset_template = "{name}-{version}.tar.gz"
tag_template = "{name}-v{version}"
token_env = ["GH_TOKEN", "GITHUB_TOKEN"]

[[verticals]]
id = "pitch-agent"
path = "."
```

`PackManifest::load(Path::new("packs/pitch-agent"))` must parse this file. Keep the pack descriptor limited to the schema in `crates/argo-vertical-packs/src/manifest.rs`; put worker/runtime details in `worker.toml` and sibling files.

### 3. Worker template

```toml
# packs/pitch-agent/worker.toml
[worker]
id = "pitch-agent"
default_display_name = "Pitch"
role = "public_representation_agent"
persona = "Investor-facing public-representation agent that answers accurately, switches pitch modes, and escalates when claims exceed approved facts."
tone = "clear, concise, founder-caliber, evidence-led, brand-safe"
dry_run = true

[delegation]
charter = "Handle public pitch conversations across rehearsal, web embed, and live meeting channels while enforcing the tenant truth boundary."
kpis = [
  "approved_fact_hit_rate",
  "unsupported_claim_blocks",
  "investor_question_resolution_rate",
  "follow_up_draft_cycle_time_minutes"
]
duties = [
  "Answer canonical investor questions from approved facts and Q&A.",
  "Switch between elevator, investor, demo, and diligence modes by context.",
  "Control pitch slides in rehearsal and live meeting contexts when approved.",
  "Draft follow-up notes with cited sources and unresolved questions.",
  "Escalate financial specifics, unannounced roadmap, legal claims, and non-public customer references."
]
escalation_policy = "Escalate before making unsupported public claims, discussing non-public roadmap, quoting financials, or referencing customers without approved source material."

[personas]
elevator_30s = "personas/elevator-30s.md"
investor_3min = "personas/investor-3min.md"
demo_10min = "personas/demo-10min.md"
diligence_deep = "personas/diligence-deep.md"
voice_and_tone = "personas/voice-and-tone.md"

[memory]
schema = "memory/schema.json"

[skills]
bundle = "skills/bundle.toml"

[policies]
truth_boundary_ref = "policies/truth-boundary-ref.toml"

[tools]
permissions = "tools/permissions.toml"

[workflows]
rehearsal = "workflows/rehearsal.toml"
web_embed = "workflows/web-embed.toml"
live_meeting = "workflows/live-meeting.toml"

[escalations]
routes = "escalations/routes.toml"

[evals]
cases_ref = "evals/cases-ref.toml"

[ui]
cohort_meta = "ui/cohort.toml"
```

### 4. Persona modes

Create four sibling persona files:

- `packs/pitch-agent/personas/elevator-30s.md` — hallway / cold-open context
- `packs/pitch-agent/personas/investor-3min.md` — storyboard default
- `packs/pitch-agent/personas/demo-10min.md` — live demos with screen control
- `packs/pitch-agent/personas/diligence-deep.md` — partner-level technical questioning

Mode selection is via skill `pitch-mode-switcher` (small skill, ≤80 LOC) that takes a context tag and returns the active persona file key. Do not extend `argo-vertical-packs` for a mode field in this brief.

### 5. Memory schema

Use JSON to match the existing `packs/*/memory/schema.json` precedent:

```json
{
  "schemas": {
    "investor_profile": {
      "fields": [
        { "name": "name", "type": "string", "required": true },
        { "name": "firm", "type": "string" },
        { "name": "thesis", "type": "text" },
        { "name": "prior_objections", "type": "text[]" },
        { "name": "podcast_appearances", "type": "url[]" },
        { "name": "introduced_by", "type": "string" },
        { "name": "stage_focus", "type": "enum", "values": ["pre-seed", "seed", "a", "b", "growth"] }
      ]
    },
    "objection_library": {
      "fields": [
        { "name": "objection", "type": "text", "required": true },
        { "name": "category", "type": "enum", "values": ["competitive", "moat", "market", "team", "tech", "compliance"] },
        { "name": "best_response", "type": "text" },
        { "name": "last_used", "type": "datetime" },
        { "name": "outcomes", "type": "json" }
      ]
    },
    "approved_facts": {
      "fields": [
        { "name": "fact", "type": "text", "required": true },
        { "name": "category", "type": "string" },
        { "name": "source", "type": "url" },
        { "name": "last_reviewed", "type": "date", "required": true },
        { "name": "review_owner", "type": "string", "required": true },
        { "name": "is_public", "type": "bool", "default": true }
      ]
    }
  }
}
```

### 6. Skills bundle

```toml
# packs/pitch-agent/skills/bundle.toml
[[skills]]
id = "canonical-qa"
priority = "hot"

[[skills]]
id = "objection-handler"
priority = "warm"

[[skills]]
id = "pitch-mode-switcher"
priority = "always"

[[skills]]
id = "slide-controller"
priority = "channel:rehearsal,live_meeting"

[[skills]]
id = "follow-up-writer"
priority = "cold"
```

`canonical-qa` comes from P-04. `pitch-mode-switcher` and `slide-controller` are small new skills in this brief. If `objection-handler` or `follow-up-writer` is not present in the current library, mark it explicitly as a stub in the pack docs rather than silently pretending it exists.

### 7. Tool permissions

```toml
# packs/pitch-agent/tools/permissions.toml
[channels.rehearsal]
allowed_tools = ["keynote.advance", "keynote.show", "browser.navigate"]
mcp_connectors = ["gmail", "calendar", "notion"]

[channels.web_embed]
allowed_tools = []
mcp_connectors = []

[channels.live_meeting]
allowed_tools = ["zoom.share-screen", "keynote.advance", "keynote.show"]
mcp_connectors = ["gmail", "calendar", "crm"]
```

### 8. Truth-boundary policy reference

```toml
# packs/pitch-agent/policies/truth-boundary-ref.toml
default_policy_id = "tenant-public-pitch"

[channel_overrides]
rehearsal = "tenant-private-rehearsal"
web_embed = "tenant-public-pitch"
live_meeting = "tenant-investor-meeting"
```

A tenant deploying the template provides its own policies; the template declares only the relationship between channels and policy IDs.

### 9. Cohort UI metadata

```toml
# packs/pitch-agent/ui/cohort.toml
department = "Sales & Investor Relations"
department_id = "sales-investor-relations"
agent_name_default = "Pitch"
level = "L4"
level_label = "Senior"

[traits]
core = ["charismatic", "accurate", "brand-safe"]
great_at = ["investor objections", "live demos", "pitch-mode switching", "follow-up composition"]
escalate_when = ["financial specifics", "unannounced roadmap", "legal claims", "non-public customer references"]

[signature_behaviors]
items = [
  "Defers to the founder on financials and roadmap.",
  "Pre-warms canonical answers to top investor questions for sub-second responses.",
  "Audits every claim with provenance."
]
```

### 10. Cohorts page wire-up

Add Sales & Investor Relations as the 8th department using the current D3 registry split:

- `web/components/skycot/cohorts/dept-registry.ts` — register department id `sales-investor-relations` and vertical `pitch-agent`
- `web/components/skycot/cohorts/tile-meta.ts` — add Pitch persona tile metadata
- `web/lib/sample-cohorts.ts` — add a Pitch sample cohort row so dev-mode `/cohorts` can render the tab and tile
- `web/app/(app)/cohorts/page.tsx` — keep may-touch for route-level glue only; likely no edit is needed if the registry drives rendering

Do not redo the D3 magazine-tile layout. Reuse the existing component shells and add only the tab/tile metadata.

### 11. Tests

- `packs/pitch-agent/pack.toml` loads cleanly via `PackManifest::load(Path::new("packs/pitch-agent"))`
- Every file referenced from `worker.toml` exists and parses as its declared format
- Every referenced skill ID (`canonical-qa`, `objection-handler`, `pitch-mode-switcher`, `slide-controller`, `follow-up-writer`) resolves to a skill present in the library or is explicitly marked as a stub in this brief
- Truth-boundary policy refs resolve at validate time
- Tenant override: deploying the template under tenant=foo and tenant=bar yields two distinct, isolated worker instances (cross-tenant negative test)

## Files you may touch

- `packs/pitch-agent/**` (NEW — directory + all files listed above)
- `web/components/skycot/cohorts/dept-registry.ts` (register the 8th dept tab)
- `web/components/skycot/cohorts/tile-meta.ts` (declare the Pitch worker tile metadata)
- `web/lib/sample-cohorts.ts` (add the Pitch sample-cohort row)
- `web/app/(app)/cohorts/page.tsx` (route-level glue only; do not redesign)
- `crates/argo-vertical-packs/tests/pitch_agent_template_test.rs` (NEW — loads + validates the pack)
- `crates/argo-skill/src/pitch_mode_switcher.rs` (NEW — ≤80 LOC small skill)
- `crates/argo-skill/src/slide_controller.rs` (NEW — ≤80 LOC, thin wrapper over computer-use)

## Files you must NOT touch

- `crates/argo-vertical-packs/src/**` — consume the existing public API only
- The locked C2-personality design for worker detail pages — reuse existing components
- The locked D3 magazine-tile design for cohorts — only add the tab/tile metadata

## Acceptance criteria

- [ ] `packs/pitch-agent/pack.toml` loads via `argo_vertical_packs::PackManifest::load`
- [ ] Every file referenced from `worker.toml` exists and parses
- [ ] Skills bundle references skills that exist in the library or are explicitly stubbed in the pack docs
- [ ] Truth-boundary policy refs resolve at validate time
- [ ] Cohorts page renders the Sales & Investor Relations tab + at least one Pitch worker tile
- [ ] Cross-tenant negative test: same template, two tenants, no cross-talk
- [ ] Pitch-mode-switcher skill ≤80 LOC, has 3+ unit tests
- [ ] Slide-controller skill ≤80 LOC, has 3+ unit tests
- [ ] No file >500 LOC

## Verification

```bash
cargo test -p argo-vertical-packs pitch_agent
cargo test -p argo-skill pitch_mode_switcher slide_controller
pnpm --dir web build
bash scripts/check-brief-freshness.sh pitch-agent-worker-template
```

Manual: open `/cohorts` in the SPA, click Sales & Investor Relations, confirm Pitch tile renders with C2-personality detail page.

## Cross-references

- **Blocked by**: P-01 (claim classes), P-03 (voice provider trait), P-04 (canonical-qa skill)
- **Unblocks**: P-06 (web embed) + P-07 (live meeting) — both reference workflows defined here
- **Consumes**: P-02 runtime gate (every utterance flows through it because the pack names a policy)
- **Memory**: `project_pitch_agent_v1_demo.md`, `project_worker_detail_design_locked.md`, `project_cohorts_page_design_locked.md`, `project_pm_cohorts_peta_patrick.md` (P-cohort precedent)
- **Tested by**: P-08 eval suite
