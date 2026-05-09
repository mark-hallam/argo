# mvp-ci-reenable — restore GitHub Actions CI gates for v1.0

**Owner**: claude. **Depends on**: all 8 split-prep briefs landed (so workspace clippy + tests are clean on origin/main). **Tier 1 MVP gate — blocks v1.0 release.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

CI was disabled (`.yml.disabled`) when GitHub Actions billing flipped earlier. Operator has since re-enabled GitHub-side billing. v1.0 cannot ship without enforced CI gates. This brief flips the workflows back live and verifies the matrix.

## Files in scope

```
.github/workflows/ci.yml.disabled          → ci.yml
.github/workflows/docs.yml.disabled        → docs.yml
.github/workflows/oci-smoke.yml.disabled   → oci-smoke.yml
.github/workflows/release.yml.disabled     → release.yml
.github/workflows/schema-doc-check.yml.disabled → schema-doc-check.yml
```

## Approach

1. **Audit** each `.disabled` file BEFORE renaming. Confirm the trigger (`on:`) is sane (e.g., `pull_request` + `push: branches: [main]`), the cargo cache step is current, and there are no cargo flags that no longer exist.
2. **Rename** by `git mv .github/workflows/<name>.yml.disabled .github/workflows/<name>.yml` for all five.
3. **Verify the gates** in `ci.yml`:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `cargo deny check`
   - `cargo audit`
   Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`. If matrix is missing entries, ADD them — Linux+macOS+Windows is required per spec.
4. Push the branch. The PR itself will trigger the new CI on itself; if any gate fails, fix narrowly. Use `--admin` merge ONLY if the failures are unrelated to this brief's scope (boundary note).

## Files you may touch

- All `.github/workflows/*.yml.disabled` (rename to `.yml`)
- `.github/workflows/ci.yml` content — narrowly: matrix entries, missing `cargo-deny` or `cargo-audit` steps if they're absent. Do NOT rewrite the workflow.
- `Cargo.toml` workspace — narrowly: pin `cargo-deny` / `cargo-audit` versions if currently unpinned

## Files you must NOT touch

- Any source code (`crates/*/src/`)
- Any tests
- `docs/build-plan/BUILD_STATUS.md` — only the IN_REVIEW flip at the end

## Acceptance criteria

- [ ] All 5 workflows renamed `.yml.disabled` → `.yml`
- [ ] `ci.yml` has all 3 matrix entries (`ubuntu-latest`, `macos-latest`, `windows-latest`)
- [ ] `ci.yml` enforces: fmt, clippy `-D warnings`, test, cargo-deny, cargo-audit
- [ ] PR checks turn green on all 3 OS matrix
- [ ] Crate-scoped clippy + tests still clean locally pre-push

## Boundary

- Pre-existing flakes documented in `docs/build-plan/KNOWN_FLAKES.md` count as Boundary notes per HARD RULE 2 IF they reproduce on clean origin/main. If a flake fails CI the PR may still merge with a Boundary section in PR body, BUT only if the flake is documented and outside the brief's may-touch list.

## Branch + PR

- Branch: `mvp-ci-reenable`
- PR title: `ci: re-enable workflow gates (Linux+macOS+Windows matrix) (mvp-ci-reenable)`
