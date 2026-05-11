---
name: author_bug_report
description: Author a full bug report carrying steps + environment + expected-vs-actual + frequency + severity-suggestion. Refuses to finalize until all four reproducibility elements are present.
inputs:
  - observed_behavior
  - expected_behavior
  - environment_hints
outputs:
  - bug_report
  - severity_suggestion
  - escalation_required
  - flags
  - confidence
---

# Skill: Author Bug Report

Produce a bug report a triage owner can act on without coming back
for clarifications. The reproducibility constraint is load-bearing:
all four elements MUST be present, or the report is not finished.

1. **Title** — single line, names the surface and the failure mode.
   "Login button returns 500 when email contains `+` alias" is good;
   "login broken" is not.
2. **Steps to reproduce** — numbered, each step a single user
   action. End with the step on which the failure surfaces. If a
   step cannot be reproduced (timing, ephemeral data), say so and
   flag the report as `repro_partial`.
3. **Environment fingerprint** — OS + browser/runtime + version + app
   version + commit hash. If the reporter cannot supply a commit
   hash, ask for it before drafting — do not guess.
4. **Expected vs. actual** — two sentences, quoting verbatim the
   wrong rendered value / error message / missing artifact. "It
   broke" is not an actual.
5. **Frequency** — `always` / `N-of-M` / `once-only`. If the
   reporter has not measured frequency, ask them to run the repro N
   times and return with the count.
6. **Severity suggestion** — Cam suggests, the triage owner picks.
   Rank by impact dimension: user-visible / data-integrity /
   revenue / security. Note which dimensions apply; do not collapse
   to a single number autonomously.
7. **Escalation required Y/N** — fires Y if PII appears in the
   repro, if pricing/billing/revenue logic is touched, or if a fix
   would require production data modification or test-disable-in-main.

Hard rules:

- A bug report missing any of steps / environment / expected-vs-actual
  / frequency is `incomplete` — return to the reporter for the
  missing element rather than file.
- If the observed behavior includes real names, emails, SSNs,
  payment data, or health data — STOP. Do not include the data
  verbatim. Escalate with `flags: ["pii_in_repro"]`.
- Never assume "user error" unless the steps + environment +
  frequency rule out a code-side cause. Code-first, test-second,
  user-third is the default ordering.
