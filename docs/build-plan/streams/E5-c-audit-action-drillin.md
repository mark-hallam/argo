# Stream E5-c — Audit timeline: action drill-in (summary / inputs / decision chain / provenance / output)

**Owner**: claude.
**Depends on**: E0 MERGED; E5-a MERGED.
**Authored against**: origin/main `393420fa` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-c-audit-action-drillin`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 3 of 8.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Action detail source | `GET /api/v1/audit/<actionId>` with mock fallback | Mandatory server endpoint; tenant-scoped; full RBAC-aware field rendering | `mission-enterprise-audit-detail-mandatory` |
| Provenance hash chain | Renders precomputed hashes from server | Verifiable hash chain — client can independently verify via verifier script (E5-f) | `mission-enterprise-audit-hash-verification` |
| Computer-use screenshot trace | Renders screenshot sequence if `action.executor == "computer-use"` | Same; v1 ships the rendering | n/a |
| PII display | Defense-in-depth from E1-e — sentinels for unredacted-disallowed viewers | Server-side first fence (mandatory); v1 client is belt + braces | `mission-enterprise-audit-pii-server-fence` (already named) |
| Output rendering | Plain-text-safe with markdown for some fields; full PII redaction | Add per-channel rendering (Slack-style messages render with formatting, voice gets transcript playback) | `mission-enterprise-audit-channel-aware-output` |
| Cross-system links | Detected via URL patterns in action body (e.g. `stripe:...`, `slack://`); rendered as "View in Stripe" links | Per-connector deep-link generator registered with each connector's manifest | `mission-enterprise-cross-system-deep-links` |

v1 is acceptable: drill-in is read-only; rendering only.
The action is already canonical at the gateway.

## Why

E5-a/-b ship the feed; clicking a row routes to
`/audit/action/<id>`. This brief lights up that page
per the wireframe: hero card + 4 detail cards (Inputs /
Decision chain / Provenance / Output) + Replay/Export
buttons.

For enterprise: this surface IS the compliance evidence.
Auditors will live here. Every field must be copy-able,
attributable, and exportable.

## Scope

- NEW route `/audit/action/<actionId>` under `(app)/`.
- 5 sections per the wireframe:
  - **Summary hero** — worker + verb + approver +
    timestamp + action ID with copy + Replay + Export
    buttons.
  - **Inputs card** — trigger (predecessor message,
    cron event, dependent action). Links to
    predecessor's drill-in if applicable.
  - **Decision chain card** — 8-step trace per the
    wireframe: memory loaded → skills considered →
    plan generated → plan selected → policy check →
    approval → execution → outcome.
  - **Provenance card** — model + prompt template +
    skill + memory snapshot + policy hashes. All
    copy-on-click.
  - **Output card** — the final artifact (sent
    message, executed action body); for computer-use
    actions, includes screenshot sequence at the
    bottom.
- Replay button → routes to `/audit/action/<id>/
  replay` (E5-d will implement).
- Export button → opens E5-e dialog (placeholder
  for now).
- Back button → returns to audit feed preserving
  filter state.
