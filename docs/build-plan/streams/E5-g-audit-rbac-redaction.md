# Stream E5-g — Audit timeline: RBAC redaction layer (cross-cutting)

**Owner**: claude.
**Depends on**: E0 MERGED; E1-e MERGED (base PII redaction primitives); E5-a/c MERGED (consumers).
**Authored against**: origin/main `69d3e28e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-g-audit-rbac-redaction`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 7 of 8.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Redaction enforcement | Render-time client layer on top of server data; companion server-fence brief mandatory | Server-side first fence with client as belt-and-braces | `mission-enterprise-audit-pii-server-fence` (already named) |
| Permission granularity | Per-category: `unredacted-email`, `unredacted-phone`, `unredacted-classification`, etc. | Same; v1 ships the surface | n/a |
| Elevated-view request workflow | Logs the request + auto-grants temporarily (1 hour) for v1 | Operator-approval-gated grants with explicit duration + audit row per grant + revocation | `mission-enterprise-elevated-view-approval` |
| Cross-tenant access | Single-tenant only | Operator tenant-switch context + per-tenant unredacted permissions | `mission-enterprise-cross-tenant-redaction` |
| Default mask state | All categories masked by default | Same | n/a |
| Audit-the-audit | Each elevated-view request emits an audit row | Same; v1 ships this | n/a |

v1 is acceptable: cross-cutting layer applies on top
of the data the gateway already RBAC-filters
server-side. The architecture is correct; v1 just
extends the existing E1-e mechanism into the audit
surfaces specifically.

## Why

E5-a/c reference an RBAC redaction layer but don't
own it. E1-e established the base PII redaction with
sentinels (`[EMAIL]`, `[PHONE]`, etc.) and a hook
for elevated-view requests. This brief **extends**
that layer into audit-specific concerns:

- The action `verb` field (varies per action type)
- The action `output.body` field (variable shape)
- The action `approver.name` (sometimes sensitive)
- Data classification field rendering
- Counterparty names in audit rows
- Memory facts in worker profiles

Single canonical implementation that all audit
surfaces use.

## Scope

- Extend the existing `lib/redaction/redact.ts` module
  (from E1-e) with audit-specific redaction logic:
  - New function:
    `redactAuditRow(row, policy, viewerRole) →
    AuditRow` — applies pattern-matching + audit-
    specific rules.
  - New function:
    `redactAuditDetail(detail, policy, viewerRole)
    → AuditDetail` — handles the deep structure
    (decision chain, output, provenance).
  - New types: `RbacPermission` enum,
    `ViewerPolicy` interface (extends
    `RedactionPolicy` from E1-e).
- `useViewerPolicy()` hook (extends E1-e's
  `useRedactionPolicy()`) — reads from session +
  computes effective permissions.
- Replace E5-a/c's inline `<RedactedText>` usage
  with the audit-aware wrappers.
