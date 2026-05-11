# Stream E4-d — Worker profile: Memory tab (facts / conversations / decisions / voice)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a MERGED.
**Authored against**: origin/main `a406d3ec` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-d-worker-profile-memory-tab`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 4 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Memory namespace isolation | Per-worker namespace; tenant_id plumbed (already shipped in `mission-memory-per-worker-scope`) | Same; no degradation | n/a |
| Fact taxonomy | Free-text per row | Operator-defined fact-category schema (e.g. "Org policy", "Customer rule", "Vendor contact") so enterprise can audit + filter | `mission-enterprise-memory-fact-categories` |
| Cross-worker memory share | Not surfaced in v1 | Explicit operator-granted cross-worker memory share with audit row per access | `mission-enterprise-cross-worker-memory-share` |
| PII handling | Defense-in-depth (E1-e) renders sentinels; raw values for unredacted-view RBAC | Server-side fence + classification of each fact (low/med/high sensitivity); enterprise can audit which workers know which categories of PII | `mission-enterprise-memory-pii-classification` |
| Edit audit | Every memory edit/delete writes an audit row (uses existing audit infra) | Same; v1 ships the audit emission | n/a |
| Conversation thread privacy | Active conversations show worker + counterparty handles only; raw message content gated by RBAC | Per-tenant policy for whether operators may read full conversation threads vs metadata only | `mission-enterprise-conversation-thread-rbac` |

v1 is acceptable: memory mutations all go through the
existing audit log; isolation is already shipped at the
backend.

## Why

E4-a's placeholder for `/workers/<id>/memory` reads
"Memory — coming in E4-d". This brief lights it up:
plain-English facts (not vector embeddings), active
conversations the worker is mid-thread on, recent
material decisions the worker has made, and the voice
profile summary with an Adjust entry-point.

For enterprise: workers accumulate substantial memory
(policies, customer rules, vendor relationships). The
Memory tab must filter + search across N=100+ facts;
v1 ships with simple list + client-side search.

## Scope

- Route `/workers/<id>/memory` renders the tab.
- Four sections per the wireframe:
  - **Facts** card: list of plain-English facts;
    edit-inline + delete per row; `[+ Add a fact]`
    button.
  - **Active conversations** card: open threads with
    counterparty handle + last-message timestamp;
    click → audit drill-in for the latest action.
  - **Recent decisions** card: material decisions
    last 14 days; one-line each + audit link.
  - **Voice profile** summary: tone + formality +
    signature phrases; `[✎ Adjust]` button (placeholder
    until E4-g).
- Add-a-fact dialog: free-text input + Save / Cancel.
- Memory namespace ID + tenant-isolation badge at the
  top (when operator view).
- Client-side search across facts (planned >=20 facts
  per worker for enterprise).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/workers/[workerId]/memory/page.tsx`
    (NEW).

## Files you may touch

- `web/app/(app)/workers/[workerId]/memory/page.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/MemoryFactsCard.tsx`
  — NEW. List + inline edit + delete + add. ≤ 280
  LOC.
- `web/components/skycot/worker/ActiveConversationsCard.tsx`
  — NEW. ≤ 180 LOC.
- `web/components/skycot/worker/RecentDecisionsCard.tsx`
  — NEW. ≤ 150 LOC.
- `web/components/skycot/worker/VoiceProfileSummary.tsx`
  — NEW. ≤ 180 LOC.
- `web/components/skycot/worker/AddFactDialog.tsx` —
  NEW. ≤ 150 LOC.
- `web/lib/api/worker-memory.ts` — NEW.
  `fetchMemory(workerId)`, `addFact()`,
  `editFact(factId, newText)`, `deleteFact(factId)`.
  Mock fallback. ≤ 250 LOC.
- `web/lib/hooks/use-worker-memory.ts` — NEW.
  Query + mutation hooks. ≤ 200 LOC.
- `web/lib/sample-worker-memory.ts` — NEW. Enterprise-
  flavored mock memory (org policies, customer rules,
  vendor relationships) per E4-a's 6 workers. ≤ 300
  LOC.
- `web/components/skycot/worker/Memory.test.tsx` —
  NEW. ≥ 8 tests. ≤ 350 LOC.

## Files you must NOT touch

- All E1/E2/E3/earlier-E4 files — read-only.
- `web/components/skycot/RedactedText.tsx` (from
  E1-e) — re-used, not modified.
- `crates/**` — backend memory edit endpoints sibling
  brief.

## Acceptance criteria

- [ ] Route `/workers/<id>/memory` renders the four
      cards in order.
