# fix-pre-record-seed-check-script-fail-on-roster-shortfall

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pre-record-seed-check-script-fail-on-roster-shortfall`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-1-and-2-claim-1-and-2.md` F1 [P0].

🔴 **P0 PROCESS-GATE — The pre-record dry-run script does NOT fail when the gateway is seeded with the wrong number of workers. At audit time `GET /api/v1/team` returned 1 worker (Maya) on a gateway that the runbook expects to host 5 canonical workers (Mariah/Peta/Priya/Jamal/Nori). The operator can finish the dry-run, roll the camera, and only discover the missing roster mid-take. This is an automation-gap, not a code-bug — but the demo MP4 is one operator-mistake away from a re-take loop.**

## Root cause

The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md` lines 64-77) prescribes running `argo demo seed --tenant $ARGO_DEMO_TENANT_ID` before recording. There is no automated guard that the seed actually populated 5 workers — `scripts/demo-dry-run.sh` exists but does not probe `/api/v1/team`'s `total_count` (or `workers.length`).

Observed at audit time (2026-05-27 06:23 AEST):

```bash
curl -fsS http://127.0.0.1:7400/api/v1/team | jq '.workers[].name'
# → "Maya"
# expected: "Mariah" "Peta" "Priya" "Jamal" "Nori"
```

The SPA `/team` fallback at `web/lib/api/team.ts` only fires on HTTP 0/500 (gateway unreachable). A successful 200 with `total_count=1` PASSES the fallback condition silently — so the page renders a single tile for Maya while the operator's voiceover narrates five names.

This same gap also masks: gateway boot-seed not firing (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`), demo-tenant binding drift (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`), and any future schema migration that wipes the seed mid-cycle.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F1 [P0] — Gateway seed missing: only "Maya" present (1 worker, not the 5-name canonical roster)

## Fix recipe

1. **Extend `scripts/demo-dry-run.sh`** with a "roster check" gate:
   ```bash
   roster_count=$(curl -fsS http://127.0.0.1:7400/api/v1/team | jq '.workers | length')
   if [ "$roster_count" -lt 5 ]; then
     echo "FAIL: /api/v1/team returned $roster_count workers; need 5 (Mariah/Peta/Priya/Jamal/Nori)."
     echo "Run: argo demo seed --tenant \$ARGO_DEMO_TENANT_ID"
     exit 1
   fi
   ```
   Place between the gateway-freshness probe and the SPA boot.
2. **Also probe the worker NAMES** (not just count) — the gateway could legitimately host 5 workers under different names. The expected name-set is in `web/lib/sample-personas.ts:DEMO_SEED_PERSONAS`; mirror it into a `scripts/lib/demo-roster.txt` and grep `workers[].name` for each.
3. **Add a complementary check for skills** (`/api/v1/skills` `total_count > 0`) and verticals (`/api/v1/verticals` `verticals.length > 0`) — same shape, same fail-loud principle. Pairs with `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`.
4. **Document in `docs/skycot/demos/v1-mission-mvp-runbook.md`** that the dry-run script is the SINGLE pre-record gate; recording without a clean dry-run is operator-error.
5. **Optional**: add the same probe to `scripts/capture-product-screenshots.sh:54` so screenshot captures never run against a wrong-roster gateway.

## Acceptance criteria

- [ ] `bash scripts/demo-dry-run.sh --demo-mode` exits non-zero when `/api/v1/team` returns fewer than 5 workers OR worker names diverge from `DEMO_SEED_PERSONAS`.
- [ ] Failure message names the missing personas and prints the `argo demo seed --tenant <id>` command verbatim.
- [ ] CI-side: a smoke test in `tests/scripts/demo-dry-run.spec.sh` (or shell harness) simulates a `/api/v1/team` returning 1 worker and asserts script exits non-zero.
- [ ] Runbook updated to say "if `scripts/demo-dry-run.sh --demo-mode` fails, do not record."

## May-touch

- `scripts/demo-dry-run.sh` (add roster + skills + verticals probes)
- `scripts/lib/demo-roster.txt` (NEW — canonical name list mirror)
- `scripts/capture-product-screenshots.sh:54` (optional — same probe)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (single-line note)

## Out of scope

- Fixing the underlying boot-seed bug (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`).
- Re-seeding canonical roster (covered by `fix-demo-seed-cli-for-real-roster`).
- Gateway binary freshness probe (covered by `fix-gateway-binary-freshness-auto-restart-or-warn`).

## Effort

~40-60 LOC across script + roster file + smoke test. ~30 min. P0 because a roster-mismatched dry-run that PASSES is the failure mode that wastes the operator's recording session.
