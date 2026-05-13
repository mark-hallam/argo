# Drift Prevention Playbook

Source-of-truth for keeping conductor-authored briefs, worker output, and the
live ledger from drifting away from what the code actually does.

This doc is **load-bearing for the swarm**. Authored 2026-05-14 after the
audit-chain (audit-1 → audit-3) surfaced a 24% drift rate in
conductor-authored briefs plus systemic parallel-system proliferation across
~95 crates. Drift wasn't worker-side — it was the *brief-authoring layer* not
checking what already shipped before authoring new work.

Read before authoring a brief. Re-read when the audit cadence triggers.

---

## What "drift" means here

Three flavors, in increasing severity:

1. **Mechanical drift** — ledger row state disagrees with on-branch reality
   (`IN_REVIEW` row whose PR is merged, duplicate brief-id rows, stale
   directives like "Returning to TODO" left on a CLAIMED row).
   Cheap to detect, cheap to fix. `scripts/find-stale-claim-drift.py`
   catches most of it.

2. **Intent drift (STALE-PATH)** — a brief tells the worker to touch a file
   or wire a surface that no longer exists or was superseded since the brief
   was authored. Worker either fails to claim or ships something wrong.

3. **Architectural drift (INTENT-DRIFT)** — a brief authors a *new parallel
   surface* (new type, new pool, new variant) alongside one that already
   ships and works. Worker ships clean code that increases dead-code
   surface and tenant-isolation risk. **This is the dangerous class.**
   ~24% of conductor-authored briefs in the 2026-05-13 audit-3 sample.

The playbook below targets all three, with the most ceremony around #3.

---

## The eight drift patterns we've actually shipped

Each pattern is named, has a real example from this codebase, and lists the
specific check that would have caught it pre-author. **Run the check.**

### 1. Infrastructure shipped without wiring
A subsystem lands (config struct, store column, capability variant, audit
kind) but the runtime never reads it. Caller-side wire-up was a separate
brief that was never authored.

**Examples (Phase 3 audit, 2026-05-14):** `argo_config::BudgetCeilingSection`
(parallel to the wired `BudgetConfig`), 5 dead `AuditKind` variants,
`spend_ceiling` read path with no enforcer.

**Pre-author check:** for any new public surface, name the consumer in the
brief and grep for it. If the consumer doesn't exist yet, the brief is
incomplete — author the wire-up brief in the same batch or fold both into
one. **"Land scaffold now, wire later" is the failure mode.**

### 2. Parallel-system / duplicate-type proliferation
Two structs with the same name and overlapping responsibilities live in
different crates, often because the second was authored without grepping
for the first.

**Examples (audit cluster 3, 2026-05-13):** `SessionId` x2,
`JudgeVerdict` x3, `Role` x2, `TrustGrade` x2, `CircuitState` x2,
`McpConnectionPool` (in `argo-agent-runner`) parallel to the canonical
`McpClientPool` (in `argo-mcp`) — and the parallel one lacked per-tenant
keying, which became a cross-tenant credential-leak P1.

**Pre-author check:** before declaring any new pub struct, enum, or type
alias, `rg 'pub (struct|enum|type) <NameCandidate>'` across `crates/`.
If a same-named or near-same-named surface exists, the brief must either
(a) extend the existing one, (b) supersede it explicitly with a deletion
sub-brief, or (c) carry a one-line justification for why parallelism is
correct (rare; usually it is not).

### 3. Brief-authoring drift (the conductor-side root cause)
Conductor authors quickly to keep queue depth above the threshold, doesn't
grep for shipped surfaces, doesn't validate against current `origin/main`,
doesn't name the consumer. Brief reads clean; worker ships clean; surface
becomes the next parallel system. **This is the meta-pattern behind #1 and
#2.**

**Pre-author check:** for every brief, the `Authored against` line must
cite an `origin/main` SHA from <24h ago. The `Validate before claim:` line
must invoke `scripts/check-brief-freshness.sh`. If the brief introduces
new surface area, the **Scope** section must include a grep line proving
the surface is not already present. Reviewer (CEO or peer conductor)
checks this before squash-merge.

### 4. State-vs-annotation ledger drift
A row's State column says `CLAIMED` or `IN_REVIEW` but its Note column
carries a directive like "Returning to TODO" or "STALE-CLAIM-DEMOTE". The
note was added by a sweep script that never updated the state column.

