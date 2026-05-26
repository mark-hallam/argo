# fix-audit-detail-adapter-deserialize-envelope

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-detail-adapter-deserialize-envelope`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-6-claim-6-trust-and-approvals.md` F2.

🔴 **P0 — `fetchAuditDetail` does not normalize the live gateway envelope. The audit drill-in only works in auth-bypass mode because mocks short-circuit the network. Switch off bypass and the entire drill-in renders blank — Scene 6's "click into a row" beat is unrecordable against real backends.**

## Root cause

`web/lib/api/audit-detail.ts:58-77` is the entire body of `fetchAuditDetail`:

```ts
const path = `/api/v1/audit/${encodeURIComponent(actionId)}`;
const raw = await apiGet<AuditDetail>(path);
return raw;
```

No adapter call. No envelope unwrap. No snake_case → camelCase mapping. The TypeScript cast to `AuditDetail` silently succeeds at compile time, but the runtime shape is wrong.

Live gateway response (verified `curl :7400/api/v1/audit/<32-char-hex>`):
```json
{
  "detail": {
    "summary": { "id":"...", "verb":"...", "ts":"...", "data_classification":"...", "prompt_template":"..." },
    "provenance": { "model": {...}, "skill": {...}, "hash_chain_anchor":"...", "signature":"..." },
    "decision_chain": [...],
    "output": {...},
    "inputs": {...},
    "memory_snapshot": {...},
    "follow_up": {...}
  }
}
```

The SPA's `AuditDetail` type (`web/lib/types/audit-detail.ts:84-90`) declares the top-level fields directly (`id`, `verb`, `provenance.model.id`, etc.) without the `detail` wrapper or snake_case. Every consumer (`DecisionChainCard`, `ProvenanceBlock`, `MemorySnapshotCard`, `OutputCard`, `ReplayButton`) reads off the un-unwrapped top-level object → undefined → renders blank.

Compare to the working pattern in `web/lib/api/audit.ts:209-222` (`normalizeAuditFeed`), which explicitly unwraps `{items, next_cursor}` and maps snake→camel. The detail client was never given the same treatment.

Audit also notes (Scene6 F11) that `provenance.hash_chain_anchor` has no home in the SPA shape — this brief surfaces it. The companion brief `fix-audit-provenance-block-render-hash-chain-anchor` already renders it once the shape exposes it.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F2 [P0] — `fetchAuditDetail` cannot deserialize the live gateway envelope; drill-in is mock-only
- scene-6-claim-6-trust-and-approvals F11 [P3] — `formatHashForDisplay` / hash_chain_anchor has no home in the SPA shape (folded — same root cause)

## Fix recipe

1. **Add `normalizeAuditDetail(raw: unknown): AuditDetail`** in `web/lib/api/audit-detail.ts`, mirroring `normalizeAuditFeed`:
   - Unwrap top-level `detail` if present (fallback: treat `raw` as already-flat for back-compat with mocks).
   - Lift `summary` fields up to the top level (`id`, `verb`, `ts`, `data_classification` → `dataClassification`, `prompt_template` → `promptTemplate`).
   - Map `decision_chain` → `decisionChain`.
   - Map `memory_snapshot` → `memorySnapshot`.
   - Map `follow_up` → `followUp`.
   - Map `provenance.hash_chain_anchor` → `provenance.hashChainAnchor`.
   - Map `provenance.signature` (keep as-is).
   - Pass-through `output`, `inputs`, `decisionChain` entries' inner fields.

2. **Update the type** `web/lib/types/audit-detail.ts:84-90` to declare:
   - `provenance.hashChainAnchor?: string`
   - `provenance.signature?: string`
   - confirm camelCase on `dataClassification`, `promptTemplate`, `decisionChain`, `memorySnapshot`, `followUp`.

3. **Call the adapter**:
   ```ts
   const raw = await apiGet<unknown>(path);
   return normalizeAuditDetail(raw);
   ```

4. **Update mocks** `web/lib/sample-audit-detail.ts` to emit the SAME flat shape `normalizeAuditDetail` produces (so the adapter is a no-op on mocks). Drop `"policepolicepolicy"` placeholder (Scene6 F1-adjacent — track as sub-fix in this brief because it's the same file).

5. **Unit tests** `web/lib/api/audit-detail.test.ts`:
   - Adapter on envelope-wrapped input returns flat `AuditDetail` with `hashChainAnchor` populated.
   - Adapter on already-flat input (mock) returns the same object (idempotent).
   - Adapter on partial input doesn't throw; missing fields default to `undefined`.

6. **Smoke test**: turn off `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`, navigate to `/audit/action/<real-hex-id>`, confirm all 4 cards (DecisionChain, Provenance, MemorySnapshot, Output) render with content.

## Acceptance criteria

- [ ] `fetchAuditDetail` returns a flat `AuditDetail` regardless of envelope shape (gateway or mock).
- [ ] Drill-in renders all 4 cards with content against the live gateway (auth-bypass OFF).
- [ ] `provenance.hashChainAnchor` is populated and consumable by `ProvenanceBlock`.
- [ ] Unit tests cover the 3 adapter cases (wrapped / flat / partial).
- [ ] Mock fixtures emit flat shape; `policepolicepolicy` placeholder removed.
- [ ] No regression on auth-bypass path (mock-fed drill-in still works).

## May-touch

- `web/lib/api/audit-detail.ts` (adapter + caller)
- `web/lib/types/audit-detail.ts` (field renames + new `hashChainAnchor`)
- `web/lib/sample-audit-detail.ts` (drop placeholder, conform to flat shape)
- `web/lib/api/audit-detail.test.ts` (NEW)

## Out of scope

- Feed-row `hash` column (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows` + `fix-audit-feed-include-self-hash-in-envelope`)
- Decision-chain empty-body rendering on meta-events (covered by `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)
- Hash anchor rendering on detail page (covered by `fix-audit-provenance-block-render-hash-chain-anchor` — sequence this brief first to land the field)

## Effort

~120-180 LOC across adapter + type + 1 test file. ~3-4 hours. **P0 — pairs with the demo-seed CLI; once real worker turns land in `/api/v1/audit`, the drill-in MUST deserialize correctly or Scene 6 falls over.**
