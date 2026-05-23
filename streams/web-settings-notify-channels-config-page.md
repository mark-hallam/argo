# web-settings-notify-channels-config-page

**Priority**: P2 (operator UX — 11 notify channels are env-var-configured only; no SPA surface)
**Owner**: claude
**Type**: SPA page build mirroring `/settings/connectors` (PR #2138 pattern)
**Size**: ~400-600 LOC across 2-3 new files + 1 settings landing tile flip
**Authored against**: origin/main `cd338914f` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-settings-notify-channels-config-page`

## Why

Per `eval-channels-v1-config-vs-code-inventory` (PR #2145), Skycot ships 11
distinct notify channels — `discord, email, imessage, matrix, push, slack,
sms, teams, voice, webinbox, whatsapp` — split 3🟢 (production-ready: email,
slack, webinbox) / 4🟡 (shipped + wired but not in any tier: discord, push,
sms, voice) / 4🔴 (refused by dispatcher: imessage, matrix, teams, whatsapp;
imessage being un-refused by a sibling brief). **None has an operator-facing
configuration surface** — every channel is env-var-configured only via
`crates/argo-cli/src/handlers/worker_actions_notify_env.rs` `required_env`
helper, and operators have to know the env var names by reading the source.

This is a 4-month-old gap; gap-manifest §"`argo-channels`" lists this brief
explicitly as the P2 follow-up to PR #2145. **The locked design pattern is
already on `main`** via PR #2138 (`web-settings-connectors-page-build`,
MERGED) — `web/app/(app)/settings/connectors/page.tsx`. Mirror it.

## What

Build `web/app/(app)/settings/notify-channels/page.tsx` as a card grid that
mirrors the connectors page exactly:

1. **Hero search bar** — D3 brand recipe per the locked Skills/marketplace
   pattern. Suggestion chips: "Slack", "Email", "iMessage", "Push".
2. **11 cards**, one per channel id. Each card shows:
   - 24x24 SVG logo from `web/public/logos/<channel>.svg` (sourced from
     svgl.app per the `feedback_logos_from_svgl_app` memory; for the 5 ids
     svgl doesn't carry — `webinbox`, `push`, `matrix`, `teams`, `voice` —
     use a lucide-icon fallback as the connectors page does).
   - Channel display name + 1-line description.
   - Status pill: 🟢 "Production" / 🟡 "Wired, not tiered" / 🔴 "Deferred
     v1.x" (verbatim from the audit report).
   - "Configure" button → opens a modal with the env vars the channel
     requires (the source-of-truth list comes from grepping
     `required_env("<channel>", "...")` call sites in
     `crates/argo-cli/src/handlers/worker_actions_notify_*.rs`; do this at
     claim time, not now — the list is small but is the production
     contract).
   - For 🔴 cards: button reads "Roadmap →" and links to
     `/changelog#deferred-v1x`. Disabled-style.
3. **Search filter** by channel id + name + status.
4. **Empty-state**: if the gateway returns 404 on the
   `GET /tenants/:id/notify-channels` shape (gateway doesn't ship this yet
   — see "Gateway surface" below), fall back to a static catalog rendered
   from a TypeScript constant. This is the same 404-→-fallback pattern the
   connectors page uses; the constant lives at
   `web/lib/notify-channels/registry.ts` (new file).
5. **Flip the settings landing tile** at `web/app/(app)/settings/page.tsx`
   so the Notify Channels tile lights up (no longer marked "Coming soon").

### Gateway surface — DO NOT add backend in this PR

The gateway has no `notify-channels` route today. **Do not add one in this
brief.** The connectors page (PR #2138) followed the same staging rule —
the SPA renders against `web/lib/notify-channels/registry.ts` as the
source-of-truth catalog, with a placeholder `lib/api/notify-channels.ts`
that wraps `fetch` and returns the static registry on a 404 from any future
gateway route. A follow-up brief
(`mission-gateway-notify-channels-list-route`) authored separately can
flip the SPA from static catalog to gateway live.

## Acceptance

- [ ] `web/app/(app)/settings/notify-channels/page.tsx` renders 11 cards on
      the locked D3 brand-recipe layout, matches connectors page visual
      register.
- [ ] Settings landing tile flips active.
- [ ] One Playwright spec at `web/tests/e2e/settings-notify-channels.spec.ts`
      that asserts: page loads, 11 cards visible, search-filter narrows to
      1 card on "imessage", all 4 🔴 cards have disabled "Roadmap →"
      buttons.
- [ ] `pnpm --filter @argo/web typecheck && pnpm --filter @argo/web lint`
      clean.
- [ ] `pnpm --filter @argo/web build` succeeds.
- [ ] Existing tests still pass — do not touch tests outside `web/tests/`.

## May-touch

- `web/app/(app)/settings/notify-channels/page.tsx` (NEW)
- `web/lib/notify-channels/registry.ts` (NEW)
- `web/lib/api/notify-channels.ts` (NEW)
- `web/tests/e2e/settings-notify-channels.spec.ts` (NEW)
- `web/app/(app)/settings/page.tsx` (1-line flip on the landing tile)
- `web/public/logos/<channel>.svg` (NEW — 11 logos sourced from svgl.app
  where available; lucide fallback for the others)

**Out of may-touch**: any Rust crate, any gateway route, any existing
connectors page code, any pricing-page copy.

## Parallel surfaces

`rg "settings/notify-channels|notify-channels/registry" web/ 2>&1` returns
zero hits — net-new SPA route. The pattern parallel is the connectors page
which this brief deliberately mirrors; that's intentional (locked design
pattern), not a drift.

## Sensible deviations

- If a channel needs a per-tenant value (most do not — they're env-only
  today), surface a TODO comment in the modal and link to the future
  `mission-gateway-notify-channels-config-write-route` follow-up; do NOT
  build the write side in this brief.
- If svgl.app is missing a logo for a channel that has a recognisable
  brand (slack, discord, etc.), check pixijs or Wikimedia Commons as a
  fallback — operator policy is "always source from svgl.app first" but
  the practical rule is "render a visually consistent card."
- If the 11-id set drifts (e.g. a channel ships or is deleted between
  authoring and claim), update the registry to match what
  `channels_v1_inventory_pin_test.rs` enforces — that test is the live
  source of truth.
