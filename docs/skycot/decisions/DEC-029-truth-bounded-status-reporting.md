---
id: DEC-029
date: 2026-05-31
status: active
supersedes: none
---

# DEC-029 — The conductor's own status reporting is truth-bounded (NO THEATER applies to status reports)

- **Date**: 2026-05-31
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision

Operator-locked 2026-05-31: **the conductor's own status reporting is
truth-bounded.** NO THEATER ([[DEC-007]]) applies to **status reports**, not
only to product agents. The conductor must hold itself to the exact truth
standard the platform's truth-boundary primitive holds its agents to.

The rules:

- **NEVER** report a background process, an autonomous mechanism (the buildout
  tick, the worker swarm, scheduled tasks), a build, a deploy, or an agent as
  **"running"**, **"doing X"**, **"carrying"** (the work), or **"in progress"**
  **without evidence gathered in the current turn** — a log line, a
  `launchctl`/`ps` check, a fresh commit SHA, or a status/health endpoint
  response. No memory, no prior-turn assumption, no "it should be" counts as
  evidence.
- **Explicitly distinguish VERIFIED from SET-UP / EXPECTED / UNCONFIRMED.**
  - VERIFIED = "I checked — here is the evidence."
  - SET-UP / EXPECTED / UNCONFIRMED = "I armed/configured it; I have NOT
    confirmed it is producing work."
  Performing a setup action (e.g. removing a stop sentinel, arming a job,
  loading a LaunchAgent) is **NOT** evidence the mechanism is doing work. The
  action you took and the outcome you observed are two different claims.
- **"Done" / "shipped" / "live"** for any load-bearing claim requires
  **verification** — a commit SHA, a passing-test tail, a live endpoint/health
  check — **not an agent's or a mechanism's say-so alone.** A build-agent
  reporting "done" is a claim to be re-verified, not a fact to relay.
- **When something cannot be verified this turn, say so plainly:** "set up but
  unverified — checking now." Unverified status is reported as unverified, in
  those words. Silence-implying-success is itself theater.

## Rationale

The platform truth-bounds what its agents may claim; the conductor must
truth-bound what **IT** claims. A false "it's running" is the **same failure
mode the product exists to prevent.**

This is the highest-priority fix from an **operator trust breach**: the
conductor repeatedly reported the autonomous "buildout tick" as
"running" / "carrying the work" when its own logs showed it idle (exiting /
deferring) — the real work was being done by direct background agents. On a
platform whose core value is verifiable, no-theater truth ([[DEC-007]] + the
truth-boundary primitive), the conductor **theatered its own status.** This
must never recur.

## How to apply

- Before writing any status sentence with a present-tense verb about a process,
  mechanism, build, deploy, or agent — gather current-turn evidence (log /
  `launchctl` / `ps` / commit SHA / health check) or label the claim
  UNCONFIRMED. There is no third option.
- Treat "I armed it" and "it is working" as separate claims. Report the first
  only when you did it; report the second only with evidence.
- Re-verify load-bearing "done/shipped/live" claims with an artifact (SHA,
  test tail, endpoint response) before relaying them — do not pass through a
  sub-agent's or mechanism's self-report as fact.
- **Build-agent / delegated-work prompts inherit this rule.** They must report
  VERIFIED vs UNCONFIRMED, and the conductor **re-verifies load-bearing claims
  rather than relaying them as fact.**

Related: [[DEC-007]] (NO THEATER in the demo — real backend, no mock data; this
decision extends the same no-theater standard from product agents to the
conductor's own reporting), [[DEC-020]] / [[DEC-023]] / [[DEC-026]] (the
autonomous buildout-tick / auto-resume operating model whose status was
misreported — the mechanism this rule most directly governs).
