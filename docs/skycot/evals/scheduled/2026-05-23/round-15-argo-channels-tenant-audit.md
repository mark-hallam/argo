# Round 15 — argo-channels tenant-scoping audit

**Date**: 2026-05-23
**Round**: 15
**Pillar / Claim**: Mission claim 6 — Multi-channel approval + multi-tenant by construction
**Verdict**: 🟡 PARTIAL — substrate behaviour matches the workspace-boundary pattern but **ZERO cross-tenant negative tests across 9597 LOC, 35 files**. Same shape as round-12 argo-orchestrator finding. P1.

## Setup

`crates/argo-channels` is 9597 LOC across 35 files spanning 13 outbound channels (email, slack, sms, push, imessage, matrix, teams, discord, voice, whatsapp, webinbox), 3 inbound (email, slack, voice), plus dispatch/fallback/operator-preferences/quiet-hours/rate-limit/token primitives. Audit walked: `lib.rs`, `dispatch.rs`, `message_dispatcher.rs`, `operator_preferences.rs`, `operator_preferences_toml.rs`, `operator_risk_overrides.rs`, `slack_inbound.rs`, `token.rs`, `fallback.rs`, `outbound_message_query.rs` (argo-store dependency).

Tests run:

```
cargo test -p argo-channels --lib
→ 130 passed; 0 failed; 0 ignored
```

## Observed

### 🟢 Tenant-scoped surfaces

**`ProductionMessageDispatcher`** ([message_dispatcher.rs:87-119](crates/argo-channels/src/message_dispatcher.rs:87)) — the production backing of the `send_message` tool. Constructor takes `tenant_id: TenantId` as a required parameter. Every outbound dedup query is tenant-bound:

- [outbound_message_query.rs:88-101](crates/argo-store/src/outbound_message_query.rs:88) — `INSERT … (message_id, tenant_id, channel, …)` binds `tenant_id.0.as_bytes()`.
- [outbound_message_query.rs:117-128](crates/argo-store/src/outbound_message_query.rs:117) — `SELECT … FROM outbound_message WHERE tenant_id = ? AND body_hash = ? AND sent_at >= ?` binds tenant on read.

Net: idempotency cache cannot leak across tenants at the SQL layer. ✅

**Per-channel `send` implementations** (slack/email/sms/push/etc.) all read transport credentials from `<workspace>/argo.toml` via the channel-config silos. Tenant boundary = workspace boundary, same pattern as `argo-credentials` (verified round 9 → 🟢).

### 🟡 Tenant-implicit surfaces (workspace-boundary mitigation)

**`OperatorPreferencesResolver`** ([operator_preferences.rs:177-249](crates/argo-channels/src/operator_preferences.rs:177)) — the central per-operator routing source. Lookup key is `OperatorId` only:

```rust
pub async fn resolve_initial(
    &self,
    op: &OperatorId,
    risk: RiskLevel,
    now: DateTime<Utc>,
) -> ChannelResult<ChannelId>
```

The TOML source reads `[operator.<id>.preferences]` from `<cwd>/argo.toml` (verified at [worker_actions_notify.rs:170-180](crates/argo-cli/src/handlers/worker_actions_notify.rs:170) — `cwd` is the per-tenant workspace). If two tenants ever shared a single workspace and registered the same `operator_id` string ("alice", "admin", "owner"), preferences would collide. **This is mitigated by single-tenant-per-workspace, NOT by anything in argo-channels itself.**

Same shape for `OperatorRiskOverrides` ([operator_risk_overrides.rs](crates/argo-channels/src/operator_risk_overrides.rs)) — also `OperatorId`-keyed, no tenant in the signature.

**`ApprovalTokenClaims`** ([token.rs:14-22](crates/argo-channels/src/token.rs:14)) — the HMAC-signed approval token carries only `approval_id` + `decision` + `exp`. There is no tenant in the token payload. The HMAC secret `ARGO_CHANNEL_HMAC_SECRET` is the implicit tenant binder — different workspaces have different secrets, so a token signed in workspace A cannot verify in workspace B. Inside a single shared workspace, the token grants approval-by-action-id alone (acceptable because action_id is a UUID and the DB lookup binds tenant_id at the gateway layer).

