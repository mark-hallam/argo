# pitch-agent-worker-template

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-agent-worker-template`.
**Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) MERGED + `pitch-voice-io-provider-agnostic-crate` (P-03) MERGED + `pitch-canonical-qa-skill` (P-04) MERGED.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-05 of 8**

The Pitch Agent worker template — a general-purpose vertical template (per platform invariant: vertical-specific behavior belongs in manifests, NOT in runtime forks) that any tenant can deploy to spin up their own public-representation agent. Skycot is the first instance; the template is general.

This brief assembles the parts authored by P-01 through P-04 into a deployable worker: persona, skills bundle, memory schema, truth-boundary policy reference, voice manifest reference, and the cohort metadata that makes it discoverable in the UI.

## Why this brief exists

Per the platform invariants, "vertical-specific behavior belongs in manifests, workflow definitions, prompt packs, memory schemas, tool permissions, escalation policies, evaluation cases, UI/gateway metadata." A worker is the deliverable form of all of these for one role.

The operator-locked scope (memory `project_pitch_agent_v1_demo.md`): "General template, Skycot is the first instance. Any tenant deploys their own Pitch Agent with their own truth boundary, canonical Q&A, and voice stack." This brief makes that concrete.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — what the agent must be able to do
- **Memory**: `project_pitch_agent_v1_demo.md`; `project_worker_detail_design_locked.md` (LinkedIn-profile UI style for worker detail pages, C2 personality register); `project_pm_cohorts_peta_patrick.md` (precedent for cohort registration)
- **Vertical pack format**: `crates/argo-vertical-packs/src/` — existing loader, manifest format, dev-link
- **Cohort metadata**: per memory `project_cohorts_page_design_locked`, the cohorts page has 7 dept tabs; this brief adds the 8th: Sales & Investor Relations

## Scope

### 1. Vertical pack directory

Vertical packs live in their own directory under the repo (or in `skycot-packs` repo per memory `project_skycot_packs_repo` — confirm with operator before authoring real corpora). For this brief, author the in-repo skeleton:

```
verticals/pitch-agent/
├── manifest.toml
├── persona/
│   ├── prompt-pack.md           // 30-sec, 3-min, 10-min, DD-deep modes
│   └── voice-and-tone.md
├── memory/
│   └── schema.toml              // investor profile, objection library, approved facts
├── workflows/
│   ├── rehearsal.toml           // Channel 1
│   ├── web-embed.toml           // Channel 2
│   └── live-meeting.toml        // Channel 3
├── policies/
│   └── truth-boundary-ref.toml  // pointer to the policy ID; actual policy in policies/truth-boundary/
├── skills/
│   └── bundle.toml              // names: canonical-qa, objection-handling, follow-up-writer, slide-control
├── tools/
│   └── permissions.toml         // MCP connectors enabled per channel
├── escalations/
│   └── routes.toml              // who gets paged for what
├── eval/
│   └── cases-ref.toml           // pointer to the eval suite from P-08
└── ui/
    └── cohort.toml              // department, level, badge, signature traits
```

### 2. Manifest

```toml
# verticals/pitch-agent/manifest.toml
id = "pitch-agent"
display_name = "Pitch"  # working name; the cohort uses Sales-cohort S-name convention
version = "0.1.0"
description = "Public-representation worker for investor / customer / press conversations."
cohort = "sales-and-investor-relations"
level = "L4"
is_template = true  # any tenant can instantiate

[persona]
prompt_pack = "persona/prompt-pack.md"
voice_and_tone = "persona/voice-and-tone.md"

[memory]
schema = "memory/schema.toml"

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

[eval]
cases_ref = "eval/cases-ref.toml"

[ui]
cohort_meta = "ui/cohort.toml"
```

### 3. Persona prompt pack (4 modes)

`verticals/pitch-agent/persona/prompt-pack.md` — four pitch modes with mode-switching directives:

- **30-second elevator** — for hallway / cold-open contexts
- **3-minute investor pitch** — the storyboard default
- **10-minute demo narrative** — for live demos with screen control
- **Deep technical due-diligence** — for partner-level technical questioning

Mode selection is via skill `pitch-mode-switcher` (small skill, ~50 LOC) that takes a context tag and returns the active prompt.

### 4. Memory schema

```toml
# verticals/pitch-agent/memory/schema.toml
[schemas.investor_profile]
fields = [
  { name = "name", type = "string", required = true },
  { name = "firm", type = "string" },
  { name = "thesis", type = "text" },
  { name = "prior_objections", type = "text[]" },
  { name = "podcast_appearances", type = "url[]" },
  { name = "introduced_by", type = "string" },
  { name = "stage_focus", type = "enum", values = ["pre-seed", "seed", "a", "b", "growth"] },
]

[schemas.objection_library]
fields = [
  { name = "objection", type = "text", required = true },
  { name = "category", type = "enum", values = ["competitive", "moat", "market", "team", "tech", "compliance"] },
  { name = "best_response", type = "text" },
  { name = "last_used", type = "datetime" },
  { name = "outcomes", type = "json" },
]

