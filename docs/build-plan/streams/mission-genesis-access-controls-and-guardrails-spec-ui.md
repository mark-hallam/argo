# mission-genesis-access-controls-and-guardrails-spec-ui

**Owner**: claude.
**Authored against**: origin/main `a85963b34` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-access-controls-and-guardrails-spec-ui`.
**Depends on**: `mission-genesis-spa-page-describe-and-watch` MERGED.

🔴 **P0 MISSION CLAIM 3 — Phase 2 Onboarding Studio — visual access-control + guardrails specification.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]], a real teammate is given access boundaries (what they can/can't do) + escalation rules (when to ask humans) + compliance constraints (regulatory + internal policy). This brief makes that specification step explicit + visual in Genesis — operator clicks toggles + sliders rather than relying on Genesis to infer everything from prose.

The investor moment: *"I set the worker's access — read-only on customer PII, refunds capped at $500 without approval, must escalate any contract clause questions to legal — and Skycot enforces all of those at runtime."*

## Scope

1. **SPA surface**: on `/genesis`, between "describe the role" and "preview + hire", add a "Set boundaries" panel. Sections:
   - **Access scope**: read-only / read-write toggles per data category (customer PII / financial / source-code / contracts / etc) — categories sourced from `argo-capability` truth-boundary policy definitions (memory `project_v1_strategic_decisions_2026_05_14` core invariant #3).
   - **Approval thresholds**: $ amount + count-per-day sliders for actions that require human sign-off (refunds, sends, posts).
   - **Escalation routes**: which channels notify (Slack, Email, Voice — see existing memory `project_approvals_page_design_locked` for severity tiers).
   - **Forbidden domains**: free-text input for "must escalate questions about X" (legal, compliance, executive decisions). Backed by `argo-capability` truth-boundary `ClaimClass::Escalate` entries.
2. **Smart defaults**: based on the role description, Genesis pre-fills sensible defaults (e.g. a Customer Success role defaults to read-only on financial, refund-cap $500). Operator reviews + tweaks.
3. **Persistence**: the final access-control + guardrails spec becomes a `TruthBoundaryPolicy` (per Track P P-01 / P-02 ship) attached to the worker's manifest. Enforced at every agent-turn exit by `evaluate_claim` (Track P P-02 runtime enforcement).
4. **Cross-tenant**: policies are tenant-scoped; cannot leak across tenants.

## Acceptance criteria

- [ ] "Set boundaries" panel renders between role description and Hire button on `/genesis`.
- [ ] Smart-default pre-fill works for at least 3 role archetypes (Customer Success, Sales SDR, Marketplace Seller).
- [ ] Operator toggles persist into the final `TruthBoundaryPolicy` written with the hired worker.
- [ ] After Hire: at the next agent-turn, an action that exceeds an operator-set threshold (e.g. trying to issue a $1000 refund when cap is $500) is BLOCKED + audit row emitted + escalation channel notified.
- [ ] Cross-tenant: policies isolated.
- [ ] Function-side: the boundary the camera SEES set IS the boundary the runtime enforces. No theatre.

## May-touch

- `web/components/skycot/genesis/BoundariesPanel.tsx` (NEW)
- `web/components/skycot/genesis/AccessScopeToggles.tsx` (NEW)
- `web/components/skycot/genesis/ApprovalThresholds.tsx` (NEW)
- `web/components/skycot/genesis/EscalationRoutes.tsx` (NEW)
- `crates/argo-genesis/src/policy_synth.rs` (NEW — translate UI spec → `TruthBoundaryPolicy`)
- `crates/argo-cli/src/handlers/gateway/routes_genesis.rs` (extend `vertical_ready` event payload to include the policy spec)
- Tests verifying the spec round-trips into `evaluate_claim` correctly

## Out of scope

- Policy templates marketplace (v1.1 brief)
- Time-based access (only active during business hours, etc) — v1.x
- Per-action explainable-AI ("why was this blocked") — covered by existing audit chain

## Effort

~300-500 LOC (UI panels + spec synth + policy round-trip test). ~4-6 hours.