### 🔴 Gap: ZERO cross-tenant negative tests in the entire crate

```
$ rg 'cross.tenant|tenant_a|tenant_b' crates/argo-channels/
(no matches in any of the 35 files)
```

This is the same shape as round 12's argo-orchestrator finding (substrate is right but no regression test pins it). For a 9597-LOC crate that is the load-bearing primitive for mission claim 6 ("multi-channel approval, multi-tenant by construction"), this is a gap. Future refactors that break the workspace-boundary assumption — e.g. ever introducing a shared `OperatorPreferencesResolver` instance across tenants — would not be caught by CI.

## Anti-theater check

This isn't "tests pass therefore safe." The 130 passing tests are happy-path / single-tenant / individual-channel-correctness coverage. None pin the workspace-boundary tenant claim. I read each of the load-bearing files end-to-end and confirmed:

- `ProductionMessageDispatcher::dispatch` literally calls `self.tenant_id` on both query call-sites (lines 154 + 217).
- `OperatorPreferencesResolver::resolve_initial` has no `TenantId` in scope at all.
- `ApprovalTokenClaims` struct definition has 3 fields, none of them tenant-related.

The audit pattern: production code is workspace-isolated by file-system position, not by SQL `tenant_id`. That's intentional for v1 (single tenant per workspace per gateway). The CI gap is that this invariant isn't tested anywhere.

## Recommended brief shapes

### P1 — `eval-channels-cross-tenant-negative-test`

**Why now**: same shape as round-12 argo-orchestrator finding. Authoring 2-3 negative tests would close the regression gap before any of the multi-tenant follow-ups ship.

**Scope** (~150 LOC):
1. `tenant_a_outbound_dedup_does_not_short_circuit_tenant_b_send` — write outbound_message row under tenant_a, attempt same `body_hash` send under tenant_b, assert NO cache hit, NO cross-tenant leak in audit.
2. `tenant_a_approval_token_does_not_verify_under_tenant_b_secret` — sign token under workspace A's `ARGO_CHANNEL_HMAC_SECRET`, attempt verify under workspace B's secret, assert `ChannelError::Auth("approval token signature mismatch")`.
3. `slack_inbound_signature_mismatch_when_workspace_secrets_differ` — same shape for Slack `X-Slack-Signature` verification.

### P2 — `claude-channels-token-bind-tenant-id`

**Why**: forward-deployable hardening. Adding `tenant_id: TenantId` as a 4th claim in `ApprovalTokenClaims` (and re-signing as `{aid}.{dec}.{exp}.{tid}.{sig}`) makes a single shared-workspace deployment impossible to confuse, regardless of secret rotation policy. Backwards-compatible: gateway accepts both 4-part legacy tokens and 5-part tokens during a transition window.

**Caveat**: only if multi-tenant-per-workspace ever becomes a v1 reality. Today it's defence-in-depth, not a real bug.

### P3 or DEFER — `claude-operator-preferences-tenant-scoped-key`

**Why**: low priority because the `cwd`-based loading IS workspace-scoped. Would only matter if a single `OperatorPreferencesResolver` instance ever serves multiple tenants in-process (no current code path does this). Defer until that becomes a real requirement.

## Where the audit pattern continues

Next sessions should pick from the round-8 gap manifest:

- `argo-vertical` loader (claim 3) — Genesis output round-trip
- `argo-tool` surface beyond the 15 in-core (claim 5) — the broader tool registry
- `argo-skill-sdk` (claim 4) — the operator-facing skill-author flow
- 27 MCP connectors per-connector credential-scoping sweep (the round-10 deferred P1)

## Manifest rows touched

In `mission-vs-code-gap-manifest.md` claim-6 section, the argo-channels row should flip from 🔴 NOT AUDITED to 🟡 with brief reference to this report. The 3 brief recommendations above are net-new manifest rows.
