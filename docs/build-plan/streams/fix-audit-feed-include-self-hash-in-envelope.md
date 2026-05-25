# fix-audit-feed-include-self-hash-in-envelope

**Owner**: codex.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-feed-include-self-hash-in-envelope`.
**Source**: e2e-ux-audit (scene-6-claim-6-trust-and-approvals) findings F5 + F11 — `docs/skycot/ux-audits/2026-05-26-0820/scene-6-claim-6-trust-and-approvals.md`.

🟡 **P1 — Audit feed JSON drops the real `self_hash`. SPA seeds synthesise a fake FNV-1a pretender that looks SHA-256-shaped but isn't. Etherscan-style "0xabc1…2345" hash cell renders blank on live data, fabricated on mock data.**

## Root cause

The hash-chain integrity story is actually solid server-side — `crates/argo-audit/src/lib.rs:10` documents `self_hash = sha256(parent_hash || kind || 0x00 || canonical_payload)`. The chain is intact in the database.

But the **JSON projection** at `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` does NOT include `hash` in its envelope. Only the **detail** endpoint `GET /api/v1/audit/<id>` and the **replay** endpoint expose hash data. The list/feed endpoint omits it entirely.

Meanwhile, `web/lib/sample-audit.ts:162-179` (`rowHash`) generates row.hash via 8 FNV-1a passes — explicit in the comment "Not a cryptographic hash; the real chain hash arrives from the gateway in production." But the gateway never sends it, so the FNV pretender is the only thing the camera-visible hash cell at `web/components/skycot/ActivityRow.tsx:166-197` can render.

Two failure modes:
1. **Live data**: hash column is blank (gateway doesn't send it)
2. **Mock data**: hash column shows a SHA-256-shaped string that isn't actually anything (FNV-1a fakery)

For Claim 6's "audit-chain-verified" trust story, both modes erode credibility. The fix is two-layer:
- (a) Server: include `self_hash` (and `parent_hash`) in the feed envelope
- (b) Client: drop the FNV fallback; either render no hash on seed rows OR seed rows with real-looking but explicitly-fake hashes labeled as such

## Curl evidence

```
curl "http://127.0.0.1:7400/api/v1/audit?..." | jq '.items[0]'
→ { "id":"...", "ts":"...", "kind":"...", ... }   ← no `hash` or `self_hash` field

curl "http://127.0.0.1:7400/api/v1/audit/<id>" | jq '.self_hash'
→ "a3b7c9d2..."   ← detail endpoint has it, feed endpoint doesn't
```

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F5 [P1] — Audit-row hash is a non-cryptographic FNV-1a pretender; gateway feed JSON doesn't carry hash at all
- scene-6-claim-6-trust-and-approvals F11 [P3] — Hash-cell tooltip says "Click to copy" but the value copied is the FNV pretender on seed rows

## Fix recipe

**Server-side** (`crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`):
1. Find the row→envelope projection (likely a `to_feed_item` or `serialize_audit_row` function).
2. Add `self_hash: String` (hex-encoded) and `parent_hash: Option<String>` fields to the envelope type.
3. Populate both fields from the underlying `audit_record` row — both columns already exist per `crates/argo-audit/src/lib.rs`.
4. Bump the gateway schema version (additive — list endpoint gains 2 fields; no removal). Update `crates/argo-store/src/migrations/registry.rs` if a migration is required (likely not — fields are derived, not stored separately).
5. Update the gateway surface-pin test at `crates/argo-cli/tests/surface_contract.rs` (or wherever audit feed is pinned) to include the new fields.

**Client-side** (`web/lib/sample-audit.ts` + `web/components/skycot/ActivityRow.tsx`):
1. Drop the FNV `rowHash` function and its callers.
2. Two options for sample rows:
   - **Option A**: render no hash on seed rows (hide the column or show "—"). Clean, but loses the visual chrome.
   - **Option B**: seed rows include a SHA-256-shaped hex string + a tooltip explicitly labeling them as "sample · not from real chain". Preserves visual but is honest.
   - Pick Option B for the camera-visible chrome, with the tooltip clearly differentiating from real hashes.
3. Update `ActivityRow.tsx:166-197`:
   - Render `row.self_hash` directly from the gateway envelope when present
   - On seed rows, render `row.sample_hash` (new field) with a `<Tooltip>Sample row — not part of the production audit chain</Tooltip>`
4. Update the type at `web/lib/api/audit.ts` (or wherever feed item type is defined) to include `self_hash?: string` and `parent_hash?: string`.

**Tests**:
- Gateway integration: `crates/argo-cli/tests/audit_feed_includes_hash.rs` — seed a row, GET feed, assert `self_hash` matches the SHA-256 the chain stores.
- SPA component: `ActivityRow.test.tsx` — render with real hash → renders without tooltip; render seed row → renders with "sample" tooltip.
- Visual: hash cell tooltip on hover differentiates production vs sample.

## Acceptance criteria

- [ ] `GET /api/v1/audit` response items include `self_hash` and `parent_hash` fields (hex strings).
- [ ] `self_hash` value matches the SHA-256 stored in `audit_record.self_hash` for that row (regression test).
- [ ] SPA `ActivityRow` renders the hash from the gateway envelope when present.
- [ ] FNV `rowHash` function deleted from `web/lib/sample-audit.ts`.
- [ ] Seed rows render a SHA-256-shaped hex (Option B) with an explicit "sample · not from real chain" tooltip on hover.
- [ ] Tooltip on real-chain hashes reads "Self-hash · click to copy".
- [ ] Click-to-copy works on both real and sample hashes (UX parity).
- [ ] Surface-pin test passes with the new fields.
- [ ] Gateway integration test + SPA component test pass.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (envelope projection)
- `crates/argo-cli/src/handlers/gateway/audit_feed_envelope.rs` (if envelope type is separate)
- `crates/argo-cli/tests/surface_contract.rs` (pin update)
- `crates/argo-cli/tests/audit_feed_includes_hash.rs` (NEW)
- `web/lib/api/audit.ts` (type for new fields)
- `web/lib/sample-audit.ts` (drop FNV; emit sample hashes labeled as such)
- `web/components/skycot/ActivityRow.tsx` (hash cell + tooltip)
- `web/components/skycot/ActivityRow.test.tsx` (NEW or extended)

## Out of scope

- Hash-chain prev/this hover popover (separate brief: `fix-audit-hash-chain-prev-this-hover-and-live-rows` already exists)
- Audit redaction banner (separate brief: `fix-audit-redaction-banner-gates-on-rows` already exists)
- Replay endpoint contract (separate brief: `fix-replay-endpoint-contract-spa-vs-gateway-mismatch`)

## Effort

~80-150 LOC across gateway projection + SPA hash cell + tests. ~2-3 hours.
