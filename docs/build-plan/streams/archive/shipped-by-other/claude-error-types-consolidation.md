# claude-error-types-consolidation — audit + doc-comment + CI script (no rewrites)

**Owner**: claude. **Depends on**: nothing. **Phase B consistency narrow.**

**Authored against**: origin/main b3949a73 on 2026-05-09 (rewrite v2 by conductor — original v1 bundled audit + workspace-wide rewrite into one brief; that's > 200 callsites and won't fit a single iteration. Splitting into audit-first, then a follow-up `claude-error-types-rewrite-callsites` for the mechanical phase).

## Why

`ArgoError` variants (`Provider(_)`, `Internal(_)`, `InvalidInput(_)`, etc.) are used **inconsistently across the workspace**. Some crates wrap an HTTP 5xx as `Provider(_)`, others as `Internal(_)`. Some wrap an unparseable manifest as `InvalidInput(_)`, others as `Provider(_)`. Gateway clients can't reliably map `ArgoError` to HTTP status codes because the variant ↔ semantics contract isn't documented or enforced.

This brief produces the contract + the audit list + the CI guard. **No callsite rewrites.** The mechanical rewrites land via per-crate follow-up briefs after this merges.

## Approach (concrete)

1. **Locate `ArgoError`** — likely `crates/argo-error/src/lib.rs`; verify with `rg 'pub enum ArgoError' --type rust`.
2. **Document the canonical contract** in the `ArgoError` doc-comment:
   - `Provider(_)` — failure originating from a downstream service (HTTP, MCP, model API, action endpoint). Maps to HTTP 502 / 504.
   - `InvalidInput(_)` — caller-supplied data was malformed (bad JSON, missing required field, invalid manifest). Maps to HTTP 400.
   - `Internal(_)` — invariant violated, bug. Maps to HTTP 500. Should be rare.
   - `NotFound(_)` — addressed entity does not exist. Maps to HTTP 404.
   - `Conflict(_)` — concurrent modification, lock contention. Maps to HTTP 409.
   - (Add others verbatim from the actual enum.)
3. **Add a doctest** demonstrating each variant's canonical use.
4. **NEW `scripts/check-error-variant-usage.sh`** (≤ 80 LOC, bash + rg) — heuristic flags:
   - `ArgoError::Internal\(.*reqwest`
   - `ArgoError::Internal\(.*serde_json`
   - `ArgoError::Provider\(.*missing field`
   - Other patterns the audit surfaces.
5. **NEW `docs/skycot/ERROR_VARIANT_AUDIT.md`** — checked-in audit report listing every mismatched callsite (path:line + current variant + recommended variant + 1-line rationale). Becomes the input for the follow-up rewrite brief.
6. **Wire CI**: add 1 step to existing CI workflow invoking `check-error-variant-usage.sh`. The CI step warns on mismatches but does NOT fail (until the rewrite brief lands and zeroes the count).

## Files you may touch

- `crates/argo-error/src/lib.rs` (or wherever `ArgoError` lives) — narrowly: add canonical contract doc-comment + 1 doctest per variant
- `scripts/check-error-variant-usage.sh` (NEW, ≤ 80 LOC)
- `docs/skycot/ERROR_VARIANT_AUDIT.md` (NEW, the audit report)
- `.github/workflows/ci.yml` — narrowly: 1 step (warning-only) invoking the new script

## Files you must NOT touch

- **Any callsite of `ArgoError::*` in any crate** — rewrites belong to the follow-up brief `claude-error-types-rewrite-callsites`. This brief is doc + audit + CI-warning only.
- Any test file other than the new doctests on `ArgoError`
- Tracing span names

## Acceptance criteria

- [ ] `ArgoError` carries a doc-block with the canonical contract (one bullet per variant + HTTP-status mapping)
- [ ] `docs/skycot/ERROR_VARIANT_AUDIT.md` exists with one row per mismatched callsite (or a "no mismatches found" note + the audit invocation that produced it)
- [ ] `scripts/check-error-variant-usage.sh` runs and reports a count
- [ ] CI workflow has a new warning-only step invoking the script
- [ ] `cargo build && cargo clippy -p argo-error --all-targets -- -D warnings && cargo test -p argo-error` clean (single-crate scope — no callsite changes elsewhere)
- [ ] No public API change
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-error-types-consolidation`
- PR title: `error: ArgoError canonical contract + variant-usage audit + CI guard (claude-error-types-consolidation)`

## Notes for the implementer

- The audit step is the deliverable. The output (`ERROR_VARIANT_AUDIT.md`) is what the follow-up rewrite brief consumes.
- Keep CI warning-only here so this brief lands without forcing the rewrite to land in the same iteration. The follow-up brief will flip CI to fail.
- The doc-comment + doctest go on the `ArgoError` enum itself. Single-crate change.
- If the audit surfaces a missing variant (e.g. no `Conflict(_)` and several callsites would benefit), note it in the audit report — DO NOT add the variant here. That's a separate brief.

## Follow-up

Once this lands, the conductor will queue `claude-error-types-rewrite-callsites` whose only job is to mechanically apply the audit report's recommendations + flip CI from warning to fail. That brief depends on this one.
