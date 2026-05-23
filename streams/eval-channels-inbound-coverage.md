# eval-channels-inbound-coverage

**Authored against:** `origin/main` e03680c99 on 2026-05-24 (conductor tick-131).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-channels-inbound-coverage`
**Owner:** codex (or claude)
**Priority:** 🟡 P2 — CLAIM-6 substrate (multi-channel approval, inbound reply path)
**Source:** `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` row `Channel inbound paths` (🔴 GAP).

Mission claim 6 says "each channel impl handles BOTH outbound (request approval) AND (where applicable) inbound (receive the operator's reply)". On origin/main I see `argo-channels/src/email_inbound.rs` and `slack_inbound.rs` only — the other 9 channel ids (discord, imessage, matrix, push, sms, teams, voice, webinbox, whatsapp) appear to have no inbound path. Need to lock the inventory.

## What to do

Read `crates/argo-channels/src/` end-to-end. Build an authoritative table of the 11 channel ids × {outbound dispatcher arm in `worker_actions_notify_env.rs`, inbound webhook route in `argo-cli/src/handlers/gateway/`, inbound message handler in `argo-channels/src/<channel>_inbound.rs`}.

Write `crates/argo-channels/tests/inbound_coverage_pin_test.rs` (or similar) that pins the inventory:

1. **Outbound dispatch coverage**: assert which channel ids `worker_actions_notify_env::send_notify` routes (vs returns `UnsupportedKind`). Sibling pattern: `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs`.
2. **Inbound webhook coverage**: assert the gateway exposes inbound routes for which channels (e.g. `POST /webhooks/slack`, `POST /webhooks/email`, etc.) — grep `crates/argo-cli/src/handlers/gateway/**/*.rs` for `inbound`/`webhook`.
3. **Inbound message handler coverage**: assert which `*_inbound.rs` files exist in `crates/argo-channels/src/`.
4. **Approval-reply consumer**: trace from each inbound entry point to the approval-decision sink (`argo-orchestrator::resolve_approval` or equivalent). Note any inbound route that does NOT reach the sink.

Write a report `docs/skycot/evals/scheduled/2026-05-24/eval-channels-inbound-coverage.md` (note: 2026-05-24 dir; create if missing). For every gap (channel with outbound but no inbound), flag whether the approval flow can complete via channel-fallback (operator can reply on a different channel) or is **stuck**.

## May-touch

- `crates/argo-channels/tests/inbound_coverage_pin_test.rs` (new)
- `docs/skycot/evals/scheduled/2026-05-24/eval-channels-inbound-coverage.md` (new)
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (flip the `Channel inbound paths` row + back-cite)

## Acceptance

- 4 inventory-pin test groups pass.
- Report committed listing the 11 × 4 matrix.
- Per-crate + workspace clippy clean.
- No production source touched (audit-only — if a fix is needed, queue a separate fix-brief in the ledger).

## LOC budget

~300-500 LOC (test + report). Sibling: `eval-channels-v1-config-vs-code-inventory` (already MERGED).
