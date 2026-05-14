# claude-delete-consumer-mcp-stubs — Delete 4 consumer/redundant MCP connector stubs

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-delete-consumer-mcp-stubs`.

## Scope

Operator decision 2026-05-14: SKYCOT is **enterprise-focused, not consumer**. Of the 7 zero-LOC stub MCP connectors flagged in the Phase A audit, 4 are consumer-targeted or redundant with existing enterprise connectors. Delete them.

**DELETE (consumer or redundant):**
- `crates/argo-mcp-connector-signal/` — consumer messaging; SKYCOT covers messaging via iMessage + SMS already.
- `crates/argo-mcp-connector-telegram/` — consumer messaging; not on the enterprise stack.
- `crates/argo-mcp-connector-mailgun/` — email API; overlaps with shipped SendGrid connector.
- `crates/argo-mcp-connector-resend/` — email API; overlaps with shipped SendGrid connector.

**KEEP AS STUBS (enterprise placeholders, DEFER not delete):**
- `crates/argo-mcp-connector-apollo/` — Apollo.io B2B sales prospecting (enterprise).
- `crates/argo-mcp-connector-zoominfo/` — B2B contact data (enterprise).
- `crates/argo-mcp-connector-zoom/` — enterprise video conferencing.

These 3 stay as zero-LOC stubs with a one-line note in their Cargo.toml description: `(DEFERRED-v1.x; enterprise connector placeholder)`.

LOC: ~150 (mostly directory deletes + workspace Cargo.toml updates + 3 description tweaks).

## Files you may touch

**`git rm -r` (whole-directory deletions):**
- `crates/argo-mcp-connector-signal/`
- `crates/argo-mcp-connector-telegram/`
- `crates/argo-mcp-connector-mailgun/`
- `crates/argo-mcp-connector-resend/`

**TOUCH:**
- Workspace `Cargo.toml` — remove the 4 deleted crates from `[workspace.members]`. **≤8 modified lines.**
- `crates/argo-mcp-connector-apollo/Cargo.toml` — append ` (DEFERRED-v1.x; enterprise B2B sales connector placeholder)` to description.
- `crates/argo-mcp-connector-zoominfo/Cargo.toml` — append ` (DEFERRED-v1.x; enterprise B2B data connector placeholder)`.
- `crates/argo-mcp-connector-zoom/Cargo.toml` — append ` (DEFERRED-v1.x; enterprise video connector placeholder)`.
- `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` — annotate the deletion + the 3 deferred.
- `docs/skycot/V1_DEFINITION.md` — note enterprise-focus decision in §4 Claim 5 row.
- Any other references to the 4 deleted crates surfaced by `rg argo-mcp-connector-signal` / `rg argo-mcp-connector-telegram` / etc. (likely just docs).

## Files you must NOT touch

- The 30 LIVE MCP connectors (slack, gmail, github-issues, jira, linear, asana, airtable, salesforce, zendesk, hubspot, discord, stripe, stripe-events, trello, typeform, figma, clickup, calendly, mailchimp, sendgrid, monday, pagerduty, fireflies, onedrive, image-gen, etc.).
- The 3 STAY-AS-STUB enterprise connectors (apollo, zoominfo, zoom) — only the description field is touched.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 4 stub crate directories removed via `git rm -r`.
- [ ] Workspace `Cargo.toml` no longer lists the 4 deleted crates.
- [ ] 3 enterprise stubs have updated description fields.
- [ ] `rg argo-mcp-connector-signal` returns zero matches in code; doc historical references annotated only.
- [ ] `cargo build --workspace` clean (resolves workspace member removal).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-delete-consumer-mcp-stubs`
- PR title: `cleanup: delete 4 consumer MCP connector stubs; defer 3 enterprise stubs (claude-delete-consumer-mcp-stubs)`

## Notes for the implementer

- Mission-claim alignment: production hardening (workspace cleanup; reduces stale-fixture noise for the audit; aligns with enterprise focus).
- This is a pure delete — no new pub types introduced — so parallel-surface drift check is N/A.
- Don't delete apollo/zoominfo/zoom. They look the same as the consumer ones at first glance (both are 0-LOC stubs), but the operator wants the enterprise placeholders preserved.
