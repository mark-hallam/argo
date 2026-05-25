# fix-audit-redaction-banner-gates-on-rows

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-redaction-banner-gates-on-rows`.
**Source**: e2e-ux-audit Agent (scene-6-claim-6-trust-and-approvals) finding F11 (`docs/skycot/ux-audits/2026-05-25-2309/scene-6-claim-6-trust-and-approvals.md`).

🟢 **P2 — "PII redacted — your role shows masked data" banner renders on `/audit` even when the feed is empty. Buyer reasonably asks "what's being hidden from me?" on a page with nothing to redact.**

## Root cause

`web/app/(app)/audit/page.tsx:202-206` renders the `<RedactionBanner>` unconditionally:

```tsx
<RedactionBanner
  policy={policy}
  presentCategories={presentCategories}
  surface="audit-feed"
/>
```

When `rows.length === 0` AND `presentCategories.length === 0`, the banner still appears claiming "PII redacted — your role shows masked data — Request elevated view →". There's literally nothing to redact. Worse: it reads as "the system is hiding something from you" on a page that's just empty.

Polish nit, not a demo blocker — but the audit page is positioned as the "forensic + compliance" surface and a misleading banner on it undermines trust.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F11 [P2] — Audit page's "PII redacted" banner renders on an empty feed

## Fix recipe

1. Touch site: `web/app/(app)/audit/page.tsx:202-206`.
2. Gate the banner on real redaction signal:
   ```tsx
   {rows.length > 0 && presentCategories.length > 0 && (
     <RedactionBanner
       policy={policy}
       presentCategories={presentCategories}
       surface="audit-feed"
     />
   )}
   ```
3. Verify `<RedactionBanner>` component itself handles the empty-categories case gracefully (in case it's used elsewhere).
4. Pin with a test at `web/app/(app)/audit/page.test.tsx`: render with `rows: []`, assert banner is NOT present; render with rows containing PII categories, assert banner IS present.

## Acceptance criteria

- [ ] `/audit` on empty feed: no redaction banner.
- [ ] `/audit` with rows containing PII categories: banner renders.
- [ ] No regression on the existing copy / styling of the banner when it does render.
- [ ] Test pin passes.

## May-touch

- `web/app/(app)/audit/page.tsx`
- `web/components/skycot/audit/RedactionBanner.tsx` (only if it needs an internal empty guard)
- `web/app/(app)/audit/page.test.tsx` (NEW or extend)

## Out of scope

- The unknown-worker hipaa rows (separate brief `fix-audit-unknown-worker-and-hipaa-fixture-rows`)
- Audit hash-chain column (separate brief `fix-audit-hash-chain-prev-this-hover-and-live-rows`)
- Audit list/detail id mismatch (separate brief `fix-audit-list-detail-endpoint-id-mismatch`)

## Effort

~10-30 LOC + 1 test. ~20 min.
