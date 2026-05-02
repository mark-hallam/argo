# PR Checker

The first reference vertical for the Argo platform (#652). An
**advisory, read-only** code-review agent that walks a PR diff,
checks it against the project's invariants, and posts a JSON
summary the eval suite scores against.

## What it does

For each PR you point it at:

1. Reads the diff (via `web_extract` from a URL, or `file_read`
   from a local diff file).
2. Identifies the change category (feature / fix / refactor /
   docs / chore / mixed).
3. Walks the project's hard invariants. Examples:
   - No file > 500 LOC
   - No `unwrap()` outside `#[cfg(test)]`
   - Migrations are additive only
   - Capability checks on every tool dispatch
4. Surfaces likely problems in priority order:
   - **Blockers** — invariant violations that would fail CI
   - **Risks** — non-blocking but plausibly load-bearing
   - **Nits** — readability / naming, only if blockers + risks
     are empty
5. Closes with a fenced JSON block the eval harness reads.

## What it does NOT do

By manifest declaration, this vertical is **read-only**:

- ❌ Never writes files (`file_write`, `file_patch` denied)
- ❌ Never runs commands (`terminal` denied)
- ❌ Never delegates to other agents (`delegate_task` denied)
- ❌ Never posts comments (no GitHub-API tool wired today)

The deny list is the security boundary; the allow list is the
discoverability hint to the agent.

## How it's wired

| Concern | File |
|---|---|
| Manifest | `vertical.toml` |
| System prompt | `prompts/system.md` |
| Persona | `prompts/persona.md` |
| Skills | `skills/identify_invariant_violations.skill.md`, `skills/post_review_summary.skill.md` |
| Eval suite | `eval/invariant_detection.jsonl` (10 fixtures) |
| Memory schema | `memory.schema.toml` |
| Memory seed | `memory/seed/known_invariants.md` |

## How to use it (today, post-#652)

This PR ships **assets only**. The discovery surface from #649
+ #650 sees it; the session-binding surface from #651 lets a
session declare it. But the runtime activations (system prompt,
skill auto-attach, capability enforcement, eval automation) are
follow-up PRs (#653–#656). Until those land, a session bound to
`pr-checker` is metadata-only.

**What works today:**

```bash
# Discovery
argo vertical list                 # pr-checker shows up
argo vertical show pr-checker      # full manifest summary
argo vertical validate pr-checker  # cross-refs all resolve

# Session binding
argo new --provider anthropic --model claude-sonnet-4-5 \
  --vertical pr-checker --title "review PR #N"

# Bound session is filterable
argo session list --vertical pr-checker --json
```

**What needs follow-up PRs:**

- `argo vertical eval pr-checker` (#656) — runs the eval suite
- System prompt actually loaded at run-time (#654)
- Skills auto-attached at session-create (#653)
- Tool deny-list actually enforced (#655)

## Adapting this vertical to your own project

The invariants list is project-specific. To customize:

1. Edit `skills/identify_invariant_violations.skill.md` — the
   table near the top is the source of truth for what counts as
   a "blocker."
2. Edit `memory/seed/known_invariants.md` — the seed-loaded
   memory list the agent reads on first run.
3. Edit `eval/invariant_detection.jsonl` — add fixtures for
   your project's PRs (or your project's *bad* PRs that should
   have been caught).

The manifest's `[provider]` and `[tools]` rarely need changes.
