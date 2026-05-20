# web-approvals-inbox-ops-dashboard-completion — STATUS: UI SHIPPED, backend signal-plumbing remains

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-approvals-inbox-ops-dashboard-completion`.
**Priority**: P1 — the UI components SHIPPED, but the backend approvals API
returns rows that don't carry the new fields (severity / dollarAtRisk /
confidence / slaExpiresAt). Production approvals render with the new
components hidden because the gateway envelope is silent on these fields.
**Sizing**: ~600-800 LOC — backend column adds + signal-derivation logic
+ 4-section sample-data fixture cleanup + end-to-end smoke + a few minor UI
polish items.

---

## Status — UI 100% SHIPPED, backend signal flow incomplete

Components shipped on origin/main (each with sibling .test.tsx file):

| Component | Location | Wired? |
|-----------|----------|--------|
| `<KpiStrip>` (5 KPIs + 7-day ring) | `web/components/skycot/approvals/KpiStrip.tsx` | ✅ in `web/app/(app)/page.tsx` |
| `<SeverityPill>` (P0/P1/P2 tier badge) | `web/components/skycot/approvals/SeverityPill.tsx` | ✅ in `ApprovalCard` |
| `<SLABar>` (green ≥4h / amber ≥1h / red <1h) | `web/components/skycot/approvals/SLABar.tsx` | ✅ in `ApprovalCard` |
| `<DollarAtRisk>` (monetary impact) | `web/components/skycot/approvals/DollarAtRisk.tsx` | ✅ in `ApprovalCard` |
| `<ConfidenceDonut>` (worker self-confidence) | `web/components/skycot/approvals/ConfidenceDonut.tsx` | ✅ in `ApprovalCard` |
| `<ApprovalRateRing>` (7-day rate donut) | `web/components/skycot/approvals/ApprovalRateRing.tsx` | ✅ in `KpiStrip` |
| `ApprovalCard` extended props | `web/components/skycot/ApprovalCard.tsx` | ✅ |
| Sample-data severity / SLA / $ / confidence | `web/lib/sample-approvals.ts` (16 field references) | ✅ |

What this means in practice: in **dev mode with the sample-approvals
fallback**, the inbox looks polished. In **prod-against-gateway**, the
gateway's `/api/v1/approvals` returns the old envelope shape (no severity,
no SLA, no $ at risk, no confidence) so the new components silently render
nothing — the page degrades back to the pre-dashboard look.

This follow-up brief makes those 4 fields LIVE in production by:
1. Adding columns to the `approval_request` / `pending_action` storage table.
2. Deriving the 4 signals at emission time (where they don't already exist).
3. Surfacing in the gateway envelope.
4. Removing the sample-data fallback in production paths.

---

## Pre-flight verification

```bash
# 1. Confirm the 6 UI components + tests are present.
ls web/components/skycot/approvals/*.tsx | wc -l   # expect ≥ 12 (6 components × 2 files)

# 2. Confirm KpiStrip is wired into the page.
rg "KpiStrip" web/app/\(app\)/page.tsx

# 3. Confirm gateway envelope today.
rg "severity\|sla_expires\|dollar_at_risk\|confidence" crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs
# Expect ZERO matches — that's the gap this brief closes.

# 4. Confirm the approval-request schema.
rg "CREATE TABLE.*pending_action\|CREATE TABLE.*approval_request" crates/argo-store/migrations/
```

---

## Phase 1 — Storage columns

### 1a. Migration

**File**: `crates/argo-store/migrations/NNN__approval_signal_fields.sql` (next slot)

```sql
-- Stream `web-approvals-inbox-ops-dashboard-completion` — add the 4
-- signal fields the locked E-ops-dashboard design carries.
-- Additive; existing rows get sensible defaults.

ALTER TABLE pending_action ADD COLUMN severity TEXT NOT NULL DEFAULT 'p2';
ALTER TABLE pending_action ADD COLUMN sla_expires_at_micros INTEGER;
ALTER TABLE pending_action ADD COLUMN dollar_at_risk_cents INTEGER;
ALTER TABLE pending_action ADD COLUMN confidence_pct INTEGER;
                          -- 0..100, NULL when the proposing worker did not surface confidence

-- Optional severity-priority index (sort: severity desc, sla asc).
CREATE INDEX IF NOT EXISTS idx_pending_action_severity_sla
    ON pending_action(tenant_id, severity, sla_expires_at_micros);
```

The table name is `pending_action` (verify with the schema grep above — if
the table is named differently, swap throughout this brief).

### 1b. Severity assignment policy

The 3 tiers map to existing concepts in the codebase:

| Tier | Color | Criteria |
|------|-------|----------|
| P0 | red | `risk_level = "high"` AND (`dollar_at_risk_cents > 100_00` OR escalated previously) |
| P1 | amber | `risk_level = "medium"` OR `dollar_at_risk_cents > 0` |
| P2 | slate | everything else (default) |

The `risk_level` field already exists on pending actions (see
`approvals.rs` — `risk_level: "low" | "medium" | "high"`). Derive severity
from risk + dollar.

### 1c. Where the fields get populated

Each pending action is created at the agent-loop tool-dispatch stage (in
`argo-agent-runner`). The shipping path for these fields:

1. **`dollar_at_risk_cents`**: when the proposed action is a `spend.*` or
   `payout.*` tool call, the dispatcher already extracts the amount. Plumb
   into the action-creation INSERT.
2. **`confidence_pct`**: when the assistant message that proposes the
   action carries `confidence: <0..1>` JSON field (already a contract; see
   `argo-agent-runner::message_extract`), persist `(confidence * 100).round()`.
3. **`sla_expires_at_micros`**: `created_at_micros + sla_duration_micros`,
   where `sla_duration_micros` comes from the tool's
   `escalation.sla_minutes` field in the vertical manifest (default 60).
4. **`severity`**: derived at INSERT time from the 3 rules above.

---

## Phase 2 — Gateway envelope

**File**: `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs`

Find the response builder (the function that constructs the JSON for each
pending action — `rg "pending_actions\[\]\|pending\":" crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs -B 5`).

Add the 4 fields to the JSON object built per action:

```diff
  json!({
      "id": row.id,
      "worker_id": row.worker_id,
      "tool": row.tool,
      "risk_level": row.risk_level,
+     "severity": row.severity,
+     "sla_expires_at": row.sla_expires_at_micros.map(epoch_to_iso),
+     "dollar_at_risk": row.dollar_at_risk_cents.map(|c| c as f64 / 100.0),
+     "confidence": row.confidence_pct.map(|p| p as f64 / 100.0),
      "created_at": ...,
      ...
  })
```

### KPI envelope additions

The 5-KPI strip + 7-day ring need:

```
GET /api/v1/approvals/kpis
→ {
    pending_count: <i>,
    resolved_this_week: <i>,
    avg_response_seconds: <i>,
    dollar_at_risk_total: <f>,         // sum of pending dollar_at_risk_cents / 100
    approval_rate_7d: {
      approved: <i>,
      rejected: <i>,
      expired: <i>,
    }
  }
```

New handler `kpis_response` in `routes_approvals_json.rs` — 5 simple SQL
queries against `pending_action` + `audit_record` (for the historical
approve/reject/expire counts).

---

## Phase 3 — Web wire-up cleanup

### 3a. Remove sample-data fallback in prod

**File**: `web/lib/api/approvals.ts`

The existing sample-approvals fallback was for dev-without-gateway. In prod
(`NODE_ENV === "production"` or `isStrictProduction()` from the wave-2 fix),
fall through to a real error rather than silently substituting sample data.

The wave-2 audit fix already added `isStrictProduction()` for this path —
verify it's gating the fallback, and add severity/SLA/$/confidence to the
sample-data shape ONLY for dev mode.

### 3b. Wire KPI fetch

**File**: `web/lib/api/approvals.ts` — add:

```ts
export interface KpiSnapshot {
  pendingCount: number;
  resolvedThisWeek: number;
  avgResponseSeconds: number;
  dollarAtRiskTotal: number;
  approvalRate7d: { approved: number; rejected: number; expired: number };
}

export async function fetchKpis(): Promise<KpiSnapshot> {
  return apiGet<KpiSnapshot>("/api/v1/approvals/kpis");
}
```

Then `web/app/(app)/page.tsx` uses TanStack Query (already in use) to load
KPIs:

```tsx
const kpiQuery = useQuery({
  queryKey: ["approvals", "kpis"],
  queryFn: fetchKpis,
  refetchInterval: 30_000,        // refresh every 30s
  staleTime: 15_000,
});
const kpiCells = kpiQuery.data
  ? computeKpiCells(kpiQuery.data)
  : computeKpiCells({ /* defaults */ });
