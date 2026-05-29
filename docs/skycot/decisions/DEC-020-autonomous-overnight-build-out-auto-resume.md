---
id: DEC-020
date: 2026-05-29
status: active
supersedes: none
---

# DEC-020 — Autonomous overnight build-out + auto-resume authorized

## Decision

The platform build-out ([[DEC-019]]) is authorized to run **autonomously** —
through the night, across usage-limit resets, and across session death — driven
by:

1. **The conductor session** as the sole Rust integrator/builder (serialized
   cargo through one warm target, disk- and memory-safe).
2. **Self-committing build-agents** for light, file-partitioned work (web/TS,
   read-only audits, brief authoring). Each agent reads `CLAUDE.md` +
   `docs/skycot/DRIFT_PREVENTION.md` + the target files and **greps for existing
   surfaces before writing** (drift-free), and **commits + pushes its own
   green work** (verify gate: tests + clippy/tsc green), run one at a time to
   avoid push races.
3. **A launchd auto-resume tick** — `com.skycot.buildout-resume` (checks every
   30 min, singleton-guarded) — that re-fires the conductor continuation after a
   usage-limit reset or a dead session. Progress persists in the build-out
   tracker, so resume is stateless: a fresh conductor reads the queue + log and
   picks up.

## Why

- Operator directive (2026-05-29 night): work autonomously, build → test →
  commit; on a blocker needing operator input, mark PARTIAL/BLOCKED and move to
  the next-highest value; auto-resume across usage limits so progress continues
  unattended. By morning: maximal real, tested, committed progress.
- The bash-loop worker swarm is intentionally **NOT** relit for this run: the
  queue is near-empty, disk is tight, and parallel-cargo memory pressure is the
  exact failure that previously halted it. The conductor + serialized-agent
  model is safer and directly controlled.

## Off-switch (documented so the operator stays in control)

```bash
touch ~/.argo/buildout-stop
launchctl unload ~/Library/LaunchAgents/com.skycot.buildout-resume.plist
```

The `~/.argo/buildout-stop` sentinel halts the autonomous build-out; unloading
the launchd plist stops the resume tick from re-firing. (Distinct from the swarm
halt sentinels `/tmp/argo-stop` + `~/.argo/stop`.)

## Guardrails

- Single Rust integrator ⇒ rebasing is largely avoided; genuine conflicts are
  reconciled inline. **No force-push** (`--force-with-lease` only if ever
  unavoidable). Never break the running demo.
- Gateway-dispatcher-touching builds (editing `routes_dispatch_*.rs` / gateway
  `mod.rs`) must NOT run in parallel with each other — serialize those.
- A fix landed mid-run: macOS has no `timeout`, so the auto-resume tick now uses
  `gtimeout`/a background watchdog (earlier overnight ticks had silently
  no-op'd). See commit `5ddaa3d6e`.

Related: [[DEC-019]] (the program this executes), [[DEC-022]] (Sprint-2 scope),
[[DEC-006]] (web/test/doc merge lane), [[DEC-005]] (skip-clippy on pre-verified).
