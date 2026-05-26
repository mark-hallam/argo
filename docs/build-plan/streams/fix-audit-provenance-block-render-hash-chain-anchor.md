# fix-audit-provenance-block-render-hash-chain-anchor

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-provenance-block-render-hash-chain-anchor`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-6-claim-6-trust-and-approvals.md` F7 [P1].

🟠 **P1 — Gateway already returns `provenance.hash_chain_anchor` as a 64-char hex string. The SPA's ProvenanceBlock renders 5 rows (Model / Prompt template / Skill version / Memory snapshot / Policy version) and drops the chain anchor entirely. The cryptographic trust signal is computed server-side, returned over the wire, and never shown.**

## Root cause

The detail endpoint payload includes:
```json
"provenance": {
  "model": {...},
  "skill": {...},
  ...
  "hash_chain_anchor": "d37efe0d8c4a..." (64 hex chars)
}
```
`web/components/skycot/audit/ProvenanceBlock.tsx:70-78` renders 5 rows only. No sixth row for chain anchor. Per the locked B3 design (memory `project_audit_page_design_locked`), the chain anchor is the load-bearing cryptographic claim — it anchors a single row to the immutable chain. Showing it adds zero new backend work and visibly increases the cryptographic-claim density on Scene 6's drill-in.

## Code evidence

`web/components/skycot/audit/ProvenanceBlock.tsx:70-78` — 5 hardcoded ProvenanceRows.
Live gateway response confirms `hash_chain_anchor` field is present.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F7 [P1] — `hash_chain_anchor` exposed by gateway, never read or displayed by SPA

## Fix recipe

1. **Extend the audit-detail TypeScript type** `web/types/audit-detail.ts` to include `provenance.hashChainAnchor: string` (snake_case to camelCase if mapper exists, otherwise match wire format).
2. **In `web/components/skycot/audit/ProvenanceBlock.tsx`**, add a sixth ProvenanceRow:
   ```tsx
   {detail.provenance.hashChainAnchor && (
     <ProvenanceRow
       label="Chain anchor"
       value={detail.provenance.hashChainAnchor}
       full={detail.provenance.hashChainAnchor}
       truncate={12}
       monospace
     />
   )}
   ```
3. **Optionally re-label the block title** from "Provenance" to "Provenance & Chain" so a buyer's eye is drawn to the cryptographic anchor without scrolling.
4. **Style** the chain anchor row with a small Etherscan-style "0x..." prefix to match the B3 design lock visual register.
5. **Hover + click-to-copy** affordance the same as the existing HashCell pattern in `ActivityRow.tsx`.

## Acceptance criteria

- [ ] When `provenance.hash_chain_anchor` is present, ProvenanceBlock renders a sixth row labelled "Chain anchor".
- [ ] The row truncates to 12 chars by default with hover showing full 64-char hash.
- [ ] Click-to-copy works (same UX as HashCell).
- [ ] No regression on rows where the field is absent.
- [ ] Web test asserts the new row renders given a fixture with the field.

## May-touch

- `web/types/audit-detail.ts` (extend type)
- `web/components/skycot/audit/ProvenanceBlock.tsx` (add row)
- `web/components/skycot/audit/ProvenanceRow.tsx` (verify shape supports monospace + truncate; minor extension only if needed)
- `web/tests/audit-detail-provenance.test.ts` (NEW or extend)

## Out of scope

- Hash-chain prev→this hover affordance (separate brief: `fix-audit-hash-chain-prev-this-hover-and-live-rows`)
- Chain validation logic / re-derivation in the SPA (deferred — the server is authoritative)

## Effort

~30-50 LOC + 1 test. ~1 hour. Highest-leverage trust-signal increase per LOC in the audit drill-in.
