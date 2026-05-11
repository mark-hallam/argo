---
name: investigate_flake
description: Given a failing CI log + recent commits, produce a ranked per-cause hypothesis list, recommend the next investigation step, and classify the flake (test-bug / code-race / external-dep / environment).
inputs:
  - failing_ci_log
  - recent_commits
  - test_id
outputs:
  - ranked_hypotheses
  - recommended_next_investigation
  - classification
  - flags
  - confidence
---

# Skill: Investigate Flake

A flake is a category, not a verdict. Cam produces a RANKED hypothesis
list with evidence — operators act on rankings, not on "this is flaky."

1. **Ranked hypotheses** — list at least two candidate causes, most
   likely first, each carrying:
   - **Cause name** — a one-line label ("uses wall-clock for
     timeout", "shared DB row mutated by parallel suite").
   - **Evidence** — the log line, stack-trace frame, or commit hunk
     that supports the hypothesis. Quote verbatim, with source.
   - **Likelihood** — `high` / `medium` / `low`, calibrated against
     the evidence strength.
   - **Falsification step** — one experiment that would falsify the
     hypothesis if it ran clean.
2. **Recommended next investigation** — single concrete step. Read
   commit X. Run the test 50 times in isolation. Check whether
   service Y was rate-limiting. Avoid "investigate further" as the
   recommendation — name the experiment.
3. **Classification** — one of:
   - `test-bug` — the test itself is wrong (uses `sleep`, leaks
     state, asserts on non-deterministic output).
   - `code-race` — the production code has a race the test exposes.
   - `external-dep` — a service the test depends on is unreliable.
   - `environment` — CI runner / OS / kernel / hardware
     non-determinism (clock skew, disk pressure, OOM kills).
   If the evidence does not support a classification, return
   `unclassified` and flag the investigation — never guess a class.

Hard rules:

- Never close a flake-investigation as "intermittent" without naming
  a hypothesis. If no hypothesis ranks above `low`, flag the test
  as `needs_more_repros` and route to the test-owner.
- If the failing log contains PII (real names / emails / payment
  data) — STOP. Do not quote the log verbatim. Redact and escalate
  with `flags: ["pii_in_ci_log"]`.
- Never recommend disabling the test on main as the next step. The
  ranked options are: fix the test / fix the code / disable with
  ticket-tracked owner / ring-fence the flake. Cam ranks; humans pick.
- If `flake_history` shows the test has flaked >5 times in the prior
  30 days with no fix landed, flag `repeat_offender` and route to
  the test-owner for a deeper root-cause review.
