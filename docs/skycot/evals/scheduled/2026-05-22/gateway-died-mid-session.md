# Eval: Operational — main gateway (PID 28198) died mid-eval session

**Date**: 2026-05-22 (round 2)
**Pillar**: Operational / fleet-monitoring
**Mission claim**: 3 — Platform builds workers (the live demo path)
**Verdict**: 🟡 **OPERATIONAL FINDING** — the main `argo gateway start` process died sometime between round-1 wrap (~12:30 PM Melbourne) and round-2 start (~8:00 PM Melbourne); a watchdog restart attempt failed.

## Setup

Round 1 wrote against a live gateway at `http://127.0.0.1:7400` (PID 28198, uptime ~92h at probe time). Round 2 reopened — same probe returns `Connection refused`.

## Action

```bash
ps -p 28198             # the old PID
cat ~/.argo/gateway.pid # the canonical pid file
cat ~/.argo/gateway.log # any boot log
lsof -iTCP:7400 -sTCP:LISTEN
```

## Observed

```
$ ps -p 28198
  PID TTY           TIME CMD
# ← empty. PID 28198 is gone.

$ cat ~/.argo/gateway.pid
cat: ~/.argo/gateway.pid: No such file or directory

$ cat ~/.argo/gateway.log
nohup: ./target/release/argo: No such file or directory

$ lsof -iTCP:7400 -sTCP:LISTEN
# ← empty.
```

The `gateway.log` line is the smoking gun: something (a watchdog, the conductor, or a launchctl-style restarter) tried to bring the gateway back up by invoking `./target/release/argo`, but that path doesn't exist on this machine — the gateway runs from `/tmp/argo-shared-target/release/argo` (per the earlier `ps` output for PID 28198). The restart command was written against the wrong build path.

Side-effect: `~/.argo/sessions.db` is still on disk (3.3 MB) and intact — the 7 EVAL workers from round 1 + the conductor's actual workspace state survived. But no in-process gateway means nothing can read or write that DB right now.

The 4 small test gateways spawned by integration tests during the day (`--port 50522 / 50519 / 50964 / 51425`) are still alive but each has its own tempdir DB; they're not the production gateway and can't be used to verify against the real workspace state.

## Anti-theater check

The proof that the main gateway crashed (vs. was deliberately stopped) is partly circumstantial:

1. No clean shutdown log — the gateway has no `Shutting down` line in `~/.argo/gateway.log`.
2. The pid file was removed — implying a process did cleanup, but the new gateway never came up.
3. The watchdog restart command (`nohup: ./target/release/argo: No such file`) suggests an external operator-side or conductor-side restart attempt — broken because the binary path is wrong.

I cannot pin down exactly when between round-1 wrap and round-2 start the process died. The worker NDJSON logs in `~/.argo/worker-logs/` show worker activity continuing past round 1 (latest log `20260522T090505Z-...ndjson` modified at 19:17), so the swarm wasn't fully blocked — but those workers don't need the gateway, they're claude-code subprocesses driving git/cargo directly.

## Concrete impact on this eval session

- 🔴 Scenario 6 (cross-tenant audit-scoping) — cannot verify the prod fix shipped in #1989/#1992 against a real running gateway. Reverted to running the cargo unit test added by those PRs (passes).
- 🔴 Scenario 7 (worker downstream state) — cannot query the 7 EVAL workers' overview/duties/memory via `GET /api/v1/workers/<id>/...`. Reverted to static source-read + argo-identity cargo unit tests (all 6 pass).
- 🟢 Scenario 8 (marketing routes) — the Next.js SPA dev server at `:3000` doesn't depend on the gateway for `/landing`, `/compare`, `/pricing`, `/customers`, `/docs`, `/changelog`, `/security`, `/privacy`, `/terms`, `/sla`, `/help`, `/for/*` — all 17 returned HTTP 200.

## Gap / follow-up brief candidates

**🟡 P1 — `mission-gateway-restart-runbook`**: Per the conductor's `5df7add84 briefs: replenish — tick-50 cron-fired (rebase-dirty-cluster + gateway-restart-runbook from auditor T09-40)`, a brief for gateway restart procedure already landed today. Good — this scenario is the second voice confirming it's needed. Acceptance for that brief should include: (a) correct binary path (`/tmp/argo-shared-target/release/argo`, not `./target/release/argo`), (b) drop-stale-pid-file step, (c) wait-for-listener-then-write-pid step, (d) `:7400` health probe before declaring "up".

**🟢 INFO**: Once the new gateway is up, re-run scenarios 6 + 7 against it. The fixes from #1989/#1992 should make audit_detail return 404 for cross-tenant probes, and `/api/v1/workers/<id>/overview` should return enriched bundles for the 7 EVAL workers.

**🟢 INFO — operator note**: This kind of operational state change (the running gateway dying or being restarted) is something the evaluator should DETECT and surface at the **start** of every run, not as a side-discovery mid-session. Adding to my own session-start checklist: `curl :7400/health` first. If 200 → record schema_version. If anything else → log and pivot to static / cargo-test only.
