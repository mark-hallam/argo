# Sales Follow-up — example outputs

Companion to `examples/marketplace-seller/` and
`examples/ai-receptionist/`. Shows what SKYCOT produces when
running the `sales-follow-up` vertical against typical
warm-lead follow-up prompts.

## What's here

- **`sample-prompts.md`** — 5 lead scenarios (different
  context types: post-demo nudge, post-trial check-in, role
  change signal, podcast / press signal, dropped lead).
- **`sample-bundle/`** — what `.skycot/outputs/<job_id>/` looks
  like for a successful follow-up worker run. Hand-crafted to
  match what the worker emits with the sales discipline:
  research / draft / approval-checklist sections + the v3
  action ledger entries.

## When to read this

- You're considering SKYCOT for outbound sales workflows and
  want to see what the artifact shape looks like.
- You're authoring a fourth vertical and want to see all
  three reference verticals' artifact bundles side by side.

## Running the equivalent yourself

```bash
mkdir my-sales-fleet && cd my-sales-fleet
argo workspace init
argo work add --vertical sales-follow-up \
  --prompt "$(cat ../examples/sales-follow-up/sample-prompts.md | sed -n '/^## post-demo nudge/,/^##/{//!p}')"
argo work serve --vertical sales-follow-up --once --max-steps 5
```

Your output's shape should match `sample-bundle/` exactly — the
section headings (`research`, `draft`, `approval_checklist`)
are part of the worker's output discipline.
