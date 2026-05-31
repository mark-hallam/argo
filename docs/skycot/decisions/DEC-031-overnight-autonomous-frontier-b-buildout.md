---
id: DEC-031
date: 2026-06-01
status: active
supersedes: none
---

# DEC-031 — Overnight autonomous frontier-B ("everything real") build-out authorized + executed

## Decision

The operator (2026-06-01, going to bed) put the conductor **in charge of building out and
testing frontier B ("everything real", per [[DEC-030]]) autonomously overnight** — *"build
frontier b… keep pushing out and moving from one piece of work to the next, testing and
continuously going the entire time… built and tested when I wake up."* The operator's
*"you build at 2,500×"* framing is **encouragement to be maximally ambitious, NOT a literal
output claim and NOT a license to inflate the morning report** ([[DEC-029]] still governs).

This executes [[DEC-030]] frontier B under the [[DEC-026]] overnight-autonomous pattern.

## Why

Frontier A (Voice) shipped and the demo was recordable; the operator wanted the rest of the
product made genuinely real — not just the scripted demo path — while away. "Everything real" =
close the honest-gaps so any surface a viewer touches is the real thing.

## How (operating constraints held)

- **Truth-bounded ([[DEC-029]]):** every load-bearing "done/live" claim re-verified by the
  conductor with current-turn evidence (curl, commit SHA, browser walk) — never relayed from an
  agent's say-so. Morning report is honest DONE / PARTIAL / BLOCKED.
- **Proven pattern per piece:** build (worktree, deploy held) → cargo/tsc/vitest → conductor
  deploys (one gateway rebuild) → conductor live-verifies on :7400 / :3000.
- **One Rust build at a time** (the Mac runs near-OOM during gateway rebuilds; sccache /
  capped-jobs / real-disk target dir mitigate). Web + read-only work parallelized in worktrees.
- **NO THEATER ([[DEC-007]]):** where a surface had no real data source, the fix OMITS the field
  or shows an honest empty state — never fabricates (e.g. C1 telemetry dropped the invented
  per-run latency + worker attribution + promotion arc).

## Outcome (2026-06-01, verified live)

Frontier B "everything real" depth — all on `origin/main`, live on `:7400` (build `043f5ae`,
schema 87), each conductor-verified:

- Tier-1 real data (notifications / workflow-runs / knowledge / trust grades); the 4 backend-route
  gaps (workflow-head bypass-tolerance, `/api/v1/api-keys`, `/api/v1/providers/keys`,
  skills-usage name→id); the **api_key store** (full CRUD, hash-only, migration v86); **C1 real
  learning-loop telemetry** (retired the `sample-skill-telemetry` fixture — killed the camera-
  visible "184 runs" mock); **worker-memory** (real facts + conversations + audit-derived
  decisions, migration v87); the record-readiness **seed-prune**.
- Robustness: the bell-click homepage 500 crash, the full SPA enum-map crash class
  (ChannelChip / NotificationRow / ActionInputsCard), and the false "Gateway unreachable" banner
  all eliminated; the `/approvals` severity-tier card layout bug fixed.
- Plus Voice (A) and Windows's eval historical-replay seam earlier.

Demo: 🟢 **GREEN-TO-RECORD** cold, clean-8 roster.

**Known caveats (documented, not blocking the scripted take):**

1. The seed-prune is built + tested but does **not fire on the current live binary** — the rebuild
   served stale seed objects (sccache / incremental quirk; root cause unconfirmed). A re-seed will
   NOT auto-remove a worker hired live during the take. Workaround: record the Genesis hire as the
   last beat, or `argo worker archive add <id> --tenant 019e7344-78e1-77f1-ae99-dff2699683ea`
   afterward. A clean rebuild would resolve it (deferred to avoid a near-OOM rebuild at wake-time).
2. ~60 stale agent worktrees accumulated (disk ~92%); harness-managed — deferred to a careful cleanup.

Frontier C is scoped + ready (C1 done; C2 real marketplace search + C3 versioning queued).

Related: [[DEC-030]] (frontier order), [[DEC-026]] (overnight-autonomous pattern),
[[DEC-029]] (truth-bounded reporting), [[DEC-007]] (no theater), [[DEC-021]] (voice),
[[DEC-017]] (eval harness).
