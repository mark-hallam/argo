# Round 86 — Typecheck regression sweep on current `origin/main`

- **Date**: 2026-05-26T12:55Z (Melbourne 22:55 AEST)
- **Round**: 86
- **Pillar / Claim**: SPA build-gate hygiene + r85-S2 carry follow-through
- **Verdict**: 🟡 PARTIAL — P2 (2 TS errors on main; runtime risk LOW for #1, CONDITIONAL for #2)
- **Carry status**: r85-S2 known regression still TODO (brief authored); 1 NEW regression discovered this round

## Setup

```bash
cd web
pnpm exec tsc --noEmit 2>&1 | grep -E ': error TS'
```

## Observed

```
components/skycot/audit/ProvenanceBlock.tsx(36,56): error TS2345:
  Argument of type 'string | undefined' is not assignable to parameter of type 'string'.

lib/onboarding/use-provision.ts(58,3): error TS2741:
  Property 'created' is missing in type '{ tenantId: string; workspaceId: string; }'
  but required in type 'ProvisionResult'.
```

Exactly **2 TS errors** on main. Total scope:

### Error 1 — `lib/onboarding/use-provision.ts:58` (r85-S2 known)

Already flagged in round 85; ledger row `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false`
is TODO (conductor T12-04 synthesis 2026-05-26T12:04Z). **No new action this round** — the brief exists;
the fix is unclaimed. Recommend conductor surface to operator if it remains unclaimed by next round.

### Error 2 — `components/skycot/audit/ProvenanceBlock.tsx:36` (NEW this round)

**Introduced by**: PR #2366 (`fix(audit): render empty chain fallbacks`), commit `5408d63a2`,
merged earlier today (~3-4h ago).

**Defect**: the `chain-anchor` row added at `ProvenanceBlock.tsx:91-98` omits an `id` field:

```ts
function buildRows(data: ProvenanceBlockData): ProvenanceRow[] {
  const chainAnchor = chainAnchorFromProvenance(data);
  return [
    { key: "model",  label: "Model",            id: data.model.id,         full: data.model.version },
    { key: "prompt", label: "Prompt template",  id: data.promptTemplate.id, full: data.promptTemplate.hash },
    { key: "skill",  label: "Skill version",    id: data.skill.id,         full: data.skill.version },
    { key: "memory", label: "Memory snapshot",  id: data.memorySnapshot.id, full: data.memorySnapshot.id },
    { key: "policy", label: "Policy version",   id: data.policy.id,        full: `${data.policy.version} · ${data.policy.hash}` },
    ...(chainAnchor
      ? [{ key: "chain-anchor", label: "Chain anchor", full: chainAnchor, display: formatChainAnchor(chainAnchor) }]
      : []),
  ];
}
```

`ProvenanceRow.id` is `id?: string` (optional, line 77). The empty-rows guard at line 36:

```ts
const emptyProvenance = rows.every((row) => !hasText(row.id) && !hasText(row.full));
```

calls `hasText(row.id)` which expects `string` (line 192-194):

```ts
function hasText(value: string): boolean {
  return value.replace(/[·\s]/g, "").length > 0;
}
```

**Runtime risk**: CONDITIONAL but real. `Array.every()` short-circuits on the first `false`, so the
chain-anchor row is only inspected if every preceding row has empty `id` AND empty `full`. In that
case `hasText(undefined)` → `undefined.replace(/[·\s]/g, "")` → `TypeError: Cannot read properties
of undefined`. The audit drill-in page (Scene 6 of the demo, which we JUST fixed via #2405) would
crash whenever a row reaches the chain-anchor in `.every()`. The empty-chain test (`audit-detail-empty-chain`)
passes because its rows don't reach the chain-anchor in `.every()` (presumably non-empty `id`s upstream).

**Fix shape**: tighten `hasText` to accept `string | undefined`:

```ts
function hasText(value: string | undefined): boolean {
  return value != null && value.replace(/[·\s]/g, "").length > 0;
}
```

Or coerce at the callsite: `!hasText(row.id ?? "")`. The signature widening is preferable since it
prevents the regression class.

## Anti-theater check

`tsc --noEmit` is not a test-rewrites-itself-green surface — it's the canonical TypeScript
type-checker. The 2 errors are independently surfacable; cargo/Rust untouched. I read the
production code paths for both errors and walked the failure modes (runtime is fine for #1, runtime
crash conditional for #2). The brief candidate for #1 already exists; the recommendation for #2 is
the smallest possible fix that closes the regression class (signature widening, not callsite-by-callsite
patching).

## Gap / follow-up brief candidates

### P2 — NEW recommended brief

`fix-pr-2366-provenance-block-hastext-accepts-optional` (~3 LOC)

- **Pillar**: Build-gate hygiene
- **Priority**: P2 (typecheck breaks main; runtime crash conditional on empty-row prefix at audit drill-in)
- **Scope**: `web/components/skycot/audit/ProvenanceBlock.tsx` only
- **Tasks**:
  - widen `hasText(value: string | undefined)` and null-check
  - add 1 vitest pin in `__tests__/audit/provenance-block-empty.test.tsx` (or co-located equivalent)
    that constructs a `ProvenanceBlockData` with empty `model.id`/`prompt.id`/etc. AND a chain
    anchor present, asserts no throw on render
- **Why bundle with existing audit work?**: the brief naturally pairs with #2405 (Scene 6 demo
  surface) and #2366 (empty-chain fallbacks). Same file, same drill-in surface, prevents Scene 6
  from regressing on a corner-case demo fixture.

### Carry — r85-S2 still TODO

`fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` (TODO, ~1 LOC)

- Brief exists in ledger (T12-04); recommend conductor surface to operator if unclaimed by
  next round (this is the 2nd round it's been outstanding).

### Process recommendation — pre-merge typecheck gate

The conductor-pre-merge-gate.sh skip-clippy lane appears to be permitting TS errors through (#2366
and #2404 both introduced TS errors that landed on main). Out of the conductor's lane scope per
permanent web-only fix-* auth, but the evaluator notes that **2 TS regressions in <12h** is a pattern
worth surfacing. Possible companion brief: `ops-conductor-pre-merge-gate-web-typecheck-pin` — run
`pnpm typecheck` in the gate for any PR touching `web/`. Out of v1.0 demo critical path; P3.

## Process note

Cumulative anti-theater verifications this week: 82nd. 28-round 🟢 streak r57→r86 holding on
mission-claim dimensions; the TS regressions are a build-hygiene finding, not a claim regression
(scenarios S1 demo Scene 5 PASS is the binding claim verdict for this round).
