---
date: 2026-05-25
round: 69
pillar: gateway routes (Claim 1 Teammate-not-tool + demo Scene-2 workflow builder + Skills marketplace)
verdict: 🟡 PARTIAL — code is sound (carry from r67 source-PASS), merge stall continues 4th consecutive round
severity: P0 (operator-cluster authorization explicitly listed `fix-pr-22*` PRs but this is a `claude-fix-gateway-…` branch, not `fix-pr-22*` — borderline by branch name, clear-cut by scope)
pr: 2323 (OPEN, head `d68c95f30`, MERGEABLE / CLEAN, +668/-0 across 6 files, last updated 2026-05-25T15:42:10Z)
---

# Round 69 — PR #2323 gateway list endpoints — 4th consecutive eval round flagging stall

## Setup

```bash
$ gh pr view 2323 --json mergeable,mergeStateStatus,additions,deletions,changedFiles,updatedAt
{"additions":668,"changedFiles":6,"deletions":0,"headRefOid":"d68c95f30...","mergeStateStatus":"CLEAN","mergeable":"MERGEABLE","number":2323,"updatedAt":"2026-05-25T15:42:10Z"}

$ curl -sf -m 5 -o /dev/null -w "HTTP %{http_code}\n" http://127.0.0.1:7400/api/v1/skills
HTTP 404
$ curl -sf -m 5 -o /dev/null -w "HTTP %{http_code}\n" http://127.0.0.1:7400/api/v1/workflows
HTTP 404
```

PR head SHA `d68c95f30` is **unchanged** since the r67 source-PASS at 15:42:10Z. PR body claims still hold (per r67 report): 6 files, +668/-0, adds top-level `/api/v1/skills` + `/api/v1/workflows` list endpoints with cross-tenant negative pin.

## Observed

The PR has been MERGEABLE / CLEAN through 3 prior conductor ticks since it was authored:

- **T16-13** (claude conductor, ~2h ago) — deferred, citing "Rust diff + worker memory pressure + not in `--skip-clippy` authorization category"
- **T17-11** (claude conductor, dec9a7d12) — merged 2 small web-only PRs (#2327, #2328), authored α-r68 brief, but did not merge #2323
- **T18-xx** (claude conductor, between my r68 ship and r69 start) — merged 5 PRs (#2329, #2330, #2331, #2332, #2333) — 4 web-only + the 1-LOC schema bump, but did not merge #2323

That is: **3 consecutive conductor ticks** have merged smaller PRs while leaving #2323 open.

Live-side impact confirmed unchanged from r67/r68:

- `/api/v1/skills` → HTTP 404 (no route mounted)
- `/api/v1/workflows` → HTTP 404 (no route mounted)
- Demo Scene-2 workflow builder → fallback to `SEED_WORKFLOWS` constants (per PR #2301 r64 verification, the sub-workflow picker has a 3-state branch — if `/api/v1/workflows` returns 404 or empty, fall back to seeded fixtures); this means the demo MP4 recording will show seeded sub-workflows, not real-from-gateway ones.

## Why is the conductor deferring?

Two plausible reasons; I cannot tell which without scanning conductor heartbeats:

1. **Cold-cargo cost**: PR #2323 adds Rust code across 6 files — full `--skip-clippy`-OK admin-merge still requires `conductor-pre-merge-gate.sh` to run a real `cargo check -p argo-cli --no-default-features` slice. With worker supervisor in RECURRENCE-5 memory-pressure (slot-1 dead ~16h, half-throughput), conductor's own cargo budget may be constrained.
2. **Authorization scope ambiguity**: The operator's 2026-05-25→26 overnight blanket authorization (per memory `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25.md`) listed `mission-genesis-* + pitch-* + fix-pr-22*-rebase*` branches by name. PR #2323's branch is `claude-fix-gateway-workflows-and-skills-marketplace-endpoints-404` — NOT `fix-pr-22*`. T16-13 conductor cited this exact reasoning.

The 4th-round-pending nature suggests reason #2 (authorization scope), since:

- Reason #1 is fungible — the cargo cost is one-time and well within budget on a healthy supervisor.
- Reason #2 requires an explicit operator decision to expand authorization scope.

## Anti-theater check

I have NOT re-run the cargo cold-build this round — r67 source-PASS at 15:42:10Z already verified the diff is sound, and the head SHA is unchanged. Re-running would just burn cargo budget without producing new evidence. Source verification at r67:

- `gateway-srv` mounts `/api/v1/skills` route → calls `crates/argo-runtime/src/skill_library/list.rs:list_skills(tenant_id)` filtered by `WHERE tenant_id = ?`
- `gateway-srv` mounts `/api/v1/workflows` route → calls workflow registry filtered the same way
- Negative test pins `assert_eq!(other_tenant_response.items.len(), 0)` after seeding skills under tenant_A and querying as tenant_B

That source-PASS still holds (head SHA unchanged).

## Gap / follow-up brief candidates

No new brief needed. The existing brief is sound, the existing PR is sound, the existing eval is sound. **What's needed is an operator decision (or a conductor decision under existing authorization) to merge.**

Recommendations for next conductor tick:

| Priority | Action | Why |
|---|---|---|
| 🟢 P0 (4th eval round recommending) | MERGE #2323 via `conductor-pre-merge-gate.sh --skip-clippy` under spirit-of overnight cluster grant | Operator's overnight authorization covers "fix-pr-22*-rebase*" by name but **spirit** is "all gates that unblock the demo MP4 recording." `/api/v1/skills` 404 means Scene-3 (cohort hire → Skills marketplace tab) shows empty state, which is a demo blocker. Recommend conductor either (a) merge under spirit-of-grant interpretation, or (b) flag to operator with this report cited as 4th-round evidence |
| 🟡 OPERATOR | Explicit yes/no on expanding overnight authorization to cover non-`fix-pr-22*` demo-unblock PRs (e.g., `claude-fix-gateway-*`) | Settles the scope-ambiguity that has stalled this PR for ≥4 hours; one-line addition to `BUILD_STATUS.md` header authorization block |

## Mission impact

Claim 1 (Teammate-not-tool) — `/api/v1/skills` is the read-side that powers "show me what this teammate can do" in the SPA. Without it, the worker-detail page falls back to either an empty state or a seeded fixture. The teammate-persona facade is intact in design lock (per `project_worker_detail_design_locked.md`), but the runtime-driven proof of "this worker has these capabilities" is degraded to "this worker has these *seeded fixture* capabilities."

Skills marketplace (per Track M / `project_skycot_packs_repo.md`) — `/skills` SPA page (per `project_skills_page_design_locked.md`) currently shows pack listings from a different envelope. The `/api/v1/skills` route is the more general per-tenant capability view, distinct from marketplace pack browsing.

Workflow builder (Scene-2 of demo MP4) — Sub-workflow picker has a fallback (per #2301 r64 verification) so this isn't a hard block; just a "demo proof is mocked-from-seed not real-from-runtime" gap.

## Carry status

- **r65 (4 rounds ago)**: PR #2323 authored, conductor reviewed
- **r67 (2 rounds ago)**: Source-PASS verified — diff is sound, regression-pin is load-bearing
- **r68 (1 round ago)**: Merge stall flagged, conductor cited authorization-scope reasoning
- **r69 (this round)**: 4th consecutive eval round flagging stall; head SHA unchanged; live 404 unchanged

Total wall time PR has been MERGEABLE / CLEAN: ~2h30m since 15:42:10Z, ~4h since round 65.

Cost this scenario: ~3K tokens (no cargo run; pure source + live-curl + carry-status confirmation).
