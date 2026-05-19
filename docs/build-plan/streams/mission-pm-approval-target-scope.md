# Stream P0-3 — per-target approval grant scope

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-approval-target-scope`.
**Source**: skycot-pm-worker REPORT.md §5.2 + §6.2 + §11 P0 #3.

## Scope

Today `add_session_grant(sid, tool, source, note, one_shot)` (`crates/argo-cli/src/handlers/session_approval.rs:54-56`) grants the entire `(session, tool)` pair. Once Riley is approved to call `update_roadmap` once, she can update **any** roadmap line in the rest of the session. That is a trust hazard — operators reasonably expect "approve this update of Q3-2026 line item L42" to mean exactly that.

This brief adds an optional `target_glob` to session grants. The consume-side check looks up the most-specific grant matching `(session, tool, target_glob)`. Existing grants with `target_glob = NULL` continue to behave as session-global (back-compat). New CLI flag `--target <glob>` on `argo session approve`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__session_grant_target.sql` (NEW) — `ALTER TABLE session_grant ADD COLUMN target_glob TEXT;` plus a non-unique index on `(session_id, tool, target_glob)`. Verify slot with `scripts/check-migration-slots.sh`.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned wrapper for the raw SQL.
- `crates/argo-store/src/session_grant.rs` (or equivalent) — narrowly scoped: extend `add_session_grant` signature with `target_glob: Option<&str>`; extend the consume-side check (`take_session_grant` or equivalent) to match `target` against the stored glob via the existing glob library used by `Capability::ToolDispatch`.
- `crates/argo-cli/src/handlers/session_approval.rs` — add `--target <glob>` flag to `approve`; thread through to the store call. Audit payload includes `target_glob`.
- `crates/argo-cli/src/handlers/session_approval_tests.rs` (NEW sibling) — unit + integration coverage.

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — UI changes are a separate brief (the card can keep showing target-less approvals).
- Any tool dispatch code outside the grant-consume path.
- The `Principal` / `Capability` types — this is grant-layer, not capability-engine layer.

## Acceptance criteria

- [ ] Migration is additive: existing rows have `target_glob = NULL` and behave as session-global (back-compat invariant).
- [ ] `add_session_grant(sid, tool, ..., target_glob: Option<&str>)` accepts a glob. Stored as TEXT.
- [ ] Consume-side check: when `target_glob` is set on a grant, the inbound dispatch must include a `target` field; the check uses the existing glob matcher (same one `Capability::ToolDispatch::tool_name_glob` uses) and rejects mismatches with `ArgoError::Provider("approval grant target mismatch")`.
- [ ] When `target_glob` is `NULL`, the grant matches any target (current behaviour).
- [ ] `argo session approve <sid> --tool <name> --target <glob>` works end-to-end; without `--target` the grant remains session-global.
- [ ] Audit payload for `ApprovalGranted` includes `target_glob` (null-or-string).
- [ ] Tests: 6 cases — (a) grant without target_glob matches any target; (b) grant with `Q3-*` glob matches `Q3-2026-L42` and rejects `Q4-2026-L01`; (c) grant with target_glob requires target on consume; (d) one-shot + target_glob consumes on first match; (e) two grants on same tool with different globs apply most-specific match; (f) migration is idempotent.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-store -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-approval-target-scope`
- PR title: `approval: per-target glob on session grants (mission-pm-approval-target-scope)`

## Notes for the implementer

- Reuse the existing glob matcher. Do NOT introduce a new regex/wildcard library. If the existing matcher is in `argo-capability`, expose a public function rather than copy-pasting.
- "Most-specific match" rule: if two grants apply (one without target_glob, one with), prefer the targeted one. This prevents an accidental session-wide grant from masking a narrower one. Test (e) covers this.
- The `target` value on dispatch comes from the action declaration's `payload` — typically a field like `line_item_id` or `prd_id`. The brief does NOT prescribe which field; the caller passes a string.
- This brief is grant-layer only. A future brief will surface the `target_glob` in `ApprovalCard` so the approver sees "approve update_roadmap for Q3-2026-*" instead of "approve update_roadmap".

## Out of scope

- UI surfacing of target glob (separate brief).
- Auto-suggesting glob scope based on the inbound request's payload.
- Cross-session target grants (a workspace-wide grant) — current scope is session-bound.
