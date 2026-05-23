# Round 20 — Claim 4 substrate trace (skill install → invoke → audit)

- **Date**: 2026-05-23
- **Round number**: 20
- **Pillar/Claim**: 4 (Skills shared + learned)
- **Verdict**: 🟡 PARTIAL — substrate is ~80% complete for draft LIVE-4A;
  one concrete gap blocks the regression test from landing as drafted.
  **Severity**: P1 if 4A locks as drafted; **P2** if 4A reformulates (option c).

## Why this scenario this round

Operator reverted the round-18 amendment locking LIVE criteria for
claims 4/5/6 because they want more time to digest each criterion
before signing off. The most useful thing this round can produce
is **concrete substrate data per criterion** so when the operator
sits down to lock 4A they know exactly what would need to change
in code vs. what's testable today.

This report walks the Claim 4 path and reports gaps inline against
the draft criterion at
[`docs/skycot/evals/scheduled/2026-05-23/draft-live-criteria-claims-4-5-6.md`](docs/skycot/evals/scheduled/2026-05-23/draft-live-criteria-claims-4-5-6.md).

## Setup

```bash
# 1. Enumerate every Skill-related AuditKind variant
grep -n "Skill" crates/argo-audit/src/event_kind.rs
#    → SkillApproved (684), SkillRejected (684), SkillCandidatePromoted
#      (697), SkillRolledBack, SkillRegressionDetected
#    → NO `SkillInvoked` variant exists.

# 2. Trace where Skill→tool-call audit happens
grep -rn "AuditKind::ToolCall" crates/argo-agent-runner --include="*.rs"
#    → 1 emit-site: turn_helpers_run.rs:228-235.
#    Payload at line 226: serde_json::json!({"tool": call.name, "call_id": call.id})
#    → NO skill_name / skill_source / skill_provenance field.

# 3. Confirm SkillStore::install signature + existing test coverage
grep -n "pub async fn install" crates/argo-skill/src/store.rs
#    → store.rs:64  `install(&self, raw: &str, learned_in: Option<SessionId>) -> ArgoResult<Skill>`
cat crates/argo-skill/tests/end_to_end_claim4_loop.rs
#    → 246-line test `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`
```

## Observed — substrate status per draft 4A requirement

Draft 4A criterion verbatim:

> **Claim 4 LIVE** = `argo skill install <pack>` writes the skill to
> the workspace's skill library, the worker can list it via
> `argo skill list`, invoking it through a workflow step succeeds,
> and a `SkillInvoked` audit row is written with the skill's
> provenance (source pack + version + author).

| Requirement | Status | Evidence |
|---|---|---|
| `argo skill install <pack>` writes skill to library | ✅ LIVE | `SkillStore::install` at [store.rs:64](crates/argo-skill/src/store.rs:64); CLI surface per [worker_skills.rs:16](crates/argo-cli/src/handlers/worker_skills.rs:16) + [routes_skills_show.rs:87](crates/argo-cli/src/handlers/gateway/routes_skills_show.rs:87) |
| Skill appears in `argo skill list` / worker prompt | ✅ LIVE | `LayeredLookup::with_library` + `list_for_action` rendered into prompt; verified by [`end_to_end_claim4_loop.rs:161-166`](crates/argo-skill/tests/end_to_end_claim4_loop.rs:161) (`prompt.contains("claim4_price_check")` + `prompt.contains("library:claim4_price_check")`) |
| Cross-tenant isolation | ✅ LIVE | [`end_to_end_claim4_loop.rs:168-181`](crates/argo-skill/tests/end_to_end_claim4_loop.rs:168) — other tenant's library install does not surface in tenant_a's prompt |
| Provenance in frontmatter (source + hash) | ✅ LIVE | `source: learned:claim4-session` asserted at [line 86](crates/argo-skill/tests/end_to_end_claim4_loop.rs:86); SkillFrontmatter has provenance.source + provenance.hash |
| Invoking through a workflow step succeeds | 🟡 SUBSTRATE | `WorkerSessionRunner` MERGED via #2089 (round 16) routes workflow step → MCP dispatch → audit. The skill→tool-dispatch wiring exists end-to-end. **No test pins skill-installed → worker-invokes path.** |
| `SkillInvoked` audit row with provenance | 🔴 **GAP** | No `AuditKind::SkillInvoked` variant exists. `AuditKind::ToolCall` payload at [turn_helpers_run.rs:226](crates/argo-agent-runner/src/turn_helpers_run.rs:226) is `{"tool": call.name, "call_id": call.id}` — no `skill_name`, `skill_source`, `skill_version`, or `skill_author` fields |

**Result**: 4A as drafted has 1 hard blocker (no `SkillInvoked` audit
kind / no skill provenance in ToolCall payload) and 1 soft gap (no
end-to-end install→invoke regression test pinning the chain).

## Anti-theater check

