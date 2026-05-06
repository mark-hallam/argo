# V1.1 — Multi-step (N-of-M) approval signing

**Owner**: codex. **Depends on**: D-email (#810),
D-slack-approvals merged.

## Scope

High-risk actions need N approvers, not 1. Worker emits an
action with `min_signatures: u8`; each approving channel
counts 1 signature; action only executes once N collected.

- New `WorkerAction::min_signatures: u8` (default 1; 0 = no
  approval required, ≥2 = N-of-M)
- New table `worker_action_signature(action_id, signer,
  channel, signed_at)`
- Auto-approval gate refuses to execute until
  `count(signatures) >= min_signatures`
- Each NotifyChannel reply with "approve" inserts one row
  (idempotent on `(action_id, signer)`)
- New CLI: `argo worker action sigs <id>` lists collected
  signatures + remaining

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-identity/src/types.rs` — narrowly: add
  `min_signatures: u8` field to ActionRecord (default 1)
- `crates/argo-identity/src/auto_approval.rs` — narrowly:
  signature-count check before AutoApprove returns
- `crates/argo-store/migrations/<next>__action_signatures.sql`
  (NEW)
- `crates/argo-store/src/action_signatures.rs` (NEW)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` —
  narrowly: hold action in `pending_approval` until
  signature count satisfied (sibling-file extension pattern)
- `crates/argo-cli/src/handlers/worker_action_sigs.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: add
  `Sigs` subcommand variant

The action schema lives in `argo-identity/src/types.rs`
(NOT `worker_action.rs`); the auto-approval gate is in
`argo-identity/src/auto_approval.rs` (NOT a nonexistent
argo-runtime crate). Notify-channel reply paths that
already mark approve/deny are reused — they should now
INSERT a signature row instead of immediately approving.

## Acceptance criteria
- [ ] `min_signatures` plumbed through emit→gate→execute
- [ ] Signatures persisted; idempotent on `(action_id, signer)`
- [ ] Action stays in `pending_approval` until N collected
- [ ] `argo worker action sigs <id>` lists signers + channels
- [ ] 6 unit tests + 1 integration test (3-of-3 happy path)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-multi-step-approval`
- PR title: `worker: N-of-M approval signing for high-risk actions (codex-multi-step-approval)`
