# web-audit-feed-per-row-hash — surface the hash chain on every audit-feed row (Etherscan parity)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-audit-feed-per-row-hash`.
**Priority**: P1 — closes the wave-1 trust+safety audit gap (audit feed renders no hash per row, breaking the chosen Etherscan metaphor).
**Sizing**: ~150-250 LOC (component edit + truncation logic + copy-to-clipboard + a11y label + tests).

## Why this brief exists

From the wave-1 trust+safety audit (`docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`):

> Audit feed renders no hash per row (`web/components/skycot/ActivityRow.tsx`).
> The "forensic hash chain" claim is invisible until the buyer drills
> into a single action. Etherscan — SKYCOT's chosen metaphor — shows
> hash on every row. The drill-in (/audit/action/<id>) is excellent;
> the feed isn't.

The /landing + /pricing + /compare all promise "forensic-grade audit
chain on every action." The drill-in delivers — but the feed reads as
a regular activity log without surfacing the receipt. A Diego-persona
buyer screenshots the feed expecting to see the hash; they don't.

## Goals

1. Render a truncated hash (first 8 chars + ellipsis or `0xabc12345…`) on every audit row.
2. Click-to-copy the full hash (uses the existing copy-on-hover microinteraction noted as "viral sleeper" in wave-5 viral-moments).
3. Visual: monospace tone, slate-color, right-aligned so it doesn't compete with the verb.
4. Tooltip on hover shows full hash + "Click to copy".
5. A11y: `aria-label="Audit chain hash <full hash>, click to copy"`.

## Steps

1. Read `web/components/skycot/ActivityRow.tsx` (current row component).
2. Find where the row data carries hash info; if not in the model, extend `AuditRow` type in `web/lib/api/audit.ts`.
3. Add the hash cell to ActivityRow. Use the existing `Copy` icon + `useClipboard` hook if present, or add via `navigator.clipboard.writeText`.
4. Update seed data in `web/lib/sample-audit.ts` to include a deterministic-but-realistic hash per row (use crypto.subtle.digest or a stable SHA-256 of `${worker.id}|${verb}|${timestamp}` truncated).
5. Verify Etherscan-style alignment on `/audit`: hash cell right-aligned, truncated to ~`0xabc1…2345`.

## may-touch

- `web/components/skycot/ActivityRow.tsx`
- `web/lib/api/audit.ts` (extend `AuditRow` type)
- `web/lib/sample-audit.ts` (extend seed with deterministic hashes)
- `web/app/(app)/audit/page.tsx` (only if column header needs adjusting)
- `web/components/skycot/ActivityRow.test.tsx`

## must-NOT-touch

- `web/app/(app)/audit/action/[id]/page.tsx` (drill-in already shows full hash chain — out of scope)
- Approvals page (this is audit-page specific)

## Acceptance criteria

- [ ] Every audit row renders a truncated hash (e.g. `0xabc1…2345`).
- [ ] Click-to-copy works: clicking the hash places full value in clipboard + shows a brief "Copied" toast.
- [ ] Hover tooltip shows full hash.
- [ ] `aria-label` on the hash element describes its purpose for screen readers.
- [ ] Seed-data hashes are deterministic — same input produces same hash so the demo reproduces identically run-to-run.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Trust+safety re-audit: audit-feed Etherscan-parity score moves from ~4 to 9.

## References

- Audit: `docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`
- Viral-moments mention: "the action-id copy-on-hover microinteraction in the audit feed is a viral sleeper, worth a 2s pause in the demo MP4."
