---
name: triage_failing_build
description: Given a failing build's test report + recent commit log, categorize each failure into one of four buckets — genuine regression, flaky-test, infra-related, environment-drift — with severity, suggested-next-action, and a most-likely-commit guess (with confidence) for regressions. Autonomous; written only to the report sink.
inputs:
  - build_id
  - failing_test_report
  - recent_commit_log
  - prior_flake_history
outputs:
  - failure_categorizations
  - per_failure_severity
  - per_failure_next_action
  - flags
  - confidence
---

# Skill: Triage a failing build

Per-failure triage on a build that came back red. Every failure
lands in exactly one of four buckets, with a next-action and a
severity. The bucket vocabulary is the operator contract — do not
invent new categories.

1. **Four categories, exhaustive and exclusive:**
   - `regression` — production code changed and the test now fails.
     Evidence: the failure correlates with a recent commit
     touching the code under test, and prior runs were green.
   - `flaky` — historic intermittent failure with no recent code
     change explaining it. Rerun is the likely fix.
   - `infra` — CI runner / pipeline machinery failure: timeout,
     OOM kill, dependency-pull error, container start failure,
     artifact upload failure.
   - `environment_drift` — versions, services, or fixtures the test
     depends on have changed: image tag bumped, DB schema drifted
     from fixture, third-party API returned a new shape.
   If the evidence does not support any bucket, return
   `unclassified` for that failure and flag it — never guess.
2. **`most_likely_commit` is a GUESS, not a verdict.** For each
   `regression`, emit `most_likely_commit: <sha>` + `confidence:
   high|medium|low`. `high` = commit touched a file directly under
   test; `medium` = same-crate / same-package file; `low` = anything
   else. Never emit a regression entry without both fields.
3. **Severity is per-failure.** `critical` (security / auth /
   payment / data-integrity) / `high` (user-visible feature in a
   primary journey) / `medium` (internal or secondary path) / `low`
   (flake or transient infra). Aggregate severity is the operator's
   call; this skill emits per-failure rows only.
4. **`suggested_next_action` is one concrete step per failure.**
   `rerun` (flaky) / `bisect <sha-a>..<sha-b>` (regression) / `check
   runner logs for OOM` (infra) / `verify fixture vs schema`
   (environment_drift). Avoid "investigate" — name the experiment.
5. **Output JSON shape:**
   ```json
   {
     "build_id": "...",
     "failure_categorizations": [
       {
         "test_id": "...",
         "category": "regression",
         "most_likely_commit": "abc123",
         "confidence": "medium",
         "severity": "high",
         "suggested_next_action": "bisect abc100..abc123",
         "evidence": "<one-line, verbatim log snippet>"
       }
     ]
   }
   ```

Hard rules:

- **Autonomous** — emit the categorization without a human in the
  loop. Downstream consumers (the build pipeline, oncall dashboard)
  decide what to do with it. This skill never reruns, retries,
  reverts, or modifies CI config.
- **Never categorize a security/auth/payment-path failure as
  `flaky`** even if its history shows historical intermittency.
  Escalate to operator with `flags: ["security_path_failure"]` and
  emit `unclassified` until the operator confirms.
- If `prior_flake_history` shows this test has flaked ≥ 5× in 30
  days with no fix landed, emit `flaky` AND flag `repeat_offender`
  for routing to the test-owner.
- If `recent_commit_log` is empty or `< 24h` of history, refuse to
  guess `most_likely_commit` for `regression` failures — emit them
  as `unclassified` with `flags: ["insufficient_commit_history"]`.
- Quote evidence verbatim where possible (one log line per
  failure). If the log contains PII, redact before quoting and flag
  `pii_in_build_log`.
