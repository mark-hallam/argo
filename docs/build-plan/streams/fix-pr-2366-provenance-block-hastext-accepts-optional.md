# fix-pr-2366-provenance-block-hastext-accepts-optional

**Owner**: claude
**Priority**: 🟡 P2
**LOC estimate**: ~3 LOC
**Lane**: web-only fix-*
**Authored against**: origin/main `78f162a5a` on 2026-05-26
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2366-provenance-block-hastext-accepts-optional`

## Source

Evaluator round 86 §S2 (NEW finding) — docs/skycot/evals/scheduled/2026-05-26/round-86-typecheck-regression-sweep.md.

`pnpm tsc --noEmit` exit 2 on main with 2 TypeScript errors. This brief
fixes the new one introduced by PR #2366 (commit `5408d63a2`); the
sibling regression from PR #2404 is covered by
[[fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false]].

## Problem

`web/components/skycot/audit/ProvenanceBlock.tsx:36` calls
`hasText(row.id)` where `row.id` is typed `id?: string` (optional)
because PR #2366 added a chain-anchor row to `buildRows()` with no
`id` field. `hasText` accepts only `string`, so the call site fails
TS2345.

Runtime risk is CONDITIONAL: `.every()` short-circuits — the crash
would only fire when **all** preceding rows have empty `id` AND empty
`full` AND the chain-anchor row is present. That's a corner case
that could hit Scene 6 audit drill-in. Type-safety regression in the
audit primitive surface is reason enough to fix regardless.

## Fix

Widen `hasText` to accept `string | undefined` and treat `undefined`
as falsy. ~3 LOC at `web/components/skycot/audit/ProvenanceBlock.tsx:192`:

```ts
function hasText(value: string | undefined): boolean {
  if (value === undefined) return false;
  return value.replace(/[·\s]/g, "").length > 0;
}
```

## Acceptance criteria

- `pnpm --dir web tsc --noEmit` exits 0 (this regression closed; the
  sibling `use-provision.ts:58` regression is the other open brief).
- No behaviour change for callers passing real strings (the `===
  undefined` guard short-circuits before the existing `.replace()`).
- ProvenanceBlock empty-state detection still works as before — the
  chain-anchor row's missing `id` is now correctly treated as
  "no text" instead of throwing a type error.

## May touch

- `web/components/skycot/audit/ProvenanceBlock.tsx` (the only file)

## Tests

- Existing vitest under `web/__tests__/audit/` should still pass.
- No new test needed — the typecheck pin itself is the regression
  test (will recur on regression).

## Out of scope

- Stage 3 ProvenanceRow refactor (the chain-anchor row's distinct
  shape — `id` absent, `full` always present — suggests a discriminated
  union long-term but that's a bigger refactor).
- Adding a `pnpm --dir web typecheck` stage to the conductor
  pre-merge gate — that's a separate brief
  [[ops-conductor-pre-merge-gate-web-typecheck-pin]].

## Parallel-surface check

- `rg 'pub function hasText' web/` → only this one definition.
- No parallel `hasText` in nearby modules.
