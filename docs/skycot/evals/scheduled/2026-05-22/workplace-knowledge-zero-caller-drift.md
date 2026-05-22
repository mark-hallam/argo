# Eval: Drift — `argo-workplace-knowledge` crate ships with zero callers

**Date**: 2026-05-22 → 2026-05-23 (round 3)
**Pillar**: Cross-pillar / drift hygiene (mission spirit)
**Mission claim**: All — drift is a system-health metric, not a pillar
**Verdict**: 🟡 **DRIFT CONFIRMED — pattern from CLAUDE.md anti-patterns**. The brief-pre-flight gate caught it before extending the parallel surface, which is the system working as designed. Surfaced here as evidence that the underlying lint (wire-or-defer) is not enforced at merge time.

## Setup

Round-2 git log surfaced this ledger row:

> `web-workplace-knowledge-editor BLOCKED — brief's dependency `mission-workplace-knowledge-gateway` is still BLOCKED on main (not MERGED). The 6 HTTP gateway endpoints this page consumes do not exist on disk — `rg workplace.knowledge crates/argo-cli/src/handlers/` returns zero.`

The substrate (`argo-workplace-knowledge`) MERGED. The audit kinds MERGED. The gateway — the consumer that wires substrate → HTTP → SPA — never merged.

## Action

Verify the drift by running the same grep the ledger references + walking the dependency chain.

## Expected

If the drift is real:
- `crates/argo-workplace-knowledge/src/lib.rs` exists with substrate code.
- Workspace `Cargo.toml` lists `argo-workplace-knowledge`.
- No Rust crate consumes it (no `use argo_workplace_knowledge::…`).
- No HTTP endpoints touch its types (no string matches under `crates/argo-cli/src/handlers/`).

## Observed

```bash
$ ls -la crates/argo-workplace-knowledge/src/lib.rs
-rw-r--r-- 1 operator staff 2083 May 22 18:49 crates/argo-workplace-knowledge/src/lib.rs

$ rg "argo_workplace_knowledge|argo-workplace-knowledge" crates/ Cargo.toml \
    | grep -v "^crates/argo-workplace-knowledge"
Cargo.toml:  "crates/argo-workplace-knowledge",
crates/argo-audit/src/event_kind.rs:    /// §7 Pillar 1). The substrate (`argo-workplace-knowledge`) holds
crates/argo-store/migrations/129__workplace_knowledge.sql:-- `argo-workplace-knowledge::doc` / `::revision`).

$ rg "workplace.knowledge|knowledge_doc" crates/argo-cli/src/handlers/
# (empty)
```

**Confirmed.** The crate exists, is in the workspace, and has NO Rust consumers. The two matches outside the crate itself are both **comment references**, not real `use` imports:

- `argo-audit/src/event_kind.rs` — a docstring mentioning the substrate as the origin of new `AuditKind` variants. The variants exist (`WorkplaceKnowledgeDocCreated` etc.) but no handler EMITS them.
- `argo-store/migrations/129__workplace_knowledge.sql` — a SQL comment naming the consumer crate. The table exists (`workplace_knowledge_doc`) but no Rust code reads or writes it.

So we have **three nested drift surfaces, none of which are wired**:
1. Substrate crate (`argo-workplace-knowledge`) with zero `use` consumers.
2. Audit-kind variants for events nothing emits.
3. SQL tables nothing reads.

## Anti-theater check

This isn't false-positive grep. I also checked:

```bash
$ ls crates/argo-cli/src/handlers/ | grep -i knowledge
# (empty)

$ rg "WorkplaceKnowledgeDoc" crates/ --no-heading -l
crates/argo-audit/src/event.rs        # AuditKind variant definitions
crates/argo-audit/src/event_kind.rs   # the docstring
# (no callers / emitters)
```

The audit variants are declared but no `AuditSink::write(..., AuditKind::WorkplaceKnowledgeDocCreated, ...)` call exists anywhere in `crates/`.

## Why this matters

The CLAUDE.md anti-pattern catalog calls this out explicitly:

> **Wire or defer.** Every new public surface in a brief names its consumer. If the consumer doesn't exist yet, the brief either includes the wire-up or is paired with a follow-up brief in the same batch. **"Land scaffold now, wire later" is the failure mode.**

The pattern that landed:
1. Brief 1: `mission-workplace-knowledge-substrate` — MERGED (substrate crate).
2. Brief 2: `mission-workplace-knowledge-audit-kinds` — MERGED (audit variants).
3. Brief 3: `mission-workplace-knowledge-gateway` — BLOCKED (would have wired the substrate to HTTP — never merged).
4. Brief 4: `web-workplace-knowledge-editor` — BLOCKED on brief 3.

So the swarm shipped the bottom of the stack without the middle. **The brief-pre-flight gate caught the BLOCKED state for briefs 3+4** — the ledger flagged it. Good. But briefs 1+2 had already merged independently without a "wire by end of week" guardrail.

## The system working AS DESIGNED (partly)

In fairness: the brief-pre-flight rule `Validate before claim: bash scripts/check-brief-freshness.sh <brief-id>` and the worker's manual pre-claim check ("Both must succeed before claiming. Otherwise wait.") did catch the BLOCKED state. The worker correctly refused to claim brief 3 because brief 1's outputs weren't on disk yet (in an earlier sequence). And brief 4 was correctly BLOCKED on brief 3.

But this only catches drift **downstream**. The merges of briefs 1+2 went through without the rule "scaffold must merge alongside or before its first consumer". The substrate has been sitting tenant-tested-but-uncalled for at least a day.

## Gap / follow-up brief candidates

**🟡 P1 — `claude-zero-caller-substrate-lint`**: Add `scripts/check-zero-caller-crates.sh` that walks `crates/*/Cargo.toml` and asserts each crate has at least one `use <crate_name>` in another crate, OR is allowlisted in `docs/skycot/audit/allowed-zero-caller-crates.txt` (legit cases: prelude crates, binary-only crates). Run in CI. Acceptance:

- Re-running the script today flags `argo-workplace-knowledge`.
- After `mission-workplace-knowledge-gateway` MERGES with real `use argo_workplace_knowledge` consumers, the script no longer flags it.

**🟡 P2 — `mission-workplace-knowledge-gateway` is the highest-leverage unblock right now**. Per the ledger, it's been BLOCKED for two days. Unblocking it lights up brief 4 (the editor) AND removes the zero-caller drift. The auditor has flagged `mission-workplace-knowledge-substrate` as a 3rd-consecutive P0 in audit T21-40 — that was about the original substrate brief, but the cascade now waits on the gateway brief.

**🟢 INFO — for the conductor**: When a future brief stack has dependencies A → B → C → D, the conductor should consider authoring them as a **brief train**: one ticket that pins all 4 to ship together (mark all 4 CLAIMED at once, the manager-worker pattern). Splitting them with merge-checkpoints is what lets the substrate land independently.

## What this scenario proves

⚠ **The wire-or-defer rule is documented but not enforced at the merge gate.** The substrate landed without a consumer; the consumer-gateway is now stuck because its own pre-flight refuses to claim. The ledger correctly tracks the dependency, but the substrate could have been a P1 if it had landed alone without the downstream briefs even existing.

🟢 **The brief-pre-flight check is real.** Two consecutive briefs (3 + 4) refused to claim with explicit reasons cited in the ledger. The drift surface is paused at exactly the right point.
