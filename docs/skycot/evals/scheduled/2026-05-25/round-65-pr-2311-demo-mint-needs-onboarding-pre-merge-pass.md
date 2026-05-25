# Round 65 · PR #2311 PRE-MERGE verify — `fix(demo-mint): derive needs_onboarding from real tenant state` 🟢 PASS

- **Date**: 2026-05-25 (Melbourne 2026-05-26 00:47 AEST)
- **Round**: 65
- **Pillar / Claim**: v1.0 demo MP4 ship gate — Scene 1 entry-path unblock; partly substantiates Claim 1 (Teammate-not-tool, "you already have hires") + Claim 2 (Delegation-as-unit, "your team is already set up").
- **Brief**: `docs/build-plan/streams/fix-demo-mint-needs-onboarding-flag-bounces-scene-1.md`
- **UX-audit source**: `docs/skycot/ux-audits/2026-05-25-2309/scene-1-and-2-claim-1-and-2.md` finding F7

## Verdict

🟢 **PRE-MERGE PASS** — P0 (v1.0 demo MP4 Scene-1 blocker). Recommend MERGE.

## Setup

- PR head: `551cfa13b3232f726b1e77106caea383e8a38d75`
- Mergeable: `MERGEABLE` (gh metadata)
- Files: 2 (1 modified `route.ts` +60/-1, 1 added `route.test.ts` +206/0)
- Worktree A: `/tmp/argo-r65-pr2311-wt` (PR head) — `web/node_modules` symlinked from main repo.
- Worktree B: `/tmp/argo-r65-pr2311-main-wt` (origin/main head `5edaf77aa`) — PR's `route.test.ts` copied over; `route.ts` left at main's pre-fix state.

## Observed

### PR head (worktree A): 8/8 PASS

```
$ pnpm exec vitest run app/api/auth/demo-mint/route.test.ts
 ✓ returns needs_onboarding=false when the tenant has workers + a vertical (seeded) 4ms
 ✓ returns needs_onboarding=true for a never-seeded tenant (empty roster + no verticals) 1ms
 ✓ defaults to needs_onboarding=true when only workers are present (no vertical installed) 0ms
 ✓ falls back to needs_onboarding=true when a gateway probe fails 0ms
 ✓ rejects requests without an email 0ms
 ✓ surfaces gateway errors when mint-principal fails 0ms
 ✓ sets the __Host-session and __Host-csrf cookies on success 0ms
 ✓ derives needs_onboarding from `workers.length` when `total_count` is absent 0ms
 Tests  8 passed (8) · Duration 923ms
```

### Anti-theater check: main pre-fix (worktree B) — 2/8 FAIL

```
 × returns needs_onboarding=false when the tenant has workers + a vertical (seeded)
   AssertionError: expected true to be false  (route.test.ts:69)
 × derives needs_onboarding from `workers.length` when `total_count` is absent
   AssertionError: expected true to be false  (route.test.ts:204)
 Tests  2 failed | 6 passed (8) · Duration 1.04s
```

The 2 failing tests are exactly the ones asserting `needs_onboarding === false` in seeded/legacy-shape scenarios — i.e. the **new behaviour PR #2311 introduces**. The other 6 tests pass on main because they assert `needs_onboarding === true` (which is what main hardcodes) or test orthogonal concerns (cookies, missing-email, mint-failure).

### Production-code diff (verified directly, not via test)

```diff
+ async function deriveNeedsOnboarding(sessionToken: string): Promise<boolean> {
+   const base = gatewayBase();
+   const headers = { Accept: "application/json", Authorization: `Bearer ${sessionToken}` };
+   try {
+     const [teamRes, verticalsRes] = await Promise.all([
+       fetch(`${base}/api/v1/team`, { headers, cache: "no-store" }),
+       fetch(`${base}/api/v1/verticals`, { headers, cache: "no-store" }),
+     ]);
+     if (!teamRes.ok || !verticalsRes.ok) return true;
+     const team = (await teamRes.json().catch(() => null)) as GatewayTeamList | null;
+     const verticals = (await verticalsRes.json().catch(() => null)) as GatewayVerticalsList | null;
+     const workerCount = typeof team?.total_count === "number"
+       ? team.total_count
+       : Array.isArray(team?.workers) ? team.workers.length : 0;
+     const verticalCount = Array.isArray(verticals?.verticals) ? verticals.verticals.length : 0;
+     return !(workerCount > 0 && verticalCount > 0);
+   } catch { return true; }
+ }
- needs_onboarding: true,
+ needs_onboarding: needsOnboarding,
```

- Probe is parallel (`Promise.all`) — single round-trip cost.
- `total_count` checked first, falls back to `workers.length` — handles both shapes.
- `false` only when BOTH workers > 0 AND verticals > 0 — wizard is the safe default everywhere else.
- Error path: any `!teamRes.ok || !verticalsRes.ok` or thrown exception returns `true` — no silent skip of onboarding.

## Anti-theater proof

Three independent signals:

1. **Negative test on main**: the 2 NEW `false` assertions in `route.test.ts` fail when the same test runs against unmodified `route.ts`. The PR isn't shipping a test that "rewrites itself green" — it's shipping a test that asserts behaviour main demonstrably lacks.
2. **Direct grep of main**: `grep "needs_onboarding: true" web/app/api/auth/demo-mint/route.ts` on `origin/main` returns 1 hit (the hardcoded literal at line 158); on PR head returns 0 hits — that literal is **DELETED**, not gated behind a flag.
3. **Probe shape matches gateway reality**: the test mocks `/api/v1/team` returning `{workers: [...], total_count: 5}` and `/api/v1/verticals` returning `{verticals: [...]}`. Those response shapes match what `crates/argo-cli/src/handlers/gateway/routes_team.rs` and `routes_verticals.rs` actually emit (confirmed from prior rounds — the same shapes are exercised by the SPA at `/team` and `/workers`).

## Gap / follow-up

- None. Recommend MERGE via `conductor-pre-merge-gate.sh --skip-clippy` (no Rust touched; the per-task --skip-clippy authorization for evaluator-pre-verified PRs applies).
- This is the third demo-recording unblock PR (after #2299 Scene-6 + #2301 Scene-2) that round-64/65 has cleared. Once the trio MERGE, the v1.0 demo MP4 entry path is theoretically recordable (modulo operator macOS perms + the other open Scene briefs).

## Cost note

Wall: ~7 min. Tokens: ~10K. Worktree setup amortised against later scenarios in the same round.
