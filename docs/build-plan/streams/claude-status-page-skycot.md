# claude-status-page-skycot — status.skycot.com (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED + `codex-hosted-deployment-flyio` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.

## Scope

P1. Public status page at `status.skycot.com`. Simple Next.js app polling `api.skycot.com/health` + showing uptime + incident timeline.

Components:
- Live status: green/yellow/red dot per service (gateway, SPA, marketplace).
- 30-day uptime history.
- Active incidents list (manually posted by operator).
- Subscribe to updates (email).
- RSS feed for status.

Alternative if time-pressed: use a hosted service (Better Uptime, UptimeRobot) and `status.skycot.com` → redirect there.

LOC: ~400 (or ~50 LOC for hosted-service redirect).

## Files you may touch

**If building in-house:**
- `status-page/` (NEW Next.js app).
- Standard Next.js structure + components from design system.
- Polling logic: 60s interval, retry on failure.

**If using hosted service:**
- `deploy/status-page/README.md` (NEW) — Better Uptime setup runbook.
- DNS CNAME `status.skycot.com → status.betteruptime.com/skycot-prod` or similar.

## Acceptance criteria

- [ ] `status.skycot.com` resolves + shows current state.
- [ ] At minimum: green/red dot for each of (skycot.com, app.skycot.com, api.skycot.com).
- [ ] Operator can post incidents.
- [ ] Brand-consistent (uses brand tokens if in-house; configured to match if external).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-status-page-skycot`
- PR title: `infra: status.skycot.com status page (claude-status-page-skycot)`

## Notes

- For 48hr scope: hosted Better Uptime free tier is the recommended path. Operator runs through setup in ~15 min. In-house can come in v1.x.