- [ ] **Facts card:**
      - Top: count of facts + search input (`"Search
        facts..."`).
      - Each row: plain-English fact + edit (✎) +
        delete (🗑) buttons.
      - Inline edit: click ✎ → textarea swap → blur/
        Enter saves, Esc cancels.
      - Delete: confirm dialog → mutation → row
        removed from list.
      - `[+ Add a fact]` button below the list opens
        `AddFactDialog`.
      - Empty state: `"${workerName} is still learning
        about your business."` + body `"Add what she
        should know above, or let her pick it up as
        she works."`
      - Search filters facts case-insensitive
        substring match.
- [ ] **Active conversations card:**
      - Each row: counterparty handle (PII-redacted)
        + topic snippet + last-msg time.
      - Click row → routes to the latest audit drill-
        in for that conversation.
      - Empty state: `"No open conversations right
        now."`
- [ ] **Recent decisions card:**
      - 5-10 most material decisions in last 14 days.
      - Each row: short decision label + outcome +
        timestamp + audit link.
      - Empty state: `"No material decisions in the
        last 14 days."`
- [ ] **Voice profile summary:**
      - Tone: `Warm` / `Neutral` / `Formal`
      - Formality slider value as text
      - Signature phrases list
      - `[✎ Adjust]` button opens RetrainDialog
        placeholder (toast `"Adjust voice in E4-g"`).
- [ ] Memory namespace ID + tenant badge at top of
      page (only when `permissions.canViewSettings`
      is true).
- [ ] Add-a-fact dialog: free-text textarea ≥ 4 rows
      + Save / Cancel.
- [ ] Save → POST `/api/v1/workers/<id>/memory/facts`
      → optimistic add → toast.
- [ ] Delete → DELETE
      `/api/v1/workers/<id>/memory/facts/<factId>`
      → confirm → optimistic remove.
- [ ] Mock fallback (dev) for all mutations.
- [ ] Production hard-fails on 404.
- [ ] PII redaction layer (E1-e) applies to fact
      text + conversation snippets.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: visit memory, add a fact,
      assert it appears; delete it, assert it
      disappears.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-d-worker-profile-memory-tab`
- PR title: `web(workers): Memory tab — facts + conversations + decisions + voice (E4-d)`

## Notes for the implementer

- **Sample memory per worker — enterprise-shaped
  facts.** For Priya (Senior Recruiter):
  - "Pre-screen all senior engineering candidates
    against the 6 must-haves in JD-2024-08."
  - "Diversity hiring goals: balanced gender + 30%
    underrepresented minorities; flagged for HR
    review at offer stage."
  - "Use 'guests' not 'candidates' externally per
    employer brand voice."
  - "Never agree to compensation above the role's
    posted range without VP People sign-off."
  - "Friday afternoons: queue rejection emails for
    Monday morning send (no rejection emails on
    Fridays per company policy)."
  - "Workato integration: candidates from referral
    program get priority screening."

  For Sam (AP Specialist):
  - "Invoices > $10K route to Controller AND CFO."
  - "Vendor net-30 terms apply unless contract
    overrides."
  - "Duplicate detection: flag any invoice number
    seen in last 90 days from same vendor."
  - "Q4 close: all invoices must be processed by
    Dec 28 (24h freeze before year-end)."
  - "PO-required vendors: refuse invoices without
    a matched PO."

  For Mariah (Vendor Manager):
  - "Vendor onboarding requires Compliance review
    for any vendor over $50K/year spend."
  - "Annual renewals route to Procurement + the
    business owner."
  - "Use 'strategic partners' not 'vendors' for
    Tier 1 vendors per Operations branding."

- **Conversation rendering.** Use the audit-row's
  `verb` field for the snippet. Counterparty
  handles come from the conversation thread's
  metadata (e.g. "Diego Alvarez" for a candidate;
  "vendor#tomas-industries" for a vendor).

- **Recent decisions.** Pulled from audit rows where
  `action_outcome ∈ ("approved", "refused")` AND
  `decision_meta.material = true` (server-side
  flag). v1 mock data should include 3-5 such
  decisions per worker.

- **Add-a-fact endpoint shape:**
  - POST `/api/v1/workers/<id>/memory/facts` with
    body `{ text }`.
  - Response: `{ factId, text, createdAt }`.
  - PUT for edit, DELETE for delete.

- **Memory namespace ID + tenant badge** styled as
  small monospace chips:
  ```
  Namespace: ns_priya_4f7a  Tenant-scoped ✓
  ```
  Only visible when
  `permissions.canViewSettings === true`.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-memory-fact-categories` —
  operator-defined fact-category schema.
- `mission-enterprise-cross-worker-memory-share` —
  explicit cross-worker share with audit.
- `mission-enterprise-memory-pii-classification` —
  server-side sensitivity classification per fact.
- `mission-enterprise-conversation-thread-rbac` —
  per-tenant policy on operator thread access.

Sibling backend brief:

- `mission-gateway-worker-memory-endpoints` —
  fact CRUD endpoints.
