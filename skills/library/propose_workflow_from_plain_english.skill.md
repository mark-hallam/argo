---
name: propose_workflow_from_plain_english
version: "0.1.0"
description: "Translates a delegator's plain-English description of an ongoing responsibility (e.g. 'every Friday, look at this week's tickets, cluster into themes, draft an all-hands update, run it past head of product, post to Notion') into a strict workflow TOML conforming to schema_version=\"1\". Output is a proposal only — the operator approves (or edits) before it lands as a workflow_definition via the gateway. No external side-effects."
provenance:
  source: system:bundled
  hash: sha256-pending
---

# When to use

Call when an operator (delegator) is asking their teammate to "add a
new responsibility" by describing it in plain English on the worker's
profile page. The output is a workflow TOML proposal that the
operator reviews; on accept, it is POSTed through the workflow edit
API (`POST /api/v1/verticals/{vertical_id}/workflows/{name}`) to
become a real, callable workflow.

This skill is **proposal-only**. It does not call the workflow edit
API, never writes to disk, and never executes the steps it proposes.
The operator is always in the loop.

# Inputs you expect

- the operator's plain-English description of the responsibility
  (one paragraph minimum)
- optional cadence hint ("every Friday", "monthly", "when a P0
  ticket lands")
- optional escalation hint ("run it past head of product", "needs
  legal sign-off")
- the worker's vertical id (so step names + skill references stay
  consistent with the vertical's existing skills)

# Workflow schema you must produce

Output is a single TOML document. Top-level shape:

```toml
schema_version = "1"

[workflow]
name        = "<snake_case_unique_name>"   # required, non-empty
description = "<one paragraph>"            # optional

[[step]]
name = "<snake_case_unique_step_name>"     # required, unique per workflow
kind = "skill_invoke"                      # one of: skill_invoke | prompt | tool_call
skill = "<skill_name>"                     # required when kind = skill_invoke

[[step]]
name = "<...>"
kind = "prompt"
prompt = "<literal user-message text>"     # required when kind = prompt
depends_on = ["<earlier step name>"]       # optional; metadata only in v1

[[step]]
name = "<...>"
kind = "tool_call"
tool = "<tool name>"                       # required when kind = tool_call
args = { key = "value" }                   # optional literal JSON arguments
```

Hard rules the parser enforces (`argo_vertical::workflow::parse_workflow`):

- `schema_version` MUST be the literal string `"1"`.
- `[workflow].name` MUST be non-empty.
- At least one `[[step]]` is required.
- Step names MUST be unique within the workflow.
- `kind` MUST be one of `skill_invoke`, `prompt`, `tool_call` (any
  other value is rejected).
- `kind = "skill_invoke"` requires a non-empty `skill`.
- `kind = "prompt"` requires a non-empty `prompt`.
- `kind = "tool_call"` requires a non-empty `tool`.
- Every `depends_on` entry MUST reference an earlier step name in
  the same workflow.

Unknown fields are rejected. Do not invent fields like `on_failure`,
`retry`, `schedule`, `cron`, `gate`, `approval` — they are NOT in
v1. Cadence and approval semantics belong outside the workflow
shape; surface them in the description and let the operator wire
them up through the worker's approval policy.

# Output shape

A single fenced toml block. No prose around it. No alternative
versions. Exactly one workflow per response.

```toml
schema_version = "1"

[workflow]
name        = "triage_inbound_feedback_weekly"
description = "Every week: cluster fresh feedback rows, draft a one-page summary, route to head of product for review, post the approved version to Notion."

[[step]]
name = "fetch_feedback"
kind = "tool_call"
tool = "file_read"
args = { path = "workspace/inbound/this-week.jsonl" }

[[step]]
name = "cluster"
kind = "skill_invoke"
skill = "triage_feedback"
depends_on = ["fetch_feedback"]

[[step]]
name = "draft_summary"
kind = "skill_invoke"
skill = "draft_prd"
depends_on = ["cluster"]

[[step]]
name = "request_review"
kind = "prompt"
prompt = "Send the draft summary to head of product and pause until they respond."
depends_on = ["draft_summary"]
```

# Examples

## Example 1 — triage workflow

**Operator input:**

> Every Monday morning, look at last week's inbound feedback,
> cluster it into themes, then hand it to me for review.

**Proposed workflow (this is what you emit):**

```toml
schema_version = "1"

[workflow]
name        = "weekly_feedback_triage"
description = "Weekly pull-and-cluster of inbound user feedback; pauses for operator review before any downstream action."

[[step]]
name = "read_inbox"
kind = "tool_call"
tool = "file_read"
args = { path = "workspace/inbound/last-week.jsonl" }

[[step]]
name = "cluster_themes"
kind = "skill_invoke"
skill = "triage_feedback"
depends_on = ["read_inbox"]

[[step]]
name = "operator_review"
kind = "prompt"
prompt = "Pause and surface the theme set to the operator for review before any further action."
depends_on = ["cluster_themes"]
```

## Example 2 — content draft + approval

**Operator input:**

> Every Friday at 3pm, take this week's themes, draft the all-hands
> update, run it past Priya (head of product), then post to Notion.

**Proposed workflow (this is what you emit):**

```toml
schema_version = "1"

[workflow]
name        = "friday_all_hands_update"
description = "Weekly all-hands content draft from current themes; routes through the head-of-product approver before publish to Notion."

[[step]]
name = "load_themes"
kind = "tool_call"
tool = "file_read"
args = { path = "workspace/themes/current.json" }

[[step]]
name = "draft_update"
kind = "skill_invoke"
skill = "draft_prd"
depends_on = ["load_themes"]

[[step]]
name = "send_for_approval"
kind = "prompt"
prompt = "Send the draft to head of product and pause until they reply with explicit approval or edits."
depends_on = ["draft_update"]

[[step]]
name = "publish_to_notion"
kind = "tool_call"
tool = "http_post"
args = { url = "https://api.notion.com/v1/pages", body_path = "workspace/themes/current-update.md" }
depends_on = ["send_for_approval"]
```

# Rules

- **Emit exactly one fenced ```toml block**, nothing else. No
  prose introduction, no "here's a draft" preamble, no closing
  commentary. The chat surface renders the TOML directly.
- **Use snake_case** for workflow name and step names. Lowercase
  letters, digits, underscores only.
- **Never invent skills** that don't exist in the worker's vertical.
  If the description requires a capability the vertical lacks, model
  it as a `prompt` step that asks the operator to add the skill, not
  as a `skill_invoke` step against a phantom skill.
- **Approval gates are `prompt` steps**, not a separate kind.
  Surface "run it past X" / "needs sign-off from Y" as a `prompt`
  step that explicitly pauses and asks the operator.
- **Cadence belongs in the description, not the workflow.** v1
  workflows have no `cron`/`schedule` field. Mention "every Friday"
  in the description; the operator wires up the schedule separately.
- **Refuse to emit invalid TOML.** If the description is too vague
  to determine a step sequence, emit a workflow with a single
  `prompt` step that asks the operator the clarifying question. Do
  NOT emit an empty workflow (the parser rejects zero-step
  workflows).
- **Never include credentials, tokens, or secrets** in `args`. If
  a step needs auth, model it as a tool that resolves credentials
  from the runtime, not as a literal in the workflow.
- **Cap the workflow at 8 steps.** If the operator describes a
  longer sequence, fold related actions into a single skill_invoke
  step or surface a clarifying `prompt` step.
