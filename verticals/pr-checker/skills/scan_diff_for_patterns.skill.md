---
name: scan_diff_for_patterns
version: "0.1.0"
description: "Scans a PR diff for known anti-patterns mapped to CLAUDE.md invariants; emits a 3-severity JSON list (info / advisory / blocker)."
provenance:
  source: vertical/pr-checker
  hash: sha256-pending
---

# When to use

Call this skill **after** `identify_invariant_violations` and
**before** `post_review_summary`. It produces a structured
machine-readable scan of every changed file against the project's
anti-pattern catalogue. Read-only; emits findings, never edits.

# Inputs

- `diff`: unified diff for the PR (e.g. `gh pr diff <num>`).
- `repo_root` (optional): for resolving cited paths when a finding
  needs the post-patch file length (LOC ceiling check).

# The patterns

Each row maps 1:1 to a CLAUDE.md / SWARM_PROTOCOL.md invariant.

| Pattern | Severity | Cites |
|---|---|---|
| File grows past 500 LOC after the patch | blocker | CLAUDE.md "No file > 500 LOC" |
| Function exceeds 80 LOC | blocker | CLAUDE.md "No fn > 80 LOC" |
| `unwrap()` or `expect()` added outside `#[cfg(test)]` | blocker | CLAUDE.md "No unwrap/expect outside #[cfg(test)]" |
| New `unsafe` block outside marked FFI shim | blocker | CLAUDE.md "No unsafe outside marked FFI shims" |
| New `static mut` / `lazy_static! { static MUT … }` / Mutex singleton | blocker | CLAUDE.md "No global mutable state" |
| `>200` LOC added to one file in a single PR | advisory | review-load signal; large single-file diffs need split |
| New `pub` item without `///` rustdoc | advisory | CLAUDE.md "Every public Rust API has rustdoc with at least one example" |
| `>5` commits with no squash-prep on the branch | info | reviewer load; merge will squash anyway |
| New TODO/FIXME comment without an issue/brief link | info | drift risk; orphan TODOs accumulate |

# Output shape (exact)

A single fenced JSON block, no prose:

```json
[
  {
    "pattern": "file_loc_ceiling",
    "severity": "blocker",
    "location": "crates/argo-cli/src/handlers/foo.rs:512",
    "evidence": "file ends at line 512 after patch",
    "cites": "CLAUDE.md: No file > 500 LOC",
    "suggestion": "Split sibling module before adding to this file."
  }
]
```

Severity is exactly three values: `blocker`, `advisory`, `info`.
`blocker` items feed `post_review_summary.blockers`; `advisory`
feeds `risks`; `info` feeds `nits` (only when blockers + risks
are empty). `[]` is the right answer when nothing matches.

# Examples

**Clean diff:** `[]`

**Findings:**

```json
[
  {
    "pattern": "unwrap_outside_test",
    "severity": "blocker",
    "location": "crates/argo-store/src/migrate.rs:88",
    "evidence": ".unwrap() in non-test fn",
    "cites": "CLAUDE.md: No unwrap/expect outside #[cfg(test)]",
    "suggestion": "Return ArgoError or use the ? operator."
  }
]
```

# Notes

- Patterns are project-specific (Argo CLAUDE.md); adapt the table
  for other repos.
- Read-only — recommend, never edit.
- HARD RULE 5 (may-touch violations) is **not** here; that is
  `compare_against_brief`'s job.
