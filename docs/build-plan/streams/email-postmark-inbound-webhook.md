# email-postmark-inbound-webhook (FINAL SPRINT) — Postmark inbound event types → full Live

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh email-postmark-inbound-webhook`.

## Why (evidence)
Postmark **outbound** is live; the inbound webhook handles ONLY the approval-decision
reply, not the Postmark event-record types — so the FEATURE_MATRIX row reads:
`docs/build-plan/FEATURE_MATRIX.md:147` "Email — Postmark (approvals) | 🟡 Partial |
Outbound real; **inbound is a stub** (no IMAP/SES)".

Concretely:
- `crates/argo-cli/src/handlers/gateway/routes_channels_email.rs:14`
  `handle_email_webhook` verifies the `X-Postmark-Signature` (HMAC), extracts +
  verifies the approval token, and calls `settle()` (lines 20-52) — that's the
  **approve/reject reply** path only.
- The inbound *event* parser already EXISTS but is **never called outside tests**:
  `crates/argo-channels/src/email.rs:80` `pub fn parse_inbound_event(raw, approval_id,
  decision) -> InboundEvent` (handles `From`/`FromFull`/`StrippedTextReply`/`TextBody`)
  — `rg parse_inbound_event` shows zero non-test callers.
- The route does NOT branch on Postmark's `RecordType` (`Bounce` / `Open` / `Click` /
  `Inbound`) — `rg 'RecordType|Bounce|Click|Inbound' routes_channels_email.rs` is
  empty.

So the gap: branch the inbound webhook on the Postmark `RecordType`, route a genuine
operator **inbound reply** (RecordType `Inbound`) through `parse_inbound_event` →
approval-settle, and acknowledge bounce/click record types (200, audited) instead of
mis-parsing them as approval tokens. This flips email to full Live.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'handle_email_webhook|extract_token|settle|RecordType' crates/argo-cli/src/handlers/gateway/routes_channels_email.rs` — EXTEND this handler; do NOT add a second email webhook route.
- `rg -n 'parse_inbound_event|struct InboundEvent|enum .*ApprovalDecision' crates/argo-channels/src/email.rs crates/argo-channels/src/token.rs` — CALL the existing `parse_inbound_event`; reuse `InboundEvent` / `ApprovalDecision`; do NOT redefine them.
- `rg -n 'pub(super) async fn settle|enum Decision' crates/argo-cli/src/handlers/gateway/routes_channels.rs` — settle through the SAME path SMS/iMessage use; do NOT fork an email-only settle.
- `rg -n 'POST /channels/email/webhook' crates/argo-cli/src/handlers/gateway/routes_channels.rs` — the route is already registered (lines 43-44, 82); reuse it.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_channels_email.rs` (branch on
  `RecordType`: `Inbound`/reply → existing token-verify + `settle`; an explicit
  operator inbound reply with no token → parse via `parse_inbound_event` and settle by
  matched approval id; `Bounce`/`SpamComplaint`/`Open`/`Click` → 200 + an audit row,
  not a 400). Keep < 500 LOC; if it grows, add a sibling
  `routes_channels_email_events.rs`. fns < 80 LOC.
- new (only if needed) `crates/argo-cli/src/handlers/gateway/routes_channels_email_events.rs`.
- `crates/argo-channels/src/email.rs` (ONLY if `parse_inbound_event` needs a small
  additive helper — keep the existing signature/tests green).
- the existing test module + a new test sibling.

## Acceptance (truth-bounded + testable)
- A test: a valid Postmark **Inbound** reply payload (with a verified token) settles
  the matching approval (the same outcome the current reply path produces).
- A test: a Postmark **Bounce** payload returns 200 (acknowledged + audited), NOT a
  400/`missing approval token` — proving the route no longer mis-treats event records
  as approval tokens.
- A signature-failure test stays: a bad `X-Postmark-Signature` still returns 401.
- **curl-verify**: a documented `curl` of a valid signed Inbound payload against the
  running gateway returns `200` (capture the command + response in the PR body; this
  is the truth-bounded "it actually accepts a real payload" evidence).
- `cargo test -p argo-cli` (+ `-p argo-channels` if touched) green.

## On ship
Flip `docs/build-plan/FEATURE_MATRIX.md:147` Email row 🟡 Partial → ✅ Live (inbound
event types wired) with the commit SHA + route in the SAME commit, then
`python3 scripts/render-feature-matrix.py`.

## Effort
S (Rust). ~150–250 LOC incl. tests. Additive; no schema change (reuses
`approval_request` + the existing settle path).
