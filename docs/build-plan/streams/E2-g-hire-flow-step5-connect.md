# Stream E2-g — Hire flow: Step 5 (Connect) — OAuth connectors + channel preference

**Owner**: claude.
**Depends on**: E0 MERGED; E2-a..e MERGED. (E2-f optional — independent.)
**Authored against**: origin/main `17b3a70f` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-g-hire-flow-step5-connect`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 7 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| OAuth provider list | Hardcoded set per vertical (Gmail, Slack, Google Calendar, Facebook Marketplace via CU, etc.) | Workspace-configurable allowlist; only the operator's approved providers shown | `mission-enterprise-connector-allowlist` |
| OAuth scope review | Static description per connector ("read + reply on your listings") | Per-tenant scope-policy enforcement: operator pre-approves which scopes a worker may request; UI shows the approved scope-set + diff if a connector wants more | `mission-enterprise-connector-scope-policy` |
| Token storage | Backend stores via existing `argo-connector` crate (assumed) | Per-tenant token encryption at rest with workspace-managed KMS keys; rotation policy mandatory | `mission-enterprise-connector-kms-rotation` |
| Computer-use session capture | "Log in once on this device" stores credentials at gateway | Tenant-isolated browser profile per worker per tenant; device-attestation before capture | `mission-enterprise-cu-tenant-isolated-profiles` |
| Channel preference | Single-select primary + fallback list | Per-channel-kind enterprise routing rules (e.g. "Slack inside business hours, SMS outside, escalate to voice for amounts > $500") | `mission-enterprise-channel-routing-policy` |
| Skip flow | Hire commits an inert worker; visible "inert until connected" state | Tenant policy may require ≥ 1 connector before hire commits | `mission-enterprise-connector-required-policy` |

v1 is acceptable because Step 5 is the *last* gate before
the worker exists; if connectors aren't ready, the worker
sits inert (visible state on the Team page) until the
operator connects them. No actions fire from an inert
worker — fail-closed at the action layer.

## Why

Step 5 is the bridge from "I have a teammate drafted" to
"my teammate has access to the tools they need". Per the
wireframe ([wireframe-hire-flow.md §"Step 5 — Connect"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md)),
the user sees a list of vertical-relevant connectors with
Connect / Disconnect actions and chooses how the worker
should reach them for approvals.

Connect failures are common (revoked tokens, scope
denials, network issues) and the flow must degrade
gracefully — Hire works even if zero connectors are
attached; the worker just sits inert until connected
later.

## Scope

- Replace `/hire/step/5` placeholder with the Connect
  view per wireframe.
- Vertical-aware connector list: which connectors to
  surface inferred from `workerDraft.suggestedVertical`
  (Step 3 output) + the Step 1 problem text.
- Per-connector row: logo + name + status + action
  button. Status states: `Not connected`, `Connecting…`,
  `Connected`, `Re-auth needed`, `Connect failed`.
- `Connect` action:
  - OAuth-based connectors: open the standard OAuth
    popup via the existing `argo-connector` flow at
    `${gateway}/connect/<connector-id>?worker=<id>`.
    Listen for the popup-completes postMessage.
  - Computer-use-based (e.g. Facebook Marketplace):
    open a session-capture flow at
    `${gateway}/connect/cu/<connector-id>?worker=<id>`.
    Different UX flag: "Sign in on this device → we'll
    save the session."
- Channel-preference section below the connectors:
  - Multi-select toggle of available channels (Slack,
    iMessage, Email, SMS, Push, WhatsApp, Voice — only
    those wired in `mission-notify-channels-rest-of-
    pack` MERGED).
  - One designated `primary`.
- Footer: `Back` · `Skip for now` · `Continue`.
  - Skip routes to `/hire/done/<workerId>` with the
    inert flag set on the draft.
  - Continue routes to `/hire/done/<workerId>`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(hire)/hire/step/[step]/page.tsx`
  (step=5 case)

## Files you may touch

- `web/app/(hire)/hire/step/[step]/page.tsx` — narrowly
  scoped: add `step=5 → Step5Connect`. ≤ 350 LOC after.
- `web/components/skycot/hire/Step5Connect.tsx` — NEW.
  Main step view. ≤ 350 LOC.
- `web/components/skycot/hire/ConnectorList.tsx` — NEW.
  Connector list container. ≤ 200 LOC.
- `web/components/ui/OAuthConnectorRow.tsx` — NEW.
  Generic OAuth row component (logo + name +
  description + status + action). Reusable on the
  worker-profile page later. ≤ 250 LOC.
- `web/components/skycot/hire/ChannelPreferenceRow.tsx`
  — NEW. Channel multi-select with primary marker. ≤
  200 LOC.
- `web/lib/connectors/registry.ts` — NEW. Catalog of
  connector metadata (id, name, logo, description,
  auth-type, vertical-affinity). Hardcoded for v1 (the
  workspace-configurable allowlist swaps in via the
  Track-H brief). ≤ 250 LOC.
- `web/lib/connectors/relevance.ts` — NEW. Pure function:
  given a `WorkerDraft`, return the ordered list of
  relevant connectors. ≤ 150 LOC.
- `web/lib/api/connectors.ts` — NEW. Client helpers:
  `getConnectorStatus(workerId, connectorId)`,
  `disconnectConnector(workerId, connectorId)`. The
  actual OAuth popup itself opens at the gateway URL
  via `window.open()`. ≤ 200 LOC.
- `web/lib/hire/use-connect-flow.ts` — NEW. Hook that:
  opens the popup, listens for postMessage, updates
  connector status on success/failure, handles popup
  closed before completion. ≤ 200 LOC.
