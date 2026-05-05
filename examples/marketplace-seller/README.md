# Marketplace seller — example outputs

This directory shows what SKYCOT produces when running the
`gtm-engineer` vertical against marketplace-seller prompts.
Everything here is hand-crafted to match what the worker
actually emits — use it to set expectations before kicking off
your first run, and to copy-from when authoring your own
vertical's templates / docs.

## What's here

- **`sample-prompts.md`** — 8 operator prompts that produce
  good worker output. Cover single-item, multi-item,
  research-heavy, and ambiguity-handling cases.
- **`sample-bundle/`** — every file SKYCOT writes for one
  worker job, with annotations explaining what each piece is
  for. Mirrors `.skycot/outputs/<job_id>/` exactly.
- **`operator-flow.md`** — 6 decision points an operator hits
  during a typical run, with concrete guidance on when to
  approve / retry / fail / abandon.

## Reading order

1. `sample-prompts.md` — what should the operator type?
2. `sample-bundle/` — what does the worker hand back?
3. `operator-flow.md` — how does the operator decide what to do?

## What's NOT here

- A real DB or memory store — `argo work serve` writes those;
  this directory is the customer-facing artifact bundle only.
- Per-action bundles (`.skycot/outputs/<job_id>/actions/<id>/`)
  — those land at `execute` time. See `sample-bundle/actions/`
  below for a sample.

## Running the equivalent yourself

```bash
mkdir my-seller && cd my-seller
argo workspace init
argo work add --vertical gtm-engineer \
  --prompt "$(cat ../examples/marketplace-seller/sample-prompts.md | sed -n '/^## skates/,/^##/{//!p}')"
argo work serve --vertical gtm-engineer --once --max-steps 5
ls .skycot/outputs/*/
```

The shape of YOUR output should match this directory's
`sample-bundle/` layout exactly. If files are missing, see
`docs/skycot/operator_runbook.md` for troubleshooting.
