# DEC-012 — Dev/demo LLM routes through Claude Code subscription

- **Date**: 2026-05-29
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
In dev/demo/build-phase, route LLM calls through the Claude Code subscription
via the `claude-code-cli` subprocess provider (`claude --print --output-format
json`) instead of the metered Anthropic developer API. Production deploys flip
routing back to `anthropic.toml` + the real API key.

## Rationale
API billing burns through quota too fast during dev iteration. The Claude
Code subscription is already paid + has a fair-use quota model. Cost-control
for the build phase.

## How to apply
Dev/demo: copy `docs/skycot/operator/argo.dev.toml.example` → repo-root
`argo.toml` (gitignored) + `export ARGO_PITCH_LLM_PROVIDER=claude-code-cli`.
Production: `anthropic.toml` default, API key in `~/.argo/env.local`.
Shipped via #2663 + #2666.
