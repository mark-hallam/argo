---
name: post_review_summary
version: "0.1.0"
description: "Emits the closing JSON summary block the eval harness scores against — intent + blockers + risks + nits."
provenance:
  source: vertical/pr-checker
  hash: sha256-pending
---

# When to use

Call this skill **last** in every PR review turn. It produces
the closing JSON block the eval suite scores against.

# Output shape (exact)

A single fenced JSON block, no prose around it:

```json
{
  "intent": "feature|fix|refactor|docs|chore|mixed",
  "blockers": ["…"],
  "risks":    ["…"],
  "nits":     ["…"]
}
```

Field semantics:

- **`intent`** — exactly one of the five strings, or `mixed` if
  the diff genuinely spans more than one category. The eval
  suite scores this against an expected label per fixture PR.
- **`blockers`** — every invariant violation from
  `identify_invariant_violations`. Plus any correctness bug
  obvious enough that "this would crash in production." One
  short sentence per item, no quoting.
- **`risks`** — non-blocking but plausibly load-bearing concerns:
  race conditions, missing error paths, lossy type conversions,
  silent data loss, perf regressions on the happy path.
- **`nits`** — readability, naming, comment drift. Only surface
  these if `blockers` AND `risks` are both empty (signal-to-noise
  matters).

# Empty-array convention

`[]` is the right answer when there's nothing to flag. Do not
fill an empty array with filler ("looks good!", "no issues
spotted") — the eval harness ignores prose in the JSON block.

# Examples

**Clean PR:**

```json
{
  "intent": "fix",
  "blockers": [],
  "risks": [],
  "nits": []
}
```

**Blocking violation:**

```json
{
  "intent": "feature",
  "blockers": [
    "crates/foo/src/bar.rs:512 exceeds the 500-LOC budget — extract a sibling module"
  ],
  "risks": [
    "new tool dispatch arm at routes_post.rs:88 skips capability check"
  ],
  "nits": []
}
```
