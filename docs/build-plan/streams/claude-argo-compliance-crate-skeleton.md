# claude-argo-compliance-crate-skeleton — extract compliance handlers into a dedicated `argo-compliance` crate

**Owner**: claude. **Depends on**: nothing (precursor refactor).
**Authored against**: origin/main `b77b7474` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-argo-compliance-crate-skeleton`.

## Scope

Precursor brief for the 2 DEEP-DRIFT compliance briefs (`J17-compliance-hipaa-disclosure-log`, `J18-compliance-generic-export`) that both cite a non-existent `argo-compliance` crate. Today compliance lives in `crates/argo-cli/src/handlers/compliance_*.rs` (4 modules + 3 test modules, totaling ~1981 LOC):

- `compliance_delete.rs` (411 LOC) + tests (386 LOC) — tenant data deletion + grace window
- `compliance_export.rs` (343 LOC) + tests (129 LOC) — tenant data export
- `compliance_soc2.rs` (350 LOC) + tests (228 LOC) — SOC2 action classification
- `compliance_tenant_tables.rs` (134 LOC) — deletion order definition

Extract these into a new `crates/argo-compliance/` crate so:
1. The 2 DEEP-DRIFT compliance briefs (HIPAA + generic-export) have a real `argo-compliance` to extend.
2. Compliance logic is reusable beyond the CLI (gateway routes, future audit-export-CLI tooling can `use argo_compliance::*`).
3. The CLI handlers shrink to thin shims that call into `argo-compliance` (drops `argo-cli` LOC pressure).

**Not in scope**: changing compliance behavior, adding new compliance features (those are the follow-up briefs). Pure crate-extraction.

## Approach

1. Create `crates/argo-compliance/` with `Cargo.toml`, `src/lib.rs`, and the 4 modules moved verbatim:
   - `crates/argo-compliance/src/delete.rs` (411 LOC, was `compliance_delete.rs`)
   - `crates/argo-compliance/src/export.rs` (343 LOC, was `compliance_export.rs`)
   - `crates/argo-compliance/src/soc2.rs` (350 LOC, was `compliance_soc2.rs`)
   - `crates/argo-compliance/src/tenant_tables.rs` (134 LOC, was `compliance_tenant_tables.rs`)
2. Move the test modules alongside (each file's `#[cfg(test)] mod tests {...}` stays embedded; the dedicated `_tests.rs` test files in `argo-cli/src/handlers/` can either move to `argo-compliance/src/<module>_tests.rs` siblings via `#[path]` OR migrate to `tests/` integration tests — implementer's call; document choice in PR body).
3. In `argo-cli/src/handlers/compliance_*.rs`, replace each file with a thin wrapper that:
   - Re-exports the CLI arg structs (since they live in `argo-cli`).
   - Calls into `argo_compliance::<topic>::run(...)` for the actual logic.
   - Aims for ≤50 LOC per wrapper.
4. Add `argo-compliance` to `Cargo.toml [workspace.members]` and to `argo-cli/Cargo.toml [dependencies]`.

After the extraction:
- `crates/argo-compliance/` exists with 4 modules + tests
- 4 thin shims remain in `argo-cli/src/handlers/`
- 2 DEEP-DRIFT briefs (HIPAA + generic-export) become claimable with `argo-compliance::*` as their target

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `<NEW: crates/argo-compliance/Cargo.toml>` (≤30 LOC) — workspace dep declarations matching what `compliance_*.rs` currently uses (`argo-store`, `argo-types`, `argo-audit`, etc.). Grep the existing handlers for `use ` lines to enumerate deps.
- `<NEW: crates/argo-compliance/src/lib.rs>` (≤40 LOC) — `pub mod delete; pub mod export; pub mod soc2; pub mod tenant_tables;` + re-exports.
- `<NEW: crates/argo-compliance/src/delete.rs>` (≤420 LOC — move verbatim from `compliance_delete.rs`; only `use crate::tenant_tables` etc. paths change).
- `<NEW: crates/argo-compliance/src/export.rs>` (≤350 LOC).
- `<NEW: crates/argo-compliance/src/soc2.rs>` (≤360 LOC).
- `<NEW: crates/argo-compliance/src/tenant_tables.rs>` (≤140 LOC).
- `<NEW: crates/argo-compliance/src/delete_tests.rs>` etc. (≤400 LOC each, moved via `#[path]` or as integration tests under `tests/`).
- `crates/argo-cli/src/handlers/compliance_delete.rs` — replace with ≤50-LOC thin shim.
- `crates/argo-cli/src/handlers/compliance_export.rs` — same.
- `crates/argo-cli/src/handlers/compliance_soc2.rs` — same.
- `crates/argo-cli/src/handlers/compliance_tenant_tables.rs` — same.
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add ONE `argo-compliance = { path = "../argo-compliance", version = "0.0.1" }` dep.
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-compliance` to `members`.

## Files you must NOT touch

- `crates/argo-audit/**` — compliance uses audit but does NOT modify audit.
- `crates/argo-store/**` — compliance reads store but does NOT modify store.
- Any CLI subcommand definition (the CLI args structs stay in `cli_subcommands*.rs`; only the handler bodies move).
- Other `argo-cli/src/handlers/*.rs` files unrelated to compliance.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `crates/argo-compliance/` exists with `Cargo.toml`, `lib.rs`, 4 module files.
- [ ] Public surface: `argo_compliance::delete::run`, `argo_compliance::export::run`, `argo_compliance::soc2::run`, `argo_compliance::soc2::classify`, `argo_compliance::tenant_tables::delete_order` (mirror the existing function signatures).
- [ ] All 4 thin shims in `argo-cli/src/handlers/` ≤ 50 LOC each.
- [ ] `cargo build -p argo-compliance -p argo-cli --all-targets` clean.
- [ ] `cargo clippy -p argo-compliance -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-compliance` passes (the moved tests).
- [ ] `cargo test -p argo-cli compliance_` passes (the shim integration).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-argo-compliance-crate-skeleton`
- PR title: `compliance: extract dedicated argo-compliance crate (claude-argo-compliance-crate-skeleton)`

## Notes for the implementer

- This is a mechanical extraction. Behavior identical. The only "design" decision is where the test modules land (embedded `#[cfg(test)] mod tests` inside each module vs sibling `_tests.rs` via `#[path]` vs `tests/` integration); document the choice in PR body.
- Confirm the deps each `compliance_*.rs` uses by grepping `use ` at the top of each. Common deps: `argo-store::SessionStore`, `argo-audit`, `argo-types`, `clap`, `serde`, `serde_json`. Make `argo-compliance/Cargo.toml` match.
- If a handler accesses CLI-internal helpers (e.g. argument parsing), move only the BUSINESS LOGIC; leave the CLI-specific argument parsing in the shim.
- Unblocks 2 DEEP-DRIFT briefs once landed: `J17-compliance-hipaa-disclosure-log` + `J18-compliance-generic-export`. Those briefs will be rewritten in a follow-up tick.
