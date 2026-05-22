# No-unwrap discipline

**Audience:** operators and release captains checking v1 launch
hygiene while GitHub Actions are disabled.

`CLAUDE.md` makes this a repository invariant: production Rust must
not call `.unwrap()` or `.expect(` outside test-only contexts. A panic
from production code turns an ordinary bad input, missing config, or
race into a process crash. SKYCOT handles those cases by returning an
error with context, falling back deliberately, or documenting a narrow
invariant.

## Manual check

Run the scanner from the repo root:

```bash
bash scripts/check-no-unwrap-outside-test.sh
```

For a release spot-check with counts:

```bash
bash scripts/check-no-unwrap-outside-test.sh --verbose
```

The command exits `0` when there are no production violations. It
exits `1` and prints `file:line:column` rows for any production-path
callsite that needs review.

## Optional pre-commit hook

Install the local hook:

```bash
bash scripts/install-no-unwrap-hook.sh
```

The hook runs the same scanner when staged Rust files under `crates/`
are present. It is WARN mode by default: findings print, but the
commit is allowed. Flip it to blocking mode with:

```bash
git config skycot.no-unwrap-enforce enforce
```

Return to warning mode with:

```bash
git config skycot.no-unwrap-enforce warn
```

The hook is opt-in and local to the checkout. It is not installed by
clone, build, test, or bootstrap scripts.

## Legitimate cases

These are expected and should not block a release:

- Test-only code under `#[cfg(test)]`, `tests/`, `benches/`, or
  `examples/`.
- Rustdoc examples where the surrounding example is explanatory, not
  production-path code.
- Line comments that mention `.unwrap()` or `.expect(` while
  explaining the rule.
- A production invariant that has explicit code-owner approval and a
  nearby `// SAFETY: invariant ...` explanation. Today the scanner is
  intentionally strict and still reports those callsites, so keep any
  exception rare and document the waiver in the PR.

Preferred fixes are `?` with a typed error, `ok_or_else`, `map_err`
with context, or an explicit fallback such as `unwrap_or_default` only
when the fallback is semantically correct.