The risk here is declaring 4A "almost LIVE" because the existing
`claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`
covers most of it. I verified:

- The existing test uses `:memory:` SQLite, NOT a file-backed
  tempdir — so the "writes to workspace's skill library" reload
  half is **not** proven against disk persistence.
- The existing test never invokes a tool; it asserts prompt-render
  + cross-tenant negative. The "invoking through a workflow step
  succeeds" half is **not** proven.
- `grep -rn "SkillInvoked" crates/` returns ZERO matches. The
  variant doesn't exist anywhere in the codebase.

So 4A as drafted requires real code changes — it isn't a wording-only
unlock.

## Three resolutions for operator sign-off

### Option (a) — Add `AuditKind::SkillInvoked` variant

- ~80 LOC: new enum variant in `crates/argo-audit/src/event_kind.rs`
  + emit-site in `argo-agent-runner` when a tool call originates
  from a skill body + payload contract `{skill_id, skill_name,
  skill_source, skill_version, skill_author, tool, call_id}`.
- Brief: `mission-audit-kind-skill-invoked` (~80 LOC).
- Regression test: `claim_4_live_skill_install_invoke_audit.rs`
  (~200 LOC).
- **Benefit**: clean forensic — `argo audit query --kind SkillInvoked`
  gives every skill invocation across the chain. Matches the
  "skills are shared and learned" mission framing.
- **Cost**: smallest new surface but a new AuditKind is a
  documented, hash-chained event; needs care.

### Option (b) — Extend `ToolCall` payload with optional skill_* fields

- ~30 LOC: optional `skill_name` + `skill_source` fields added to
  the `call_payload` json in `turn_helpers_run.rs:226`. Populated
  iff the call originates from a skill body.
- Brief: `mission-tool-call-payload-skill-provenance` (~30 LOC +
  test).
- Regression test reuses 4A as drafted but matches on payload
  `skill_name` not on AuditKind.
- **Benefit**: smaller change; no new AuditKind variant; backward
  compat for existing audit consumers.
- **Cost**: forensic queries need a payload filter
  (`AuditKind::ToolCall AND payload.skill_name IS NOT NULL`)
  rather than a single-variant query.

### Option (c) — Reformulate 4A to use existing audit kinds

- 0 LOC code change; the criterion in MISSION.md changes to:
  > **Claim 4 LIVE (4A revised)** = with a file-backed tempdir
  > SKYCOT_HOME, `SkillStore::install` writes the skill, the
  > worker prompt rendering shows the skill with its provenance,
  > a workflow step invokes the skill's action through
  > `WorkerSessionRunner`, and **`AuditKind::ToolCall` is written
  > with tenant_id + the tool name**. Persistence proven via store
  > drop + reopen + re-render.
- Brief: `eval-claim-4-live-regression-test` (~200 LOC test only).
- **Benefit**: testable today with no substrate change.
- **Cost**: drops the explicit "provenance in audit row" guarantee;
  provenance lives only in the skill frontmatter + the worker's
  rendered prompt, not the forensic audit chain.

## My recommendation for operator

**Option (b) is the best trade**. The mission claim "skills are
shared and learned" is meaningfully weakened if forensic-grade
audit cannot tell "this tool call came from skill X version Y by
author Z". But adding a whole new AuditKind variant for one extra
payload field is heavier than warranted. Option (b) extends the
existing `ToolCall` payload with `skill_name` + `skill_source`
when the call originates from a skill — small, additive, gives the
operator the full forensic story.

If option (b) lands, the LIVE-4A regression test is ~200 LOC and
ships in a single session.

## Gap / follow-up brief candidates

| Brief | Priority | Scope | Why |
|---|---|---|---|
| `mission-tool-call-payload-skill-provenance` | P1 (if 4A locks as drafted, with option b) | ~30 LOC + test | Adds optional `skill_name` + `skill_source` to ToolCall payload; smallest viable substrate change to unlock 4A |
| `eval-claim-4-live-regression-test` | P1 (FOLLOWS the substrate brief) | ~200 LOC test only | File-backed install + render + WorkerSessionRunner invoke + audit-row payload assert |
| `mission-audit-kind-skill-invoked` | P2 (alternative to brief 1, only if operator chooses option a) | ~80 LOC | New AuditKind variant if forensic separation > minimal-surface |

**Do NOT author** until operator signs off on which option (a/b/c)
defines the LIVE criterion. Authoring before sign-off risks the same
drift the round-18 amendment hit.

## What this gives the operator

The "4A LIVE" question is no longer "do I trust the wording?" —
it's a concrete trade between **option (a)** (cleaner forensic,
heavier surface), **option (b)** (minimal surface, slightly
weaker forensic query), and **option (c)** (zero substrate change,
drops audit-grade provenance promise). All three are testable
today; the operator picks which one matches the mission framing
they want pinned.
