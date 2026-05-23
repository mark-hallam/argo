# eval-channels-v1-config-vs-code-inventory

**Owner**: claude.
**Depends on**: nothing (read-only audit).
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-channels-v1-config-vs-code-inventory`.

🟡 **P1 — 13 channel impls exist in code; only 3 are in the v1.0 Pro tier sales promise.** Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` row `argo-channels` (line 90).

> "13 channel impls present — Discord, WebInbox, Email, Slack, iMessage, SMS, Teams, WhatsApp, Matrix, Voice, Push. Pro tier card says 'Multi-channel (Slack, iMessage, Email)' — 3 of 13 in v1.0 sales promise. Others may be code-shipped but un-configured."

For mission claim 6 (multi-channel approval) to be honestly LIVE in v1.0, operators need to know per-channel: (a) is the impl present? (b) is it wired to the outbound dispatch? (c) can the operator configure it via `/settings/connectors`? (d) which pricing tier promises it?

## Scope

For each of the 13 channels (Discord, WebInbox, Email, Slack, iMessage, SMS, Teams, WhatsApp, Matrix, Voice, Push — and any others discovered in walk):

1. **Impl-present** — does `crates/argo-channels/src/<channel>.rs` exist with non-stub send/receive methods? (grep for `todo!()` / `unimplemented!()`.)
2. **Outbound wired** — does the channel handler register itself in the `worker_actions_notify_*` dispatcher? (grep for the channel name in `argo-channels/src/dispatch*.rs` + `argo-cli/src/handlers/notify*.rs`.)
3. **Inbound wired** — does the channel have an `<channel>_inbound.rs` file with a webhook/poll loop that converts replies → approval decisions?
4. **Operator-configurable** — does the credential-shape exist in `argo-credentials` + does `/settings/connectors/page.tsx` (or any settings page) expose a Connect button for it?
5. **Tier** — which pricing tier promises it? Read `/pricing` page + `docs/skycot/marketing/` tier copy + map.

Output: a 13-row matrix in a new report at `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md` with columns (channel | impl | outbound | inbound | config | tier | verdict).

Verdicts: 🟢 LIVE-in-v1.0 / 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0 / 🔴 STUB-OR-NOT-WIRED.

Surface counts to the gap manifest's `argo-channels` row (flip cell-by-cell) + author 1-3 follow-up fix-briefs if the audit finds 🔴 cells in v1.0-promised channels.

## Files you may touch

- `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md` — NEW report (~300-500 LOC markdown).
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip the `argo-channels` row cells (additive note in `Δ since last audit`).
- `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs` — NEW pin test (~200 LOC) asserting that the 13 known channel ids appear in `Channel::all()` (or similar) so a future delete breaks compile.
- `docs/build-plan/streams/<follow-up-fix-brief>.md` — NEW 1-3 follow-up fix-briefs (only if 🔴 found in v1.0-promised channels).

## Files you must NOT touch

- Production source under `crates/argo-channels/src/` — this is an audit, not a fix.
- Pricing page or marketing copy.
- Gateway routes.

## Acceptance criteria

- [ ] Report enumerates all channels found in `crates/argo-channels/src/` with the 5 columns above.
- [ ] Pin test passes + asserts the 13 channel ids (so any future delete breaks compile).
- [ ] Gap-manifest row updated.
- [ ] If any v1.0-promised channel (Slack, iMessage, Email per the Pro tier card) has a 🔴 verdict, a follow-up fix-brief is authored same tick.
- [ ] PR body summarizes the 3 most-surprising findings.
- [ ] `cargo clippy -p argo-channels --tests -- -D warnings` clean.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Crate-scoped cargo clippy + cargo test for the new pin test.
- Audit shape: peer-reviewed by reading the source not by running real channels.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Fixing 🔴 channels (that's the follow-up brief's job).
- Auditing per-channel cross-tenant isolation (separate brief `eval-channels-cross-tenant-scope-sweep`).
- The inbound coverage matrix specifically — that's a separate brief `eval-channels-inbound-coverage` (which can land independently).

~400-600 LOC across the report + pin test + gap-manifest edit. Read-heavy / write-light evaluator surface.
