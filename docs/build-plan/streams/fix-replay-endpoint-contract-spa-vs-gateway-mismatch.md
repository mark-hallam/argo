# fix-replay-endpoint-contract-spa-vs-gateway-mismatch

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-replay-endpoint-contract-spa-vs-gateway-mismatch`.
**Source**: e2e-ux-audit (scene-6-claim-6-trust-and-approvals) finding F1 — `docs/skycot/ux-audits/2026-05-26-0820/scene-6-claim-6-trust-and-approvals.md`.

🔴 **P0 DEMO-BLOCKER — Scene 6 drill-in "Replay" cannot render anything but a `failed` / `leak-detected` state against real data. The gateway returns a hash-recompute verdict; the SPA expects a sandboxed-rerun verdict. The two contracts are completely different concepts.**

## Root cause

The gateway's `POST /api/v1/audit/<id>/replay` (at `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs:81-94`) returns:
```json
{
  "audit_id": "...",
  "seq": 1840,
  "kind": "http.request",
  "verdict": "same",
  "expected_self_hash": "...",
  "actual_self_hash": "...",
  "diff": {}
}
```

The SPA's `ReplayOutcome` discriminated union (`web/lib/api/replay.ts:27-46`) expects:
```ts
{
  kind: "success" | "unavailable" | "failed" | "leak-detected",
  verdict: "reproducible" | "drifted-aligned" | "drifted-misaligned",
  result: { provenanceDiff: { model, promptTemplate, skill, memorySnapshot, policy }, ... }
}
```

**These are not the same concept**:
- Gateway = recomputing the hash chain integrity (real integrity check; good)
- SPA = re-running the action in a sandbox and diffing outputs + provenance

The two never overlap. When the SPA POSTs and gets the hash-recompute body, the discriminator (`outcome.kind`) is `undefined`. `OutcomeView` walks the if-chain and falls through to `<LeakView>` (the last `return`), which renders the destructive "Replay aborted — side-effect leak detected" banner with `details: undefined`.

Scene 6 narration calls out replay as proof of "deterministic re-execution." What the demo would actually render is either a render crash or a red leak-detected alert — neither matches the locked design.

## Curl evidence (from the audit)

```
curl -X POST "http://127.0.0.1:7400/api/v1/audit/<real_id>/replay"
→ {"verdict":"same","expected_self_hash":"...","actual_self_hash":"...","kind":"http.request",...}

No `kind:"success"` discriminator. SPA renders LeakView. Visual disaster.
```

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F1 [P0] — Replay endpoint contract mismatch: SPA expects sandboxed-rerun verdict; gateway returns hash-recompute verdict

## Fix recipe

The audit recommends Option (b) — SPA adapter — because it unblocks the demo immediately without a gateway-side schema change. Ship Option (b) first; file a follow-up brief for Option (a) to converge the contracts properly.

**Option (b) — SPA adapter (this brief)**:

1. Touch `web/lib/api/replay.ts` — add a `adaptGatewayReplay` function that maps the gateway hash-recompute envelope into the SPA's `ReplayOutcome` union:
   ```ts
   function adaptGatewayReplay(raw: GatewayReplayBody): ReplayOutcome {
     // Gateway hash-recompute verdicts
     if (raw.verdict === "same") {
       return {
         kind: "success",
         verdict: "reproducible",
         result: {
           provenanceDiff: {
             model: { matches: true },
             promptTemplate: { matches: true },
             skill: { matches: true },
             memorySnapshot: { matches: true },
             policy: { matches: true },
           },
           hashMatch: { expected: raw.expected_self_hash, actual: raw.actual_self_hash },
         },
       };
     }
     if (raw.verdict === "drifted") {
       return {
         kind: "success",
         verdict: "drifted-aligned",   // or drifted-misaligned based on diff content
         result: {
           provenanceDiff: provenanceDiffFromGatewayDiff(raw.diff),
           hashMatch: { expected: raw.expected_self_hash, actual: raw.actual_self_hash },
         },
       };
     }
     return { kind: "failed", details: { code: "unknown_verdict", message: `Gateway returned verdict=${raw.verdict}` } };
   }
   ```

2. Add a runtime guard: when the gateway response shape doesn't match either the SPA contract OR the known gateway shape, return `{ kind: "unavailable", details: { reason: "Replay returned an unrecognized envelope" } }` instead of crashing.

3. Wire the adapter into the existing replay fetch path so all callers go through it.

4. Mark the adapter `// TEMPORARY: gateway returns hash-recompute envelope. Once gateway ships the SPA-shaped response per fix-gateway-replay-envelope-spa-contract, delete this adapter.`

**Option (a) — gateway converges (separate follow-up brief)**:
File a sibling brief `fix-gateway-replay-envelope-spa-contract.md` that has the gateway emit the SPA-shaped envelope natively. Out of scope for this brief; track separately.

## Acceptance criteria

- [ ] `web/lib/api/replay.ts` includes the `adaptGatewayReplay` function with a clear TEMPORARY comment.
- [ ] When the gateway returns `verdict: "same"`, the SPA renders `<SuccessView>` with "Reproducible · hash matched".
- [ ] When the gateway returns `verdict: "drifted"`, the SPA renders `<SuccessView>` with "Drifted" badge + diff details.
- [ ] When the gateway returns an unknown verdict, the SPA renders `<UnavailableView>` (not LeakView crash).
- [ ] `OutcomeView` no longer falls through to `<LeakView>` for legitimate gateway responses.
- [ ] Unit test at `web/lib/api/replay.test.ts` covering: same-verdict → success/reproducible; drifted-verdict → success/drifted; unknown-verdict → unavailable.
- [ ] Manual smoke: `curl ... → {"verdict":"same",...}` paired with the SPA replay page → renders the locked success state.
- [ ] Sibling brief `fix-gateway-replay-envelope-spa-contract` filed for the proper convergence.

## May-touch

- `web/lib/api/replay.ts` (adapter + wiring)
- `web/lib/api/replay.test.ts` (NEW or extended)
- `web/components/skycot/audit/ReplayHeader.tsx` (verify rendering with the adapted envelope — likely no change)
- `web/app/(app)/audit/action/[actionId]/replay/page.tsx` (verify; likely no change)

## Out of scope

- Gateway-side schema change (separate brief: `fix-gateway-replay-envelope-spa-contract`)
- `synthesizeAuditDetail` fabrication problem (separate brief: `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids` already exists)
- Audit detail endpoint id mismatch (separate brief: `fix-audit-list-detail-endpoint-id-mismatch` already exists)

## Effort

~80-150 LOC across adapter + tests. ~2-3 hours. Ship before gateway convergence to unblock Scene 6 immediately.
