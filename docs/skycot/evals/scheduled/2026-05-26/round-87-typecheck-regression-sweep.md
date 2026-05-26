# Round 87 — TypeScript regression sweep — both r85+r86 carries persist

- **Date**: 2026-05-26
- **Round**: 87
- **UTC**: 2026-05-26T13:20Z
- **Pillar**: Process — pre-merge typecheck gate gap
- **Verdict**: 🟡 PARTIAL (P2) — same two errors, no new ones, briefs authored but unclaimed across 2-3 evaluator rounds

## Setup

```bash
cd ~/code/argo/web && pnpm exec tsc --noEmit
```

## Observed

Exit 2. Two errors, identical to the closing state of round 86:

```
components/skycot/audit/ProvenanceBlock.tsx(36,56): error TS2345: Argument of type 'string | undefined' is not assignable to parameter of type 'string'.
  Type 'undefined' is not assignable to type 'string'.
lib/onboarding/use-provision.ts(58,3): error TS2741: Property 'created' is missing in type '{ tenantId: string; workspaceId: string; }' but required in type 'ProvisionResult'.
```

### Production source (unchanged since r86)

- `web/lib/onboarding/use-provision.ts:56-62` — `buildDemoProvisionResult()` returns `{ tenantId, workspaceId }` without `created: boolean`. `ProvisionResult` (from #2376) requires `created`.
- `web/components/skycot/audit/ProvenanceBlock.tsx:34-37` — `rows.every((row) => !hasText(row.id) && !hasText(row.full))`. `hasText(value: string)` rejects `string | undefined`; the chain-anchor row added by `buildRows()` has no `id` field.

### Brief status (ledger lines 302-304)

| Brief | Authored | First-seen | Status now | Rounds unclaimed |
|---|---|---|---|---|
| `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` | T12-04 (r85→86 synth) | r85-S2 | TODO unclaimed | **3** (r85 → r86 → r87) |
| `fix-pr-2366-provenance-block-hastext-accepts-optional` | T12-20 (r86 synth) | r86-S2 | TODO unclaimed | **2** (r86 → r87) |
| `ops-conductor-pre-merge-gate-web-typecheck-pin` | T12-20 (r86 synth) | r86-process | TODO unclaimed | **2** (r86 → r87) |

All three rows are TODO under `web-only` / `ops-*` lanes, ~3-30 LOC each. The first was already surfaced to operator in conductor tick summary T12-20 per ledger note.

### What hasn't changed

- No new TS errors introduced by the merge wave since r86 (#2407 is Rust-side + one minor `web/lib/api/skills.ts` change; PRs #2408 #2409 #2410 are still OPEN so they don't affect main yet).
- The `use-provision.ts` brief carrying into a 3rd round means the underlying drift class — TypeScript regressions slipping through the Rust-only `conductor-pre-merge-gate.sh` — is still active. Any web/test-only auto-merge path or web/lib-only --skip-clippy admin-merge does not currently run `pnpm tsc --noEmit`.

## Anti-theater check

The two TS errors are not stripped by suppressions or @ts-expect-error. The chain-anchor row crash (ProvenanceBlock) is conditional — `.every()` short-circuits unless ALL preceding rows have empty `id` AND `full`, then the chain-anchor row is reached. In practice for the demo this would only hit Scene 6 audit drill-in if upstream `data` is sparse. The use-provision error is dead-code-during-typecheck because `buildDemoProvisionResult` is only called inside `provisionDemoWorkspace`, which is dispatched from the demo-mode short-circuit added by #2404 — so the actual runtime path returns the object, and `created: false` is the semantically-correct fix (demo runs are idempotent local replays).

Both regressions are real type holes the compiler is catching correctly — not stylistic false positives.

## Recommendation — for the conductor

1. 🟡 **P1 ESCALATE** — surface the 3rd-round unclaimed status of `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` to the operator at next tick. A 1-LOC fix sitting TODO across 3 evaluator rounds is the kind of brief a worker should auto-claim on first contact; if no claude/codex worker has picked it up by now it suggests either (a) the worker pool is misreading the priority section headers, or (b) the brief's `Authored against` SHA has gone stale and the freshness gate is rejecting claims. Run `bash scripts/check-brief-freshness.sh fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` next tick and either bump the SHA or convert to a conductor mechanical-fix.
2. 🟡 **P2** — same treatment for `fix-pr-2366-provenance-block-hastext-accepts-optional` if it stays unclaimed by r88.
3. 🟢 **P3** — `ops-conductor-pre-merge-gate-web-typecheck-pin` (the systemic fix) becomes more attractive every time a TS regression repeats. After 2 consecutive rounds of the same root cause, this is no longer optional — recommend escalating from P3 → P2 for the conductor's Step 8 author cycle.

## Gap

The drift class — TS regressions sailing through the gate — has now generated three pieces of work (two leaf fixes + one systemic). The leaf fixes are 1-3 LOC each; the systemic fix is ~30 LOC bash. The cost of the systemic fix is now smaller than the running cost of the leaf fixes if a third regression lands.
