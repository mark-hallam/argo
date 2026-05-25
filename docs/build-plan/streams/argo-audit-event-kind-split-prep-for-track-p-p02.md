# argo-audit-event-kind-split-prep-for-track-p-p02

**Owner**: claude or codex.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh argo-audit-event-kind-split-prep-for-track-p-p02`.
**Unblocks**: Track P P-02 (`pitch-truth-boundary-runtime-enforcement`) — currently re-BLOCKED 2026-05-25T09:48Z by codex pre-claim per `crates/argo-audit/src/event_kind.rs` being at exactly 500 LOC, and P-02 needs to add 3 new variants (`ClaimAllowed` / `ClaimBlocked` / `ClaimEscalated`).

🔴 **P0 ARCHITECTURAL-PREP — splits `event_kind.rs` so Track P P-02 can land.**

The truth-boundary runtime enforcement (P-02) requires adding 3 `AuditEventKind` variants to thread the gate's Allow / Block / Escalate decisions through audit emission. But the destination file is at the 500-LOC ceiling (HARD RULE 6); any addition fails the pre-merge gate. This brief splits the file ahead of P-02 so the P-02 worker can land cleanly.

## Scope

1. **Verify the current state**: `wc -l crates/argo-audit/src/event_kind.rs` should report exactly 500.
2. **Pick the split axis**. Three options — pick whichever yields the cleanest sibling cluster:
   - **Option A (variant-group split)**: keep the `pub enum AuditEventKind` in `event_kind.rs`, move the `impl AuditEventKind` block (and any `fn`s on it) to `event_kind_impl.rs`. Drops ~80-120 LOC from event_kind.rs.
   - **Option B (cluster-by-domain split)**: split variants into `event_kind_worker.rs` (Worker* variants), `event_kind_skill.rs` (Skill* variants), `event_kind_channel.rs` (CrossTenant* + Channel* variants), with `event_kind.rs` re-exporting. Most ambitious; risks parallel-surface drift.
   - **Option C (extract doc-comments)**: a fair chunk of event_kind.rs may be doc-comments. Extract per-variant doc into `event_kind_docs.rs` if doable. Smallest blast radius.
3. **Implement the chosen split**. Keep `pub enum AuditEventKind` and all variants accessible at the existing `argo_audit::event_kind::AuditEventKind` import path — no caller-side change required.
4. **Verify all callers still compile**: `cargo check --workspace --no-fail-fast` clean.
5. **Re-confirm room for 3 new variants**: post-split, `event_kind.rs` should be ≤ 450 LOC so P-02 can add `ClaimAllowed` / `ClaimBlocked` / `ClaimEscalated` without re-hitting the ceiling.
6. **Once MERGED**: flip `pitch-truth-boundary-runtime-enforcement` BLOCKED → TODO (conductor ops).

## Acceptance criteria

- [ ] `wc -l crates/argo-audit/src/event_kind.rs` ≤ 450 LOC post-split.
- [ ] `cargo check --workspace --no-fail-fast` clean.
- [ ] `cargo test -p argo-audit` passes (any existing tests on event_kind still work).
- [ ] No caller-side rename (`use argo_audit::event_kind::AuditEventKind` still resolves).
- [ ] PR body documents the chosen split option (A / B / C) + LOC delta + zero-caller-rename rationale.

## May-touch

- `crates/argo-audit/src/event_kind.rs` (split)
- `crates/argo-audit/src/event_kind_*.rs` (NEW sibling files per chosen option)
- `crates/argo-audit/src/lib.rs` (re-export plumbing only — narrow)
- This brief file (status updates)

## Out of scope

- Adding new `AuditEventKind` variants (P-02's job)
- Renaming the enum or re-exporting under a different module path
- Touching `event.rs` / `sink.rs` / `csv.rs` / other audit modules

## Effort

~50-150 LOC delta + sibling files. ~30-60 min depending on chosen option.
