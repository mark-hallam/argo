# Round 75 — PR #2347 PRE-MERGE PASS (replay gateway hash envelope adapter)

- **Date**: 2026-05-26 (Melbourne 2026-05-26 10:11 AEST)
- **Round**: 75
- **Pillar/Claim**: Mission Claim 1 (Teammate-not-tool — audit/replay is the surface that proves a teammate's actions are reproducible) + supports Scene-6 of the v1.0 demo runbook
- **PR**: [argo#2347](https://github.com/mark-hallam/argo/pull/2347) `fix(replay): adapt gateway hash replay envelope` at head `3f4ad9dca05495b9f13bbbc607379a86097e80c7`, MERGEABLE, 3 files +282/-7
- **Verdict**: 🟢 **PASS** — adapter is non-vacuous, tests assert specific load-bearing behavior, fixes a real false-positive failure mode

## Setup

Round 62 noted that `/replay` page had a fabrication-gate bug (R62 referenced PR #2296). The follow-up: when the gateway DOES respond, it returns a **hash-recompute envelope** with shape `{audit_id, seq, kind, verdict, expected_self_hash, actual_self_hash, diff}` — NOT the canonical SPA `ReplayOutcome` shape (`{kind: "success"|"failed"|"unavailable"|"leak-detected", ...}`). Without an adapter, the SPA's `apiPost<ReplayOutcome>` typecast lies, and the page falls through to `leak-detected` for valid `same`/`drifted` envelopes — a false positive shown to users.

PR #2347 introduces `adaptGatewayReplay()` in [web/lib/api/replay.ts](web/lib/api/replay.ts), wired at the single fetch site, and adds a new outcome reason `unrecognized-envelope` so genuinely garbage responses are flagged as unavailable rather than leak-detected.

Live `/api/v1/replays` HTTP probe still returns 404 — the gateway hasn't yet shipped the route; the PR body explicitly defers that to a separate follow-up [argo#2344](https://github.com/mark-hallam/argo/pull/2344). This PR is a **defensive frontend shim** that becomes a no-op once gateway converges.

## Observed

**Test execution** (on PR head `3f4ad9dca`):

```
$ pnpm exec vitest run lib/api/replay.test.ts
 ✓ lib/api/replay.test.ts (7 tests) 164ms
 Test Files  1 passed (1)
      Tests  7 passed (7)
   Duration  2.99s

$ pnpm exec vitest run 'app/(app)/audit/action/[actionId]/replay/page.test.tsx'
 ✓ app/(app)/audit/action/[actionId]/replay/page.test.tsx (11 tests) 191ms
 Test Files  1 passed (1)
      Tests  11 passed (11)
   Duration  1.30s
```

**Source verification** (3 files in the diff):

- [web/lib/api/replay.ts:103-220](web/lib/api/replay.ts:103) — new `adaptGatewayReplay` function + type guards (`isReplayOutcome`, `isGatewayReplayBody`), provenance-axis mapping table, `hashBody()` formatter, single export
- [web/lib/api/replay.ts:262](web/lib/api/replay.ts:262) — wire site: `return adaptGatewayReplay(await apiPost<unknown>(path));` — type changed from `apiPost<ReplayOutcome>` to `apiPost<unknown>`, forcing every gateway response through the adapter
- [web/lib/api/replay.test.ts:135-238](web/lib/api/replay.test.ts:135) — 5 new tests covering all 4 verdict branches + the unrecognized-envelope fallthrough
- [web/app/(app)/audit/action/[actionId]/replay/page.tsx:148-204](web/app/(app)/audit/action/[actionId]/replay/page.tsx:148) — UnavailableView changed from `modelName:string` prop to full `outcome` object, renders different reason text per `outcome.reason`

## Anti-theater check

This is the load-bearing question for a defensive adapter PR: are the tests just calling the adapter and asserting it returns SOMETHING, or do they assert the **specific failure-mode-was-broken-and-is-now-fixed** behavior?

The 5 new tests assert specific, hard-coded output shapes:

1. **`same` verdict → reproducible success** with body string containing `hash_same` and `provenanceDiff.model.changed === false`. Not just "kind === success".
2. **`drifted` without policy → drifted-aligned** with `provenanceDiff.skill.from === "skill v1"` and `.to === "skill v2"`. Asserts the diff axis mapping.
3. **`drifted` WITH policy → drifted-misaligned**. This is the load-bearing safety distinction (was policy violated? → severity tier).
4. **`mystery` verdict → unavailable (NOT leak-detected)**. The fix is specifically that the pre-PR fall-through routed unknown verdicts to `leak-detected` — a false positive shown to users.
5. **`{ok:true}` garbage envelope → unavailable (NOT leak-detected)**. Same fix, different ingress path.

Tests 4+5 prove the failure mode the PR title claims to fix is genuinely closed. If I removed the adapter wire in `replayAction()` (`adaptGatewayReplay(...)` → bare `apiPost<ReplayOutcome>(...)`), tests 1-3 would fail (verdict-mapping logic absent) and tests 4-5 would silently let garbage through as `leak-detected`. That's the inverse-test confirmation needed.

**The adapter is wired at exactly one site** (`replayAction()` line 262), proving the change is not just a dead exported function that tests touch but production never reaches.

**File size check**: all 3 changed files under 500 LOC ceiling per CLAUDE.md invariant.

## Gap / follow-up brief candidates

None blocking. The PR body cites [argo#2344](https://github.com/mark-hallam/argo/pull/2344) as the gateway-side convergence work (sunset this adapter once gateway returns canonical `ReplayOutcome` shape). That follow-up is already filed.

One P3 candidate (optional, ~30 min):

- **`eval-replay-adapter-sunset-when-gateway-converges`** — when #2344 ships, the `TEMPORARY:` comment block at [replay.ts:217](web/lib/api/replay.ts:217) calls for deletion of `adaptGatewayReplay`. Worth pinning a tracking row in the ledger so this doesn't become permanent shim debt. Not blocking v1.0.

## Recommendation

**🟢 P1 MERGE** PR #2347 under the overnight `--skip-clippy` ops-* lane (3-file frontend-only PR, vitest 18/18 PASS on PR head, anti-theater check holds, defensive shim that closes a real false-positive failure mode shown to users on the demo).

This is the **64th anti-theater verification this week** (63→64 streak).
