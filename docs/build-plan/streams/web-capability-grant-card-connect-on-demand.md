# web-capability-grant-card-connect-on-demand

**Owner**: claude or codex — SPA component; conductor review before merge.
**Depends on**: existing `connector_authorize` backend subsystem (shipped) +
the approval-card SPA pattern (shipped). No new backend in this brief.
**Authored against**: origin/main `96cd531b7` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-capability-grant-card-connect-on-demand

## Goal (operator-locked design 2026-06-11)

Wire the **operator-locked capability-grant / connect-on-demand design**
(`G2 · Elevated`) into the real SPA as a reusable React component. The locked
mockup is the contract for look + behaviour:
`docs/skycot/design-explorations/2026-06-11/capability-grant/option-G2-elevated.html`
(see also [[project_capability_grant_design_locked]]).

This is the moment a teammate needs a tool/connector it doesn't have — e.g.
"To pull the Q3 pipeline I'll need to connect to Salesforce" — surfaced **in the
teammate's conversation**, leading with a one-line plain-language ask, a
left-to-right **flow-map illustration** (teammate ▸ connector ▸ requested scopes,
read = teal arrow flowing IN, write = purple→pink arrow flowing BOTH ways, the
write edge animates), and a **read/write segmented toggle per scope**, plus
Connect / Not-now actions. Brand: warm canvas, gradient-bordered card, Audiowide
eyebrow ("CONNECT ON DEMAND"), gradient Nora orb, shimmer CTA. Locked tokens only
(purple→pink `#8b5cf6→#ec4899`, Audiowide wordmark/eyebrow, Inter body, JetBrains
Mono map labels, teal `#14b8a6` = read). Consume `web/app/globals.css` tokens —
do NOT hardcode hexes that duplicate tokens.

## Scope (frontend, additive)

1. **New presentational component** `web/components/skycot/worker/CapabilityGrantCard.tsx`
   realizing the locked G2-Elevated design. Typed props contract (no data fetching
   inside the component):
   ```ts
   type GrantScope = { id: string; label: string; description: string;
                       access: "read" | "write"; grantable: ("read"|"write")[] };
   type CapabilityGrantRequest = {
     connector: { id: string; name: string; logoUrl?: string };
     teammateName: string;
     askLine: string;              // the one-line plain-language ask
     scopes: GrantScope[];
   };
   props: { request: CapabilityGrantRequest;
            onConnect: (scopes: {id:string; access:"read"|"write"}[]) => void;
            onDismiss: () => void; busy?: boolean }
   ```
   The flow-map SVG is generated from `scopes` (1 node + 1 edge per scope; edge
   style keyed on the toggle's current access). Toggling read/write updates the
   matching edge live. Keep the component **< 500 LOC** (split the SVG map into a
   `CapabilityGrantFlowMap.tsx` sibling if needed — both < 500).
2. **Render host**: mirror `ProbationApprovalCard` — render `CapabilityGrantCard`
   in `web/components/skycot/worker/WorkerOverviewContent.tsx` (it already hosts
   `ProbationApprovalCard` at ~line 286) when a pending capability/connector
   grant-request exists for the worker. Gate behind a real-session check
   (`hasRealSession()` per the SPA mock-gating rule) — never render against mock
   fixtures on a real session.
3. **Connect action**: `onConnect` calls the existing connector-authorize path
   (the gateway `connector_authorize` subsystem — confirm the exact route via
   `rg 'connector.?authorize' crates/argo-cli/src/handlers/gateway/`), POSTing the
   connector id + the chosen per-scope access. Honest error surface on failure
   (no silent swallow); `busy` disables the button + shows progress.
4. **Tests**: vitest on the component (renders the ask + one toggle per scope +
   the flow map has one edge per scope; toggling write→read changes the edge
   class; `onConnect` fires with the selected access set; `onDismiss` fires).
   A Storybook story if the repo's Storybook covers this dir.

## Out of scope (name the dependency, don't build it here)

- The **backend signal** that emits a `capability_request` when a teammate hits a
  fenced connector mid-run. The capability fence + approval system already exist
  (`WorkerCapabilityProfile`, the approvals subsystem); CONFIRM whether a
  fenced-connector miss already surfaces a pending grant-request the SPA can read,
  and if not, author a sibling brief
  `backend-emit-capability-request-on-fenced-connector`. This brief renders from
  whatever request shape exists / is mocked in the test; it does not invent the
  emission.
- The **revoke** and **tier-promote** moments (reuse this flow-map later).

## Acceptance

- `CapabilityGrantCard` renders the locked G2-Elevated design (ask + Audiowide
  eyebrow + flow map + per-scope read/write toggles + Connect/Not-now), brand
  tokens from globals.css, no purple→cyan anywhere.
- Toggling a scope's access live-updates its flow-map edge (read=teal, write=
  gradient/both-ways).
- `onConnect` fires with the selected `{id, access}[]`; wired to the connector-
  authorize route; honest error on failure; `busy` state handled.
- Renders only on a real session (mock-gated), mirroring `ProbationApprovalCard`.
- vitest green; each touched file < 500 LOC; web typecheck clean.

## Parallel-surface check

`rg -i 'CapabilityGrant|ConnectCard|GrantCard|connect-on-demand' web/` →
none today (new surface). Reuse the existing `ApprovalCard` / `ProbationApprovalCard`
structure + the `connector_authorize` backend; do NOT fork a parallel
connector-connect path or a second approval-card abstraction.