- "View related actions" link in summary → routes
  to feed filtered by worker + same hour.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/audit/action/
  [actionId]/page.tsx` (NEW)

## Files you may touch

- `web/app/(app)/audit/action/[actionId]/page.tsx`
  — NEW. ≤ 350 LOC.
- `web/components/skycot/audit/ActionSummaryHero.tsx`
  — NEW. ≤ 280 LOC.
- `web/components/skycot/audit/ActionInputsCard.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/audit/DecisionChainCard.tsx`
  — NEW. ≤ 350 LOC.
- `web/components/skycot/audit/ProvenanceBlock.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/audit/ActionOutputCard.tsx`
  — NEW. Includes screenshot trace renderer for
  CU actions. ≤ 320 LOC.
- `web/lib/api/audit-detail.ts` — NEW.
  `fetchAuditDetail(actionId)`. Mock fallback. ≤
  200 LOC.
- `web/lib/hooks/use-audit-detail.ts` — NEW. ≤ 100
  LOC.
- `web/lib/types/audit-detail.ts` — NEW. Detail
  payload type. ≤ 200 LOC.
- `web/lib/sample-audit-detail.ts` — NEW.
  Enterprise-flavored full audit-detail mock per
  3-5 sample action IDs. ≤ 400 LOC.
- `web/components/skycot/audit/AuditActionNotFound.tsx`
  — NEW. Inside-AppShell 404. ≤ 100 LOC.
- `web/app/(app)/audit/action/[actionId]/page.test.tsx`
  — NEW. ≥ 8 tests. ≤ 350 LOC.

## Files you must NOT touch

- All E1/E2/E3/E4 / earlier-E5 files except E5-a's
  audit hook (read-only).
- `crates/**` — sibling backend brief for the detail
  endpoint.

## Acceptance criteria

- [ ] Route `/audit/action/<actionId>` loads.
- [ ] Breadcrumb: `Audit › <action verb summary>`.
- [ ] **Summary hero:**
      - Worker avatar + name
      - Verb + counterparty (PII-redacted via E1-e)
      - Approver + channel + timestamp
      - Action ID (monospace) with copy button
      - `[ ▶ Replay ]` + `[ ⤓ Export ]` buttons
- [ ] **Inputs card:**
      - Trigger description (one-line)
      - Predecessor link (if `inputs.predecessor` is
        set) → routes to predecessor's drill-in
- [ ] **Decision chain card:**
      - 8 numbered steps per the wireframe.
      - Each step: numbered heading + content.
      - Expandable rows for long-form data (e.g.
        full plan text).
- [ ] **Provenance card:**
      - 5 rows: Model, Prompt template, Skill
        version, Memory snapshot, Policy version.
      - Each row shows the canonical id + short
        hash (e.g. `claude-opus-4-7` ·
        `4f3a...`).
      - Copy-on-click each value.
      - `[ Copy hash chain ]` button at bottom
        copies a single canonical JSON payload.
- [ ] **Output card:**
      - Final artifact rendered as plain text or
        structured JSON depending on action_type.
      - For computer-use actions: screenshot
        sequence at the bottom (collapsed by
        default).
      - Delivery confirmation if available
        (timestamp, downstream system response).
      - Linked buyer/customer response if exists
        (link to follow-up drill-in).
- [ ] Replay button routes to `/audit/action/<id>/
      replay` (E5-d placeholder — 404 until that
      brief).
- [ ] Export button opens E5-e single-action export
      dialog (placeholder toast until E5-e).
- [ ] Action not found: `<AuditActionNotFound>`.
- [ ] PII redaction layer applies throughout (sentinels
      for unredacted-disallowed; small banner: "PII
      redacted — your role shows masked customer
      data." + `[ Request elevated view ]` link).
- [ ] Cross-system jump links auto-detected for
      Stripe / Slack / GitHub URLs (renders "View
      in <System>" link inline).
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: click an audit row, see drill-
      in, copy action ID, click Replay placeholder.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 12KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-c-audit-action-drillin`
- PR title: `web(audit): action drill-in — summary + decision chain + provenance + output (E5-c)`

## Notes for the implementer

- **AuditDetail type** (locked):
  ```ts
  export interface AuditDetail extends AuditRow {
    inputs: {
      triggerKind: "message" | "cron" | "dependent_action";
      triggerSummary: string;
      predecessor?: { id: string; verb: string };
    };
    decisionChain: DecisionStep[];
    provenance: {
      model: { id: string; version: string };
      promptTemplate: { id: string; hash: string };
      skill: { id: string; version: string };
      memorySnapshot: { id: string };
      policy: { id: string; version: string; hash: string };
    };
    output: {
      kind: "text" | "json" | "computer-use";
      body: string | Record<string, unknown>;
      screenshots?: Array<{ url: string; timestamp: string; description?: string }>;
      delivery?: { sentAt: string; receivedAt?: string; downstreamId?: string };
      followUp?: { id: string; verb: string };
    };
  }

  interface DecisionStep {
    n: number;
    title: string;
    body: string;
    details?: Record<string, unknown>;  // expandable
  }
  ```

- **Decision chain rendering.** 8 steps numbered
  1-8:
  ```
  1. Memory loaded
     • inventory (cached) [hash 4f3a...]
     • pricing rules
  2. Skill: marketplace-reply-quote v3 (trust 92%)
  3. Plan generated:
     "Yes still avail, $90 firm" + 2 alts
     [Expand to see all 3 plans...]
  4. Plan selected by score
  5. Policy: marketplace-reply-needs-ask v2.1 → ASK
     (user OK'd)
  6. Approval received via Slack
     (Sam Harlow, 14:32:08 NZST)
  7. Execution: Browser executor on FB MP
  8. Outcome: sent, 200 OK
  ```

- **ProvenanceBlock copy** for the "Copy hash chain"
  button:
  ```json
  {
    "actionId": "act_01H9Z7",
    "timestamp": "2026-05-11T14:32:11.0Z",
    "model": {"id": "claude-opus-4-7", "version": "..."},
    "promptTemplate": {"id": "...", "hash": "..."},
    "skill": {"id": "...", "version": "..."},
    "memorySnapshot": {"id": "..."},
    "policy": {"id": "...", "version": "...", "hash": "..."}
  }
  ```

- **Output card rendering by kind:**
  - `text`: rendered as `<pre>` (preserves
    formatting). Width-constrained; horizontal
    scroll when needed.
  - `json`: rendered as collapsed JSON with
    `<details>` per level. ≤ 4 levels deep
    auto-expanded.
  - `computer-use`: text body shows the action
    log; screenshot sequence rendered as
    thumbnails below (click → lightbox).

- **Cross-system jump links — v1 detection rules:**
  - URLs starting with `https://dashboard.stripe.com/`
    → "View in Stripe"
  - URLs starting with `https://slack.com/archives/`
    → "View in Slack"
  - URLs starting with `https://github.com/<owner>/
    <repo>/issues|pull/` → "View in GitHub"
  - URLs starting with `https://app.linear.app/` →
    "View in Linear"

  Per-connector deep-link generators are Track-H.

- **Sample data — enterprise-shaped action IDs.**
  Mock detail for at least 5 actions:
  - `act_priya_001` — Priya screened candidate
    Diego Alvarez (full decision chain showing
    must-have matching).
  - `act_sam_001` — Sam processed invoice
    INV-2024-08741 (line-item extraction trace,
    flagged for CFO).
  - `act_mariah_001` — Mariah drafted RFP
    response (multi-skill execution: research +
    draft + format).
  - `act_jamal_001` — Jamal resolved ticket
    SUP-29382 (used KB article SK-413).
  - `act_nori_001` — Nori validated quote
    QT-2024-7283 (pricing-rule check + Deal Desk
    referral).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-audit-detail-mandatory` —
  server endpoint always present.
- `mission-enterprise-audit-hash-verification` —
  independent verifier script + verifiable hash
  chain.
- `mission-enterprise-audit-channel-aware-output`
  — per-channel rendering (Slack formatting,
  voice transcript playback).
- `mission-enterprise-cross-system-deep-links` —
  per-connector deep-link generators.

Sibling backend brief:

- `mission-gateway-audit-detail-endpoint` — `GET
  /api/v1/audit/<actionId>`.
