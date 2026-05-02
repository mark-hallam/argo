# AI Receptionist

The second reference vertical for the Argo platform (#657). A
**conversation-only, escalation-friendly** front-desk agent that
greets visitors, classifies inbound intent, answers FAQs from
memory + workspace docs, and routes everything else to a human.

This vertical exists to **stress-test whether the manifest
abstraction generalizes** beyond the PR-Checker shape — a
hospitality-tone, escalation-heavy, soft-skills agent is about as
different from a code-review agent as you can stay inside one
platform.

## What it does

For each inbound visitor message:

1. Identifies the intent: greeting / info_request / book_meeting /
   support_issue / complaint / out_of_scope / chitchat.
2. For info_requests, looks up the answer in seeded memory + the
   workspace's FAQ docs. Misses → escalate.
3. Drafts a 1-3 sentence reply using the receptionist persona.
4. Sets `escalate=true` for support / complaint / book_meeting /
   out_of_scope.
5. Closes with a fenced JSON block the eval harness scores.

## What it does NOT do

By manifest declaration, this vertical is **conversation-only**:

- ❌ Never writes files (`file_write`, `file_patch` denied)
- ❌ Never runs commands (`terminal` denied)
- ❌ Never delegates to other agents (`delegate_task` denied)
- ❌ No external integrations wired in v0.1.0 — no Twilio for
  SMS, no Calendar.book, no CRM upsert. Those are intentionally
  deferred (rule: "do not add SaaS hosting, …, large external-
  service integrations unless explicitly required"). The
  receptionist is text-only at this version.

The deny list is the security boundary; capability enforcement
(#655) blocks any attempt to call a denied tool with an
`RbacDenied` audit row.

## How it's wired

| Concern | File |
|---|---|
| Manifest | `vertical.toml` |
| System prompt | `prompts/system.md` |
| Persona | `prompts/persona.md` |
| Skills | `skills/identify_intent.skill.md`, `skills/draft_reply.skill.md` |
| Eval suite | `eval/intent_classification.jsonl` (15 fixtures) |
| Memory schema | `memory.schema.toml` |
| Memory seed | `memory/seed/known_faqs.md` |

## How to use it

```bash
# Discovery (#650)
argo vertical list                 # receptionist + pr-checker
argo vertical show receptionist    # manifest summary
argo vertical validate receptionist

# Eval (#656)
argo vertical eval receptionist    # structural pass on the JSONL

# Bound session (#651, #653, #654, #655)
argo new --provider anthropic --model claude-sonnet-4-5 \
  --vertical receptionist --title "live front desk"
# → installs + attaches identify_intent + draft_reply skills
# → loads the receptionist system prompt at run-time
# → enforces the manifest's tool deny list

# Bound sessions are filterable
argo session list --vertical receptionist --json
```

## Adapting this vertical to your own business

The receptionist persona + intent labels are intentionally
generic. To customize:

1. Edit `memory/seed/known_faqs.md` — replace the placeholder
   FAQs with your real hours, location, contact channels, escalation
   targets.
2. Edit `prompts/persona.md` — match your brand voice. Stay
   short. Avoid corporate filler.
3. Edit `eval/intent_classification.jsonl` — add fixtures for
   the inbound shapes specific to your product (e.g., if you're
   a clinic, add medical-triage examples → expected
   `out_of_scope`).
4. To wire real integrations later (Twilio, Calendar, CRM): add
   an MCP server in `mcp-servers/<your-integration>/` and add
   the tools to the manifest's `[tools].allowed` list. Per the
   architecture: integrations live in MCP, never in core.
