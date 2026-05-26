# Round 76 — PR #2347 POST-MERGE anti-theater (replay envelope adapter)

- **Date**: 2026-05-26 (Melbourne 2026-05-26 11:13 AEST / 01:13 UTC)
- **Round**: 76
- **Pillar / Claim**: Claim 6 (multi-tenant + multi-channel) — the Scene-6 demo-replay
  drill-down — and a Track P "no theater" follow-on (the false-positive
  `leak-detected` banner shown to demo viewers from garbage gateway responses).
- **Verdict**: 🟢 PASS · close R75-S2 carry · severity tier N/A (no follow-up).

## Setup

PR #2347 (`fix(replay): adapt gateway hash replay envelope`) was PRE-MERGE-PASSED
at round 75 (head `3f4ad9dca`). The conductor merged it 2026-05-26T01:10:33Z as
squash commit `3f4ad9dca05495b9f13bbbc607379a86097e80c7` on `origin/main` —
literally ~1 minute before this round started, so the head we verified at r75 is
the same commit now landed.

Reran the same vitest specs against current `origin/main` (post-merge, not on PR
head) to confirm:

- the adapter wire shipped intact through the squash,
- the assertions still pin down the false-positive bug,
- no regression from other web changes in the 9 commits added since merge.

```bash
cd web && pnpm exec vitest run \
  lib/api/replay.test.ts \
  'app/(app)/audit/action/[actionId]/replay/page.test.tsx' \
  --reporter=verbose
```

## Observed

**23/23 PASS** in 3.62s on `origin/main`:

- `lib/api/replay.test.ts` — 12/12 PASS (7 pre-existing fall-through guards
  + 5 new gateway-adapter cases).
- `app/(app)/audit/action/[actionId]/replay/page.test.tsx` — 11/11 PASS.

The 5 load-bearing new assertions all green:

| Assertion | Verdict mapped |
|---|---|
| `same` verdict → `reproducible` w/ `provenanceDiff.model.changed === false` | ✅ |
| `drifted` no-policy → `drifted-aligned` w/ skill from→to mapping | ✅ |
| `drifted` w/ policy axis → `drifted-misaligned` | ✅ |
| unknown verdict → `unavailable` (NOT `leak-detected`) | ✅ |
| `{ok:true}` garbage envelope → `unavailable` (NOT `leak-detected`) | ✅ |

Wire site verified on main at [web/lib/api/replay.ts:265](web/lib/api/replay.ts:265):

```typescript
return adaptGatewayReplay(await apiPost<unknown>(path));
```

The `unknown` type return on `apiPost` is what forces every gateway response
through `adaptGatewayReplay()` — there is no untyped escape hatch. The two
defensive branches inside the adapter that fix the false-positive:

- [replay.ts:209–210](web/lib/api/replay.ts:209) — body shape mismatch →
  `unavailableReplay("Replay returned an unrecognized envelope.")`
- [replay.ts:227](web/lib/api/replay.ts:227) — unknown verdict →
  `unavailableReplay(\`Replay returned an unknown verdict: ${...}\`)`

Both bypass the prior fall-through that landed every unhandled shape on
`leak-detected`. The `unavailable` branch is what the page renders with the
neutral "Replay unavailable" fallback ([replay/page.test.tsx:14ms case]).

## Anti-theater check

Three vacuity vectors checked:

1. **Are the new assertions actually fired by the production code path?** Yes
   — the wire at line 265 is the SPA's *only* call into the gateway from
   `replayAction()`. Strict mode is `apiPost<unknown>` so the adapter cannot
   be bypassed by mis-typed responses.
2. **Could a future test author "fix" this by re-asserting the old
   behavior?** No — the test names spell out the negation:
   *"maps unknown gateway verdicts to **unavailable instead of leak-detected**"*
   and *"maps unrecognized replay envelopes to **unavailable instead of
   leak-detected**"*. Anyone deleting that semantic change has to rename
   the test, which trips review.
3. **Is the fix scope creep beyond the demo gate?** No — 3 files
   (`replay.ts`, `replay.test.ts`, `replay/page.tsx`), all in `web/`, no
   gateway-side change. PR body cites #2344 as the gateway-side convergence
   follow-up that would let us delete the adapter once the gateway honors
   the canonical `ReplayOutcome` contract.

## Gap / follow-up

None new. The previously-recommended follow-up #2344 (gateway-side envelope
SPA-contract convergence) is operator/swarm-territory not evaluator. R75-S2
carry is now CLOSED.

This is the **65th anti-theater verification this week**.
