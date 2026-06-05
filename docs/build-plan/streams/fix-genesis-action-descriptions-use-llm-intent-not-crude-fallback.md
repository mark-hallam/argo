# fix-genesis-action-descriptions-use-llm-intent-not-crude-fallback

**Owner**: claude.
**Status**: TODO — enqueued 2026-06-06 (conductor) from live e2e findings.
**Authored against**: origin/main `2203fd6e2` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-genesis-action-descriptions-use-llm-intent-not-crude-fallback`.

## Why (live-reproduced this session)

Genesis renders each action with a **broken-grammar mechanical description** in the
"REVIEW BEFORE YOU HIRE" panel — confirmed live on `/genesis`:

```
archive_article   → "Archive article as the assistant named Remy reads"
extract_takeaways → "Extract takeaways as the assistant named Remy reads"
read_article      → "Read article as the assistant named Remy reads"
```

These come from the crude fallback at **`crates/argo-genesis/src/spec.rs:325`**:
```rust
format!("{} as the {role_descriptor}", capitalize_first(&phrase))
```
which concatenates `<action phrase> as the <persona descriptor>` — producing
ungrammatical, repetitive, zero-information text. It's **camera-visible in the
Genesis review panel**, so it reads as low-quality in a demo / first-hire flow
(a deploy-readiness polish issue).

The tension: **`crates/argo-genesis/src/author.rs:82`** ALREADY instructs the LLM
author to do better — *"action_intents: write a SPECIFIC one-line description for
EACH declared action, in THIS teammate's context. Never a generic restatement of
the verb (e.g. NOT 'draft email as the assistant')."* So the author is *meant* to
produce good descriptions, but the rendered action description falls back to the
`spec.rs:325` template — the author's `action_intents` aren't reaching the output
(or the fallback overrides them on the broad-role / non-classified path).

## What

1. Trace where an action's rendered description is set (author `action_intents`
   → the `CapabilitySpec` action → materialize → the SSE/review payload) and
   **plumb the LLM author's specific description through** so it's used when
   present.
2. Keep `spec.rs:325` as a **last-resort fallback only** (when the author gave no
   intent), and make the fallback **grammatical** — drop the "as the <persona>
   <verb>" tail; a plain "{capitalized action phrase}." reads far better than the
   current concatenation.
3. Do NOT change the action NAMES, risk levels, or approval flags — only the
   human-readable description string.

Grep-before-write: extends the EXISTING description plumbing on `CapabilitySpec` /
the author output — no new types. `rg 'action_intents|fn .*description' crates/argo-genesis`
before declaring anything.

## Verification
- Unit test in `argo-genesis`: an action WITH an author-provided intent renders
  that intent (not the fallback); an action with NO intent renders the
  grammatical fallback (no "as the …" tail).
- `cargo test -p argo-genesis` green + clippy clean.
- Manual: a `/genesis` run shows natural per-action descriptions in the review
  panel, not "<verb> as the assistant named <name> <verb>".

## Out of scope
- The risk-classifier noise (e.g. `extract_takeaways` mislabeled HIGH RISK) — a
  separate finding; this brief is descriptions only.
