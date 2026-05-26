# fix-onboarding-install-vertical-receipt-memory-and-gates

**Owner**: codex.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-install-vertical-receipt-memory-and-gates`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F8 (P1).

🟠 **P1 — The pick-vertical step's sub-copy promises "We install the skills, memory schema, and approval gates." The install-vertical handler's response confirms only 1 of 3 (`skills_added: N`). There's no `memory_schema_added` flag, no `approval_gates_added` count. The operator's receipt is smaller than the promise.**

## Root cause

`POST /api/v1/onboarding/install-vertical` returns:
```json
{ "installed": true, "skills_added": 5, "vertical_id": "marketing-intern", "workflows_added": 0 }
```

The handler (likely `routes_onboarding_install_vertical.rs` or similar in `crates/argo-cli/src/handlers/gateway/`) may actually be attaching memory schemas and approval gates — the audit couldn't verify from the response alone. But even if it is, the user-visible receipt confirms only skills. Two outcomes:

1. **All three are attached** → response is too narrow; expand the projector to include `memory_schema_attached: bool` and `approval_policy_attached: string`.
2. **Only skills are attached** → the sub-copy is over-promising; either narrow the copy ("We install the skills your teammates will use") or wire the missing two in the handler.

The audit calls this function-side because the install handler determines what actually happens; but the user-visible receipt is what the operator reads, so the SPA-side fix is to surface whatever the handler does.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F8 [P1] — Wizard says it installs skills + memory schema + approval gates but only 2/3 actually fire (skills counted, others silent)

## Fix recipe

1. Inspect the install-vertical handler in `crates/argo-cli/src/handlers/gateway/` (likely `routes_onboarding_install_vertical.rs`). Determine which of the three (skills / memory schema / approval gates) actually attach when called.
2. If all three attach: expand the JSON response to include `memory_schema_attached: bool` and `approval_policy_attached: string` (or count). Update SPA-side type at `web/lib/api/onboarding.ts` (or similar) to consume the new fields.
3. If only skills attach: file a follow-up brief to wire the missing two, AND narrow the sub-copy in `pick-vertical/page.tsx` to match what ships ("We install the skills your teammates will use; memory and approvals come from the workspace defaults you'll configure in Settings.").
4. The pick-vertical page (after the install completes) should render a 3-checkmark receipt: ✓ N skills · ✓ Memory schema · ✓ Approval gates (M policies) — instead of the silent transition that exists today.

## Acceptance criteria

- [ ] The install-vertical response confirms each of the three things the sub-copy promises (or the sub-copy is narrowed to match what ships).
- [ ] SPA-side: a 3-row receipt is visible on the pick-vertical confirmation moment.
- [ ] Integration test asserts the response shape includes all three fields (skills/memory/approvals).
- [ ] No regression to the `installed: true` short-circuit the SPA uses for redirect.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding_install_vertical.rs` (or similar — ~30 LOC for projector expansion)
- `web/lib/api/onboarding.ts` (~10 LOC type extension)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (~20 LOC receipt rendering)
- `crates/argo-cli/tests/onboarding_install_vertical.rs` (~30 LOC integration test)

## Out of scope

- Wiring the missing 2/3 if they truly aren't shipping (defer to a separate brief once Option-B narrowed copy lands).
- The vertical-decoration cards on the picker grid (covered by `fix-onboarding-vertical-list-align-with-landing-personas` and `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`).

## Effort

M — ~150 LOC across handler projector + SPA type + UI receipt + integration test. ~3 hours. P1 because the sub-copy is the user's mental-model anchor — "what just got installed" is the function-side commitment to the trust pitch.
