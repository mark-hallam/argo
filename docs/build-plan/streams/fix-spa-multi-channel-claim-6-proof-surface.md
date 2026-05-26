# fix-spa-multi-channel-claim-6-proof-surface

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-multi-channel-claim-6-proof-surface`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` Claim 6 rollup.

🟠 **P1 — Claim 6 (multi-channel + multi-tenant) is asserted in /landing copy ("lives in Slack like a colleague") but the SPA has no visible surface that proves a single worker pinged Slack + Email + iMessage in one beat. Investor asks "show me", camera has nothing to point at.**

## Root cause

Claim 6 has two halves:
- **Multi-tenant** — proven by `/api/v1/team` + `/api/v1/cohorts` returning `tenant_id`, and the runtime test `claim_6_live_cross_tenant` pinning isolation. Real, demonstrable.
- **Multi-channel** — asserted in landing copy + connectors marketplace at `/settings/connectors` listing Slack/Gmail/Outlook/Linear/Jira/Asana. But there is NO live timeline view showing "Mariah pinged your Slack at 09:14 and your Email at 09:15."

The audit row schema supports it — every `AuditRow` has a `channel: "slack" | "email" | "web" | "sms" | "voice"` field. The runtime emits these. But the SPA aggregates audit rows by `worker.id` or `verb`, not by `channel × worker`. No surface shows the cross-channel pattern.

For the v1.0 demo MP4, the camera needs ONE recordable beat that says "Mariah handles your Slack + your Email + your iMessage" with provenance the operator can point at.

## Findings this brief closes

- whole-site-mvp-claims-coverage Claim 6 rollup — multi-channel asserted in copy only, no SPA proof beat

## Fix recipe

1. **Add a `<ChannelStrip>` card** to the worker detail page at `web/components/skycot/worker/ChannelStripCard.tsx`:
   - Render the channels this worker has acted on in the last 7 days, with a count per channel.
   - Each channel renders as a row: icon (Slack/Gmail/SMS/web) + count + last-seen relative time + a click-to-filter affordance ("Show me Mariah's Slack activity").
   - Data source: aggregate `/api/v1/audit?worker_id=<id>&range=7d` by `channel`.

2. **Add to WorkerOverviewContent** — slot it between `DutiesCard` and `KpiStrip` so the camera sees it on the worker drill-in (Scene 1/2 finale).

3. **Empty-state copy** when worker has only `web` channel activity: "Mariah hasn't picked up Slack or email yet — connect her at Settings → Connectors." Direct path to `/settings/connectors`.

4. **Multi-channel demo seed**: extend `fix-demo-seed-cli-for-real-roster` (out-of-scope; sibling brief) to emit audit rows for at least one persona across `slack` + `email` + `web` channels in the last 24h, so the ChannelStrip is populated on first paint.

5. **Cross-link to /audit**: each channel row in `<ChannelStrip>` is a `<Link>` to `/audit?worker_id=<id>&channel=<slack|email|...>`. `/audit` filter chip rail already supports channel filtering (confirm with `web/app/(app)/audit/FilterChipRail.tsx`).

6. **Tests**:
   - `web/components/skycot/worker/ChannelStripCard.test.tsx`: render with mixed-channel mock data, assert 3 channel rows.
   - Empty state: only `web` channel → render the connect-CTA.

## Acceptance criteria

- [ ] `<ChannelStripCard>` mounts on `/workers/<id>` overview tab.
- [ ] Rendered with seeded demo data, shows >= 2 channels per persona (Slack + email at minimum).
- [ ] Each channel row links to `/audit?worker_id=<id>&channel=<channel>` and the audit page filters correctly.
- [ ] Empty state directs to `/settings/connectors`.
- [ ] Unit tests cover populated + empty states.
- [ ] Scene 6 / Scene 1 finale recordable: "Look — Mariah pinged Slack 4 times and email 2 times yesterday. Here's the audit trail for one of those Slack messages."

## May-touch

- `web/components/skycot/worker/ChannelStripCard.tsx` (NEW)
- `web/components/skycot/worker/ChannelStripCard.test.tsx` (NEW)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (slot the card)
- `web/lib/api/audit.ts` (verify channel aggregation; add helper `aggregateByChannel(rows)` if missing)
- `web/app/(app)/audit/FilterChipRail.tsx` (verify `channel` URL param is accepted; add if not)

## Out of scope

- New runtime channel adapters (Slack/email/SMS executors are out of scope here — the runtime already emits these)
- Per-channel cost rollup (separate brief — financial reporting is v1.1)
- The demo seed itself (sibling: `fix-demo-seed-cli-for-real-roster` — sequence after)

## Effort

~150-220 LOC across 1 new component + 1 new test + integration into the overview + small client-helper additions. ~4-5 hours. P1 because this is the only Claim 6 multi-channel surface in v1.0; without it the demo narration is unsupported.