- `web/components/skycot/hire/Step5Connect.test.tsx`
  — NEW. ≥ 8 tests: renders vertical-relevant
  connectors, connect opens popup, success updates
  status, popup-closed-early shows error, skip route
  works, continue route works, channel toggle updates
  state, primary-channel default. ≤ 350 LOC.

## Files you must NOT touch

- All E2-a..f files — locked.
- `web/components/skycot/ApprovalCard.tsx` — locked.
- `crates/**` — `argo-connector` and connector OAuth
  flows are pre-existing; no changes in this PR.

## Acceptance criteria

- [ ] `/hire/step/5` renders Step 5 Connect.
- [ ] If draft has no `workerId`, redirect to
      `/hire/step/4` (hire commit didn't happen).
- [ ] Headline: `"Mariah needs access to a few
      things."` Sub-line: `"You can skip — she'll work
      with whatever you have."`
- [ ] Connector list: 3-6 rows depending on the
      vertical inferred from `workerDraft`.
- [ ] Each row: logo + name + 1-line description +
      status pill + action button.
- [ ] Connect (OAuth):
      - Opens popup at gateway URL.
      - Row enters "Connecting…" state during the
        popup.
      - On success: status flips to "Connected (now)".
      - On error: status flips to "Connect failed" +
        small Retry button.
      - Popup closed without completion: status reverts
        to "Not connected" + dismissible toast.
- [ ] Connect (computer-use):
      - Different button text: "Sign in on this
        device".
      - Opens a separate session-capture flow.
      - Same status states.
- [ ] Disconnect: confirm dialog →
      `disconnectConnector` API → status flips to
      "Not connected".
- [ ] Channel preference section:
      - Multi-select toggle list of all 7 channel
        kinds.
      - One designated primary (radio in the toggle
        row).
      - Auto-detected default: the channel the user
        is currently signed in via (cookie/session).
- [ ] Footer: Back, Skip, Continue.
      - Back → `/hire/step/4`.
      - Skip → `/hire/done/<workerId>` with draft
        `inert: true`.
      - Continue → `/hire/done/<workerId>` with the
        connector + channel state attached.
- [ ] Skip confirm dialog: `"Mariah will be hired but
      won't have anything to work on yet. You can
      connect tools later from her profile. Continue?"`
      `[ Cancel ]` `[ Yes, skip for now ]`.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: full Steps 1→5 with stubs;
      assert `/hire/done` reached.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 7KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-g-hire-flow-step5-connect`
- PR title: `web(hire): Step 5 (Connect) — OAuth connectors + channel preference (E2-g)`

## Notes for the implementer

- **Connector registry (v1):**
  ```ts
  export const CONNECTORS: ConnectorMeta[] = [
    { id: "gmail",          authType: "oauth", ... },
    { id: "google-calendar", authType: "oauth", ... },
    { id: "slack",           authType: "oauth", ... },
    { id: "facebook-marketplace", authType: "cu", ... },
    { id: "gumtree",         authType: "cu", ... },
    { id: "ebay",            authType: "oauth", ... },
    { id: "stripe",          authType: "oauth", ... },
    { id: "intercom",        authType: "oauth", ... },
    { id: "linear",          authType: "oauth", ... },
    { id: "github",          authType: "oauth", ... },
    ...
  ];
  ```
  Each connector has a `verticalAffinity: string[]`
  field — e.g. `["gtm-engineer", "marketplace-seller"]`
  for Facebook Marketplace. `relevance()` returns
  connectors whose affinity overlaps with the inferred
  vertical, ordered by affinity strength.

- **OAuth popup flow.** Open a centered popup at
  `${gateway}/connect/${connectorId}?worker=${workerId}
  &return=${origin}/hire/oauth-return`. The gateway
  handles the OAuth dance + redirects back to the
  return URL. The return page posts the result back
  to the opener via `window.opener.postMessage({
  type: "oauth-result", connectorId, status, error?
  })`.

- **Why popup + postMessage?** OAuth providers
  require a registered callback URL; using a popup
  keeps the main hire flow on `/hire/step/5` without
  losing draft state. The result postMessage updates
  the row's status without a full page reload.

- **postMessage origin check.** ALWAYS verify
  `event.origin === window.location.origin` before
  trusting the payload. Drop messages from other
  origins silently.

- **Computer-use connector flow.** Different button
  text + a different gateway path
  (`/connect/cu/<id>?worker=<id>`). The CU flow opens
  a webview/popup that the user logs into; the
  gateway captures the session cookies. Same status-
  state machine as OAuth — the user sees "Connecting…"
  during, "Connected" after.

- **Channel preference defaults.** Auto-detect:
  - If the user signed in via Slack OAuth, default
    primary = Slack.
  - If via email magic link → Email.
  - If via iMessage in mobile webview → iMessage.
  - Fallback → Slack.

- **Save channel state on Continue.** Updates the
  worker's record at the gateway:
  `PATCH /api/v1/workers/<workerId>` with
  `{ channels: { primary, fallback[] } }`.

- **Skip path.** Routes to `/hire/done/<workerId>`
  with `?inert=1` query param. The Done page (E2-h)
  reads this and surfaces the inert state in the
  celebration banner.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-connector-allowlist` — operator-
  managed allowlist of approved connectors per
  workspace.
- `mission-enterprise-connector-scope-policy` — per-
  tenant scope policy + UI diff when a connector
  requests more.
- `mission-enterprise-connector-kms-rotation` —
  tenant-managed KMS keys for connector token
  encryption + rotation policy.
- `mission-enterprise-cu-tenant-isolated-profiles` —
  per-tenant browser profile isolation for CU
  connectors.
- `mission-enterprise-channel-routing-policy` —
  per-channel-kind enterprise routing rules.
- `mission-enterprise-connector-required-policy` —
  tenant policy requiring ≥ 1 connector before hire
  commits.
