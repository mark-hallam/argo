# Per-vertical regression

`argo-vertical-regression` is a static structural test crate. For
every role-bearing vertical under `verticals/<id>/`, it loads the
manifest + every `eval/seed.jsonl` fixture and asserts:

1. Each fixture's `expected.action_type` (or every entry in
   `expected.action_chain`) names an action declared in the
   manifest's `[actions]` vocabulary.
2. When a fixture sets `expected.requires_approval`, the
   manifest's `[escalation.human_approval_required]` agrees.

No LLM calls, no E2B — those live in `argo eval run`. Smoke-only
fixtures (`expected_executor = "human"` or `requires_e2b: true`)
and fixtures using only the `must_include`/`must_not_include`
schema are skipped.

Run it with:

```bash
cargo test -p argo-vertical-regression --test regression_all
```

Pre-existing fixture↔manifest drift is pinned via `KNOWN_DRIFT`
in `crates/argo-vertical-regression/src/lib.rs`. New drift fails
the test; if a fix lands for one of those rows, delete the
matching `KNOWN_DRIFT` entry in the same PR — the test fails if
the baseline stays stale.
