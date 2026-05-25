# Stream mission-slack-inbound-tenant-mismatch-audit-emission — emit audit row when Slack inbound resolves to a different tenant than the addressed session

**Owner**: codex (backend, Rust, Slack connector).
**Depends on**: none.
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-slack-inbound-tenant-mismatch-audit-emission`.

🟡 **P2.** Source: round-50 evaluator finding (`round-50-pr-2257-channels-cross-tenant-negative-pre-merge-verify.md`) + r51/r52 carry-forward (3rd-consecutive round). PR #2257 added the negative test that proves cross-tenant inbound replies are rejected silently — round-50 flagged the next-level gap: there's no AUDIT ROW emitted when a tenant mismatch is detected, which means cross-tenant probing is silent on both sides (good) AND silent in the audit log (bad — operators can't detect probing).

## Scope

In the Slack inbound handler (the function that processes Slack events / webhook POSTs and routes them to the addressed approval/session/worker), when the resolution finds a `session.tenant_id` that does NOT match the incoming Slack workspace's bound tenant, emit an audit row with:

- `kind`: a new variant (e.g. `cross_tenant_inbound_rejected`) — propose-and-define if not already present in `crates/argo-audit/src/...` enum.
- `tenant_id`: the **addressed** session's tenant_id (the would-be victim of leakage).
- `actor`: the **probing** tenant_id (or "unknown" if Slack workspace isn't bound to any tenant).
- `payload`: the message_ts + channel_id + redacted text-hint (first 40 chars, no PII).
- Severity: medium (probing detected, no data leaked).

The existing rejection path (silent) is the right runtime behavior — DO NOT change rejection semantics. Only ADD the audit emission inside the rejection branch.

## Grep before write

Find the inbound handler:
```
rg -n "fn handle_inbound|fn on_slack_event|InboundReply" crates/argo-mcp-connector-slack/src/ crates/argo-cli/src/handlers/ crates/argo-channels-*
```

The PR #2257 file diff is the load-bearing locator — open `gh pr diff 2257` to find exactly which function got the negative-test treatment. Add the emission inside the same function, in the rejection branch.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [Slack inbound handler]

## Files you may touch

- The Slack inbound handler module (TBD via grep above; likely `crates/argo-mcp-connector-slack/src/inbound.rs` or `crates/argo-cli/src/handlers/slack_*.rs`).
- `crates/argo-audit/src/...` — add `cross_tenant_inbound_rejected` to the audit-kind enum + the matching `AuditRow` constructor helper (canonical type per the 2026-05-15 type-canonicalization audit).
- The corresponding test file added in #2257 — extend with 1 assertion: after the rejection, query the audit store and assert exactly 1 row of the new kind exists, with the correct addressed-tenant + probing-tenant fields.

## Tests

- Extend the #2257 cross-tenant negative test (~30 LOC additional assertions).
- Add 1 unit test in `crates/argo-audit/...` for the new kind's serialization round-trip.

## Definition of done

- Audit kind `cross_tenant_inbound_rejected` defined + serializes cleanly.
- Slack inbound rejection branch emits the audit row.
- Extended #2257 test asserts the audit row appears with correct fields.
- `cargo test -p argo-mcp-connector-slack` green; the canonical channels test green.
- PR body includes a sample audit row from the new kind (anonymized).

Notes: do NOT change the rejection semantics — operators rely on silent rejection for security. The audit row is for backend visibility only; the Slack client sees the same "no reply" UX as before.
