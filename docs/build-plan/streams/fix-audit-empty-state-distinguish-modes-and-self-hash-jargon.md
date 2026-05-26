# fix-audit-empty-state-distinguish-modes-and-self-hash-jargon

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-empty-state-distinguish-modes-and-self-hash-jargon`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-6-claim-6-trust-and-approvals.md` F9 [P2] + F13 [P3].

🟡 **P2 — Two related polish defects on the audit feed:**
**(a)** `/audit` empty-state copy "Quiet day · Worker actions and approvals will appear here as they happen" is identical whether the operator is signed out, the gateway is offline, the tenant is fresh, or it's literally a quiet hour. If env-var fixes (sister briefs) deliver an empty queue, Scene 6 narrates "the audit timeline" against a flat uninformative screen.
**(b)** Hash-cell tooltip on every row reads "Self-hash · click to copy" — engineer-y jargon for a buyer-facing surface. Etherscan calls it "transaction hash"; an enterprise audit surface reads cleaner as "Action hash" or "Audit row ID".

## Root cause

### F9 — undifferentiated empty state

`web/app/(app)/audit/page.tsx:397 AuditEmpty` renders the same copy regardless of the underlying reason:
- 0 rows + no filters active
- network error (gateway unreachable)
- signed-out (no tenant context)
- fresh workspace (no actions yet)
- active filters that match nothing

A buyer or investor seeing "Quiet day" can't tell if the system is healthy-but-idle, broken, or filtering them into emptiness.

### F13 — self-hash jargon

`web/components/skycot/ActivityRow.tsx:232` builds the tooltip title as:
```tsx
title={`${label} · click to copy\n${hash}`}
```
Where `label` is "Self-hash" (or "Sample row — not part of the production audit chain" for fixture rows). "Self-hash" is jargon; "Action hash" or "Audit row hash" reads cleaner and matches the visible Etherscan-style design intent without bleeding cryptography vocabulary.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F9 [P2] — `/audit` feed empty-state ("Quiet day") doesn't differentiate signed-out vs no-activity
- scene-6-claim-6-trust-and-approvals F13 [P3] — Hash-chain rows say "Self-hash · click to copy" — operator-facing copy reads engineer-y

## Fix recipe

### F9 empty state

1. **Audit the data-source hints** the feed query returns. The hook (likely `useAuditFeed`) should expose:
   - `isLoading` — fetch in flight
   - `isError` — network/CORS failure
   - `dataSource` — `gateway` / `mock-bypass` / `sample-fixture`
   - `totalRows` — pre-filter row count
   - `filteredRows` — post-filter row count
   - `activeFilters` — what's filtering

2. **Branch `AuditEmpty` copy** at `web/app/(app)/audit/page.tsx:397` (or whatever the empty component is):
   ```tsx
   if (isError) return <AuditUnreachable />;
   if (activeFilters.length > 0 && totalRows > 0) return <AuditNoFilterMatches />;
   if (totalRows === 0 && isFreshWorkspace) return <AuditFreshWorkspace />;
   return <AuditQuietDay />;  // default
   ```
   Each variant has its own copy:
   - `AuditUnreachable`: "Couldn't reach the audit feed. Check your connection or contact support."
   - `AuditNoFilterMatches`: "No rows match the current filters. Clear filters to see all activity."
   - `AuditFreshWorkspace`: "No audit activity yet — your team's actions will appear here as they happen."
   - `AuditQuietDay`: existing "Quiet day" copy (only for tenants with prior activity but no recent rows).

### F13 hash jargon

3. **Rename "Self-hash" copy** at `web/components/skycot/ActivityRow.tsx:232`:
   ```tsx
   const label = isFixtureRow
     ? "Sample row — not part of the production audit chain"
     : "Action hash";  // was "Self-hash"
   ```
4. **Verify any other references** — `grep -rn '"Self-hash"\|Self-hash' web/components/ web/lib/` and update consistently.

### Tests

5. **Add a Playwright spec** at `web/playwright/audit/empty-state-variants.spec.ts`:
   - Boot with no audit rows + no filters → assert `AuditQuietDay` or `AuditFreshWorkspace` copy depending on the dataSource hint.
   - Apply a filter that matches nothing → assert `AuditNoFilterMatches` copy.
   - Mock a 5xx on the gateway → assert `AuditUnreachable` copy.

## Acceptance criteria

- [ ] Audit empty-state copy varies by underlying reason (error / no-match / fresh / quiet).
- [ ] Hash-cell tooltip reads "Action hash" (or "Audit row ID"), not "Self-hash".
- [ ] Fixture rows continue to display their "Sample row" label.
- [ ] New Playwright spec passes.

## May-touch

- `web/app/(app)/audit/page.tsx` (empty-state branching)
- `web/components/skycot/audit/AuditEmpty.tsx` (split into variants OR pass mode prop)
- `web/components/skycot/ActivityRow.tsx:232` (hash label)
- `web/lib/api/audit.ts` (verify hook exposes the data-source hints needed)
- `web/playwright/audit/empty-state-variants.spec.ts` (NEW)

## Out of scope

- Audit feed envelope hash columns (covered by `fix-audit-feed-include-self-hash-in-envelope`).
- Hash-chain modal (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows`).
- HTTP self-trace row filtering (covered by `fix-audit-feed-action-kind-variety`).
- Redaction banner (covered by `fix-audit-redaction-banner-gates-on-rows`).

## Effort

~80-150 LOC + 1 spec. ~3-5 hours. P2 — improves buyer-facing legibility but not a demo-recording blocker.
