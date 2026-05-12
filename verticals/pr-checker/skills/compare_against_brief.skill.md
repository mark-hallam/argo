---
name: compare_against_brief
version: "0.1.0"
description: "Compares a PR diff against the brief it cites in docs/build-plan/streams/<brief-id>.md and emits a Markdown alignment report."
provenance:
  source: vertical/pr-checker
  hash: sha256-pending
---

# When to use

Call this skill when the PR title or branch references a brief id
(e.g. `(mission-foo-bar)`, branch `claude-mission-foo-bar`). Brief
drift is the #1 cause of conductor rejections — highest-leverage
check. Read-only.

# Inputs

- `pr_title`, `pr_body`, `pr_diff`: from `gh pr view` + `gh pr diff`.
- `repo_root`: for reading `docs/build-plan/streams/<brief-id>.md`.

# Brief-id extraction

1. Trailing parenthesised id in the PR title.
2. Else: head branch with `claude-` / `codex-` prefix stripped.
3. Neither: emit `Verdict: NO_BRIEF`; do not invent one.

# What to check

For brief at `docs/build-plan/streams/<brief-id>.md`:

| Check | How |
|---|---|
| Files touched ⊆ may-touch list | `diff --name-only` ∩ may-touch; flag each outside path. |
| LOC caps respected | Each `≤ N LOC` may-touch entry: `wc -l` ≤ N. |
| Acceptance ticks | Every `- [ ]` in brief Acceptance maps to a `- [x]` in PR body. |
| Sensible-deviations declared | Any out-of-may-touch edit requires a PR-body section. |
| Branch + title | Match the brief's `## Branch + PR` section. |

# Output shape (exact)

Four fixed H2 sections — each lists `OK` or one bullet per issue —
then a one-line verdict.

```
## Brief
- id: <brief-id>
- file: docs/build-plan/streams/<brief-id>.md
- authored against: <sha> (N commits behind main)

## May-touch boundary
- OK | <path>: outside may-touch list (HARD RULE 5)
- OK | <path>:<lines>: exceeds <N> LOC cap

## Acceptance ticks
- OK | unticked: "<exact text of brief checkbox>"

## Branch + PR metadata
- OK | branch: expected `<X>`, got `<Y>`
- OK | sensible-deviations missing despite out-of-may-touch edits

Verdict: ALIGNED|DRIFT|NO_BRIEF — <one short clause>.
```

# Example (aligned)

```
## Brief
- id: mission-vertical-pr-checker-skills
- file: docs/build-plan/streams/mission-vertical-pr-checker-skills.md
- authored against: b825b1d5 (201 commits behind main)

## May-touch boundary
- OK

## Acceptance ticks
- OK

## Branch + PR metadata
- OK

Verdict: ALIGNED — no drift detected.
```

# Notes

- Read-only — flag drift, never edit the PR or brief.
- "Authored against" drift > 100 commits is `advisory`, not
  `blocker`; pair with `scan_diff_for_patterns`.
- Brief missing on `origin/main` → `Verdict: NO_BRIEF`.
