# GTM Engineer

A SKYCOT/Argo vertical agent that **designs, executes, analyzes,
and improves** go-to-market strategies. Not a one-shot outbound-
message generator — a learning GTM operator that grows
intelligence over time through the role → work → detect-gap →
propose-skill → review → reuse loop.

## What it does

| Capability | Skill |
|---|---|
| Define / refine an Ideal Customer Profile | `define_icp` |
| Identify target accounts that match the ICP | `find_target_accounts` |
| Research a single company for GTM relevance | `research_company` |
| Research a person or buying role | `research_person` |
| Draft personalized outreach (drafts only — no sending) | `generate_outreach` |
| Analyze campaign outcomes + extract winning/failing patterns | `analyze_campaign` |
| Detect repeatable GTM patterns from accumulated evidence | `detect_gtm_pattern` |
| Propose reusable learned skills (always pending review) | `propose_gtm_skill` |

## How it differs from Clay-style workflow tools

Clay-style tools help users *build* GTM workflows — find accounts,
enrich them, plug them into a sequencer.

The GTM Engineer is designed to *be* a GTM operator that builds
and improves the strategy itself.

```
Traditional GTM workflow:
  find accounts → enrich → message → send

GTM Engineer learning loop:
  hypothesis → research → draft → analyze → detect pattern →
  propose skill → operator review → reuse → improve
```

The output of one campaign feeds the next. Patterns the operator
approves become skills the agent reuses. The agent's GTM
intelligence compounds; a workflow tool's doesn't.

## Safety model

This vertical is **draft-only** by manifest:

| Tool | Allowed | Why |
|---|---|---|
| `web_search` | ✓ | finds target-account signals |
| `web_extract` | ✓ | reads the canonical source for any cited fact |
| `file_read` | ✓ | reads operator-supplied campaign data |
| `memory` | ✓ | accumulates GTM intelligence across runs |
| `terminal` | denied | no shell |
| `file_write` / `file_patch` | denied | no on-disk mutation |
| `delegate_task` | denied + escalation-gated | no spawning sub-agents |

It does **not**:
- send emails
- post Slack/LinkedIn messages
- update CRMs
- execute shell commands
- mutate files
- perform any external state change

A future "send" capability would land behind explicit
`[escalation].human_approval_required` AND operator opt-in —
never silent removal of a deny.

## Run

```bash
# Validate the manifest cross-refs resolve
argo vertical validate gtm-engineer

# Lint for hygiene
argo vertical lint gtm-engineer

# Composite health check (validate + lint + eval)
argo vertical doctor gtm-engineer

# Inspect what the runtime sees
argo vertical show gtm-engineer

# Boot a session bound to this vertical
argo new --provider anthropic --model claude-sonnet-4-6 \
  --vertical gtm-engineer --title "first GTM run"

# Eval-suite structural check (live LLM scoring is a separate command)
argo vertical eval gtm-engineer --json
```

## Example input

> "Define an ICP for SKYCOT and propose the first outbound
> experiment we should run."

## Example output (intent)

The GTM Engineer should produce:

1. **ICP definition** (via `define_icp`) — industry, company
   size, buyer roles, pain points, trigger signals, exclusions,
   confidence + evidence gaps.
2. **Target-account criteria** (via `find_target_accounts`) — a
   ranked, evidence-backed list (not a dump).
3. **Research plan** — what to verify before drafting.
4. **Draft outreach angles** (via `generate_outreach`) — with
   personalization rationale per line.
5. **Risks + assumptions** — explicitly labelled.
6. **Suggested next experiments** — the hypotheses worth testing.
7. **Learned-skill candidates** — only when ≥3 reinforcing data
   points exist.

## Learning behaviour

When repeated evidence appears, the GTM Engineer proposes a
learned skill via `propose_gtm_skill`. The proposal lands in
`memory.learned_skill_candidates[]` with `lifecycle_state =
"pending_review"` and shows up in `argo skill review list`.

Example:

