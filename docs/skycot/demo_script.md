# SKYCOT demo script — marketplace seller end-to-end

A 15-minute live walkthrough showing what an operator hour looks
like when SKYCOT is running the marketplace-seller vertical.

## Setup (one-time, before the demo starts)

```bash
mkdir demo-seller && cd demo-seller
argo workspace init
argo setup        # configure Anthropic or OpenAI key
```

Confirm the vertical loaded:

```bash
argo vertical list | grep gtm-engineer
```

## Scene 1 — Operator describes the job

The operator wants to sell a pair of used inline skates. They drop
the brief into the worker queue:

```bash
argo work add \
  --vertical gtm-engineer \
  --prompt "I have a pair of Powerslide Imperial 110mm inline skates, size 12 US, used 8x. Sell on Facebook Marketplace + eBay. Reserve $150."
```

**Talk track:** "Notice we don't tell the worker WHAT to research,
or HOW to write the listing, or WHICH price to set. The worker's
manifest declares its action vocabulary, its safety boundaries, and
what evidence threshold it has to clear before claiming approval-
readiness. The operator just describes the goal."

## Scene 2 — Worker runs

```bash
argo work serve --vertical gtm-engineer --once --max-steps 5 --json
```

The worker runs through ~5 multi-step turns. In each step it can:
- Call `web_search` to find comp listings.
- Call `web_extract` to pull pricing pages.
- Assemble its sectioned output (research/pricing/listings/checklist).
- Emit ` ```action ``` ` blocks for the OS to land in the ledger.

It stops with one of:
- `[STOP: TASK_COMPLETE]` — autonomous-safe work done.
- `[STOP: APPROVAL_REQUIRED]` — drafts ready, human review next.
- `[STOP: NEED_INFO ...]` — missing critical fact.

For the skate prompt, expect the worker to land
`stop_reason: approval_required`.

## Scene 3 — Inspect what landed

```bash
ls .skycot/outputs/<job_id>/
```

```
transcript.md
assistant_output.md
research.md
pricing.md
listings.md
approval_checklist.md
execution_plan.md
actions.json
job.json
result.json
```

Open the ones that matter:

```bash
cat .skycot/outputs/<job_id>/research.md       # comp links + cited URLs
cat .skycot/outputs/<job_id>/pricing.md        # comparison table
cat .skycot/outputs/<job_id>/listings.md       # draft copy for both platforms
cat .skycot/outputs/<job_id>/execution_plan.md # action plan grouped by state
```

**Talk track:** "Note the BLOCKED banner on `approval_checklist.md`
when research evidence is thin. The worker doesn't get to claim
approval-readiness on top of zero search hits — that's enforced by
the OS, not the prompt."

## Scene 4 — Review the action ledger

```bash
argo worker actions list --vertical gtm-engineer --state all --json | jq -c '.'
```

You'll see something like:

| state    | type           | requires_approval | platform              |
|----------|----------------|-------------------|-----------------------|
| pending  | verify_price   | false             | -                     |
| pending  | post_listing   | true              | facebook_marketplace  |
| pending  | post_listing   | true              | ebay                  |

**Talk track:** "Three rows. `verify_price` doesn't need approval —
that's autonomous-safe. The two `post_listing` rows do. Until the
operator approves them, the OS will refuse to advance them past
`pending`. This is the core safety primitive."

## Scene 5 — Try to bypass approval

```bash
# This should fail.
argo worker actions execute <post_listing-action-id>
```

Output:

```
Error: action <id>: approval required (state pending,
requires_approval=true); run `argo worker actions approve <id>` first
```

**Talk track:** "There's no way to advance an approval-gated action
past `pending` without going through `approve`. The state machine
holds the line."

## Scene 6 — Approve + execute the eBay listing

```bash
argo worker actions approve <ebay-post_listing-id> --note "prices comp out OK"
argo worker actions execute <ebay-post_listing-id> --json
```

The execute command returns the bundle path:

```json
{
  "action_id": "...",
  "state": "executing",
  ...
  "execution_bundle_path": ".../actions/<action_id>"
}
```

```bash
cat .skycot/outputs/<job_id>/actions/<action_id>/instructions.md
```

You see an **eBay-specific** checklist — item specifics, listing
format, photo count, returns policy. Different from the
Facebook-Marketplace one for the same action_type.

**Talk track:** "The instruction template is platform-aware. The
manifest declares per-platform overrides; the executor renders
against the action's payload. For a buyer the worker is going to
message, this would render the message body inline — flagged HIGH
RISK — so the operator can edit it before sending."

## Scene 7 — Operator carries the action out

The operator opens eBay in a browser, follows the checklist,
publishes the listing. They copy the URL.

```bash
argo worker actions complete <ebay-post_listing-id> \
  --note "live" \
  --result-json '{"listing_url": "https://www.ebay.com/itm/...", "platform": "ebay"}'
```

## Scene 8 — Render the workspace-level ledger

```bash
argo worker actions render --vertical gtm-engineer
```

```
wrote 1 (queue), 1 (completed), 0 (blocked) under .skycot
```

```bash
cat .skycot/approval_queue.md         # 1 entry: facebook_marketplace post_listing
cat .skycot/completed_actions.md      # 1 entry: ebay listing live
cat .skycot/blocked_actions.md        # empty
```

**Talk track:** "These three files are the operator's daily
review surface. `argo worker actions render` re-renders them after
each transition. They span every job + every vertical, so you can
see your whole worker fleet's state at a glance."

## Scene 9 — Trace the audit trail

```bash
argo worker actions status <ebay-post_listing-id>
```

Shows every state-row in time order:

```
action_id   <id>
type        post_listing
job         <job_id>
transitions (4 state-row(s)):
  2026-05-05T10:00:00Z — pending
  2026-05-05T10:15:42Z — approved
      approved_by: operator
  2026-05-05T10:16:01Z — executing
  2026-05-05T10:25:08Z — completed
```

**Talk track:** "Every transition is an append-only memory row.
You never lose state. Audit trail is the data model itself, not a
side channel."

## Wrap

What we just demonstrated:

1. The worker did the legwork — research, pricing, drafting, the
   approval checklist, the eBay-specific instructions.
2. The operator did the legwork the worker can't legally do —
   pressed `approve`, signed into eBay, hit publish.
3. The OS held the line — refused `execute` without approval,
   downgraded weak-evidence emissions to `blocked`, kept an
   immutable transition trail.

Time on operator: ~3 minutes (review research, click approve,
follow checklist, paste URL). Time saved vs starting from blank:
~30–60 minutes per listing.
