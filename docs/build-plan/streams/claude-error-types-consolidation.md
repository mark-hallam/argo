# claude-error-types-consolidation — audit + standardize ArgoError variant usage

**Owner**: claude. **Depends on**: nothing. **Phase B consistency narrow.**

## Why

`ArgoError` variants (`Provider(_)`, `Internal(_)`, `InvalidInput(_)`, etc.) are used **inconsistently across the workspace**. Some crates wrap an HTTP 5xx as `Provider(_)`, others as `Internal(_)`. Some wrap an unparseable manifest as `InvalidInput(_)`, others as `Provider(_)`. Gateway clients can't reliably map `ArgoError` to HTTP status codes or to user-facing diagnostics because the variant ↔ semantics contract isn't documented or enforced.

This bites: error-mapping code in `crates/argo-cli/src/handlers/gateway/routes_*.rs` has multiple ad-hoc `match err { Provider(_) => 502, Internal(_) => 500, ... }` arms with subtly different mappings. Telemetry buckets misclassify failures.

## Approach (concrete)

**Two-phase: audit + rewrite.**

### Phase 1 — audit (read-only, write findings to a single doc)

1. Locate `ArgoError` definition. Likely `crates/argo-error/src/lib.rs`; verify with `rg 'pub enum ArgoError' --type rust`.
2. Enumerate every variant with its current intended semantics (read the doc-comments + every callsite).
3. `rg 'ArgoError::(Provider|Internal|InvalidInput|...)\(' --type rust > /tmp/argo-error-callsites.txt`. Categorise each callsite by what kind of failure it is (input validation, network, manifest schema, downstream service, etc.).
4. Identify mismatches — e.g. a `serde_json::from_str` parse failure wrapped as `Internal(_)` when the input came from the user (should be `InvalidInput(_)`), or a `reqwest` error wrapped as `Internal(_)` when it's a remote service failure (should be `Provider(_)`).

### Phase 2 — rewrite (narrow edits)

1. **Document the canonical contract** in `crates/argo-error/src/lib.rs` doc-comment for `ArgoError`:
   - `Provider(_)` — failure originating from a downstream service (HTTP, MCP, model API, action endpoint). Maps to HTTP 502 / 504.
   - `InvalidInput(_)` — caller-supplied data was malformed (bad JSON, missing required field, invalid manifest). Maps to HTTP 400.
   - `Internal(_)` — invariant violated, bug. Maps to HTTP 500. Should be rare.
   - `NotFound(_)` — addressed entity does not exist. Maps to HTTP 404.
   - `Conflict(_)` — concurrent modification, lock contention. Maps to HTTP 409.
   - (Add others verbatim from the actual enum.)
2. **Rewrite mismatched callsites** identified in Phase 1. Narrow surgical edits — change only the variant, keep the message string identical (so log-grep doesn't break).
3. **Add a doctest** demonstrating each variant's canonical use.
4. **Add a workspace-wide CI check** (script under `scripts/check-error-variant-usage.sh`) that flags suspicious patterns (e.g. `ArgoError::Internal` wrapping a `reqwest::Error`).

## Files you may touch

- `crates/argo-error/src/lib.rs` (or wherever ArgoError lives) — narrowly: add canonical contract doc-comment + ≤ 1 doctest per variant
- `crates/argo-cli/src/handlers/**/*.rs` — narrowly: rewrite mismatched callsites
- `crates/argo-channels/src/**/*.rs` — narrowly: rewrite mismatched callsites
- `crates/argo-marketplace/src/**/*.rs` — narrowly: same
- `crates/argo-vertical/src/**/*.rs` — narrowly: same
- `crates/argo-store/src/**/*.rs` — narrowly: same
- `scripts/check-error-variant-usage.sh` (NEW, ≤ 80 LOC)
- `.github/workflows/*.yml` (existing CI workflow) — narrowly: add 1 step invoking the new script

## Files you must NOT touch

- Any test file other than to add doctests on `ArgoError` itself
- Tracing span names (separate brief)
- Any error type *other than* `ArgoError`

## Acceptance criteria

- [ ] `ArgoError` carries a doc-block with the canonical contract (one bullet per variant + HTTP-status mapping)
- [ ] Every `ArgoError::*` callsite in the workspace matches the contract (verified by re-running the audit script)
- [ ] `scripts/check-error-variant-usage.sh` runs in CI and is green
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No public API change (variant names + signatures unchanged; only callsite variants flipped)
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-error-types-consolidation`
- PR title: `error: standardize ArgoError variant usage across workspace (claude-error-types-consolidation)`

## Notes for the implementer

- The audit step is the long pole. Budget ~60% of effort to it. The rewrite is mechanical once the mismatches are identified.
- If the audit surfaces a missing variant (e.g. there's no `Conflict(_)` and several callsites would benefit), file a **separate follow-up brief** — DO NOT widen scope here.
- Keep error message strings byte-identical — log-grep regressions are silent and painful.
- The CI check should be a `grep` / `rg` based heuristic, not a full AST analysis. False-positive tolerance: < 5%.