**Examples:** routine — has happened across multiple ticks.

**Detection:** `scripts/find-stale-claim-drift.py` (read-only). Run on
every conductor tick. Hard-zero rows means the ledger is internally
consistent.

### 5. Recovery-brief cascade
A PR is `DIRTY` for >24h, conductor authors `claude-rebase-pr-N`. Worker
ships, but `N`'s underlying brief was already superseded by another merge,
so `claude-rebase-pr-N` ships a stale path that needs its *own* rebase.
This stacks.

**Examples:** `claude-rebase-pr-1540`, `claude-rebase-pr-1542` rows
sitting `IN_REVIEW`/`TODO` while their underlying PRs had already been
merged or rebased through another path.

**Pre-author check for recovery briefs:** before authoring
`claude-rebase-pr-N`, re-validate that the underlying intent is still
load-bearing on current `origin/main`. If the bug or feature has already
been fixed by another PR, the recovery brief is unnecessary — close the
original PR with a STALE-FIXED note instead.

### 6. Mission-claim alignment drift
A merged PR's commit message and brief don't trace to any of the six
mission claims (Teammate / Delegation / Safe-by-construction /
Observability / Vertical-templates / Multi-channel-approvals). It still
might be useful work, but it dilutes the demo story.

**Examples (audit-2):** ~31% of merges in the 2026-05-13 sample had no
explicit claim trace.

**Pre-author check:** every non-cleanup brief should name the claim it
advances in its header. Cleanup briefs (deletes, rebases, splits) are
exempt. If a brief can't name a claim, ask whether it should be authored
at all — or whether the goal is actually cleanup and should be
re-labeled.

### 7. Connector boilerplate proliferation
Each new `argo-mcp-connector-*` crate duplicates ~60 LOC of `main.rs`,
config loader, capability map, and Cargo.toml stanza. We have ~27
connector crates, of which 17 had **zero workspace consumers** at the
2026-05-14 audit (cluster 4) — pure shape-B dead code.

**Pre-author check:** before authoring "build connector X", confirm the
*vertical* that needs it actually exists, has a `vertical.toml`, and
references the connector by name. If the vertical is paper-only, the
connector brief is paper-only too — defer. Cleanup is documented in
`claude-delete-orphan-shape-b-connectors.md` (the model brief for this
class).

### 8. Conductor pushing feature code
Project memory says **"Never push feature code from this tab — hand off to
conductor"** (the operator's conductor process). The exception is when the
CEO authorizes a same-tab push for a P1 security fix or a chain-rebase
that's blocking workers.

**Example:** chain-rebase of #1542 (`f930a793`) — pushed in-tab with CEO
authorization for a P1 cross-tenant credential-leak fix.

**Rule:** the default is hand-off. The exception is the CEO saying "push
it from this tab" in the same chat. The exception is not inferred from
context; it must be explicit.

---

## Brief-authoring pre-flight checklist

Run mentally — or copy into the brief draft and tick — before saving any
new brief under `docs/build-plan/streams/`.

- [ ] **Freshness:** `Authored against: origin/main <SHA> on <YYYY-MM-DD>`
      where SHA is from within the last 24h.
- [ ] **Freshness gate:** brief includes
      `Validate before claim: bash scripts/check-brief-freshness.sh <brief-id>`.
- [ ] **Grep before write:** for every new pub struct/enum/fn the brief
      introduces, `rg 'pub (struct|enum|fn) <Name>' crates/` returns no
      match. If it does, brief must extend the existing surface or
      explicitly supersede.
- [ ] **Wire or defer:** every new surface names its caller. If the caller
      doesn't exist, brief either includes the wire-up or is paired with
      a follow-up brief in the same batch.
- [ ] **Ceiling check:** every may-touch file under the 500-LOC ceiling.
      `scripts/check-pre-claim-ceiling.sh` will block claim otherwise —
      author the split-prep brief first.
- [ ] **Mission claim:** non-cleanup briefs name the claim they advance
      in the header. Cleanup briefs label themselves as such.
- [ ] **Files-you-must-NOT-touch:** explicit list. Lock the canonical
      surface against accidental modification.
- [ ] **Reviewer is a different agent:** the conductor authoring the
      brief is not the worker claiming it. (Enforced by the swarm
      protocol; reaffirm here.)

