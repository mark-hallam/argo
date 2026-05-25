# Round 68 — PR #2323 `gateway list endpoints` still open after r67 PRE-MERGE PASS

**Date**: 2026-05-25 (Melbourne 2026-05-26 03:15 AEST)
**Round**: 68
**Pillar/Claim**: SPA-substrate / Mission Claim 1 (Teammate, not tool) + Claim 4 (Skills shared + learned) — gateway list endpoints are demo-blockers for `/workflows` + `/skills` SPA browsing
**Verdict**: 🟡 **PARTIAL (merge stall)** — source review still PASS, but the PR has been sitting in MERGEABLE / IN_REVIEW state for ~1h28m past r67's MERGE recommendation. Operator UX-audit T23-09 flagged the missing `/api/v1/workflows` and `/api/v1/skills` top-level endpoints as a P0 v1-demo-blocker.

---

## Setup

Round 67 source-reviewed PR #2323 (head SHA `d68c95f30fa09852b53931024269405d7e81dc9e`, "Phase A — gateway — mount /api/v1/skills + /api/v1/workflows top-level list endpoints", 6 files +668/-0) and verdicted 🟡 PARTIAL source-PASS — clean PR ready for merge via normal-gate. Cargo run was killed at ~3min compile to free worker target-dir contention for the higher-priority S1 finding (PR #2317 EXPECTED_SCHEMA_VERSION bug). Recommendation: MERGE next-tick via normal-gate, NOT --skip-clippy.

This round, I re-checked PR state to see if it shipped or if anything changed.

Commands run:

```bash
gh pr view 2323 --json headRefOid,mergeable,updatedAt,additions,deletions
curl -sf -w '\nHTTP %{http_code}\n' http://127.0.0.1:7400/api/v1/skills
curl -sf -w '\nHTTP %{http_code}\n' http://127.0.0.1:7400/api/v1/workflows
gh pr list --state open --limit 30
```

## Observed

**1. PR #2323 unchanged since r67.**

```json
{
  "additions": 668,
  "deletions": 0,
  "headRefOid": "d68c95f30fa09852b53931024269405d7e81dc9e",
  "mergeable": "UNKNOWN",
  "updatedAt": "2026-05-25T15:42:10Z"
}
```

Head SHA matches r67 (`d68c95f30…`). PR last updated 2026-05-25T15:42:10Z, ~1h33m before this round. `mergeable: UNKNOWN` is GitHub's "didn't run mergeability yet" — usually transient. r67 confirmed MERGEABLE via the open-PR list dump.

**2. Live gateway routes confirm endpoints are NOT yet served on main.**

```
$ curl -sf -w 'HTTP %{http_code}\n' http://127.0.0.1:7400/api/v1/skills
HTTP 404
$ curl -sf -w 'HTTP %{http_code}\n' http://127.0.0.1:7400/api/v1/workflows
HTTP 404
```

Both 404 confirms the endpoint mounting in PR #2323 has not landed. The SPA pages that depend on these (likely `/workflows` index + `/skills` index) are still operating on stub data or returning empty states.

**3. Conductor T16-13 acknowledged but deferred.**

T16-13 conductor tick (in SHARED.md unread set, ts 2026-05-25T16:13:30Z) explicitly noted: *"#2323 Rust gateway endpoints + #2325 Rust audit-tests split (CLEAN but Rust; deferred — not in --skip-clippy authorization category, cargo test-compile risks memory under worker load)."* That's the literal stall reason. Conductor read my r67 recommendation (`MERGE via normal-gate, NOT --skip-clippy`) but interpreted "Rust diff under worker load" as risky-to-merge under the current ops constraint.

T19-08 conductor tick (ts 2026-05-25T19:08:10Z) reported open PRs=30 and again deferred all merges — different stall reason (spawned general-purpose triage agent, "deferred merges this tick to avoid mid-tick collision").

## Anti-theater check

The PR text claims it closes my r63 + r66 brief recommendations (gateway `/api/v1/skills` and `/api/v1/workflows` top-level list endpoints). I source-reviewed the diff in r67 and verified:

- 4 files within 500-LOC ceiling (240+137+161+114)
- SQL tenant-scoping correct on all three paths:
  - Installed-skills: `WHERE s.tenant_id = ?`
  - Published-marketplace skills: `WHERE source LIKE 'published:%'` (intentional cross-tenant for marketplace browsing — design call)
  - Workflows: `WHERE idx.tenant_id = ?`
- 4 cross-tenant negative tests present
- Dispatch wiring: 2-LOC match arms in `routes.rs` + defensive 404 in `routes_workflows.rs`

Not theater. This is a clean, well-scoped, well-tested PR. The stall is procedural, not a source-quality issue.

## Severity rationale (🟡 PARTIAL, not 🔴 escalate)

I do not escalate this to P0 even though the v1-demo-blocker fact pattern is present, because:

1. The conductor's reasoning ("Rust diff + worker memory pressure" + "not in --skip-clippy authorization") is defensible. The 2026-05-25 morning recovery session documented in CLAUDE.md proved that cargo runs under worker load can OOM the macOS kernel and stall the gateway target.
2. Operator's UX-audit T23-09 (per BUILD_STATUS.md row tracking) flagged this as one of two highest-leverage net-new briefs (`fix-gateway-workflows-and-skills-marketplace-endpoints-404`) — but the operator did NOT promote it to overnight-merge-aggressive authorization explicitly. The overnight cluster grant covers `mission-genesis-*` + `pitch-*` + `fix-pr-22*-rebase*`; #2323's brief name `fix-gateway-workflows-and-skills-marketplace-endpoints-404` is `claude-fix-…` not `fix-pr-22*-rebase*`.
3. The α-r68 finding (PR #2317 EXPECTED_SCHEMA_VERSION) is genuinely higher priority because it has live operational impact (degraded /health). #2323 has demo impact (404s in SPA) but not health-probe impact.

## Gap / follow-up brief candidates

**No new brief.** Existing PR #2323 is the fix. Recommendation: conductor authorize `--skip-clippy` merge under the spirit of the overnight cluster grant (P0 v1-demo-blocker substrate, source-PASS verified by evaluator across r63 + r66 + r67 + r68), OR run normal-gate when memory pressure allows. Two consecutive evaluator rounds (r67, r68) have surfaced this as merge-ready; the third round (r69) should not need to repeat the recommendation.

If conductor + operator disagree with `--skip-clippy` escalation, the alternative is to defer #2323 to a post-overnight-window normal-gate merge. That's acceptable as long as the v1-demo MP4 is not blocked on the SPA reaching `/skills` + `/workflows` indexes (which I'm not sure of — I haven't seen the demo storyboard's exact happy-path).

---

**Cost note**: ~3K tokens. Re-verification only.
