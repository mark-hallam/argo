# Getting started with SKYCOT

A 10-minute path from zero to a worker job that emits actions you can
approve + execute.

> **Just want to confirm SKYCOT is wired end-to-end?** Run
> `./scripts/smoke-skycot.sh` from the repo root. It drives every
> state transition against a tempdir workspace in mock-mode and
> asserts each piece works. Zero API keys required.

## 1. Install Argo

Pick your OS:

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo setup
```

```powershell
# Windows
irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
argo setup
```

`argo setup` walks you through workspace init + provider key
configuration. You'll need an Anthropic or OpenAI API key for the
worker to actually call an LLM — without keys, you can still run the
mock provider for a deterministic dry-run.

For production deployments (systemd / k8s Secret / Vault patterns,
key rotation, what NOT to do), see
[`provider_keys.md`](provider_keys.md).

## 2. Initialise a workspace

```bash
mkdir my-seller && cd my-seller
argo workspace init
```

This creates `.skycot/` (where artifact bundles + ledger files
land) and registers the directory with the Argo session store.

## 3. Verify the launch vertical loads

The `gtm-engineer` vertical ships inside the Argo binary as the v1
launch product. Confirm it loaded:

```bash
argo vertical list
```

You should see `gtm-engineer` in the output. If it's missing, set
`ARGO_VERTICALS_DIR` to the directory containing `gtm-engineer/`.

## 4. Queue your first worker job

```bash
argo work add \
  --vertical gtm-engineer \
  --prompt "Research and draft a Facebook Marketplace listing for a 2019 used Yeti Tundra 65 cooler." \
  --json
```

This appends a `pending_jobs` row to memory. Inspect it:

```bash
argo worker actions list --state all --vertical gtm-engineer
```

(Empty for now — the worker hasn't run yet.)

## 5. Run the worker once

```bash
argo work serve \
  --vertical gtm-engineer \
  --once \
  --max-steps 5 \
  --json
```

This runs one job through the worker loop. With a real provider, the
worker will: search comp listings → fetch retail pages → assemble
research/pricing/listings/approval_checklist sections → emit
`action` blocks → stop with `[STOP: APPROVAL_REQUIRED]`.

## 6. Inspect what landed

A per-job artifact bundle is now under `.skycot/outputs/<job_id>/`:

```
transcript.md            full chat transcript
assistant_output.md      final assistant text
research.md
pricing.md
listings.md
approval_checklist.md    BLOCKED preamble if evidence was thin
execution_plan.md        per-job action plan grouped by state
actions.json             machine-readable ledger snapshot
job.json                 metadata
result.json              pointers + counts
```

The action ledger is also in memory:

```bash
argo worker actions list --vertical gtm-engineer --state all --json
```

You'll see one row per emitted action. Approval-gated ones (e.g.
`post_listing`) sit at `state = pending`; autonomous-safe ones
(e.g. `verify_price`) also sit at `state = pending` but don't
require an approve step.

## 7. Approve the gated action

Pick a pending+approval-gated action_id from the list above:

```bash
argo worker actions approve <action_id> --note "comp prices look right"
```

The OS validates `pending → approved`, stamps `approved_by` and
`approved_at`, and persists a NEW ledger row.

## 8. Execute it (human-executable mode)

```bash
argo worker actions execute <action_id> --json
```

The OS:

1. Validates `approved → executing` (or `pending → executing` for
   autonomous-safe).
2. Persists the new state-row.
3. Hands the row to the executor declared by the manifest. v1
   supports `executor = "human"` only.
4. The human executor renders a per-action bundle:

   ```
   .skycot/outputs/<job_id>/actions/<action_id>/
     instructions.md   ← what to do, marketplace-aware checklist
     evidence.md       ← cited sources
     action.json       ← full snapshot
   ```

   For `post_listing` on `ebay`, the instructions include the
   eBay-specific checklist (item specifics, listing format,
   shipping). Different platform → different template.

## 9. Carry the action out + record the result

Open `instructions.md`, follow the checklist, do the action in
your browser. When done:

```bash
argo worker actions complete <action_id> \
  --note "live on eBay" \
  --result-json '{"listing_url": "https://www.ebay.com/itm/...", "platform": "ebay"}'
```

If something blocks the action:

```bash
argo worker actions fail <action_id> --reason "ebay rejected — needs more photos"
```

## 10. Render the workspace ledger

```bash
argo worker actions render --vertical gtm-engineer
```

This writes 3 markdown files at `.skycot/`:

- `approval_queue.md` — currently-pending-approval actions.
- `completed_actions.md` — audit trail with operator summaries.
- `blocked_actions.md` — investigation queue with rejection reasons.

Re-run any time the ledger changes.

## 11. (optional) Verify with a real LLM

Steps 1-10 use mock-mode by default — they prove the wiring works but
don't exercise a real provider. Once you've set
`ANTHROPIC_API_KEY` (or `OPENAI_API_KEY`), run:

```bash
./scripts/smoke-skycot-real-llm.sh
```

It queues a single gtm-engineer job ("research the current price of
Bauer Vapor X5 inline skates and draft a marketplace listing
description") against a real provider, asserts the worker produced
coherent non-mock output, and reports token cost. The script
auto-skips with `=== SKIPPED: no provider key set ===` and exits 0
when no key is in the environment, so it is safe to wire into CI.

This is the smoke test to run after a fresh setup to confirm
"yes, the agent actually thinks" — the companion to the mock-mode
`./scripts/smoke-skycot.sh`.

## Automation

The real-LLM smoke is wired into GitHub Actions via
`.github/workflows/real-llm-smoke.yml`. It runs nightly at 06:00 UTC
(≈19:00 NZST) against the operator-configured `ANTHROPIC_API_KEY`
(preferred) or `OPENAI_API_KEY` (fallback) repo secret. A run with no
provider key configured reports `SKIPPED` and counts as passing, so
forks and key-less environments don't false-fail.

The workflow currently ships as `real-llm-smoke.yml.disabled` —
operators rename to `.yml` to activate, per the broader CI-disabled
posture documented in
[`CI_FALLBACK_RUNBOOK.md`](CI_FALLBACK_RUNBOOK.md).

**Manual trigger**: from the GitHub UI, navigate to
**Actions → Real-LLM smoke → Run workflow** (the
`workflow_dispatch` button). Use this to verify the workflow on a
branch before merging, or to re-trigger after a transient provider
failure.

**Failure alerts**: when a nightly run fails, the workflow posts to
the `SLACK_NOTIF_WEBHOOK_URL` repo secret. The alert links to the run
and includes the failing log line. Triage steps live in
[`CI_FALLBACK_RUNBOOK.md`'s
"Real-LLM smoke failures" section](CI_FALLBACK_RUNBOOK.md#real-llm-smoke-failures).

## What's next

- See [the demo script](demo_script.md) for a fuller walkthrough
  with the marketplace-seller use case end-to-end.
- See [the operator runbook](operator_runbook.md) for the full
  CLI reference.
- See the [Worker v3 execution design](../design/worker_v3_execution.md)
  if you're building a custom vertical.
