# claude-support-ticket-queue-integration — Customer support ticket queue (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-tenant-provisioning-endpoint` MERGED (so tickets carry tenant_id).
**Authored against**: origin/main `489e0580` on `2026-05-16`.

## Scope

P1. Customer support ticket queue. For v1, integrate with **Plain.com** or **Linear** — both have good email-to-ticket routing.

Components:
- `support@skycot.com` → forwards to ticket queue.
- In-app help widget on `app.skycot.com` (bottom-right) → opens a contact dialog → creates a ticket.
- Per-tenant context auto-attached: tenant_id, user_email, plan tier.
- SLAs documented: Free → 48hr, Pro → 24hr, Enterprise → 4hr.

LOC: ~300.

## Files you may touch

- `web/components/ui/HelpWidget.tsx` (NEW) — bottom-right floating help. ≤120 LOC.
- `web/app/(app)/layout.tsx` — narrowly: mount `<HelpWidget />`. ≤10 modified lines.
- `web/app/api/support/ticket/route.ts` (NEW) — POST that forwards to Plain/Linear webhook. ≤60 LOC.
- `web/lib/support/use-create-ticket.ts` (NEW). ≤50 LOC.
- `deploy/support/README.md` (NEW) — operator setup runbook (Plain or Linear). ≤80 LOC.
- `docs/skycot/support/sla.md` (NEW) — per-tier SLA documentation. ≤40 LOC.

## Acceptance criteria

- [ ] HelpWidget renders on every authenticated SPA page.
- [ ] Submit opens a dialog with email + message inputs.
- [ ] Submit forwards to ticket queue with tenant context.
- [ ] SLAs documented + linked from widget.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-support-ticket-queue-integration`
- PR title: `web: support ticket queue + help widget (claude-support-ticket-queue-integration)`

## Notes

- Plain.com has a clean API + good free tier. Linear has Issues API. Pick one; operator's choice.
- Don't build a full ticket UI in-app for v1; just enable submission. Replies arrive via email.
