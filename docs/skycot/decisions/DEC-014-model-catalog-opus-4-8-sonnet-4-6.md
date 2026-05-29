# DEC-014 — Model catalog: Opus 4.8 / Sonnet 4.6 / Haiku 4.5

- **Date**: 2026-05-29
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: prior catalog (claude-opus-4-6 / haiku-4-5 with stale pricing)

## Decision
Refresh the model catalog to the current Anthropic lineup
(per platform.claude.com models overview):
- Opus 4.8 (`claude-opus-4-8`) — frontier, 1M ctx, $5/$25 per MTok — the
  fixed-policy default (discovery `prefer = highest_minor`, include pinned
  to `^claude-opus-`)
- Sonnet 4.6 (`claude-sonnet-4-6`) — balanced, 1M ctx, $3/$15
- Haiku 4.5 (`claude-haiku-4-5`) — fast, 200k ctx, $1/$5 (available, but
  not in the dev routing floor per DEC-013)
Legacy `claude-opus-4-6` dropped from the live catalog.

## Rationale
Utilise the latest top models. Opus 4.8 for complex/agentic, step down to
Sonnet 4.6 for simpler work (DEC-013).

## How to apply
Catalog lives in `manifests/providers/{anthropic,bedrock,claude-code-cli}.toml`.
Computer-use executor default + forge cost table track Opus 4.8. Shipped in
the 2026-05-29 catalog-refresh commit.
