---
name: analyze_campaign
version: "0.1.0"
description: "Analyzes GTM campaign performance and extracts winning + failing patterns. Does not overfit small samples and proposes the next experiment, not just a postmortem."
provenance:
  source: vertical/gtm-engineer
  hash: sha256-pending
---

# When to use

Call when the operator supplies campaign outcomes (response data,
objections, conversion stats) for a previously-drafted campaign.
Output drives `detect_gtm_pattern` and the next-experiment
hypothesis the operator will test.

# Inputs you expect

- campaign metadata (name, segment, send window)
- messages used (or refs to `memory.winning_messages` /
  `memory.failed_messages` rows)
- response data (replies, positives, no-replies,
  bounces / opt-outs)
- explicit objections (verbatim quotes when available)
- conversion / pipeline data when the operator can supply it

# Output shape

```json
{
  "campaign": "<name>",
  "headline_metrics": {
    "messages_sent": 0,
    "replies": 0,
    "positive_replies": 0,
    "negative_replies": 0,
    "reply_rate": 0.00,
    "positive_rate": 0.00
  },
  "what_worked": [
    {"observation": "<what>", "evidence": "<which messages / replies>", "confidence": "low|medium|high"}
  ],
  "what_failed": [
    {"observation": "<what>", "evidence": "<which messages / replies>", "confidence": "low|medium|high"}
  ],
  "likely_reasons": ["<reason 1>", "<reason 2>"],
  "objection_themes": [
    {"theme": "<what>", "frequency": 0, "example_quote": "<verbatim if available>"}
  ],
  "next_experiments": [
    {"hypothesis": "<what>", "test": "<how>", "expected_signal": "<what we'd expect to see>"}
  ],
  "learned_skill_candidates": [
    {"name": "<proposed skill name>", "evidence": "<what >=3 data points support it>", "should_propose": true}
  ],
  "sample_size_warning": "<set when N is too small for confident inference>"
}
```

# Rules

- **Don't overfit small samples.** N<10 supports a hypothesis,
  not a conclusion — set `sample_size_warning` and mark every
  `confidence` as `low` regardless of effect size.
- **Separate signal from noise.** Two positives out of 100 is
  noise unless they share a clear pattern. Surface the pattern
  if there is one; say "noise" if there isn't.
- **Identify confidence per observation.** Each row in
  `what_worked` / `what_failed` carries its own `confidence`.
- **Propose the next experiment.** The whole point of the
  analysis is "what do we test next". Each `next_experiments`
  entry includes the hypothesis, the test, and the expected
  signal so the operator can decide whether the test is cheap
  enough to run.
- **Propose learned skills only when ≥3 data points.** A
  candidate with `should_propose: true` requires three+
  reinforcing observations. One-shot wins are written to
  `winning_messages[]` for context but do NOT propose a skill.

# Memory

Append a row to `memory.campaign_results[]` with the headline
metrics and a pointer to the supplied response data. Each
`learned_skill_candidates` entry is queued (in
`memory.learned_skill_candidates[]`) for `propose_gtm_skill`
to formalise — never auto-promoted.
