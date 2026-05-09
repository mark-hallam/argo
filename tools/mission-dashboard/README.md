# Mission dashboard

Operator-side dashboard that visualizes the live state of the SKYCOT swarm
by parsing `docs/build-plan/BUILD_STATUS.md` (the conductor's live ledger),
`docs/build-plan/MISSION_BACKLOG.md` (the 280-item deep-research backlog),
and recent `git log`. Pure Python stdlib, no build step, single-file
generator.

## Run it

```bash
# One-shot: regenerate the static HTML
./tools/mission-dashboard/refresh.sh

# Long-running: serve at http://127.0.0.1:8765/, regenerate on every reload,
# auto-pull origin/main into the canonical checkout every 15 min.
./tools/mission-dashboard/refresh.sh --serve --open
```

`refresh.sh` is a thin wrapper; the underlying `refresh.py` accepts:
- `--serve` — long-running HTTP server (default port 8765)
- `--port N` — override the server port
- `--auto-pull-seconds N` — background `git fetch + pull --ff-only` cadence
  in serve mode (default 900 = 15 min, `0` disables)
- `--partial-weight 0.0..1.0` — % weight assigned to `[PARTIAL]` items in
  the MISSION_BACKLOG view (default `0.50`)
- `--open` — launch the dashboard in the default browser

## How it finds the repo

`refresh.py` resolves the canonical checkout in this order:
1. `ARGO_DASHBOARD_ROOT` env var, if set and points at a valid Argo checkout
2. Two parents above the script (works when committed under
   `tools/mission-dashboard/`)
3. Hardcoded fallback `~/code/argo`
4. `git rev-parse --show-toplevel` from the script's directory

A "valid checkout" must contain `.git/` and `docs/build-plan/BUILD_STATUS.md`,
so the dashboard refuses to render against an unrelated repo.

## What it shows

**Primary lens (live ledger from BUILD_STATUS.md):**
- Hero strip with overall %, 24h run rate, current focus phase, in-flight
  count, and BLOCKED total.
- Six phase cards (A–F) auto-titled from `▼ PHASE X — <title>` divider
  rows. Click any phase → inline drawer lists every BLOCKED / IN_REVIEW /
  CLAIMED / TODO brief in that phase.
- Live ledger funnel donut (rows by status).
- Per-owner velocity panel (24h merge count + active load per owner).
- Stale-CLAIM detector (any row CLAIMED >24h with no IN_REVIEW transition,
  scored from `git log`).

**Secondary lens (MISSION_BACKLOG 280-item view):**
- Mission-claim rollup (six load-bearing claims).
- Section progress (9 deep-research sections).
- Drill-into-items table for the 280 backlog items.

**Cross-cutting:**
- Burn-up chart over BUILD_STATUS commits.
- ETA panel: per-phase + ledger-drain projections at the current 24h rate.
- Recent activity feed (last 50 commits on origin/main).
- "Live work outside the 280-item backlog" panel — surfaces the briefs the
  swarm is shipping that MISSION_BACKLOG doesn't enumerate.

## Files generated at runtime

These are gitignored:
- `dashboard.html` — the rendered static page (regenerated on every
  server GET, or one-shot when running without `--serve`)
- `serve.log` — server stdout/stderr including auto-pull tick lines
- `__pycache__/` — Python bytecode

## Why a single ~2k-LOC Python file?

Dashboard logic, CSS, and the HTML template all live in `refresh.py`.
Splitting them into modules + a templates dir would help maintainability
but isn't worth the friction for local operator tooling that has zero
dependencies and runs locally only. The 500-LOC core-platform invariant
applies to Rust crates, not personal tooling.

If the file grows much beyond 2k LOC, the natural splits are:
- `refresh.py` — CLI + HTTP server + auto-pull loop
- `refresh/parsers.py` — BUILD_STATUS / MISSION_BACKLOG / git-log readers
- `refresh/stats.py` — phase + claim + velocity + ETA computations
- `refresh/template.html` + `refresh/template.py` — HTML template renderer
