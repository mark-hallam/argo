---
name: suggest_review_priorities
version: "0.1.0"
description: "Triages a PR into 3 tiers (high / medium / low review priority) based on risk signals; emits a Markdown rationale block."
provenance:
  source: vertical/pr-checker
  hash: sha256-pending
---

# When to use

Call this skill after `scan_diff_for_patterns` and before
`post_review_summary` to give the reviewer a "where to spend
review time" steer. Read-only.

# Inputs

- `pr_diff`: unified diff (`gh pr diff <num>`).
- `pr_files`: list of changed paths (`gh pr diff <num> --name-only`).

# Tier vocabulary (3-tier, fixed)

Mirrors conductor tiers in `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`
(Tier 1 full local verify / Tier 2 targeted clippy / Tier 3
diff-shape trust). Highest-risk PR gets `Tier 1`.

| Tier | Risk | Trigger signals |
|---|---|---|
| **Tier 1** | high-stakes | schema migration under `argo-store/migrations/`, auth/RBAC/capability, gateway routes, new public trait on workspace-exported API, executor primitives, security/audit/secrets. |
| **Tier 2** | medium | multi-crate diff, new HTTP/MCP endpoint, new test infrastructure, new CLI handler module, provider manifest changes, memory-schema additions. |
| **Tier 3** | low | single-crate refactor, doc-only, comment/typo fix, dependency bump within locked range, isolated test additions. |

# Output shape (exact)

A Markdown block with three fixed sections, each naming the tier
assigned and the reasoning, then a verdict line.

```
## Assigned tier
- Tier <1|2|3>

## Triggering signals
- <signal>: <path or pattern> — <one-line why>

## Suggested reviewer focus
- <where to look first>
- <what to verify by hand>

Verdict: TIER<1|2|3> — <one short clause>.
```

If multiple tiers' signals fire, take the **highest** (most
risky) tier. Cite **every** signal, not just the deciding one,
so the reviewer can audit the call.

# Example (migration → Tier 1)

```
## Assigned tier
- Tier 1

## Triggering signals
- schema migration: crates/argo-store/migrations/0042_*.sql — additive-only invariant
- multi-crate: argo-store + argo-cli (Tier 2 also fires)

## Suggested reviewer focus
- Verify migration is additive (no DROP / RENAME in up-block)
- Confirm ensure_schema() wired; run `cargo test -p argo-store`

Verdict: TIER1 — schema migration requires full local verify.
```

# Notes

- Read-only — recommend, never edit.
- Tier is risk **of the change**, not diff size: a 3-line
  migration is Tier 1; a 400-line single-crate rename is Tier 3.
- Pair with `compare_against_brief`: Tier 3 verdict on a PR with
  may-touch drift still needs brief-alignment review.