<KpiStrip cells={kpiCells} loading={kpiQuery.isPending} />
```

### 3c. Loading skeletons

Each KPI cell needs a skeleton state for the first render. The
`<KpiStrip>` component already accepts `loading: boolean` — verify it
renders a tasteful skeleton (gray bar) when true.

### 3d. Sort respects severity desc → SLA asc

In `web/app/(app)/page.tsx` where pending actions are mapped to cards:

```diff
- const cards = pending.map(a => <ApprovalCard key={a.id} {...a} />);
+ const sortedCards = pending
+   .slice()
+   .sort((a, b) => {
+     // Severity desc: P0 < P1 < P2 numerically (lower = higher priority).
+     const severityOrder = { p0: 0, p1: 1, p2: 2 } as const;
+     const sevDiff = severityOrder[a.severity ?? "p2"] - severityOrder[b.severity ?? "p2"];
+     if (sevDiff !== 0) return sevDiff;
+     // SLA asc: earliest expiry first.
+     return (new Date(a.slaExpiresAt ?? "9999-12-31").getTime())
+          - (new Date(b.slaExpiresAt ?? "9999-12-31").getTime());
+   })
+   .map(a => <ApprovalCard key={a.id} {...a} />);
```

---

## Phase 4 — Tests

### 4a. Backend

`crates/argo-cli/src/handlers/gateway/routes_approvals_json_tests.rs` (extend):

1. `envelope_includes_severity_sla_dollar_confidence`
2. `kpis_endpoint_returns_5_fields_plus_rate_donut`
3. `severity_derivation_p0_for_high_risk_high_dollar`
4. `severity_derivation_p1_for_medium_risk_or_dollar`
5. `severity_derivation_p2_for_low_risk_no_dollar`

### 4b. Web

Existing component tests should pass. Add:

`web/app/(app)/page.test.tsx` (new or extend):
6. `cards_sort_by_severity_then_sla`
7. `kpi_strip_renders_loading_skeleton_when_pending`
8. `kpi_strip_renders_real_values_after_fetch`

---

## may-touch

- `crates/argo-store/migrations/NNN__approval_signal_fields.sql` (new).
- `crates/argo-store/src/migrations/vNN.rs` (new).
- `crates/argo-store/src/pending_action.rs` — read/write the 4 new columns.
- `crates/argo-agent-runner/src/dispatcher.rs` (or wherever pending actions
  are created) — populate severity/SLA/dollar/confidence at INSERT time.
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` — add 4 fields to envelope + new `kpis_response` handler.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — dispatch the new KPI route.
- `web/lib/api/approvals.ts` — add `fetchKpis`, ensure prod fallback is gated.
- `web/app/(app)/page.tsx` — wire `useQuery` for KPIs, add sort comparator.
- Existing test files — extend with the new behaviors.

