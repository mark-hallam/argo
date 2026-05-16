# SKYCOT public status page — `status.skycot.com`

Operator runbook for standing up a public status page at
`status.skycot.com` backed by a hosted uptime-monitoring service.

This is the v1 path. An in-house Next.js status page lives in scope
for v1.x; for the 48hr weekend wave the hosted option ships in
~15 minutes of operator clicks and zero application code.

## Why hosted

- **No new app to operate.** No additional Fly machine, no Vercel
  project, no nightly cron to babysit. The status page must be up
  when the rest of the stack is down — running it on infra adjacent
  to what it monitors defeats the point.
- **Multi-region probing for free.** Hosted services probe from 5+
  global PoPs. We would otherwise need to stand that up ourselves.
- **Operator-posted incidents + email/RSS subscriptions** are first-
  class features rather than ~300 LOC of Next.js + Postgres + a
  notification provider.
- **Cost.** Better Uptime free tier covers 10 monitors at 3-min
  resolution, public status page included. UptimeRobot free tier
  covers 50 monitors at 5-min resolution. Either fits v1 needs.

## Recommended: Better Uptime

[betteruptime.com](https://betteruptime.com) (now branded "Better
Stack Uptime"). Free tier sufficient for v1.

### One-time setup (~15 min)

1. **Sign up** at [betteruptime.com](https://betteruptime.com) with
   the operator email. Use a shared `ops@skycot.com` alias once SES
   is configured; until then `operator@example.com` is fine.
2. **Create monitors** — three at minimum, all HTTP(S) GET, 3-min
   interval, alert-after-2-failed-checks:
   - `https://skycot.com` — marketing site, expect 200.
   - `https://app.skycot.com` — SPA shell, expect 200.
   - `https://api.skycot.com/health` — gateway health,
     expect 200 + body contains `"status":"ok"`. The gateway's
     `/health` response is contracted in
     `crates/argo-cli/src/handlers/gateway/routes_health.rs` and
     asserted by `deploy/smoke/checks.ts`.
3. **Create a status page** (Better Uptime → Status pages → New):
   - Subdomain: `skycot` (`skycot.betteruptime.com` interim URL).
   - Custom domain: `status.skycot.com` (configured in step 4).
   - Attach all three monitors above.
   - Branding: upload `web/public/brand/skycot-wordmark.svg` (or
     the dark variant alongside it) and `web/public/favicon/favicon.svg`.
     Set accent colour to brand token `--brand-primary` — deep indigo
     `oklch(0.55 0.18 256)`; if Better Uptime only accepts sRGB hex,
     convert via the swatch in `docs/skycot/brand/DESIGN_TOKENS.md`.
   - 90-day uptime history (default).
   - Subscribe-to-updates: enable email; RSS is on by default.
4. **DNS** — add a CNAME record in Cloudflare (or whichever DNS
   provider per `docs/build-plan/streams/codex-dns-cloudflare-setup.md`):
   ```
   status.skycot.com.  CNAME  skycot.betteruptime.com.
   ```
   Better Uptime auto-provisions a Let's Encrypt cert for the custom
   domain once DNS resolves; allow ~5 min.
5. **Verify** — `curl -sI https://status.skycot.com` returns 200
   and `dig status.skycot.com` resolves to a Better Uptime IP.

### Posting an incident

Better Uptime dashboard → Status pages → Incidents → New. Title +
body in Markdown; severity selector; affected components. Subscribers
get an email immediately; RSS feed updates within ~60s.

For the v1 communications process see
`docs/skycot/operator/incident-response.md` (TBD) — until then,
operator discretion. Keep titles short, post updates every 30 min
during active incidents, resolve when monitors recover.

## Alternative: UptimeRobot

Same shape, slightly worse UX. Use only if Better Uptime sign-up
fails for some reason. Free tier covers 50 monitors at 5-min
resolution, public status page included.

DNS:
```
status.skycot.com.  CNAME  stats.uptimerobot.com.
```

## v1.x: in-house status page

Tracked separately. Lives as a Next.js app under `status-page/`
(NEW). Polls `api.skycot.com/health` on a 60s interval, renders
green/yellow/red dots, 30-day history from a Postgres timeseries
table, operator-posted incidents from a small admin UI, RSS via
`next-rss`. ~400 LOC.

Move off hosted once we have:
- A second region for the gateway (so the in-house status page is
  not collocated with the thing it monitors).
- An incident workflow more complex than what Better Uptime models.
- A reason to white-label per tenant (sub-status pages).

None of those are v1 needs.
