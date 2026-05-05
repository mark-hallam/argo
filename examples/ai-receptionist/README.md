# AI Receptionist — example outputs

Companion to `examples/marketplace-seller/`. Shows what SKYCOT
produces when running the `ai-receptionist` vertical against
typical inbound triage prompts.

## What's here

- **`sample-prompts.md`** — 6 inbound scenarios that produce
  good worker output (sales call, support call, recruiter,
  spam, ambiguous, urgent).
- **`sample-bundle/`** — what `.skycot/outputs/<job_id>/` looks
  like for a successful triage run. Hand-crafted to match what
  the worker actually emits with the receptionist's output
  discipline.

## When to read this

- You're choosing between SKYCOT's two reference verticals and
  want to see what each one produces.
- You're authoring a third vertical and want to compare the
  marketplace-seller and receptionist artifact shapes side by
  side.
- You're an operator about to onboard a receptionist agent and
  want to see what your daily review surface will look like.

## Reading order

1. `sample-prompts.md` — what the operator types (or what
   the call-routing system feeds in).
2. `sample-bundle/` — what the worker hands back.
3. `../../docs/skycot/operator_runbook.md` — the CLI reference
   for `approve` / `execute` / `complete` / `fail` / `retry`.

## Running the equivalent yourself

```bash
mkdir my-receptionist && cd my-receptionist
argo workspace init
argo work add --vertical ai-receptionist \
  --prompt "$(cat ../examples/ai-receptionist/sample-prompts.md | sed -n '/^## sales call/,/^##/{//!p}')"
argo work serve --vertical ai-receptionist --once --max-steps 5
```

The shape of YOUR output should match this directory's
`sample-bundle/` layout — receptionist + marketplace bundles
share the same artifact files (only the section content
differs).