## must-NOT-touch

- The 6 shipped UI components — they're stable.
- The locked design — only fill in real data.
- `BrandLoudHero` — this is a brand-restrained page; do NOT introduce.
- Worker-pages — different concern.

---

## Acceptance criteria

- [ ] Migration adds 4 columns to `pending_action`.
- [ ] Existing pending actions get default severity `p2`.
- [ ] New pending actions are stamped with derived severity (P0/P1/P2).
- [ ] Gateway envelope `/api/v1/approvals` includes severity / sla_expires_at / dollar_at_risk / confidence.
- [ ] Gateway endpoint `/api/v1/approvals/kpis` returns 5 KPIs + 7-day rate donut data.
- [ ] Web `<KpiStrip>` shows live KPI data (verifiable: gateway returns 200, strip renders the real numbers).
- [ ] Cards sort severity desc → SLA asc.
- [ ] In `isStrictProduction()`, the sample-data fallback is OFF.
- [ ] In dev mode, the sample-data still includes the 4 fields so the UI looks polished without a gateway.
- [ ] 8+ tests pass.
- [ ] `cargo clippy -p argo-store -p argo-agent-runner -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean in web/.
- [ ] Trust+safety re-audit: Approvals inbox trust score moves from ~5 to 9 on the rubric.

---

## Gotchas

1. **`risk_level` already exists** — don't rename or replace; severity is
   ADDITIVE atop risk_level. The two coexist for backward-compat with
   existing audit consumers.
2. **`confidence_pct` is NULL when worker didn't surface it** — don't
   default to 50 or 100; absent confidence is a real signal ("the worker
   didn't tell us how confident it was"). The donut renders nothing when
   confidence is null.
3. **SLA timezone**: `sla_expires_at_micros` is UTC. Web converts to local
   for display. Don't store local time.
4. **KPI refresh interval (30s)**: gentle enough to not hammer the gateway,
   tight enough to feel live. Don't drop below 15s — that's hammering with
   no upside for a dashboard.
5. **`approval_rate_7d`** SQL: filter `audit_record` to `ApprovalGranted` /
   `ApprovalReplied` (with reject) / `ApprovalExpiredByTimeout` over the
   last 7 days. **Be careful with tenant scoping** — without `tenant_id =
   ?`, the ring shows the WHOLE-WORKSPACE rate (potentially leaking).
6. **The 5th KPI "Avg response"** is computed from
   `decided_at_micros - created_at_micros` for resolved actions. The brief
   assumes `pending_action.decided_at_micros` exists; if not, derive from
   the matching audit row's timestamp.
7. **Severity-tinted row tint** is already in `ApprovalCard` via
   `data-severity={severity}` attribute (verify in the shipped code) +
   matching CSS in the stylesheet. Don't reinvent.

---

## References

- Design lock: `project_approvals_page_design_locked.md` (operator memory)
- Audit: `docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`
- Shipped UI: `web/components/skycot/approvals/` (6 components, all sibling tests)
- Wave-2 prod-fallback fix: `web/lib/api/approvals.ts` (`isStrictProduction()`)
- Pending-action substrate: `crates/argo-store/src/pending_action.rs`
