# codex-no-unwrap-enforce-doc-and-hook

**Authored against**: `origin/main` `b566866d4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-no-unwrap-enforce-doc-and-hook`.

**Fast-mode eligible**: yes — pure tooling + docs.

## Summary

#1990 landed `scripts/check-no-unwrap-outside-test.{sh,py}` + fixed 1 PRODUCTION violation. The script exists but isn't wired anywhere — it has to be run manually. Two cheap leverage moves:

1. **Pre-commit hook** that runs the script on staged Rust files. Mirrors the existing brief-lint hook pattern (`scripts/install-brief-hooks.sh`).
2. **Operator-runnable doc** so the operator can spot-check before launch even though GHA is disabled per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`.

Pre-condition: **#1990 MERGED ✅**.

## Acceptance

- [ ] `scripts/install-no-unwrap-hook.sh` — installs a pre-commit hook (mirroring `install-brief-hooks.sh` pattern) that runs `check-no-unwrap-outside-test.sh` on staged Rust files. WARN mode by default; flip to ENFORCE via `git config skycot.no-unwrap-enforce enforce`.
- [ ] `docs/skycot/operator/no-unwrap-discipline.md` — 1-page operator doc explaining: what the rule is (CLAUDE.md invariant), how to run the check manually, how to install the hook, what counts as a legit `unwrap` (test-context, doc examples, marked `// SAFETY: invariant` lines).
- [ ] No production code change.
- [ ] No new clippy lints (those are a separate brief if anyone wants them).
- [ ] Hook installation is opt-in — doesn't auto-install at clone time.

## May touch

- `scripts/install-no-unwrap-hook.sh` (new)
- `docs/skycot/operator/no-unwrap-discipline.md` (new)
- Documentation links in `CLAUDE.md` (only if adding a "see also" line — additive)

## Must NOT touch

- `scripts/check-no-unwrap-outside-test.{sh,py}` from #1990 (settled).
- Production Rust code.
- The `DISABLED_WORKFLOWS_POLICY.md` (operator-locked).

## Notes

- Pattern reference: `scripts/install-brief-hooks.sh` for the install-hook idiom.
- Sizing: 60-120 LOC (small script + 1-page doc).
- v1.0 launch hygiene — operator can demonstrate "we enforce no-unwrap" without GHA being enabled.