---

## Audit cadence

Drift accumulates between audits. The cadence below has caught it so far.

| Cadence | What runs | Why |
|---|---|---|
| Every conductor tick (~15 min) | `find-stale-claim-drift.py --quiet` over BUILD_STATUS.md | Catch mechanical drift before workers see it |
| Daily | Spot-check 3 random in-flight briefs for the pre-flight checklist | Catch authoring drift early |
| Weekly | Cluster-level audit (cluster 1–5, currently 5 clusters covering ~95 crates) | Catch parallel-system + dead-surface drift |
| Per phase boundary | Mission-claim alignment audit | Keep demo story coherent |
| After any P1 fix | Re-grep for sibling instances of the same pattern | Cross-tenant leaks rarely come alone |

The weekly cluster audit is the highest-value pass. Schedule it as a
background general-purpose agent and triage the report at the next
conductor tick.

---

## Unfucking playbook (when drift has already shipped)

Severity ladder. Pick the lowest tier that addresses the actual scope.

### Tier 0 — Mechanical fix
Ledger row inconsistency, duplicate row, stale annotation. Resolve in the
conductor tick with the standard atomic pattern
(`python3 mutation && git add && git commit && git push`). No brief
needed.

### Tier 1 — Single-row recovery brief
One PR is `DIRTY` >24h. Author `claude-rebase-pr-<N>` after confirming
the underlying intent is still load-bearing (see drift pattern #5).

### Tier 2 — Supersession
A new brief authored against current main makes an in-flight brief
obsolete. Mark the obsolete brief `SUPERSEDED-BY-<new-brief-id>` in the
ledger and close the worker PR with a comment. Do not let the worker
ship the obsolete path.

### Tier 3 — Pure delete brief
Surface is orphan (zero workspace consumers, confirmed by audit). Author
a `claude-delete-<surface>` brief — pattern-matched on
`claude-delete-argo-gateway-crate.md` and
`claude-delete-orphan-shape-b-connectors.md`. Pure-delete briefs are the
highest-leverage cleanup; one shipped delete of 6,300 LOC is worth
twenty refactors.

### Tier 4 — Wire-or-delete decision brief
Surface exists, has no consumer, but might be load-bearing for a future
claim. Author a Phase 3 wire-or-delete brief that forces a CEO decision:
either wire it to a named caller in this same brief, or delete it. The
brief must enumerate both options.

### Tier 5 — Rewrite the brief, not the code
The shipped brief was wrong but the code may still be useful. Author a
v2/v3 rewrite of the brief that retargets at the canonical surface,
mark the original `SUPERSEDED-BY-<vN>`, and re-queue. Used for Cluster D
cases 1–5 in this session.

### Tier 6 — P1 conductor-side push
Security or worker-blocking emergency. CEO authorizes a same-tab push.
Document the exception inline (PR description + ledger note). This is
drift pattern #8 and should remain rare.

---

## Tooling that backstops this doc

- `scripts/find-stale-claim-drift.py` — read-only ledger consistency
  check. Catches drift patterns #4 and most of #5.
- `scripts/check-pre-claim-ceiling.sh` — 500-LOC pre-claim gate.
  Backstops drift pattern #1 in the narrow case of "this file is already
  too big to extend".
- `scripts/check-brief-freshness.sh` — invoked by the brief itself.
  Backstops drift pattern #3.
- Cluster-audit background agents (one per Tracks A–N grouping) — the
  primary detector for drift patterns #2, #6, #7.

If a drift pattern recurs that none of these catches, extend one of them
or author a new one. **Detection that lives only in human attention does
not scale across the swarm.**

---

## See also

- [`CLAUDE.md`](../../CLAUDE.md) — top-level invariants and rules; the
  brief-authoring checklist references this doc inline.
- [`docs/build-plan/SWARM_PROTOCOL.md`](../build-plan/SWARM_PROTOCOL.md)
  — iteration sequence and worker rules.
- [`docs/build-plan/PROTOCOL.md`](../build-plan/PROTOCOL.md) —
  architecture and anti-patterns.
- [`docs/skycot/CONFIG_CALLSITE_AUDIT.md`](CONFIG_CALLSITE_AUDIT.md) —
  one example of a successful audit-pass that turned into a wire-or-delete
  batch.