> **Evidence**: technical founders responded better to
> agent-learning + auditability angles than generic productivity
> claims across 3 distinct campaigns.
>
> **Proposed skill**: `technical_founder_learning_moat_outreach`
>
> **Status**: `pending_review`

Only after `argo skill review approve <name>` does the skill
become active and reusable in future GTM Engineer sessions. The
platform's lifecycle gate (#681) holds the line; the vertical
never auto-promotes.

## Quality targets (documentation-only until typed schema lands)

The `[success_metrics]` section in `vertical.toml` only accepts
4 typed fields today (`deflection_target_pct`,
`median_response_seconds`, `csat_threshold`,
`required_audit_chain_intact` — `deny_unknown_fields` on the
struct). The richer GTM-quality targets the eval suite scores
against live here as documentation until `SuccessMetricsSection`
gains GTM-specific fields:

| Target | Threshold |
|---|---|
| `hallucination_rate_max` | ≤ 0.05 |
| `abstention_when_insufficient_context` | ≥ 0.90 |
| `personalization_quality_target` | ≥ 0.85 |
| `pattern_detection_target` | ≥ 0.80 |
| `learned_skill_proposal_quality_target` | ≥ 0.80 |

## Memory shape

Vertical-namespaced (#680) — every memory write the GTM Engineer
makes carries `vertical_id = "gtm-engineer"` so a parallel
session bound to a different vertical never sees these rows.

| Entry | Type | Purpose |
|---|---|---|
| `icp` | `list-of-objects` | ICP definitions (versioned, append-only) |
| `target_segments` | `list-of-objects` | Refined segments derived from the ICP |
| `account_research` | `list-of-objects` | Per-account research notes |
| `persona_research` | `list-of-objects` | Per-role buying-persona notes |
| `winning_messages` | `list-of-objects` | Drafts that produced positive replies (operator-confirmed) |
| `failed_messages` | `list-of-objects` | Drafts that under-performed |
| `objections` | `list-of-objects` | Objection themes seen across replies |
| `campaign_results` | `list-of-objects` | Operator-reported campaign outcomes |
| `patterns` | `list-of-objects` | Detected reusable GTM patterns |
| `winning_plays` | `list-of-objects` | Reusable end-to-end plays |
| `learned_skill_candidates` | `list-of-objects` | Skill proposals queued for operator review |

## Current limitations

This vertical drafts and analyzes GTM work. It does **not** yet
connect to:

- live CRM (Salesforce, HubSpot, etc.)
- email send infrastructure (SendGrid, Resend, Mailchimp)
- enrichment APIs (Apollo, Clay, ZoomInfo)
- sequencing platforms (Outreach, Salesloft)

These integrations should remain optional and approval-gated.
When they land, each one must:

1. Add the new tool to `[tools].denied` by default.
2. Surface it in `[escalation].human_approval_required` so the
   operator confirms each invocation.
3. Ship with its own asset tests proving the deny is the
   default-on lock.

The agent learns from operator-supplied campaign data today; in
follow-ups it should ingest data directly when an integration is
opt-in.

## File map

```
verticals/gtm-engineer/
├── README.md                              ← this file
├── vertical.toml                          ← manifest
├── prompts/
│   ├── system.md                          ← role definition + the loop
│   └── persona.md                         ← voice + tone cues
├── memory/
│   ├── schema.toml                        ← 11 typed entry shapes
│   └── seed/
│       └── charter.md                     ← loaded on first session bind
├── skills/
│   ├── define_icp.skill.md
│   ├── find_target_accounts.skill.md
│   ├── research_company.skill.md
│   ├── research_person.skill.md
│   ├── generate_outreach.skill.md
│   ├── analyze_campaign.skill.md
│   ├── detect_gtm_pattern.skill.md
│   └── propose_gtm_skill.skill.md
├── eval/
│   └── gtm_engineer_core.jsonl            ← 10 fixture cases
└── workflows/
    └── read_charter.workflow.toml         ← smoke (intentionally simple)
```
