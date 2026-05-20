# web-cleanup-broken-links-sweep — fix remaining broken links + dev-jargon

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cleanup-broken-links-sweep`.
**Priority**: P3 — wave-4 404/500 audit + wave-3 copy/voice audit found residual broken-link + jargon hits.
**Sizing**: ~100-200 LOC (per-link fixes + jargon swaps + tests).

## Why this brief exists

Wave-4 broken-link items still outstanding:
- `HelpWidget` SLA link → `/docs/skycot/support/sla.md` (source-tree path, not a route) on every (app) page
- `/changelog/rss.xml` (icon link, no route exists)
- Auth pages `/help` (top-right corner) → no route
- Signin `/support` link → redirect loop

Wave-3 copy items still outstanding:
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:348` —
  "BrowserExecutor opens FB Marketplace" (dev jargon on customer
  page)
- `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx:76` —
  "We'll draft a workflow TOML" (dev jargon — should say "playbook")

## Goals

1. **HelpWidget SLA link**: either stub a real `/sla` route or remove the link.
2. **/changelog/rss.xml**: add a real `web/app/changelog/rss.xml/route.ts` that emits a minimal RSS feed.
3. **Auth `/help` link**: stub `/help` route OR remove the link from AuthEditorialShell.
4. **`/support` from signin**: route to `mailto:support@skycot.com` or to `/contact`.
5. **"BrowserExecutor opens FB Marketplace"**: → "Your teammate's browser opens Facebook Marketplace".
6. **"workflow TOML"**: → "workflow playbook".

## Steps

1. Per finding: identify file + line + fix.
2. For new routes (sla, help, rss.xml), build minimal pages.
3. For redirects (`/support`), update the link target.
4. For copy swaps, edit text + verify.
5. Add link-checker test that crawls visible href values and asserts each resolves to a 200 or known external URL.

## may-touch

- `web/components/ui/HelpWidget.tsx` (link target)
- New: `web/app/sla/page.tsx`, `web/app/help/page.tsx`, `web/app/changelog/rss.xml/route.ts`
- `web/components/auth/AuthEditorialShell.tsx` or `WizardEditorialShell.tsx` (auth /help link)
- `web/app/auth/signin/page.tsx` (support link)
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (BrowserExecutor copy)
- `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx` (TOML copy)
- New: link-checker test

## must-NOT-touch

- Other (already-fixed) broken links from wave-4 (landing footer, /privacy etc.)
- Operator-decision items (logos, fake counter, etc.)

## Acceptance criteria

- [ ] HelpWidget SLA link resolves (either to /sla or removed).
- [ ] /changelog/rss.xml returns 200 + RSS-formatted XML.
- [ ] Auth /help link resolves.
- [ ] Signin /support links somewhere working.
- [ ] BrowserExecutor + TOML jargon swapped to plain language.
- [ ] Link-checker test passes (no broken hrefs in demo-path).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/404-500/2026-05-20-wave-4-404-500.md`
- Audit: `docs/skycot/audit/copy/2026-05-20-wave-3-copy-voice.md`
