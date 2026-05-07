# SKYCOT — AI Worker Operating System

Welcome. This is the operator-facing entry point for **SKYCOT**, the
commercial worker layer built on top of the Argo agent OS.

## What SKYCOT is

SKYCOT runs **role-bound, evidence-aware, approval-gated AI workers**.
A worker:

1. **Researches** — pulls live data via the web tools its vertical
   declares. It cites every claim.
2. **Plans + drafts** — assembles deliverables (research, pricing,
   listings, an approval checklist) into a sectioned artifact bundle
   on disk under `.skycot/outputs/<job_id>/`.
3. **Proposes actions** — emits structured ` ```action ``` ` blocks
   the OS classifies against the vertical's manifest. Each landed
   row enters an append-only **action ledger**.
4. **Asks for approval where needed** — anything that posts publicly,
   spends money, messages a buyer, or is otherwise irreversible MUST
   get an explicit `argo worker actions approve` before it can move
   past `pending`. The gate is enforced in code, not prompts.
5. **Executes the approved actions** — for the v1 release, execution
   is **human-executable**: SKYCOT renders a per-action instruction
   bundle (with a marketplace-aware checklist) and the operator
   carries it out offline.
6. **Records outcomes** — every state transition appends a new ledger
   row carrying the full snapshot at that state. The trail is your
   audit log + your retry input.
7. **Improves over time** — patterns the operator approves get
   crystallised as learned skills (managed via `argo skills` — see
   the Argo platform docs).

Short version: **the worker does the legwork; you keep the keys.**

## Who SKYCOT is for

- Solo operators and small teams who want an AI worker grinding on a
  domain task (marketplace selling, sales outreach, support triage,
  PR review) WITHOUT giving an unattended agent the ability to spend,
  message, or post on their behalf.
- Founders launching a vertical-specific worker product (a
  "marketplace seller", "AI receptionist", "sales follow-up") that
  needs the agent OS underneath but the role/safety logic on top.

## What ships in v1

Ten vertical bindings ship in the repo (snapshot 2026-05-07; run
`ls verticals/` for the live list — the directory is the
source-of-truth):

```
verticals/gtm-engineer/        # marketplace seller — the v1 reference vertical
verticals/ai-receptionist/     # front-desk worker
verticals/customer-support/    # support triage
verticals/finance-ar/          # accounts-receivable clerk
verticals/marketing-intern/    # marketing intern
verticals/marketplace-seller/  # generic marketplace listing flow
verticals/pr-checker/          # PR review aide
verticals/receptionist/        # alt receptionist binding
verticals/sales-follow-up/     # post-demo nudge cadence
verticals/sales-sdr/           # outbound SDR
```

**`gtm-engineer`** is the v1 reference vertical — declares
marketplace-action types (verify_price, post_listing,
mark_item_sold, reply_to_buyer, …) with per-platform
instruction templates for ebay / facebook_marketplace /
gumtree. The other nine verticals are configuration-only (no
shared Rust code beyond the OS-level Worker v3 contract).

Authoring an eleventh vertical is a TOML-and-prompt-only
task; see the `verticals/` directory + `argo vertical lint
<id>` for the path. The 12 archetypes covered by `argo genesis
list-archetypes` are the standard-template starting points.

## Read next

1. **[Getting started](getting_started.md)** — install Argo, init a
   workspace, run your first worker job, approve your first action.
2. **[Demo script](demo_script.md)** — the marketplace seller flow
   walked end-to-end so you can see what an operator hour looks like.
3. **[Operator runbook](operator_runbook.md)** — full reference for
   `argo worker actions {list,show,approve,execute,complete,fail,
   status,render,retry}` + `argo worker doctor`, ledger artifacts,
   and troubleshooting.
4. **[Examples — marketplace seller](../../examples/marketplace-seller/)** —
   sample prompts that work, a hand-crafted artifact bundle showing
   what the worker writes to disk, and an operator-flow walkthrough
   of the 6 typical decision points per run.
5. **[Examples — AI receptionist](../../examples/ai-receptionist/)** —
   6 inbound triage prompts (sales call, support call, recruiter,
   spam, ambiguous, urgent calendar coordination) + a hand-crafted
   sample bundle showing receptionist-domain artifact shape.
6. **[Examples — Sales Follow-up](../../examples/sales-follow-up/)** —
   5 lead scenarios (post-demo nudge, post-trial check-in, role
   change signal, podcast signal, dropped-lead handling) + sample
   research / draft / approval checklist.
7. **[Launch readiness](launch_readiness.md)** — single-page
   confidence audit: every shipped capability, what's locked in
   tests, what's intentionally out of v1 scope, and the
   pre-launch checklist.
8. **[Failure modes](failure_modes.md)** — what to do when X
   breaks: provider 503s, worker crashes, DB lock, retry
   exhaustion, stuck jobs. Symptom / cause / recovery /
   prevention per category.
9. **[Provider keys + secrets flow](provider_keys.md)** —
   how to wire `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` for
   local dev, systemd, k8s Secret, Vault. What you must
   NOT do. Production launch checklist.
10. **[Skate-selling proof run](skate_selling_proof_run.md)** —
    operator manual for the first real-world commercial run:
    starting a job, running the worker over days, listing
    item status, walking actions through approve/execute/
    complete, marking sold, recovering after restart,
    inspecting the audit trail.
11. **[Changelog](CHANGELOG.md)** — release notes per SKYCOT
    version, customer-facing.

## Design + invariants

- **[Worker v3 execution design](../design/worker_v3_execution.md)** —
  9-section spec: states + transitions, action model, lifecycle,
  approval boundary, CLI design, artifact system, vertical binding
  layer, marketplace mapping, execution-mode v1, future executors.

## Hard safety boundaries (enforced in code)

- An action with `requires_approval = true` cannot transition
  `pending → executing` without going through
  `argo worker actions approve` first. This is a state-machine
  invariant, not a prompt rule. The model cannot override it.
- Action emissions on top of insufficient research evidence
  (zero web_search hits + sub-200-char extracts) are downgraded to
  `blocked` automatically. The model never gets to claim
  "approval-readiness" on top of weak research.
- Undeclared `action_type`s land `blocked` with an
  "no capability grant" reason — the OS won't pretend the worker had
  authorisation it didn't.
- The OS stamps `risk_level` and `requires_approval` from the
  manifest declaration onto every persisted action row. Workers
  cannot override those classifications.
