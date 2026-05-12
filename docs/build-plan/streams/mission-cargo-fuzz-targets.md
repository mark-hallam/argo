# Mission load-bearing wire — cargo-fuzz targets for parser/format boundaries

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> Genesis classification, audit-chain canonicalization, redaction-rule scanning, and JSON-RPC plugin codec are all parse-once boundaries where malformed input could panic or behave unexpectedly. cargo-fuzz finds the panics + edge cases before production does.

## Scope

NEW `fuzz/` directory under select crates with cargo-fuzz harnesses:

1. **`crates/argo-audit/fuzz/`** — fuzz `compute_chain_hash` input + `verify_session` against random byte sequences
2. **`crates/argo-redact/fuzz/`** — fuzz the redaction-rule scanner against random text (catches infinite-loop regex patterns)
3. **`crates/argo-genesis/fuzz/`** — fuzz the keyword-classifier input (random Unicode strings; expect classification ∈ known set OR `unknown`)
4. **`crates/argo-mcp/fuzz/`** — fuzz the JSON-RPC frame parser (the wire-format boundary for every connector)

Each fuzz target:
- Has a `cargo +nightly fuzz run <target> -- -max_total_time=60` 60-second smoke that runs in CI (nightly cron)
- Corpus seeded from real-world inputs (capture from test fixtures)
- Crash artifacts saved under `artifacts/` (gitignored)

This is NOT in scope: full coverage-guided fuzzing as part of regular CI (cargo-fuzz needs nightly); seedless fuzzing (always seed from corpus); fuzz of binary-only inputs.

## Files you may touch

- `crates/argo-audit/fuzz/` (NEW dir, 3 files, ≤ 250 LOC total)
- `crates/argo-redact/fuzz/` (NEW dir, 3 files, ≤ 200 LOC total)
- `crates/argo-genesis/fuzz/` (NEW dir, 3 files, ≤ 200 LOC total)
- `crates/argo-mcp/fuzz/` (NEW dir, 3 files, ≤ 250 LOC total)
- `.github/workflows/fuzz-nightly.yml` (NEW, ≤ 80 LOC) — nightly cron, gated by CI-billing flag
- `docs/skycot/RUNBOOK.md` — "Fuzz testing" subsection documenting `cargo +nightly fuzz` invocation
- `.gitignore` — `**/fuzz/artifacts/`, `**/fuzz/corpus/`

## Files you must NOT touch

- The actual parser/redact/audit/mcp code (this brief is testing infrastructure only)

## Acceptance criteria

- [ ] 4 fuzz target sets compile under `cargo +nightly fuzz build` per crate
- [ ] Each target's 60-second smoke runs without immediate crash
- [ ] Seed corpus checked in from test fixtures (small, < 100kb)
- [ ] Nightly workflow defined but optionally gated (default off until CI-billing OK'd)
- [ ] RUNBOOK documents reproducing a crash from an artifact
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (fuzz targets in a separate cargo workspace, don't affect main build)
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-cargo-fuzz-targets`
- PR title: `fuzz: 4 cargo-fuzz target sets for parse-once boundaries (mission-cargo-fuzz-targets)`

## Notes

- cargo-fuzz uses LLVM libFuzzer under the hood — nightly-only (stable Rust doesn't have the runtime). The CI workflow uses `dtolnay/rust-toolchain@nightly`.
- The 4 chosen boundaries are the highest-risk: audit-chain bytes (signed, can't be replayed); redact rules (regex DOS); genesis (operator-typed); mcp (untrusted plugin output).
- Crashes found by fuzz become regular regression tests — the crash artifact is the failing input.
