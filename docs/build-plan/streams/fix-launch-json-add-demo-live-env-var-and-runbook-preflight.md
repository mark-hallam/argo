# fix-launch-json-add-demo-live-env-var-and-runbook-preflight

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-launch-json-add-demo-live-env-var-and-runbook-preflight`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-6-claim-6-trust-and-approvals.md` F1 + F1-companion + F17.

🔴 **P0 — `#2432` shipped `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` — the third env-var that opts the audit/approvals/replay clients out of auth-bypass mock short-circuits and routes them at the real gateway. But `.claude/launch.json:9` does NOT set the new var, AND still has the typo `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` instead of the consumer's expected `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. Under the default `claude preview start web` launch, the recording-day SPA bundle silently falls through to the empty live gateway: `/approvals` renders all-zero KPIs, `/audit` renders "Quiet day", Scene 6's drill-in beat has nothing to drill into. The fix is 2 LOC + a runbook pre-flight checklist update.**

## Root cause

**(1) `.claude/launch.json:9` env triplet incomplete and misnamed.** Current line:
```
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1
```

Three defects:
- `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` should be `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (the consumers in `web/lib/skycot/demo-mode.ts:37-46` read the `_ARGO_` variant).
- `NEXT_PUBLIC_ARGO_DEMO_MODE` is a stale alias with no consumer in `web/lib/`.
- `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` is missing — without it `!isDemoLive()` short-circuits don't fire on the audit/approvals/replay paths.

Effect at audit time: `isAuthBypassed()=false` (typo) AND `!isDemoLive()=true` (missing) → SPA flips into a degraded mode where most surfaces work but the audit/approvals/replay paths reach the empty live gateway and render empty states.

**(2) `web/lib/skycot/demo-mode.ts:36-47` documents** that demo-live ALSO requires a one-off `argo demo seed --tenant <demo>` step against the running gateway. That contract is not surfaced in the runbook pre-flight, so even fixing the launch.json fires up a still-empty gateway with no rows.

**(3) Scene 6 is currently unrecordable as scripted** — F17 — because the audit page renders "Quiet day" empty state, the approvals dashboard has zero rows, and the new inline Approve / Refuse buttons (post #2414) have nothing to approve. The runbook's "hover the hash-chain affordance, drill into a row, see real provenance" narration cannot be performed.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F1 [P0] — recording-day env triplet not wired in `.claude/launch.json`
- scene-6-claim-6-trust-and-approvals F1-companion [P0] — recording-mode contract documented in code but missing from runbook + launch
- scene-6-claim-6-trust-and-approvals F17 [P2] — Scene 6's `/approvals` and `/audit` are unrecordable under default-launch + live-empty-gateway

This brief supersedes-by-evolution `fix-claude-launch-json-env-var-naming-mismatch.md` (which covers the typo rename only). That existing brief should be folded into this one (or this one should reference it; same scope).

## Fix recipe

**Part 1 — Launch.json edit.** Replace `.claude/launch.json:9` runtime command with:
```
NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 pnpm --dir web dev
```
This: (a) renames the typo `_GATEWAY_AUTH_BYPASS` → `_ARGO_GATEWAY_AUTH_BYPASS`, (b) drops the stale `_ARGO_DEMO_MODE` alias, (c) adds the new `_SKYCOT_DEMO_LIVE` flag.

**Part 2 — Sweep companion scripts.** The same typo lives in:
- `scripts/demo-dry-run.sh:84` — also rename + add `_SKYCOT_DEMO_LIVE=1`.
- `scripts/capture-product-screenshots.sh:54` — same.

Verify via `rg 'NEXT_PUBLIC_GATEWAY_AUTH_BYPASS|NEXT_PUBLIC_ARGO_DEMO_MODE' .` and clean up all hits.

**Part 3 — Runbook pre-flight checklist.** Add a 5-line block to the Scene 6 section of `docs/skycot/demos/v1-mission-mvp-runbook.md`:

```markdown
### Scene 6 pre-flight (10 minutes before pressing record)

1. Launch SPA with `claude preview start web` — verify `.claude/launch.json:9` includes BOTH `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` AND `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`.
2. Run `argo demo seed --tenant 01900000-0000-7000-8000-000000000000` against the live gateway on :7400. Wait for "seeded N rows" output.
3. Verify the live gateway has >0 audit rows: `curl -s 'http://127.0.0.1:7400/api/v1/audit?range=7d' | jq '.items | length'` → expect >0.
4. Verify the live gateway has >0 approvals: `curl -s 'http://127.0.0.1:7400/api/v1/approvals' | jq '.items | length'` → expect >0.
5. Verify `/audit` and `/approvals` in the SPA render rows (not "Quiet day" or all-zero KPIs).
```

**Part 4 — Add a pre-record gate to `scripts/demo-dry-run.sh`** that fails loudly if either the env triplet is wrong OR the gateway returns empty audit/approvals after seeding.

## Acceptance criteria

- [ ] `.claude/launch.json:9` contains `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (not the typo) AND `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` (added) AND does NOT contain `NEXT_PUBLIC_ARGO_DEMO_MODE`.
- [ ] `scripts/demo-dry-run.sh` + `scripts/capture-product-screenshots.sh` use the same env triplet (no surviving typos).
- [ ] After `claude preview start web` + `argo demo seed`, `curl http://127.0.0.1:7400/api/v1/audit?range=7d` returns >0 rows AND `/api/v1/approvals` returns >0 items.
- [ ] The Scene 6 section of `docs/skycot/demos/v1-mission-mvp-runbook.md` has a pre-flight checklist covering env + seed + verify steps.
- [ ] `scripts/demo-dry-run.sh` exits non-zero if the launch env triplet is wrong OR the gateway is empty after seeding.

## May-touch

- `.claude/launch.json` (line 9)
- `scripts/demo-dry-run.sh` (env triplet + new pre-record gate)
- `scripts/capture-product-screenshots.sh` (env triplet)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 6 pre-flight section)
- `web/lib/skycot/demo-mode.ts` (no change expected; just consumer verification)

## Out of scope

- The `argo demo seed` CLI itself (covered by `fix-demo-seed-cli-for-real-roster.md` + `fix-demo-seed-cli-operator-action-runbook.md`)
- The placeholder fabrication that becomes camera-reachable after demo-live fires (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance.md`)
- The hash-chain hover affordance (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows.md`)
- The self-hash envelope shape (covered by `fix-audit-feed-include-self-hash-in-envelope.md`)

## Effort

XS — Part 1 + 2 + 3 are ~5 LOC + 2 script edits + a runbook block. ~30-45 minutes. P0 because Scene 6 is unrecordable as scripted without it.
