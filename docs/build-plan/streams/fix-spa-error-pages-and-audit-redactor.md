# fix-spa-error-pages-and-audit-redactor

**Owner**: claude or codex (`either-pool`).
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-error-pages-and-audit-redactor`.

🟡 **P2 SPA hardening (3 related defects).** Three small fixes that all surfaced during the 2026-05-24 demo SWOT cold-walk. Bundled because they share the same general theme — the SPA's error surfaces are inconsistent and one of them is triggered by a redactor that doesn't guard against legacy row shapes.

## Why this brief exists

The 2026-05-24 cold-walk surfaced three SPA defects, none individually demo-blocking but together they break the post-onboarding UX. Each is small (≤80 LOC) and worth one brief together rather than three.

1. **`/audit` 500s under demo-mode auth-bypass.** The audit chain itself is alive (verified via `curl /api/v1/audit?range=today` — at least 3 rows returned including `hipaa_compliance_check` entries). The SPA blows up in `redactAuditRow(row)` because the function reads `row.verb` without guarding for `undefined`, and some audit row shapes (notably the cron-emitted compliance check rows) don't carry a `verb` field on every shape variant. Console error:

   ```
   audit hero error: TypeError: Cannot read properties of undefined (reading 'verb')
     at redactAuditRow (web/.../audit/redact.ts:??)
     at AuditPageContent.useMemo[remoteRows]
   ```

2. **`/team` returns a full-page 500 (no sidebar).** Fresh workspaces hit this when redirected from `/workers` after at-least-one-worker exists in store but the team-list endpoint 500s. The error treatment is *also* layout-inconsistent: a contained `<ErrorShell>` inside the app layout (like `/audit`'s error) is the better pattern; full-page errors should be reserved for global app-shell failures.

3. **Error-page layout inconsistency** (SWOT #10 from 2026-05-24). The `/audit` route uses a contained error pattern (sidebar still visible); the `/workflows/[verticalId]/[name]/build/error.tsx` and the `/team` 500 use a full-page treatment (no sidebar). Pick one. The contained pattern is the better UX — keeps the user oriented inside the product. The full-page pattern should only fire on global / layout-boundary failures.

## Source of truth

- 2026-05-24 demo SWOT cold-walk findings (this session's conversation log).
- `web/app/(app)/audit/error.tsx` — the good pattern (contained ErrorShell).
- `web/app/(app)/workflows/[verticalId]/[name]/build/error.tsx` — the bad pattern (full-page).
- `web/components/ui/ErrorShell.tsx` — the shared error component.
- `web/lib/audit/redact.ts` (or equivalent — verify path at claim time) — where `redactAuditRow` lives.

## Scope

### 1. Guard `redactAuditRow` against missing fields

Find `redactAuditRow` and wrap every property read in a nullish-safe pattern:

```ts
// before
const verb = row.verb.toLowerCase();

// after
const verb = (row.verb ?? row.action_type ?? "unknown").toLowerCase();
```

Make the function pure-defensive: if `row` itself is undefined, return a placeholder row with `kind: "unknown"` / `verb: "unknown"` instead of throwing. The audit page should NEVER crash on a malformed row — it should render a `<RowRenderError row={...} />` row that flags the issue inline and continues.

### 2. Convert `/workflows/.../build/error.tsx` to contained ErrorShell pattern

Mirror `/audit/error.tsx`:

```tsx
// web/app/(app)/audit/error.tsx — the pattern to copy
import { ErrorShell } from "@/components/ui/ErrorShell";
export default function AuditError({ error, reset }) {
  return (
    <ErrorShell
      variant="contained"     // <-- key prop
      ...
    />
  );
}
```

Apply the same `variant="contained"` to the workflows error.tsx and to any `error.tsx` under `(app)/` that's currently full-page. The `(app)/` layout includes the sidebar, so contained errors naturally inherit it.

### 3. Add an `/team` 500 graceful path

If `GET /api/v1/team` returns 5xx, render a contained empty-state ("We're having trouble loading your team — try again in a moment") instead of letting the error boundary fire a full-page 500. Use the existing `<EmptyState>` component with the appropriate illustration.

Also confirm: does the team-list endpoint genuinely 500 on fresh workspaces, or does it return an empty list that the SPA misrenders? If the API 500s, that's a separate backend bug worth filing as a follow-up TODO in the BUILD_STATUS — but the SPA still needs the graceful path either way.

## Files you may touch

- `web/app/(app)/audit/error.tsx` (only if the variant prop or styling needs adjusting)
- `web/app/(app)/workflows/[verticalId]/[name]/build/error.tsx`
- `web/app/(app)/team/error.tsx` and/or `web/app/(app)/team/page.tsx`
- `web/lib/audit/redact.ts` (or wherever `redactAuditRow` lives)
- `web/components/ui/ErrorShell.tsx` (only if the contained variant doesn't exist yet)
- `web/components/empty-state/*.tsx` if a new illustration is needed

## Files you must NOT touch

- The gateway audit handler (`crates/argo-cli/src/handlers/gateway/routes_audit.rs`) — audit rows ARE coming through; the bug is purely in the SPA renderer.
- `web/app/(app)/audit/page.tsx` business logic — the redact-guard goes in the redactor, not in the page component.

## Acceptance criteria

- [ ] `/audit` renders without console error when the audit chain contains a row with no `verb` field
- [ ] `/team` shows a contained empty-state when the API returns 5xx (still in the app shell with sidebar)
- [ ] `/workflows/.../build` and any other `(app)/` error.tsx routes use `variant="contained"` ErrorShell
- [ ] A unit test in `web/__tests__/audit-redact.test.ts` covers `redactAuditRow({ kind: "hipaa_compliance_check" })` returning a safe stub rather than throwing
- [ ] Manual: cold-walk through signup → onboarding → /audit + /team + /workflows produces zero full-page 500s

## Verification

```bash
pnpm --dir web build              # type-check + bundle
pnpm --dir web test audit-redact  # the new unit test
bash scripts/check-brief-freshness.sh fix-spa-error-pages-and-audit-redactor
```

Then manually, with the gateway running with demo-mode:

```bash
open http://localhost:3000/audit       # should render rows (or empty-state), no console error
open http://localhost:3000/team        # should render empty-state with sidebar, not full-page 500
open http://localhost:3000/workflows   # ditto
```

## Cross-references

- 2026-05-24 demo SWOT cold-walk findings (#9, #10, N2, N3).
- `web/app/(app)/audit/error.tsx` — pattern to mirror.
