# codex-audit-bench-manual-is-multiple-of-fix — 1-line workspace-clippy unblock

**Owner**: codex. **Sizing hint**: fastest fast-mode — single-line edit, no test changes.
**Depends on**: none.
**Authored against**: origin/main `96827b4b` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-bench-manual-is-multiple-of-fix`.
**Source**: BOTH #1823 + #1824 boundary notes flagged `manual_is_multiple_of` clippy
in `crates/argo-audit/benches/fixtures/seed_chain.rs:84` (introduced by #1812
chain-perf bench). `scripts/codex-pre-push.sh` and
`scripts/workspace-clippy-fast-fail.sh` both fail on clean main on this
single lint — actively blocking the swarm.

`codex-cli-reference-completeness-gate` is currently BLOCKED solely on this
lint; flipping the lint flips that brief back to claimable.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — `workspace-clippy-fast-fail.sh`
  is the swarm's pre-push gate; a lint-broken main blocks every subsequent
  codex push.
- **Repeatability** (#3) — recently merged PR introduced lint drift that the
  CI gate (had it been running) would have caught. Until GHA billing is
  re-enabled, the operator-side guard relies on the pre-push script being
  green on main.

## Scope

P0 mechanical. Replace one `%` modulo boolean pattern with the
`.is_multiple_of()` rustc 1.84+ stdlib API per the lint suggestion.

### The fix

`crates/argo-audit/benches/fixtures/seed_chain.rs:84` currently:

```rust
"outcome_status": if row_idx % 20 == 0 { "failed" } else { "succeeded" },
```

Should be:

```rust
"outcome_status": if row_idx.is_multiple_of(20) { "failed" } else { "succeeded" },
```

The other three modulo sites in the same file (lines 80, 81, 85 — all inside
`format!()` template arguments, not boolean patterns) are NOT lint-flagged
and stay as-is.

LOC: 1 line modified.

## Files you may touch

- `crates/argo-audit/benches/fixtures/seed_chain.rs` — line 84 ONLY. 1 modified LOC.

## Files you must NOT touch

- Other lint sites — the brief is intentionally narrow. Workspace clippy may
  surface other warnings on unrelated files; those are out of scope.
- `Cargo.toml` MSRV — `is_multiple_of` requires Rust 1.84+; argo workspace is
  already on rustc 1.75+ but workers should verify MSRV before claiming.
  If MSRV is <1.84, swap to `#[allow(clippy::manual_is_multiple_of)]` on the
  line instead — declare the choice in the PR body.

## Acceptance criteria

- [ ] `cargo clippy -p argo-audit --bench chain_perf -- -D warnings` clean
      (was 1 failure pre-fix).
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] `bash scripts/codex-pre-push.sh` clean.
- [ ] No other files modified; `git diff --stat` shows 1 file, 1+/1-.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-audit-bench-manual-is-multiple-of-fix`
- Title: `fix(audit): use .is_multiple_of() in chain-perf bench fixture (codex-audit-bench-manual-is-multiple-of-fix)`
- Worker verification: per SWARM_PROTOCOL.md — crate-scoped clippy + pre-push gate.

## Hidden constraints

- **Smallest possible change.** Resist the urge to clippy-clean the rest of
  the bench. Other lints (if any) get their own briefs.
- **MSRV check before claim.** Run `rustc --version` against `rust-toolchain.toml`
  if present. If `is_multiple_of` isn't available, fall back to the `#[allow]`
  attribute — both close the immediate block.
- **Unblocks `codex-cli-reference-completeness-gate`.** That brief flips
  BLOCKED → TODO once this PR merges (conductor will sweep next tick).
