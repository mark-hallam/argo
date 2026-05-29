# DEC-010 — Genesis is MVP-must-have

- **Date**: 2026-05-28
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: original framing where Genesis (plain-English → vertical
  pack) was a post-MVP capability

## Decision
Genesis — the AI-Teammate Onboarding Studio — is MVP-must-have. Scenes 1
and 3 of the demo merge into one continuous Genesis-driven first-hire beat.
Genesis is the investor moment that distinguishes Skycot from "ChatGPT with
prompts."

## Rationale
Operator strategic call (promoted 2026-05-28T03:30Z). Genesis takes voice/
text role + SOP/wiki/Slack uploads + access-controls + comprehension
verification — the "platform builds workers" claim made tangible.

## Scope nuance
Phase 1 (text-only plain-English → vertical pack) is shipped + live. Phase 2
(multi-modal onboarding: voice, document upload, comprehension verification)
is the fuller vision — partially pending. The MVP gate is Phase-1 Genesis
working e2e in the live SPA.

## Timeline impact
Same MVP-scope expansion as DEC-008 + DEC-009 — folded a previously-deferred
capability into v1.0, growing the ship gate. This register entry is the
documentation that was missing at decision-time.

## Known open item (2026-05-29)
Genesis "Try again" (idempotency retry) still returns 503 in the live SPA
despite the #2641 boot_router fix — workaround is to clear
`verticals/.genesis/op-*` before each demo take. Tracked as a follow-up.

## How to apply
Genesis is demo-critical-path. The canonical-picker roster UI on /first-hire
was the WRONG shape for this (reverted in 58ea1c93f) — Genesis-driven SSE
flow IS the first hire under MVP-with-Genesis.
