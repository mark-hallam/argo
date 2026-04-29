# `routing-correctness` eval suite

Tasks that the cost router (§24) is expected to classify into the right tier
(`simple` / `medium` / `complex`).

The full suite of 20 tasks lands with **Agent V** (§28.V); this directory
ships 5 *starter* tasks alongside the Phase 2 / §8 PR so the harness has
something to point at while Agent V is in flight.

## Schema

Each `.toml` file in this directory is one task:

```toml
id = "routing-001"             # stable id (matches filename stem)
description = "..."             # what we're testing
prompt = "..."                  # input for the classifier
expected_tier = "simple"        # one of: simple | medium | complex
rationale = "..."               # why this tier (for human review)
```

The harness loads each file, asks the configured `TieredRouter` to classify
`prompt`, and asserts the chosen tier matches `expected_tier`. The §24.4
gate is **≥ 80 % pass** over the full 20-task suite.
