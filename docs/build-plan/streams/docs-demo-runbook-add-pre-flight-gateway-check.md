# docs-demo-runbook-add-pre-flight-gateway-check

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6278d31c1` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-demo-runbook-add-pre-flight-gateway-check`.

🟢 **P3 DEMO HYGIENE — pre-flight runbook misses gateway-freshness check.** Source: evaluator round-2 (`docs/skycot/evals/scheduled/2026-05-22/gateway-process-stale-vs-codebase.md`):

> "`docs-demo-runbook-add-pre-flight-gateway-check`: The demo runbook (`docs/skycot/operator/demo-runbook.md`) 'Pre-flight' section already mandates `Gateway running at http://127.0.0.1:7400` but does NOT require the gateway to be running against current code. Add: `curl /health | jq .schema_version` must equal `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1`."

This is a doc-only nit to prevent the demo failure mode where the gateway has been running for 4 days and is silently missing every route added in that time. The operator follows the runbook, sees "gateway is running" ✅, and starts recording — only to hit 404s on routes that exist in the code but not in the running process.

## Scope

Edit `docs/skycot/operator/demo-runbook.md` — the "Pre-flight" section. Add a new check that:

1. Names the check clearly: `Gateway schema_version matches codebase`.
2. Shows the one-liner: `bash scripts/check-gateway-freshness.sh` (if `mission-gateway-freshness-conductor-tick` has shipped; otherwise use the inline curl+jq+ls pipeline).
3. Documents what success looks like: `STATUS: FRESH (drift <= 2)`.
4. Documents what failure looks like + fix: `STATUS: STALE → run make restart-live-gateway`.

If `mission-gateway-freshness-conductor-tick` (which authors `scripts/check-gateway-freshness.sh`) hasn't shipped yet, the inline pipeline:

```bash
GATEWAY_VER=$(curl -s http://127.0.0.1:7400/health | jq -r .schema_version)
CODE_VER=$(ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1 | grep -oE 'v[0-9]+')
echo "gateway=$GATEWAY_VER codebase=$CODE_VER"
[ "$GATEWAY_VER" = "$CODE_VER" ] || echo "⚠️  STALE — run: make restart-live-gateway"
```

Document both forms; prefer the script-form when it exists.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/skycot/operator/demo-runbook.md` — narrowly scoped: add one new check to the Pre-flight section. ~15-25 LOC.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Anything outside `docs/skycot/operator/demo-runbook.md`. This is a pure doc-only brief.
- `scripts/check-gateway-freshness.sh` — if it doesn't exist yet, document the inline form. Don't author the script under this brief (separate brief).

## Acceptance criteria

- [ ] `docs/skycot/operator/demo-runbook.md` has a Pre-flight check named `Gateway schema_version matches codebase`.
- [ ] The check is operator-runnable in <5 seconds.
- [ ] If `scripts/check-gateway-freshness.sh` exists, the doc prefers it. Otherwise the inline curl pipeline is shown.
- [ ] Failure mode documents the remediation (`make restart-live-gateway`).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `docs-demo-runbook-add-pre-flight-gateway-check`
- PR title: `docs(demo-runbook): pre-flight gateway-freshness check (docs-demo-runbook-add-pre-flight-gateway-check)`

## Notes for the implementer

- ~20-40 LOC expected. Pure doc edit.
- **Fast-mode (mechanical).** No tests, no clippy, no LOC budget concerns.
- **Be terse.** The runbook is operator-facing; don't bloat it with explanation. One paragraph + the one-liner + the failure remediation.
- Cite the evaluator report verbatim in the PR body so the trail stays visible.
- Pairs naturally with `mission-gateway-freshness-conductor-tick`. Both can ship independently.
