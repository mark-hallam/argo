# Stream codex-cli-routes-skill-ir-preventive-split — preventively extract helpers from routes_skill_ir.rs (499 LOC)

**Owner**: codex.
**Depends on**: none (parallelizable with `codex-cli-routes-approvals-json-split-helpers`).
**Authored against**: origin/main `885a761d2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-routes-skill-ir-preventive-split`.
🟡 **P1 PREVENTIVE.** `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs` is 499 LOC — one line away from tripping the conductor pre-merge gate and blocking unrelated PRs (same failure mode as the `routes_approvals_json.rs` jam this tick). Pre-split before it bites.

## Scope

Mechanical extraction: pull error-envelope helpers + tenant/skill-id parsers + query/header utilities + audit emit out of `routes_skill_ir.rs` into a new sibling `routes_skill_ir_helpers.rs`. The 7 route helpers (`dispatch`, `dispatch_post`, `import_response`, `save_response`, `ir_response`, `export_response`, `versions_response`, `authorize`) stay in the main file.

Target: main file drops from 499 → ~330-360 LOC; new helpers file lands at ~140-170 LOC. No behavioral change.

This is a **pure mechanical split**. No new logic. Treat as fast-mode candidate.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs` — remove the extracted helpers; add `use super::routes_skill_ir_helpers::*;` (or per-name). Keep: the doc comment, the 7 `pub(super) async fn` / `async fn` route handlers (`dispatch` L44, `dispatch_post` L61, `import_response` L76, `save_response` L132, `ir_response` L182, `export_response` L240, `versions_response` L287), the `authorize` helper (L351), and the inline `#[cfg(test)] mod tests` block (if present).
- `NEW: crates/argo-cli/src/handlers/gateway/routes_skill_ir_helpers.rs` — destination for the extracted helpers. Move byte-equivalent:
  - `fn resolve_tenant(...)` (L365)
  - `fn parse_tenant_id(raw: &str) -> Result<TenantId, (&'static str, String)>` (L384)
  - `fn parse_skill_id(raw: &str) -> Result<SkillId, (&'static str, String)>` (L390)
  - `fn query_value(query: &str, key: &str) -> Option<String>` (L396)
  - `fn header_value(req: &str, name_lower: &str) -> Option<String>` (L410)
  - `fn parse_save_graph(req: &str) -> Result<SkillGraph, (&'static str, String)>` (L422)
  - `fn parse_import_bundle(req: &str) -> Result<SkillIrBundle, (&'static str, String)>` (L433)
  - `async fn emit_audit(...)` (L441)
  - `fn bad_request(...)` (L474), `fn not_found(...)` (L478), `fn forbidden(...)` (L482), `fn server_error(...)` (L486)
  - Promote visibility to `pub(super)` so the main file's `use super::routes_skill_ir_helpers::*` keeps them invisible outside the gateway module.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: add `mod routes_skill_ir_helpers;` line. No other edits.

## Files you must NOT touch

- Any `routes_*.rs` other than the two named above
- `Cargo.toml` (no new deps)
- Any callsite outside the gateway module (none should be needed — all helpers were private/`pub(super)`)
- Any test file outside `routes_skill_ir`'s own test block

## Acceptance criteria

- [ ] `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs` is < 400 LOC (target ~350)
- [ ] `crates/argo-cli/src/handlers/gateway/routes_skill_ir_helpers.rs` exists and is < 200 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli routes_skill_ir` passes (no test deltas beyond mechanical relocation of `#[cfg(test)]` rows if any move)
- [ ] `bash scripts/codex-pre-push.sh` clean
- [ ] `bash scripts/conductor-pre-merge-gate.sh` clean on the resulting branch
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `codex-cli-routes-skill-ir-preventive-split`
- PR title: `cli(gateway): preventively split routes_skill_ir helpers to sibling (codex-cli-routes-skill-ir-preventive-split)`

## Notes for the implementer

- Same template + shape as `codex-cli-routes-approvals-json-split-helpers` — they're sibling work. If both land in the same wave, use the first one as the worked example.
- Pre-existing test flakes in `argo-cli` (per `KNOWN_FLAKES.md`) reproduce on clean `origin/main`; HARD RULE 2 applies.
- No callsite refactor outside the gateway module is expected. If you find one, treat as a sensible-deviation and document in the PR body.
