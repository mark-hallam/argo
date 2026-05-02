---
name: identify_invariant_violations
version: 0.1.0
purpose: |
  Walk a diff and flag violations of the project's hard
  invariants. Returns a list of {file, line, invariant, evidence}
  citations. Pairs with the PR-Checker system prompt's "blockers"
  category — every entry here becomes a blocker in the JSON
  summary.
---

# When to use

Call this skill **first** for any PR review. The output is the
ground truth for the `blockers` array.

# The invariants

A "hard invariant" is something that, if violated, breaks a
guarantee the codebase relies on. Examples (substitute the
project's own list):

| Invariant | What violates it |
|---|---|
| **No file > 500 LOC** | Any added/modified file whose new line count exceeds 500 |
| **No `unwrap()` / `expect()` outside `#[cfg(test)]`** | Production-path code calling `.unwrap()` or `.expect("…")` |
| **No `unsafe` outside marked FFI shims** | New `unsafe` blocks not in a documented FFI module |
| **Capability checks on every dispatch** | A new tool dispatch arm that doesn't go through the capability engine |
| **Migrations are additive only** | Any `DROP COLUMN` / `DROP TABLE` / `ALTER … RENAME` in an `up` block (down blocks are fine) |
| **No global mutable state** | New `static mut`, `lazy_static! { static MUT … }`, or Mutex-wrapped singleton |
| **Tenant id plumbed through every primitive** | A new public API that takes a session/workspace param without a tenant id |

# How to use

1. Read the diff.
2. For each changed file, check it against every invariant above.
3. Output one entry per violation, formatted as:

   ```
   - [INVARIANT] <invariant name>
     file: <path>:<line>
     evidence: <one-line quote of the offending text>
     why: <one short sentence on the consequence>
   ```

4. If zero violations: output `(no invariant violations)`.

# Examples

**Violation found:**

```
- [INVARIANT] No file > 500 LOC
  file: crates/argo-cli/src/handlers/foo.rs:512
  evidence: file ends at line 512
  why: CI rejects PRs that add files over the 500-LOC ceiling
```

**Clean:**

```
(no invariant violations)
```

# Notes

- Invariants are **per-project**. The list above mirrors Argo's
  `CLAUDE.md` — adapt this skill when the vertical is wired into
  a different repo by editing the table above.
- This skill is intentionally narrow: it reports violations, it
  does not prescribe fixes. The author decides what to do.
