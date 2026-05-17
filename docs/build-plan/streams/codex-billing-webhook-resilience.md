# codex-billing-webhook-resilience — Stripe webhook retry + dead-letter + replay (P1 follow-up)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — production-hardening judgment required.
**Depends on**: `claude-stripe-subscription-integration` MERGED (#1789).
**Authored against**: origin/main `c2d3417d` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-billing-webhook-resilience`.
**Source**: PR #1789 follow-up — webhook handler ships with HMAC verify + event_id idempotency but no retry, no DLQ, no replay tool. Production needs these for billing recovery.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — billing webhooks are the only path from Stripe → tenant.tier. Lost event = lost revenue.
- **Observability** (#2) — dead-letter table + replay CLI gives operators a tool for billing-incident recovery.
- **Commercial usability** (#5) — enterprise customers can audit + reconcile billing state.

## Scope

P1. Three additions to the existing webhook handler:

### 1. Retry on transient failure

If a webhook event fails to apply (e.g. `tenant.set_tier()` returns an error because the tenant row is locked, or argo-store is mid-vacuum), the handler:
- Returns 500 to Stripe (Stripe will retry per its own schedule: 1h, 3h, 12h, 24h, 48h, then dead).
- Logs the failure with the `event_id` + reason.
- Does NOT mark the event_id as processed in the idempotency table — so Stripe's retry will re-attempt and succeed once the transient condition clears.

### 2. Dead-letter table (migration 112)

After Stripe gives up (after ~5 retries), the webhook never fires again. We need our own dead-letter for events we couldn't apply:

```sql
-- 112__billing_event_dead_letter.sql
CREATE TABLE billing_event_dead_letter (
    event_id        TEXT PRIMARY KEY,
    event_type      TEXT NOT NULL,
    tenant_id       BLOB,                      -- nullable if event has no tenant
    raw_payload     TEXT NOT NULL,             -- canonical JSON for replay
    failed_at       INTEGER NOT NULL,
    failure_reason  TEXT NOT NULL,
    retry_count     INTEGER NOT NULL DEFAULT 0,
    resolved_at     INTEGER                    -- nullable; set when operator marks resolved
);
CREATE INDEX idx_dlq_failed_at ON billing_event_dead_letter(failed_at);
CREATE INDEX idx_dlq_resolved  ON billing_event_dead_letter(resolved_at);
```

Webhook handler writes a row when it gives up retrying (configurable: `[billing.webhook.max_retries]`, default 5).

### 3. Operator replay CLI

```bash
argo billing replay --event-id <id>                    # replay a single event
argo billing replay --since <duration>                 # replay all DLQ events in window
argo billing dlq list                                   # show DLQ entries with reasons
argo billing dlq resolve --event-id <id> --note "..."   # mark resolved without replay (e.g. manual fix)
```

LOC: ~300.

## Files you may touch

- `crates/argo-store/migrations/112__billing_event_dead_letter.sql` (NEW) — additive ALTER. ≤25 LOC.
- `crates/argo-store/src/billing_dlq_query.rs` (NEW) — CRUD. ≤120 LOC.
- `crates/argo-store/src/billing_dlq_tests.rs` (NEW). ≤80 LOC.
- `crates/argo-store/src/lib.rs` — re-export. ≤3 modified LOC.
- `crates/argo-billing/src/webhook.rs` (existing, #1789) — wire retry decision + DLQ write. ≤80 modified LOC.
- `crates/argo-billing/src/webhook_tests.rs` (existing, #1789) — extend with retry/DLQ cases. ≤60 modified LOC.
- `crates/argo-billing/src/replay.rs` (NEW) — replay logic. ≤80 LOC.
- `crates/argo-cli/src/cli_billing_cmds.rs` (NEW or existing) — `Replay` + `Dlq` subcommands. ≤50 modified LOC.
- `crates/argo-cli/src/handlers/billing_replay.rs` (NEW) — handler. ≤80 LOC.
- `crates/argo-cli/src/handlers/billing_dlq.rs` (NEW) — handler. ≤80 LOC.
- `crates/argo-config/src/sections_billing.rs` (existing per #1789 or NEW) — `[billing.webhook]` config. ≤30 modified LOC.
- `docs/skycot/operator/billing-recovery.md` (NEW) — operator runbook. ≤60 LOC.

## Files you must NOT touch

- `stripe_client.rs` (Stripe API wrapper) — replay is local-DB-only; don't call Stripe for the replay.
- Tenant tier mutation surface (`Tenant::set_tier`) — replay calls the same path.
- Audit chain — DLQ writes happen pre-audit; if replay succeeds it emits the audit row.

## Acceptance criteria

- [ ] Migration 112 applies cleanly on fresh + populated DBs.
- [ ] Webhook returns 500 (not 200) when `tenant.set_tier()` fails — Stripe will retry.
- [ ] After `[billing.webhook.max_retries]` (default 5) — webhook returns 200 to Stripe AND writes row to `billing_event_dead_letter`.
- [ ] `argo billing dlq list` shows DLQ entries.
- [ ] `argo billing replay --event-id <id>` re-applies the event from `raw_payload`; if successful, sets `resolved_at`.
- [ ] `argo billing dlq resolve --event-id <id>` marks resolved without calling the actual handler.
- [ ] `argo billing replay --since 24h` re-applies all unresolved DLQ entries since the cutoff.
- [ ] All operations RBAC-gated (admin role only).
- [ ] `cargo test -p argo-billing -p argo-store -p argo-cli` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-billing-webhook-resilience`
- Title: `feat(billing): webhook retry + dead-letter + replay CLI (codex-billing-webhook-resilience)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1789 MERGED.** Worker verifies `crates/argo-billing/src/webhook.rs` exists.
- **Don't re-verify HMAC on replay.** Replay reads from `raw_payload` which was stored AFTER HMAC verify. Skip the signature step on replay.
- **Idempotency table still respected.** Replay sets the idempotency row to "processed" so a subsequent Stripe retry doesn't double-apply.
- **`resolved_at` is operator action.** If an admin manually fixes a tier and wants to mark the DLQ entry done without replay, `resolve` does that.
- **No retry loop in-process.** Stripe's retry schedule is the retry loop. Our handler just decides 200 vs 500 per attempt.
- **Replay is synchronous.** Operator-driven, not a background job. If a replay batch is large, that's a CLI flag (`--limit 100`) operator concern.
- **Don't auto-resolve from event.deleted.** A canceled subscription has its own webhook; DLQ entries persist for operator audit.