[schemas.approved_facts]
fields = [
  { name = "fact", type = "text", required = true },
  { name = "category", type = "string" },
  { name = "source", type = "url" },
  { name = "last_reviewed", type = "date", required = true },
  { name = "review_owner", type = "string", required = true },
  { name = "is_public", type = "bool", default = true },
]
```

### 5. Skills bundle

```toml
# verticals/pitch-agent/skills/bundle.toml
[[skills]]
id = "canonical-qa"  # from P-04
priority = "hot"     # runs first; short-circuits on hit

[[skills]]
id = "objection-handler"  # existing shared skill; reference only — does not author new code in this brief
priority = "warm"

[[skills]]
id = "pitch-mode-switcher"  # NEW small skill in this brief, ~50 LOC
priority = "always"

[[skills]]
id = "slide-controller"  # NEW thin skill wrapping computer-use; ~80 LOC
priority = "channel:rehearsal,live_meeting"

[[skills]]
id = "follow-up-writer"  # existing skill if present; else stub
priority = "cold"
```

### 6. Tool permissions

```toml
# verticals/pitch-agent/tools/permissions.toml
[channels.rehearsal]
allowed_tools = ["keynote.advance", "keynote.show", "browser.navigate"]
mcp_connectors = ["gmail", "calendar", "notion"]

[channels.web_embed]
allowed_tools = []  # public channel: no tool execution
mcp_connectors = []

[channels.live_meeting]
allowed_tools = ["zoom.share-screen", "keynote.advance", "keynote.show"]
mcp_connectors = ["gmail", "calendar", "crm"]
```

### 7. Truth-boundary policy reference

```toml
# verticals/pitch-agent/policies/truth-boundary-ref.toml
default_policy_id = "tenant-public-pitch"  # tenant overrides on deployment

[channel_overrides]
rehearsal = "tenant-private-rehearsal"
web_embed = "tenant-public-pitch"
live_meeting = "tenant-investor-meeting"
```

A tenant deploying the template provides their own policies; the template just declares the relationship between channels and policy IDs.

### 8. Cohort UI metadata

```toml
# verticals/pitch-agent/ui/cohort.toml
department = "Sales & Investor Relations"
department_id = "sales-and-investor-relations"
department_emoji = "🎤"
agent_name_default = "Pitch"  # tenants can rename
level = "L4"
level_label = "Senior"

[traits]
core = ["charismatic", "accurate", "brand-safe"]
great_at = ["investor objections", "live demos", "pitch-mode switching", "follow-up composition"]
escalate_when = ["financial specifics", "unannounced roadmap", "legal claims", "non-public customer references"]

[signature_behaviors]
- "Defers to the founder on financials and roadmap."
- "Pre-warms canonical answers to top investor questions for sub-second responses."
- "Audits every claim with provenance."
```

### 9. Cohorts page wire-up

Add Sales & Investor Relations as the 8th department tab in `web/app/(app)/cohorts/page.tsx` (or wherever the dept tab list is) — matching the precedent set by Project & Ops (Peta + Patrick) per memory `project_pm_cohorts_peta_patrick`. Do NOT redo the D3 magazine-tile layout; reuse the existing component shells.

### 10. Tests

- `verticals/pitch-agent/manifest.toml` loads cleanly via the existing pack loader
- All referenced files exist (loader's `validate` mode)
- All referenced skill IDs (`canonical-qa`, `objection-handler`, `pitch-mode-switcher`, `slide-controller`, `follow-up-writer`) resolve to skills present in the library or marked as stubs in this brief
- All referenced policy IDs resolve to policies in `policies/truth-boundary/`
- Tenant override: deploying the template under tenant=foo and tenant=bar yields two distinct, isolated worker instances (cross-tenant negative test)

## Files you may touch

- `verticals/pitch-agent/**` (new — directory + all files listed above)
- `web/app/(app)/cohorts/page.tsx` (add Sales & IR tab; do NOT redesign the page)
- `crates/argo-vertical-packs/tests/pitch_agent_template_test.rs` (new — loads + validates the template)
- `crates/argo-skill/src/pitch_mode_switcher.rs` (new — ~50 LOC small skill)
- `crates/argo-skill/src/slide_controller.rs` (new — ~80 LOC, thin wrapper over computer-use)

## Files you must NOT touch

- `crates/argo-vertical/` — the vertical-template loader infrastructure; this brief uses it, doesn't modify it
- The locked C2-personality design for worker detail pages — reuse existing components
- The locked D3 magazine-tile design for cohorts — only add the tab, don't redesign

## Acceptance criteria

- [ ] `verticals/pitch-agent/manifest.toml` loads via `argo-vertical-packs::Loader::load`
- [ ] Every file referenced from manifest exists and parses
- [ ] Skills bundle references skills that exist in the library (or are stubbed in this brief)
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
- **Consumes**: P-02 runtime gate (every utterance flows through it because the manifest names a policy)
- **Memory**: `project_pitch_agent_v1_demo.md`, `project_worker_detail_design_locked.md`, `project_cohorts_page_design_locked.md`, `project_pm_cohorts_peta_patrick.md` (P-cohort precedent)
- **Tested by**: P-08 eval suite
