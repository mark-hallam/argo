# PRIORITY — split `routes_dispatch_post.rs` under 500 LOC

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `88a2a769` on `2026-05-13`.

> **WHY THIS IS PRIORITY (unblocks worker swarm 2026-05-13):**
> The file is currently **540 LOC** — already over the 500-LOC
> ceiling AND triggering pre-claim ceiling refusals on the
> claude-worker-loop. Every brief that touches gateway POST
> dispatch is refused at claim time until this lands. The recent
> J7/J9/J10/J11/J13 + retros + worker-policies merges grew the
> file from ~250 to 540 LOC; future J16/J17/J18 and any new POST
> endpoint cannot land without a split.

## Scope

`crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` is at
**540 LOC**. Each new POST endpoint dispatch arm adds ~5-15 LOC.

This is a pure refactor. Public API stable via re-exports.

## Approach

Carve POST dispatch arms into domain-grouped sibling helpers. Look at
the file's `match` shape first — typical groupings (verify against
actual content):

- **workers** — `/workers/<id>/{pause,resume,archive,…}`, retrain
- **audit** — `/audit/<id>/replay` etc.
- **duties** — duty PATCH / DELETE / POST
- **memory** — facts append / supersede / tombstone
- **approvals** — approval POSTs / undo / bundle
- **retros** — retro create / finalize

Mirrors the proven split pattern (#851 / #882 / #909 / #910 / #914 /
#915 / #932 / #935). One sibling file per group, ≤350 LOC each. Main
file becomes a thin re-export + top-level `match` dispatching to
sibling helpers. Each sibling owns 2-6 arms.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — shrink to <350 LOC re-export hub
- NEW siblings: `routes_dispatch_post_workers.rs`, `routes_dispatch_post_audit.rs`, `routes_dispatch_post_duties.rs`, `routes_dispatch_post_memory.rs`, `routes_dispatch_post_approvals.rs`, `routes_dispatch_post_retros.rs` (≤350 LOC each)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — register new sibling modules

## Files you must NOT touch

- The route handlers themselves (`routes_worker_*.rs`, `routes_audit_*.rs` etc.) — this is a dispatch-layer split only
- Any business logic — pure mechanical extraction
- Tests should pass byte-for-byte unchanged (handler signatures unchanged)

## Acceptance criteria

- [ ] `routes_dispatch_post.rs` < 500 LOC (target ≤ 350)
- [ ] Every sibling < 500 LOC
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All gateway POST integration tests pass unchanged
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-routes-dispatch-post-split`
- PR title: `gateway: split routes_dispatch_post.rs under 500 LOC (claude-routes-dispatch-post-split)`

## Notes

- This is the 8th split in the proven pattern. Mechanical work.
- After this lands, the pre-claim ceiling check stops refusing
  briefs that name `routes_dispatch_post.rs` in their may-touch.
- Don't try to land business-logic improvements in the same PR. Pure rename + move.
