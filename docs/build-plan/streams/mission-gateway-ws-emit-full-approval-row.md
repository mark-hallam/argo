# Mission load-bearing wire — /inbox/ws emit full ApprovalItem payload

**Owner**: claude. **Depends on**: J3 (MERGED #1418) structured-event vocabulary; E1-f (MERGED #1420) consumer.
**Authored against**: origin/main `f1fa23f3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-ws-emit-full-approval-row`.

> SOURCE: E1-f #1420 PR-deferral. The frontend currently runs `invalidateQueries(["approvals"])` on `approval.created` because J3's payload is the bare `WebInboxRow` (operator_id, approval_id, risk_level, prompt_body, expiry, enqueued_at) — it lacks the fields the SPA needs to build a correctly-shaped `ApprovalItem` (worker.name, verb, detail, consequence_if_approved, consequence_if_refused, reason, channel, received_at). This brief widens the wire payload to carry the full row so the SPA can do a surgical insert into the TanStack cache without a follow-up refetch.

## Scope

Extend the `approval.created` envelope on `/inbox/ws` so subscribers receive a payload that is shape-compatible with the JSON returned by `GET /api/v1/approvals/<id>` (the source of truth for `ApprovalItem`). Two surfaces change:

1. **Publish-time enrichment** in `argo-channels::WebInboxChannel::send` (which is the global event hook that fans `ChannelEvent::ApprovalCreated` out to subscribers). The current call site has `WebInboxRow` in hand; the wider fields live on the `pending_approval` row in `argo-store`. Add an enrichment fetch (single SELECT joining `pending_approval` + `worker` + `worker_action`) and emit a new `ApprovalRowFull` variant alongside the existing `WebInboxRow` (subscribers ignore unknown fields).
2. **Wire envelope** on `WebInboxEvent::ApprovalCreated` widens `payload` from `WebInboxRow` to a new `ApprovalRowFull` DTO. The SPA's E1-f hook can then switch from `invalidateQueries` to `setQueryData` for the surgical insert.

Tenant-scoped — the per-tenant tokio broadcast registry stays unchanged. Falls back gracefully: if the enrichment query fails (worker deleted in the same tick, racy delete), publish the existing `WebInboxRow` shape under a `partial: true` flag and the SPA invalidates as before.

This is NOT in scope: changing `approval.{decided,undone,expired}` payloads (they already carry the approval_id which is sufficient for those events); migrating the legacy HTML inbox text-frame; adding new fields to `pending_approval` schema; reworking the channel registry.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-channels/src/webinbox.rs` (≤ 270 LOC after) — enrich publish path, define `ApprovalRowFull` DTO + `From<...>` impls
- `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs` (≤ 380 LOC after) — widen `WebInboxEvent::ApprovalCreated::payload` type
- `crates/argo-cli/src/handlers/gateway/ws_inbox_events_tests.rs` (≤ 380 LOC after) — add 3 unit tests: (a) full-row enrichment happy path, (b) worker-deleted race → partial-row fallback, (c) cross-tenant isolation under enriched payload
- `web/lib/api/types.ts` (≤ 180 LOC after) — extend the `WireApprovalCreatedPayload` interface
- `web/lib/hooks/use-approvals-live.ts` (≤ 200 LOC after) — flip `approval.created` from `invalidateQueries` to `setQueryData` surgical insert when payload carries the full row; keep the invalidate fallback when `partial: true`
- `web/lib/hooks/use-approvals-live.test.ts` (NEW, ≤ 180 LOC) — 4 vitest cases: (a) surgical insert keeps page position, (b) partial fallback invalidates, (c) dedup if approval_id already present, (d) wrong-shape payload doesn't crash

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-channels/src/voice.rs|whatsapp*|telegram*|signal*` — sibling channels unchanged
- `crates/argo-cli/src/handlers/gateway/ws_upgrade.rs` — the WS upgrade path is stable post-J3
- `web/app/(app)/page.tsx` — already at 458 LOC; no UI change needed for this brief

## Acceptance criteria

- [ ] `WebInboxEvent::ApprovalCreated::payload` is `ApprovalRowFull` (or `WebInboxRowPartial` with `partial: true`)
- [ ] `ApprovalRowFull` carries: approval_id, worker_id, worker_name, verb, detail, consequence_if_approved, consequence_if_refused, reason, channel, risk_level, received_at, expires_at
- [ ] Enrichment is single SQL roundtrip (no N+1)
- [ ] Worker-deleted race emits `partial: true` row, not a 500
- [ ] Cross-tenant isolation property holds (existing test extended)
- [ ] SPA hook does `setQueryData` on full payload, `invalidateQueries` on `partial: true`
- [ ] Bundle delta < 2 kB First Load JS (mostly type widening)
- [ ] All touched files < 500 LOC
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `pnpm typecheck` clean
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-gateway-ws-emit-full-approval-row`
- PR title: `gateway+web: /inbox/ws approval.created emits full row (mission-gateway-ws-emit-full-approval-row)`

## Notes for the implementer

- The enrichment join lives in `argo-store` — likely a new method `ApprovalStore::row_full_by_id` (sibling of existing `row_by_id`). Co-locate the SQL with the existing single-row reader.
- Don't widen `WebInboxRow` itself — that struct is the legacy HTML-inbox transport. Define a new `ApprovalRowFull` on the J3 wire envelope so HTML and JSON subscribers can drift independently.
- The SPA today uses `invalidateQueries(["approvals"])` — this triggers a full `GET /api/v1/approvals` refetch. Surgical insert keeps the operator's scroll position and avoids the 200 KB refetch on every new approval, which adds up on a busy day.
- For the partial fallback flag: include it inside the payload (`{ row: ..., partial: true }`) rather than a sibling envelope field, so subscribers that don't know about partials still receive a usable row.
- E1-f's `useApprovalsLive` already has a `LiveIndicator` status branch — partial fallback should NOT flip the indicator to `polling`. It's a brief degradation, not a transport failure.
