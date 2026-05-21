# codex-handler-error-types-thiserror-uniformity

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-handler-error-types-thiserror-uniformity`.

**Fast-mode eligible**: yes — mechanical audit + additive sweep.

## Summary

Project preferences (per `CLAUDE.md`): no `unwrap`/`expect` outside `#[cfg(test)]`. Implicit corollary: error types should be uniform `thiserror`-based enums across handlers, not ad-hoc `anyhow` or `Box<dyn Error>` (where they leak into public surfaces). Codex audits handler crates for error-type uniformity and reports gaps.

## Acceptance

- [ ] Enumerate every handler crate (`argo-cli/src/handlers/`, `argo-gateway/`, `argo-runner/`, `argo-genesis/`, etc.).
- [ ] For each handler file, classify the dominant error-handling pattern:
  - **THISERROR-CLEAN** — uses a per-handler or shared `#[derive(thiserror::Error)]` enum with `?` propagation.
  - **ANYHOW-FUNCTION-INTERNAL** — uses `anyhow::Result` internally but converts at the public boundary. Acceptable if documented.
  - **ANYHOW-LEAKED** — `anyhow::Result` is the public return type — bad, leaks implementation details.
  - **BOX-DYN-LEAKED** — `Box<dyn Error>` in public surface — bad, even less type info.
  - **MIXED** — file mixes patterns inconsistently.
- [ ] PR body includes a markdown table: crate | file | pattern | recommendation.
- [ ] For ≤3 ANYHOW-LEAKED or BOX-DYN-LEAKED public surfaces: refactor in this brief (introduce a `thiserror` enum, wire `?` propagation). Larger refactors spawn follow-up briefs.
- [ ] No semantic change — just typing discipline. All existing tests pass.

## May touch

- Handler files where a ≤30-LOC error-type refactor is genuinely additive.
- Module-level `error.rs` files where introduced.
- `Cargo.toml` files only to add `thiserror` if not already a dep.

## Must NOT touch

- The runtime / store / audit crates (those have settled error types).
- Any test code.
- Internal helper functions (only the public-handler boundary).
- Async streams or `tokio` task spawning patterns.

## Notes

- Pre-flight: `rg 'pub fn.*-> Result<' crates/argo-cli/src/handlers crates/argo-gateway` to enumerate handler returns.
- Pre-flight: `rg 'thiserror' crates/Cargo.toml` to confirm the dep is already in the workspace.
- Sizing: 200–500 LOC. Most files will get a 5–20-LOC error-type addition.
- Procurement-grade leverage — error-type discipline is what audit-time reviewers see.
