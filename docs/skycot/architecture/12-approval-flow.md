# 12 — Approval flow (multi-channel)

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** new channel added to `EnvChannelSender`, or policy resolution order changes.

How an action moves from "I need approval" to "approved" or "denied" across Slack, email, SMS, push, voice, and web inbox.

## Diagram

```mermaid
sequenceDiagram
    participant W as Worker (argo-agent-runner)
    participant G as Approval gate<br/>handlers/worker_actions_auto_approval
    participant P as Policy resolution<br/>argo-identity::policy
    participant CH as Channel dispatcher<br/>EnvChannelSender
    participant SL as Slack
    participant SMS as SMS / Voice (Twilio)
    participant WS as Web inbox (WS)
    participant OP as Operator

    W->>G: ask_operator(action, risk)
    G->>P: resolve_effective_policy(tenant, role, worker, roster)
    P-->>G: EffectivePolicy { auto_approval, quiet_hours,<br/>require_signatures, audit_chain_required }

    alt Within budget + business hours + auto-approval=true
        G->>W: ✅ auto-approved (audit row emitted)
    else Requires human OR outside policy
        G->>CH: dispatch_with_fallback(channels, risk)
        Note over CH: matches_risk filter applied (F-005 fixed)
        CH->>SL: send to risk-matching Slack channel
        CH->>SMS: fallback to SMS/Voice if Slack delivery fails
        CH->>WS: per-tenant WS broadcast (no more "*" global, F-003 fixed)
        SL-->>OP: notification
        SMS-->>OP: SMS / phone ring
        WS-->>OP: in-app card

        OP->>SL: reply "approve" / "deny"
        SL->>G: webhook with HMAC-verified signature
        G->>G: persist decision + chained audit row
        G->>W: ✅ approved (or ❌ denied)
    end
```

## Channel inventory (live in `EnvChannelSender`)

| Channel | Status | Brief |
|---|---|---|
| `slack` | ✅ shipped | base |
| `email` | ✅ shipped | base |
| `sms` | ✅ shipped | base |
| `push` | ✅ shipped | base |
| `webinbox` | ✅ shipped, post-F-003 fix | #1720 |
| `discord` | ✅ shipped | base |
| `chat` | ✅ shipped | base |
| `sse` | ✅ shipped | base |
| `voice` | 🔄 in flight | `codex-channel-dispatcher-voice-completion` (PR #1741, conflicts) |
| `imessage` | DEFERRED-v1.x | brief explicitly defers |
| `whatsapp` | DEFERRED-v1.x | brief explicitly defers |
| `teams` | DEFERRED-v1.x | brief explicitly defers |
| `matrix` | DEFERRED-v1.x | brief explicitly defers |

## Policy resolution order (F-06 enforced #1742)

`argo-identity::policy::resolve_effective_policy(tenant_id, role, worker_id, roster)` walks in order:

1. Action-specific rules (vertical manifest action gates)
2. Worker policy (worker.toml)
3. Roster policy (team-level inheritance)
4. Workspace policy
5. Workspace defaults

Effective fields enforced (post-F-06):
- `auto_approval` — auto-approve if budget + time window match
- `quiet_hours` — gate-fail outside operator's time window
- `require_signatures` — N-of-M signature threshold
- `audit_chain_required` — fail-closed if AuditSink isn't chain-verified

## Key invariants

- **Risk-appropriate channel.** F-005 fix (#1738) ensures fallback dispatch filters channels by `matches_risk` before sending. Low-risk actions cannot accidentally route to high-risk-only channels and vice versa.
- **Tenant-scoped WS broadcast.** F-003 fix (#1720) removed `WsAuth::Legacy` global `"*"` broadcast — listeners only see their own tenant's events.
- **Audit chain on every state.** Each ask, reply, auto-approve, deny gets a chained audit row.

## Source-of-truth pointers

- Approval gate: `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
- Channel dispatcher: `crates/argo-cli/src/handlers/worker_actions_notify.rs::EnvChannelSender`
- Fallback risk filter: `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs:74` (post-F-005)
- Policy resolver: `crates/argo-identity/src/policy.rs::resolve_effective_policy`
- WS inbox: `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs` (post-F-003)
- Slack signature verification: `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs:97`
