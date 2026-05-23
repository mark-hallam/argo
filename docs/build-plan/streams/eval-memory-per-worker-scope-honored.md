# eval-memory-per-worker-scope-honored

**Authored against**: origin/main `b0afe9f99` on 2026-05-23
**Owner**: claude
**Priority**: 🟡 P1 — mission-claim-1 substrate verification
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-memory-per-worker-scope-honored`

## Why this is P1

Gap manifest §"Claim 1 — Teammate, not tool" calls out: `argo-memory`
has 🔴 NOT AUDITED. Per MISSION.md: "per-worker memory scope
(workspace-shared shipped, the schema extension is queued)." The
`MemoryNs::WorkerSelf` scope exists on `Worker` but the audit has
not verified the memory store honors it at the SQL bind layer.

Round-14 verified cross-tenant isolation (gold-standard 2 negative
tests). This brief is the per-worker-WITHIN-tenant scope verification
— the next-finer granularity.

Mission claim 1 ("teammate, not tool") rests on persistent identity
+ memory per worker. If `MemoryNs::WorkerSelf` is parsed but not
filtered in SQL, two workers within the same tenant share memory
contrary to the substrate contract.

## Pre-flight grep

```
rg "MemoryNs" crates/argo-memory/ crates/argo-agent-runner/
rg "memory_namespace" crates/argo-memory/src/
rg "WorkerSelf" crates/argo-memory/src/ crates/argo-types/
```

Worker MUST verify before authoring scope: does
`crates/argo-memory/src/store.rs` (or whichever file owns the SQL)
filter `WHERE memory_namespace = ?` when the caller passes
`MemoryNs::WorkerSelf(worker_id)`? If yes → finding is
substrate-honored, write the contract-pinning negative tests + flip
the gap row 🟢. If no → finding is a P0 leak, escalate as a fix-brief.

## Scope

Read-only audit + contract-pinning tests:

1. **Trace the read path**: walk `MemoryStore::get` / `list` /
   `search` from caller (`AgentRunner::turn_helpers::memory_lookup`
   if it exists, else the in-turn memory recall site) down to SQL
   bind. Document which queries take `memory_namespace` as a
   parameter and which don't.

2. **Trace the write path**: walk `MemoryStore::insert` / `update`
   the same way.

3. **Verdict matrix** per public `MemoryStore` method (~6-10
   methods):
   - `WorkerSelf(worker_a)` write → `WorkerSelf(worker_b)` read of
     same tenant → expected NotFound (existence-leak parity).
   - `WorkerSelf(worker_a)` write → `WorkspaceShared` read of same
     tenant → expected Found (the worker can see shared but shared
     readers see the worker's row only if scope rules allow).
   - Cross-tenant read of `WorkerSelf(worker_a)` from tenant_b →
     expected NotFound (already covered by round-14 — pin existing
     coverage if it asserts WorkerSelf scope; else extend).

4. **Tests** (`crates/argo-memory/tests/per_worker_scope_test.rs` —
   NEW): 5-7 negative tests, byte-identical NotFound where required,
   no SQL-error leakage.

5. **Findings report**:
   `docs/skycot/evals/scheduled/2026-05-23/eval-memory-per-worker-scope-honored.md`
   with verdict matrix + every queried SQL statement + recommended
   follow-up brief (if any leaks found).

6. **Gap manifest row flip**: `argo-memory` row in
   `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
   from 🔴 NOT AUDITED → 🟢 AUDITED (or 🔴 LEAK with fix-brief
   citation).

## Acceptance

1. All public `MemoryStore` methods walked + verdict assigned.
2. 5-7 contract-pinning tests pass.
3. Findings report committed.
4. Gap manifest row flipped.
5. If LEAK found: P0 fix-brief authored as a sibling.

## May touch

- `crates/argo-memory/tests/per_worker_scope_test.rs` (NEW)
- `docs/skycot/evals/scheduled/2026-05-23/eval-memory-per-worker-scope-honored.md` (NEW)
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (row edit)

## Must NOT touch

- Production code in `argo-memory` (read-only audit).
- Any other crate.
- Migration files.

## LOC budget

~150-250 LOC (test file ~150 + findings report ~50 + gap-manifest
row ~5).