- Elevated-view-request flow:
  - `<RequestElevatedView>` component (from E1-e)
    reused; audit-specific copy when invoked
    inside the audit timeline.
  - Click → modal with reason input + requested-
    categories checkbox + duration dropdown (1h
    default, 4h, 8h, 24h).
  - On grant: temporarily flips viewer's effective
    policy for the requested categories. v1: client-
    only flip (mock); v1.5: server-issued time-
    bounded grant.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/lib/redaction/redact.ts`
  (extend, do not rewrite); `web/lib/hooks/use-
  redaction-policy.ts` (extend).

## Files you may touch

- `web/lib/redaction/redact.ts` — narrowly scoped:
  add `redactAuditRow` + `redactAuditDetail`.
  ≤ 350 LOC after (was ≤ 200 in E1-e).
- `web/lib/redaction/audit-rules.ts` — NEW. Audit-
  specific redaction rules (which fields, which
  patterns). ≤ 200 LOC.
- `web/lib/redaction/types.ts` — NEW. Extends
  `RedactionPolicy` from E1-e with
  `ViewerPolicy`. ≤ 150 LOC.
- `web/lib/hooks/use-viewer-policy.ts` — NEW.
  Reads session + computes per-category
  permissions. ≤ 150 LOC.
- `web/lib/hooks/use-elevated-view-grant.ts` —
  NEW. Manages temporary grants (client-only in
  v1). ≤ 180 LOC.
- `web/components/skycot/audit/AuditRedactedText.tsx`
  — NEW. Wrapper component that applies audit-
  specific redaction; renders sentinels with
  click-to-request-elevated. ≤ 150 LOC.
- `web/components/skycot/audit/ElevatedViewRequestDialog.tsx`
  — NEW. Modal with reason + categories +
  duration. ≤ 250 LOC.
- `web/components/skycot/audit/RedactionBanner.tsx`
  — NEW. Per-page banner when any redaction is
  active: "PII redacted — your role shows masked
  data." + `[ Request elevated view ]`. ≤ 120 LOC.
- `web/lib/api/elevated-view.ts` — NEW. POST
  `/api/v1/audit/elevated-view-request`. Mock
  fallback. ≤ 150 LOC.
- `web/app/(app)/audit/page.tsx` — narrowly
  scoped: mount `<RedactionBanner>` + apply
  `redactAuditRow` to feed data. ≤ 450 LOC after.
- `web/app/(app)/audit/action/[actionId]/page.tsx`
  — narrowly scoped: apply `redactAuditDetail`.
  ≤ 400 LOC after.
- `web/lib/redaction/redact.test.ts` — narrowly
  scoped: add ≥ 12 audit-specific tests covering
  each new function + per-category permission +
  elevated-view flow. ≤ 450 LOC after.

## Files you must NOT touch

- E1-e's existing `redactPII()` function — leave
  unchanged; this brief only ADDS new functions.
- `web/components/skycot/RedactedText.tsx` (from
  E1-e) — replaced by `AuditRedactedText` ONLY in
  audit surfaces; approval-inbox usage stays as-
  is.
- E1/E2/E3/E4 / earlier-E5 surfaces except the
  narrow page-level redaction wiring on E5-a/c.
- `crates/**` — server-side RBAC + elevated-view
  approval are Track-H briefs.

## Acceptance criteria

- [ ] `ViewerPolicy` type extends
      `RedactionPolicy`:
      ```ts
      interface ViewerPolicy extends RedactionPolicy {
        viewerRole: "delegator" | "operator" | "auditor" | "compliance-officer";
        permissions: {
          unredactedEmail: boolean;
          unredactedPhone: boolean;
          unredactedCard: boolean;
          unredactedClassification: boolean;
          unredactedApprover: boolean;
          unredactedCounterparty: boolean;
          ...
        };
        activeGrants: ElevatedGrant[];  // temporary unlocks
      }
      ```
- [ ] `redactAuditRow(row, policy, viewerRole)` returns
      a new AuditRow with:
      - `verb` PII-redacted per policy
      - `approver.name` redacted if `unredactedApprover
        = false` (sentinel: `[REDACTED-APPROVER]`)
      - `dataClassification` hidden (set to undefined)
        if `unredactedClassification = false`
      - counterparty handles redacted
- [ ] `redactAuditDetail(detail, policy, viewerRole)`
      returns a new AuditDetail with:
      - `inputs.triggerSummary` redacted
      - `decisionChain[]` step bodies redacted
      - `output.body` redacted
      - `provenance` left untouched (hashes aren't PII)
- [ ] `<AuditRedactedText>` wrapper:
      - Renders raw value when policy allows.
      - Renders sentinel when redacted.
      - Click on sentinel → opens
        `<ElevatedViewRequestDialog>` (instead of
        a tooltip, which is E1-e's lower-stakes
        behavior).
- [ ] `<ElevatedViewRequestDialog>`:
      - Reason input (textarea, ≥ 20 chars
        required).
      - Categories checkbox (which sentinels to
        unlock).
      - Duration dropdown: 1h / 4h / 8h / 24h.
      - Submit → POST to elevated-view-request
        endpoint → mock granted; v1.5 server-
        approved.
- [ ] `<RedactionBanner>`:
      - Renders at top of any audit page when
        viewer's policy includes ANY masked
        category that's present in the visible
        data.
      - Strings: `"PII redacted — your role shows
        masked data. [ Request elevated view → ]"`
- [ ] Audit-the-audit: each elevated-view request
      emits an audit row (mock in v1; real in
      v1.5).
- [ ] 12 unit tests pass.
- [ ] Playwright e2e: visit /audit as a delegator
      role; see redaction banner; click "Request
      elevated view"; fill reason; submit; assert
      categories unlocked temporarily.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-g-audit-rbac-redaction`
- PR title: `web(audit): RBAC redaction layer + elevated-view request (E5-g)`

## Notes for the implementer

- **Per-category permissions** — v1 ships:
  - `unredactedEmail`
  - `unredactedPhone`
  - `unredactedCard`
  - `unredactedIBAN`
  - `unredactedIRD`
  - `unredactedClassification` (data-classification
    field visibility)
  - `unredactedApprover` (approver name)
  - `unredactedCounterparty` (counterparty handles)

  Default permissions per role:
  ```
  Delegator:          { all false }
  Operator:           { all true }
  Auditor:            { Email/Phone/Approver true; Card/IBAN/IRD false }
  Compliance officer: { all true }
  ```

- **Audit-specific patterns.** Beyond E1-e's basic
  patterns:
  - **Counterparty handles** in conversation
    snippets (e.g. "@diego.alvarez", "vendor#acme")
    → mask via Set lookup against
    `worker.activeConversations[].counterpartyHandle`.
  - **Internal IDs that are PII-ish** (employee
    numbers, customer IDs) — leave alone unless
    operator config says otherwise. Customer
    IDs aren't PII per most regs but enterprise
    may prefer masking.
  - **Approver names** when viewer's role doesn't
    grant `unredactedApprover` → replace with
    `[REDACTED-APPROVER]` + role-only display
    ("approved by Operator").

- **Elevated view grant lifecycle** (v1 mock):
  ```ts
  interface ElevatedGrant {
    grantedAt: string;  // ISO
    expiresAt: string;  // ISO
    categories: string[];  // unredactedEmail, etc.
    reason: string;
    requestId: string;
  }
  ```
  Stored in client state + localStorage with TTL.
  v1.5: server-issued + cookie-bound.

- **Elevated view request dialog flow:**
  1. User clicks sentinel in a redacted field.
  2. Dialog opens with that category pre-checked.
  3. User adds reason ("Compliance review request
     #CR-2024-08").
  4. User selects duration.
  5. Submit:
     - v1 mock: client-grants immediately, sets TTL,
       emits audit row (POST to
       `/api/v1/audit/elevated-view-request`).
     - v1.5: server checks operator approval policy
       → grants if auto-allowed, else routes to
       operator queue.

- **Redaction banner placement.** Top of the page
  body, NOT in the page header (preserves header
  sticky behavior). Subtle warning-tone background.
  Dismiss × hides the banner for the session (not
  persisted — reappears on reload).

- **Performance.** Redaction runs at render time
  per row; for 50-row pages this is fine
  (≤ 5ms per row). For 1000-row exports v1.5
  server-side fence does the heavy lifting.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-elevated-view-approval` —
  operator-approval-gated grant workflow.
- `mission-enterprise-cross-tenant-redaction` —
  per-tenant redaction policy.

Sibling backend brief:

- `mission-gateway-elevated-view-request-endpoint`
  — POST `/api/v1/audit/elevated-view-request`
  + operator approval queue + grant TTL
  enforcement.
