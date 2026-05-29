# DEC-013 — Tier step-down floor is Sonnet 4.6 — never Haiku

- **Date**: 2026-05-29
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
In the cost-tier routing ladder, Sonnet 4.6 is the LOWEST tier we drop to.
Haiku is never used for the answering model. Complex tasks rise to Opus 4.8.
Even the tier classifier runs on Sonnet, not Haiku.

## Rationale
Quality floor — Haiku's near-frontier intelligence is below the bar the
operator wants for any teammate-facing or pitch-facing output.

## How to apply
`[routing.tiered]`: simple + medium → claude-sonnet-4-6, complex →
claude-opus-4-8, classifier → claude-sonnet-4-6. Any config or test that
puts Haiku in the dev routing answering path is a violation. Implemented +
real-binary-verified in #2666 (the CLI's internal title-gen Haiku side-call
is explicitly prevented from mislabeling the answer model).
