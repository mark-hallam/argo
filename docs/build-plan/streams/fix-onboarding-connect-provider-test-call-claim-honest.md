# fix-onboarding-connect-provider-test-call-claim-honest

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-connect-provider-test-call-claim-honest`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F6 (P3).

🟢 **P3 — `/onboarding/connect-provider` sub-copy promises "A $0.001 test call confirms the key works", but the handler validates the shape only (`sk-ant-` prefix check) per the comment at `routes_onboarding_helpers.rs:118`: "v1 ships without the live one-token validation call." The dressing makes a function-side promise the runtime doesn't keep.**

## Root cause

`routes_onboarding_helpers.rs:118` explicitly documents the v1 contract: "v1 ships without the live one-token validation call; the wire shape mirrors the eventual real-validation contract so a follow-up brief can swap the body without changing the response." The SPA's sub-copy in `web/app/(setup)/onboarding/connect-provider/page.tsx` was written aspirationally and never reconciled with what shipped. The "$0.001 test call" line is a function-side promise; today's response is a shape check.

Two valid resolutions:
1. **Honest copy now** — change the sub-copy to "We check the key shape (`sk-ant-...`). A live one-token validation call ships in v1.1."
2. **Real call now** — author the live one-token call (the rate-limited Anthropic endpoint) and tighten the response shape.

(1) is the v1.0 path. (2) is a v1.1 brief.

The audit notes the camera doesn't probe this in Scene 3 (the operator types a key and clicks Next), so this is a post-record / trial-path concern, not a demo blocker.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F6 [P3] — "$0.001 test call" claim has no live validation behind it

## Fix recipe

1. Locate the "$0.001 test call" sub-copy in `web/app/(setup)/onboarding/connect-provider/page.tsx`.
2. Replace with shape-honest copy:
   > "We verify the key shape (`sk-ant-...`). A live one-token validation call ships in the next release."
3. Optional follow-up: file a separate brief for the real live-validation call once the gateway can rate-limit-safely hit Anthropic.

## Acceptance criteria

- [ ] `rg '\$0.001' web/app/(setup)/onboarding/` returns zero matches.
- [ ] The connect-provider step's sub-copy describes only what actually happens (shape check) and signals when the live call ships.
- [ ] No regression in the shape-check behavior itself.

## May-touch

- `web/app/(setup)/onboarding/connect-provider/page.tsx` (~3 LOC copy edit)

## Out of scope

- Implementing the live one-token validation call (separate v1.x brief).
- The `routes_onboarding_helpers.rs` shape-check itself.

## Effort

XS — ~5 minutes copy edit. P3 / nit; ship in the next dressing batch.
