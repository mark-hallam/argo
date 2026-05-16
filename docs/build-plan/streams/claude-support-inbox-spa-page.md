# claude-support-inbox-spa-page — operator triage page for inbound support tickets (Wave 4 P2)

**Owner**: claude. **Depends on**: `claude-support-ticket-queue-integration` MERGED (#1762, cbb5aa7).
**Authored against**: origin/main `2b984b94` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-support-inbox-spa-page`.
**Source**: PR #1762 follow-up — ticket creation shipped (POST `/api/support/ticket` → webhook out to Plain/Linear), but no in-SPA triage view for operators.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — single pane for operator to see + reply to support requests without leaving SKYCOT.
- **Commercial usability** (#5) — visible support engagement for paying customers.

## Scope

P2. New SPA route at `/support/inbox` (admin-only) showing inbound tickets with reply UI.

Source-of-truth for tickets: a NEW `support_ticket` table (no existing storage — current path is webhook-out only). This brief adds the storage layer + the triage page.

### Backend additions

```sql
-- migration 109__support_ticket.sql
CREATE TABLE support_ticket (
    id              BLOB PRIMARY KEY,
    tenant_id       BLOB,                          -- nullable until enriched
    user_email      TEXT NOT NULL,
    subject         TEXT NOT NULL,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open',  -- 'open' | 'pending' | 'resolved' | 'closed'
    priority        TEXT NOT NULL DEFAULT 'normal', -- 'low' | 'normal' | 'high' | 'urgent'
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    assigned_to     BLOB                            -- UserId of operator handling, nullable
);
CREATE INDEX idx_ticket_status   ON support_ticket(status);
CREATE INDEX idx_ticket_tenant   ON support_ticket(tenant_id);

CREATE TABLE support_ticket_reply (
    id              BLOB PRIMARY KEY,
    ticket_id       BLOB NOT NULL,
    author_kind     TEXT NOT NULL,                 -- 'user' | 'operator' | 'system'
    author_id       BLOB,                          -- nullable for 'system' and 'user'
    body            TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (ticket_id) REFERENCES support_ticket(id)
);
CREATE INDEX idx_reply_ticket ON support_ticket_reply(ticket_id);
```

### API surface
- `GET /api/v1/support/tickets?status=open&limit=50` — paginated list, admin-only (RBAC).
- `GET /api/v1/support/tickets/:id` — single ticket + thread.
- `POST /api/v1/support/tickets/:id/reply` — operator reply.
- `PATCH /api/v1/support/tickets/:id` — update status / priority / assignee.

### SPA additions

- `/support/inbox` — Kanban-style 3-column view (Open / Pending / Resolved).
- `/support/inbox/:id` — thread view with reply box.
- Auth-gated to `role.owner` or `role.admin` only.

### Modify existing ticket-create flow

- `POST /api/support/ticket` (existing from #1762) — now also persists to `support_ticket` table; webhook send continues but is in-addition-to storage (not in-place-of).
- If webhook delivery fails, ticket still stored (graceful degradation).

LOC: ~600.

## Files you may touch

**Backend (Rust):**
- `crates/argo-store/migrations/109__support_ticket.sql` (NEW) ≤25 LOC.
- `crates/argo-store/src/support_query.rs` (NEW) — CRUD + list/filter. ≤180 LOC.
- `crates/argo-store/src/support_query_tests.rs` (NEW). ≤120 LOC.
- `crates/argo-store/src/lib.rs` — re-export. ≤3 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_support_tickets.rs` (NEW) — list/get/reply/update handlers. ≤180 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_support_tickets_tests.rs` (NEW). ≤120 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_support_ticket_create.rs` (extend, added by #1762 — file may also be `routes_support.rs`; worker grep first) — add persistence side-effect. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — mount new routes. ≤10 modified LOC.

**SPA:**
- `web/app/(app)/support/inbox/page.tsx` (NEW) — Kanban view. ≤180 LOC.
- `web/app/(app)/support/inbox/[id]/page.tsx` (NEW) — thread view. ≤140 LOC.
- `web/lib/support/use-tickets.ts` (NEW) — React Query hooks for list/get/reply/update. ≤80 LOC.
- `web/components/support/TicketCard.tsx` (NEW). ≤60 LOC.
- `web/components/support/TicketThread.tsx` (NEW). ≤100 LOC.
- `web/__tests__/support/*.test.tsx` (NEW). ≤80 LOC.

## Files you must NOT touch

- Existing `HelpWidget` (from #1762) — operator-side only, the help widget stays as user-facing submit surface.
- Auth / RBAC system — consume the existing `Principal.role` check.
- Email/webhook adapters — out of scope; this brief is in-app surface only.

## Acceptance criteria

- [ ] Migration 109 applies cleanly.
- [ ] `POST /api/support/ticket` persists to `support_ticket` table even on webhook delivery failure.
- [ ] `GET /api/v1/support/tickets?status=open` returns admin's paginated tickets.
- [ ] Non-admin user → 403 on the `/api/v1/support/*` endpoints.
- [ ] Kanban page renders 3 columns; drag-to-update-status persists.
- [ ] Reply box posts via `POST /reply` and refreshes thread.
- [ ] `cargo test -p argo-cli -p argo-store` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] `pnpm typecheck && pnpm test && pnpm lint` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-support-inbox-spa-page`
- Title: `feat(support): operator inbox + persistent ticket store (claude-support-inbox-spa-page)`
- Worker verification: crate-scoped + workspace clippy + web checks per SWARM_PROTOCOL.md.

## Hidden constraints

- **Status as string enum.** Use string discriminator instead of int for ergonomics; trivial size cost, far better operability.
- **No `unsubscribe` button** — these are inbound support requests; no opt-out concept. Resolve/close instead.
- **Replies go via webhook out + email.** When operator replies, the route triggers an outbound notification (email send via `argo-email-templates` SupportReplyEmail template, available now per #1766). Out of scope for this brief — file a follow-up `claude-support-reply-outbound-wire` if not landed yet.
- **Tenant enrichment is best-effort.** Tickets created from unauthenticated marketing-site contact form have `tenant_id=NULL`; admin can manually assign later.
- **No ticket assignment workflow this round.** `assigned_to` column exists for forward-compat; UI doesn't expose it yet — file follow-up.
