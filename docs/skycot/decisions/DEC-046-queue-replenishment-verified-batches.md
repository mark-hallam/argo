# DEC-046 — Queue replenishment: never-zero + verified batches (supersedes the 2026-05-10 depth amendment)

- **Date:** 2026-06-10
- **Status:** **active** — operator-approved 2026-06-10 ("yes change it to
  your recommendation") in the morning conductor session.
- **Owner:** conductor (Claude); applies to every conductor session and any
  agent authoring briefs.
- **Supersedes:** the operator amendment of 2026-05-10 in the
  BUILD_STATUS.md header ("queue-depth check: `claude` TODO ≤ 25 → fire
  replenishment… target depth 30–40").

## 1. Why

Two findings from lived swarm history:

1. **Mass authoring to a numeric target produced the drift.** The audit
   chain of 2026-05-13/14 measured a ~24% intent-drift rate in briefs
   authored in bulk (parallel surfaces, stale premises). The practice that
   actually works — used overnight 2026-06-09 and through 2026-06-10 — is
   small **verify-then-author** batches where every premise is checked
   against current code before the brief exists.
2. **The threshold "≤25" stopped carrying information.** Queue depth now
   rarely exceeds 25, so the old rule kept the conductor permanently "in
   replenishment", while the REAL failure signal — workers standing down on
   an empty queue — was what mattered (observed twice on 2026-06-10:
   TODO=0, workers exit rc=1 and idle-respawn, burning quota for nothing).

## 2. The rule

- The TODO queue must **never sit at 0 while a conductor session is
  active**. TODO=0 with workers running = replenishment is overdue NOW,
  before merges (workers idling burn quota with zero output).
- Replenish in **verified batches of 5–10 briefs** via
  discovery → curate → author → `scripts/lint-brief.sh`, every premise
  verified against current code (verify-before-fix; freshness header +
  validate-before-claim line mandatory).
- **Depth target 10–15** fresh, claimable rows. More is allowed when a
  verified source produces it (e.g. an audit), but never author filler to
  hit a number.
- Brief sources, in priority order: operator directives → security/audit
  findings → DEC-042 stage items → PR-body deferrals + boundary notes →
  mission-gap discovery fan-outs.

## 3. Verifiable

- A conductor tick that ends with TODO=0 is a FAILED tick (the CRANK.md §5
  close-out requires TODO > 0).
- Every replenishment batch's briefs pass `lint-brief.sh` and carry
  `Authored against: origin/main <SHA>` headers dated within 24h.
- The north-star heartbeat (`queue_depth.todo`) gives the longitudinal
  signal; sustained 0s while a session was active indicate the rule was
  skipped.
