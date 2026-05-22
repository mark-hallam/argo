# Eval: Gateway process — 92h uptime is 6 schema-versions behind current code

**Date**: 2026-05-22
**Pillar**: Cross-pillar — affects every scenario that hits the live gateway
**Mission claim**: 3 — Platform builds workers (live demo path most affected)
**Verdict**: 🟡 **PARTIAL / OPERATIONAL** — the running gateway works for what it knows, but is **silently missing every route added in the last 4 days**.

## Setup

- `~/.argo/gateway.pid` → PID 28198.
- `GET /health` → `{"status":"ok","version":"0.0.1","schema_version":70,"uptime_seconds":333413}`.
- 333413 seconds = ~92 hours. Process started ~2026-05-18.

## Action

Compared the gateway's reported `schema_version` against the migrations present on the current `main` checkout, and probed routes that are documented in code but absent from this process.

```bash
# Current code state:
ls crates/argo-store/src/migrations/v*.rs | wc -l
# → 76

# Latest migration:
ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1
# → v76.rs

# Recent code adds workflow CRUD under /api/v1/verticals/<id>/workflows
# (per routes_workflows.rs:1-40). The running gateway:
curl -s "http://127.0.0.1:7400/api/v1/verticals/marketplace-seller/workflows" \
  -H "X-Argo-Principal: human:evaluator" -H "X-Argo-Role: owner" \
  -w "\nHTTP %{http_code}\n"
```

## Expected

A gateway started against current `main` should:
- Report `schema_version: 76` (or the head of the migrations registry).
- Respond to `/api/v1/verticals/<id>/workflows` GETs with either a JSON list or a 401/403.

## Observed

```
schema_version: 70           ← 6 versions behind
uptime_seconds: 333413        ← ~92 hours = ~3.85 days
```

```
GET /api/v1/verticals/marketplace-seller/workflows → HTTP 404
GET /api/v1/verticals/data-entry-clerk/workflows  → HTTP 404
```

The gateway returns generic `{"error":"not found"}` from `routes_orchestrator.rs:404` — because `routes_workflows::dispatch_get` (mounted by current code at `routes_dispatch_get.rs:34`) does not exist in the running binary at all.

## Anti-theater check

The route IS in the source code right now — verified at `routes_workflows.rs:7` (`"/api/v1/verticals/"` PREFIX const) and `routes_dispatch_get.rs:34` (wired). The fact that the running process returns 404 is direct evidence the binary is older than the source. The git log shows ~30 PRs landed since the gateway started:

```bash
git log --oneline --since="2026-05-18" | wc -l
# → ~100 commits (incl. status flips), with these substantive workflow PRs:
#   #1846 — workflow-definition primitive (the dependency for routes_workflows)
#   #1967 — workflow rollback CLI
#   #1969 — sub-workflow node + Add Step
#   #1971 — live-execution overlay
#   #1974 — left-sidebar palette
#   #1980 — workflow version rollback API + viewer UI + audit kind
#   #1993 — sticky notes + per-step input-data pinning
#   #1996 — time-bound temporary access
#   #1998 — http and code nodes
```

## Why this matters

Three concrete blast-radius points:

1. **The conductor's "merge a PR, see it work" gate is incomplete.** Workers ship PRs that depend on routes the running gateway doesn't have. Static `cargo build` passes; live `curl` against `:7400` 404s. The conductor + ledger don't reflect this gap.

2. **The "v1.0 demo MP4 ship-gate" recording (per `project_v1_demo_mp4_ship_gate.md`) cannot be made against this gateway.** The runbook's section 04:45 — `/workflows/sales/qualify_lead/build` — depends on routes that only exist in current-code, not in this binary.

3. **Any evaluator/auditor results inferred from `:7400` reflect the 2026-05-18 state, not current.** I would have over-reported "missing route" findings if I hadn't checked schema_version. Future evaluator runs should always:
   ```
   live_schema=$(curl -s http://127.0.0.1:7400/health | jq -r .schema_version)
   code_schema=$(ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1 | grep -oE '[0-9]+')
   [ "$live_schema" = "$code_schema" ] || echo "WARNING: gateway is at v$live_schema, code at v$code_schema — restart required"
   ```

## Gap / follow-up brief candidates

**P0 — `mission-gateway-restart-on-merge`** *(operator-decisional)*: Decide whether the swarm's gateway should auto-restart on merge (probably no — would interrupt the conductor) or whether the `/health` schema_version drift should be surfaced in the conductor tick. Suggested concrete: add a `scripts/check-gateway-freshness.sh` that compares `:7400`'s schema_version to `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1` and exits 1 if drift > 2. Conductor reads exit code; appends a "GATEWAY STALE: v70 vs v76" line to the next tick prompt.

**P1 — `docs-demo-runbook-add-pre-flight-gateway-check`**: The demo runbook (`docs/skycot/operator/demo-runbook.md`) "Pre-flight" section already mandates `Gateway running at http://127.0.0.1:7400` but does NOT require the gateway to be running against current code. Add: `curl /health | jq .schema_version` must equal `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1`.

**No-code follow-up**: When demo-recording day arrives, the operator must kill PID 28198 (or its successor), `cargo build --release -p argo-cli`, and re-spawn the gateway. Otherwise the recording will 404 at the build canvas.

## Side note — implication for the rest of this eval run

I cannot evaluate the workflow lifecycle (`scenario #4`) end-to-end against `:7400`. I pivot to the `workflow_edit_api_smoke.rs` cargo integration test, which boots its own ephemeral gateway against the current code via `tempfile::tempdir() + SKYCOT_DB_KEY_BASE64`. Results land in `workflow-build-save-publish-rollback.md`.
