# fix-demo-seed-cli-operator-action-runbook

**Owner**: claude or codex.
**Authored against**: origin/main `48dc620b1` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-seed-cli-operator-action-runbook`.
**Source**: UX-AUDIT-T00-28 DELTA.md load-bearing root cause ("the gateway seed is empty — 1 worker `Maya/marketing-intern`, 0 workflows, 0 approvals, 0 skills, 0 agent-turn audit rows"). Follow-up to `fix-demo-seed-cli-for-real-roster` MERGED-STALE-FIXED.

🔴 **P0 — The `argo demo seed` CLI now exists on main (shipped by #2284 + #2291 on 2026-05-25), but the LIVE gateway DB stays empty until the CLI is run against it. No SPA lens reads ≥4 GREEN until either an operator runs the CLI or the gateway auto-seeds on bootstrap.**

## Root cause

`#2284` (`demo: canonize seed worker roster with Peta`) merged 2026-05-25T10:28Z and `#2291` (`demo: argo demo seed CLI for real-backend v1.0 demo workspace`) merged 2026-05-25T15:12Z. Both landed >9h before the T00-28 UX audit, which still observed the live gateway returning only 1 worker (`Maya/marketing-intern`) + 0 workflows / 0 skills / 0 audit rows.

The CLI works; nothing invokes it against the live gateway DB.

Parallel precedent: post-#2323 (gateway CORS routes), the operator had to run `bash scripts/restart-live-gateway.sh --force --no-prompt` for routes to actually serve. Same pattern here — code-side LIVE, operator-action required to materialise the demo state.

## Fix recipe

Two tracks. Pick one (or both) at claim time:

### Option (a) — runbook surfacing (minimum-viable, ~30-60 min)

Add a new "Run the seed CLI" pre-record step to `docs/skycot/demos/v1-mission-mvp-runbook.md`, placed immediately after the `restart-live-gateway.sh` step. The step instructs the operator:

```bash
# After gateway is healthy on port 7400
argo demo seed --tenant <demo-tenant-id>

# Verify
curl http://127.0.0.1:7400/api/v1/team | jq '.workers | length'
# expected: 5  (Mariah, Peta, Priya, Jamal, Nori)
```

Include: where to find the demo tenant id (`argo tenant list | grep demo`), how to confirm success (curl `/api/v1/team`, `/api/v1/workflows`, `/api/v1/skills` each return >0 rows), and what to do if the seed is rerun (idempotent — `INSERT OR IGNORE` semantics per #2291).

### Option (b) — auto-seed-on-bootstrap (~60-90 min, recommended)

In `crates/argo-cli/src/handlers/gateway/boot.rs`: if `ARGO_DEMO_AUTO_SEED=1`, after migrations apply, check whether the demo tenant `team` table is empty. If empty, invoke the seed-CLI library function (same code path as `argo demo seed`) once and log the result. Idempotent by construction.

Optionally extend `scripts/restart-live-gateway.sh`: when `ARGO_DEMO_AUTO_SEED=1`, the restart script sets the env var on the new gateway process so the auto-seed runs on first healthy boot.

## Acceptance criteria

- [ ] (Option a) `docs/skycot/demos/v1-mission-mvp-runbook.md` documents the seed-CLI operator step with verification curls.
- [ ] (Option b — if implemented) A live gateway restarted with `ARGO_DEMO_AUTO_SEED=1` against an empty demo tenant populates 5 workers + N workflows + N skills on first healthy boot.
- [ ] (Option b — if implemented) Auto-seed is idempotent — second boot does NOT duplicate workers (curl `/api/v1/team` still returns exactly 5).
- [ ] `curl http://127.0.0.1:7400/api/v1/team | jq '.workers | length'` returns 5 against a seeded gateway.
- [ ] Pre-merge gate clean (LOC budget; if option b touches Rust, `cargo check --tests` PASS).

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (option a — required)
- `crates/argo-cli/src/handlers/gateway/boot.rs` (option b only — auto-seed gate behind env var)
- `scripts/restart-live-gateway.sh` (option b only — surface the env var through the restart path)

## Out of scope

- Programmatically seeding workflows + skills + audit rows beyond what `#2291`'s `seed_workers` already covers. Brief assumes the existing CLI is the source of truth — extending the CLI's seed surface is a separate brief.
- Auto-seed for non-demo tenants. The env-var gate is the safety boundary.

## Effort

- Option (a) only: 30-60 min, 1 file, doc-only.
- Option (b) implemented: 60-90 min, +20-40 LOC Rust + bash, idempotency test in `crates/argo-cli/tests/` or `crates/argo-demo-seed/tests/`.

## Why this matters

This brief closes the single load-bearing root cause for the T22-20 → T00-28 MISSION-claim regression (4🟢 → 1🟢 under cold-investor lens). Without it, the demo MP4 recording opens to an empty gateway regardless of how complete the SPA is. With it shipped + run, the next UX audit should jump back to ≥4 🟢.
