# fix-spa-inbound-channels-webhook-urls-and-recent-activity

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-inbound-channels-webhook-urls-and-recent-activity`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-6-claim-6-trust-and-approvals.md` F9 [P2 → P1 for v1.0 if Scene-6 narrates this].

🟠 **P1 — Inbound channel webhooks (SMS via PR #2224 + Discord via #2222 + Voice DTMF via #2208) are server-side reachable but invisible in the SPA. Operator can't see: (a) webhook URLs to paste into Twilio/Discord, (b) recent inbound activity, (c) a test button. For Scene 6's "Claim 6 multi-channel" narrative, operator can't point at the inbound surface and say "approvals come back from any of these channels".**

## Root cause

PR #2224 ships `POST /twilio/sms/decision`. PR #2222 ships `POST /channels/discord/decision`. PR #2208 ships `POST /twilio/voice/decision`. All routes are server-reachable (verified — SMS returns 500 without HMAC signature). But there's no SPA surface showing them. The closest is `/settings/notify-channels` which is OUTBOUND-only. Inbound URLs live as `hint:` strings in `web/lib/notify-channels/registry.ts:198` env-var requirement list.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F9 [P2] — Inbound channels (SMS / Discord / Voice DTMF) invisible to the operator in the SPA

## Fix recipe

1. **Add an "Inbound" tab to `/settings/notify-channels`** (or a new top-level `/settings/inbound-channels`). The tab shows three sub-sections (SMS / Discord / Voice DTMF), each with:
   - **Webhook URL** to paste into Twilio / Discord console (copy-to-clipboard button + show full URL like `https://<your-skycot-host>/twilio/sms/decision`)
   - **HMAC secret** (masked, copy button) for signing inbound payloads
   - **Status** (active / inactive based on whether the env var + secret are configured)
   - **Recent inbound events** (last 20, with timestamp / from / decision summary / status)
   - **Send test** button that opens a modal explaining how to test (e.g. for SMS: "Reply YES to last alert" or use the Twilio test console)

2. **Backend endpoint to list recent inbound events**:
   - `GET /api/v1/inbound-channels/{kind}/recent` returning the last N rows from the audit feed filtered to `verb in [inbound.sms.received, inbound.discord.interaction, inbound.voice.dtmf]`

3. **Update `web/lib/notify-channels/registry.ts`** to declare an inbound-shaped variant for each of the three channels alongside the outbound counterparts. Reuse the existing `hint:` URLs as the displayed webhook target.

4. **Add a Playwright spec** at `web/playwright/settings/inbound-channels-shows-webhook-urls.spec.ts`:
   - Load `/settings/inbound-channels` (or the new tab)
   - Assert SMS / Discord / Voice DTMF sections present
   - Assert each section has a webhook URL displayed
   - Assert copy-to-clipboard button present

5. **Update the v1-mission-mvp-runbook** for Scene 6 to optionally include the inbound surface beat ("here's where SMS approvals come back from operations").

## Acceptance criteria

- [ ] `/settings/notify-channels` (or new route) has an Inbound tab
- [ ] Inbound tab shows 3 sections: SMS / Discord / Voice DTMF
- [ ] Each section displays the webhook URL and copy button
- [ ] Each section lists last 20 inbound events from the audit feed
- [ ] New Playwright spec passes
- [ ] Scene 6 runbook references this surface

## May-touch

- `web/app/(app)/settings/notify-channels/page.tsx` (add tab) OR new `web/app/(app)/settings/inbound-channels/page.tsx`
- `web/lib/notify-channels/registry.ts` (inbound shape)
- `web/lib/api/inbound-channels.ts` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_inbound_channels.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/dispatch.rs` (register new route)
- `web/playwright/settings/inbound-channels-shows-webhook-urls.spec.ts` (NEW)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 6 update)

## Out of scope

- The inbound handler implementations themselves (already shipped)
- HMAC signing UX (operator pastes secret into Twilio/Discord console; no UI flow)
- Webhook authentication beyond HMAC (separate concern)

## Effort

~400-700 LOC across SPA + backend route + spec + runbook. ~10-15 hours. P1 if Scene 6 narrates this beat; P2 otherwise.
