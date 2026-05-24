# Stream codex-cli-routes-approvals-json-split-helpers — extract error envelopes + cursor/parse helpers from routes_approvals_json.rs

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `885a761d2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-routes-approvals-json-split-helpers`.
🔴 **P0 GATE-UNBLOCK.** `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` is 526 LOC and trips the conductor pre-merge gate, jamming PRs #2199 / #2202 / #2207 / #2212 plus any future PR that runs the gate. Ship this first.

## Scope

Mechanical extraction: pull the error-envelope helpers, cursor encoding, and query-string parsers out of `routes_approvals_json.rs` into a new sibling `routes_approvals_json_helpers.rs`. The 3 public route handlers (`list_response`, `decide_response`, `undo_response`) plus the audit/tenant emit helpers stay in the main file. Convention matches the existing `routes_worker_skills.rs` + `routes_worker_skills_helpers.rs` split (commit `a7d9d8884`).

Target: main file drops from 526 → ~410 LOC; new helpers file lands at ~110-130 LOC. No behavioral change. No public surface change (the helpers are all private `fn` / `pub(super) fn` inside the gateway module — moving them to a sibling under the same `super::` is byte-equivalent at the call sites once `mod` declaration is added).

This is a **pure mechanical split**. No new logic. No clippy or test changes beyond what mechanical relocation requires. Treat as fast-mode candidate.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` — remove the extracted helpers; replace with `use super::routes_approvals_json_helpers::*;` (or per-name imports). Keep the doc comment, struct `DecideBody`, the 3 `pub(super) async fn` route handlers (`list_response` L58, `decide_response` L140, `undo_response` L272), and the audit/tenant async helpers (`resolve_workspace_tenant_id` L220, `emit_approval_declined` L234, `request_summary` L258, `approval_row_json` L325, `decision_from_record` L371, `decider_role_from_record` L405).
- `NEW: crates/argo-cli/src/handlers/gateway/routes_approvals_json_helpers.rs` — destination for the extracted helpers. Move these byte-equivalent:
  - `fn short_payload(v: &Value) -> String` (L362)
  - `fn parse_limit(tail: &str) -> Result<usize, (&'static str, String)>` (L428)
  - `fn parse_query(tail: &str, key: &str) -> Option<String>` (L441)
  - `fn encode_cursor(ts: &str, id: &str) -> String` (L450)
  - `fn decode_cursor(c: &str) -> Option<(String, String)>` (L455)
  - `fn ok_decide(approval_id: &str, decision: &str) -> (&'static str, String)` (L463)
  - `fn bad_request(message: &str) -> (&'static str, String)` (L475)
  - `fn not_found(id: &str) -> (&'static str, String)` (L479)
  - `fn conflict(id: &str, current: &str) -> (&'static str, String)` (L487)
  - `fn gone(id: &str) -> (&'static str, String)` (L495)
  - `fn server_error(message: &str) -> (&'static str, String)` (L503)
  - `fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String)` (L507)
  - Constants used only by these helpers: `DEFAULT_LIMIT`, `MAX_LIMIT` (L49-50). `UNDO_WINDOW_SECS` (L52) stays in the main file (used by `undo_response`).
  - Update visibility on each moved fn to `pub(super)` so the main file can `use super::routes_approvals_json_helpers::*` without surfacing them beyond the gateway module.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: add `mod routes_approvals_json_helpers;` line alongside the existing `mod routes_approvals_json;` declaration. No other edits.
- `NEW: crates/argo-cli/src/handlers/gateway/routes_approvals_json_helpers_tests.rs` — OPTIONAL. If you move the inline `#[cfg(test)] mod tests` block from the main file (only the tests that exercise the moved helpers — `parse_limit`, cursor encode/decode), put them here. If all the inline tests touch route handlers (which stay in main), leave the `mod tests` block as-is. Implementer judgment.

## Files you must NOT touch

- Any `routes_*.rs` other than the two named above
- `Cargo.toml` (no new deps; no workspace edits)
- `crates/argo-store/`, `crates/argo-audit/`, `crates/argo-memory/` — no callsite changes needed
- Any web/TypeScript file

## Acceptance criteria

- [ ] `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` is < 500 LOC (target ~410)
- [ ] `crates/argo-cli/src/handlers/gateway/routes_approvals_json_helpers.rs` exists and is < 200 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli routes_approvals_json` passes (all existing tests for the 3 route handlers, including the 4 pre-existing flakes documented in `KNOWN_FLAKES.md` — only fail the brief if a NEW test breaks; the pre-existing parallel-test isolation flakes reproduce on clean `origin/main` per HARD RULE 2)
- [ ] `bash scripts/codex-pre-push.sh` clean (workspace clippy gate)
- [ ] `bash scripts/conductor-pre-merge-gate.sh` clean on the resulting branch
- [ ] PR body acknowledges this unblocks PRs #2199, #2202, #2207, #2212
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `codex-cli-routes-approvals-json-split-helpers`
- PR title: `cli(gateway): split routes_approvals_json helpers to sibling (codex-cli-routes-approvals-json-split-helpers)`

## Notes for the implementer

- Template: read `routes_worker_skills.rs` + `routes_worker_skills_helpers.rs` end-to-end before starting (shipped via #2144, commit `a7d9d8884`). Match its visibility + import shape exactly.
- The split MUST be byte-equivalent at the behavior level. No refactor, no rename, no signature change. If you find yourself "improving" something while moving, stop and revert — that's a follow-up brief, not this one.
- The `#[cfg(test)]` inline test block at the bottom of `routes_approvals_json.rs` is large and references many of these helpers. Keep the test block in the main file unless every test inside it can be moved cleanly (decision: probably keep in main — those tests exercise the route handlers, which stay).
- The 4 jammed PRs (#2199, #2202, #2207, #2212) are all gate-blocked on this single file's LOC. Once this lands, the conductor re-runs the pre-merge gate against each — there's no rebase needed for them (none touch this file).
- Sensible deviation allowed: if you find a helper that's referenced from a different module (cross-`super::` import), keep it in the main file and pick a different helper to move that achieves the LOC target. The acceptance criterion is "< 500 LOC", not "move these specific 11 functions."
