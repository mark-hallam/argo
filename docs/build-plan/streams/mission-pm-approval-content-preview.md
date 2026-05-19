# Stream P0-2 — content preview in ApprovalCard

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-approval-content-preview`.
**Source**: skycot-pm-worker REPORT.md §6.3 + §11 P0 #2.
**Splittable**: yes — backend payload extension and frontend collapsible body are two reviewable changes; the worker MAY split this into `-backend` + `-frontend` sub-briefs if PR sizing demands. If split, the frontend half claims after the backend half merges.

## Scope

The delegator persona (`docs/skycot/ux-research/delegator.md:51-53`) explicitly demands: "see exactly what the worker is about to do BEFORE it does it, for at least the first week. Dry-run mode is not optional." Today `ApprovalCard.tsx:28-63` shows `verb + detail + consequenceIfYes + consequenceIfNo + reason` — intent and consequences only. **The actual draft (PRD body, email body, roadmap delta) is not surfaced.** Approvers rubberstamp blind.

This brief threads a `draft_content` (server) / `draftContent` (client) field through the approval payload and renders it as a collapsible body in the card. Default-collapsed so the card stays glanceable; one-click expand to read the full draft.

This is the **single highest-leverage trust fix** identified in the investigation. Without it, every shipped vertical's approval is partial.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

### Backend
- `crates/argo-types/src/approval.rs` or equivalent — narrowly scoped: add `pub draft_content: Option<String>` field to the approval-request payload shape. Backward-compatible (defaults to `None`).
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — populate `draft_content` from the action's executor-side preview when available (e.g. document_creation executor's rendered body, gmail.send draft body).
- `crates/argo-cli/src/handlers/gateway/routes_*` (whichever surfaces `/api/v1/approvals`) — propagate `draft_content` through the JSON serialization layer.
- `crates/argo-store/src/approvals.rs` — narrowly scoped: add `draft_content TEXT` column via additive migration (next free slot — verify with `scripts/check-migration-slots.sh`). Nullable; existing rows read as `NULL`.

### Frontend
- `web/components/skycot/ApprovalCard.tsx` — add `draftContent?: string` prop; render in a `<details>` element below `detail`, default-collapsed, with a "Show draft" label that becomes "Hide draft" when open.
- `web/components/skycot/ApprovalCard.test.tsx` — coverage for draft-rendering + collapsed-by-default.
- `web/app/(app)/page.tsx` — narrowly scoped: pass `draftContent` from the API response into `<ApprovalCard>`.

## Files you must NOT touch

- The `ApprovalCard` structural contract (props `verb`, `detail`, `consequenceIfYes`, `consequenceIfNo`) — those are load-bearing; `draftContent` is purely additive.
- `redact.ts` policy — the existing `redactionPolicy` prop already covers PII in `detail`; extend it to also cover `draftContent` (same policy applies).
- Any `verticals/` files.
- Any other approval-related component (popovers, badges).

## Acceptance criteria

- [ ] Approval payload (DB row + API JSON) carries optional `draft_content` field; absence keeps current behaviour.
- [ ] Migration is additive and idempotent (`ALTER TABLE ... ADD COLUMN ... DEFAULT NULL`).
- [ ] `worker_actions_auto_approval.rs` populates `draft_content` for at least the document_creation and gmail.send executors (others land as follow-ups).
- [ ] `ApprovalCard` renders `draftContent` inside a default-collapsed `<details>` element; a11y label is `"Show draft" / "Hide draft"`.
- [ ] `redactionPolicy` applies to `draftContent` the same way it applies to `detail` (defense-in-depth).
- [ ] Existing `ApprovalCard.test.tsx` tests still pass; new test covers (a) absent `draftContent` (no `<details>` rendered) and (b) present `draftContent` (collapsed by default, expands on click).
- [ ] Rust: `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Web: `pnpm lint && pnpm test` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-approval-content-preview`
- PR title: `approval: content preview in card + payload (mission-pm-approval-content-preview)`

## Notes for the implementer

- The `<details>` element is intentional — native a11y for "expandable region" without a custom dropdown.
- Do NOT auto-expand the draft on focus or on hover — the persona is a delegator in 90-second slices (`delegator.md:42-46`). Collapsed-by-default is load-bearing.
- For long drafts (>2KB), clamp the rendered height to ~30vh and let the body scroll internally; do not let one long draft push the action buttons off-screen.
- If splitting the brief: the backend half is mergeable independently (frontend just doesn't render the new field yet); the frontend half cannot merge before the backend.
- Cross-check the audit chain: when a draft is approved, the audit payload should include the draft body's SHA-256 so post-hoc reviewers can prove what was approved (not just what was intended). Stretch — drop if it inflates the PR.

## Out of scope (follow-up briefs)

- Diff view between successive draft revisions (Track L Phase 2).
- Inline draft edit before approval ("approve with edits").
- Per-channel rendering tweaks (Slack vs. iMessage layout).
