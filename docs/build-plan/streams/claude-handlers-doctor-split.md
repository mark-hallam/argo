# PRIORITY — split `handlers/doctor.rs` under 500 LOC

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `88a2a769` on `2026-05-13`.

> **WHY THIS IS PRIORITY:** `crates/argo-cli/src/handlers/doctor.rs`
> is currently **520 LOC** — over the 500-LOC ceiling. Pre-claim
> ceiling refusals are blocking every TODO brief that names this
> file in its may-touch list (e.g. throttle/scim/security-check
> follow-ups). Split into domain-grouped siblings.

## Scope

`crates/argo-cli/src/handlers/doctor.rs` at **520 LOC**. The `argo
doctor` subsystem groups naturally by check category:

- **channels** — `doctor channels` (already extracted in
  `doctor_channels.rs` per a previous split, verify)
- **security** — `doctor security-headers`, `doctor secrets`,
  `doctor approvals`, `doctor throttles`
- **storage** — `doctor sqlite`, `doctor migrations`, `doctor disk`
- **gateway** — `doctor http`, `doctor metrics`, `doctor routes`
- **runtime** — `doctor runtime`, `doctor sandbox`, `doctor providers`

Public API stable: each `pub async fn doctor_<thing>` re-exported
from the main `doctor.rs` shim.

## Approach

1. Identify the groupings inside the current file (run `rg -n 'pub
   async fn|pub fn' crates/argo-cli/src/handlers/doctor.rs`).
2. Carve into ≤350 LOC sibling files (`doctor_security.rs`,
   `doctor_storage.rs`, `doctor_gateway.rs`, `doctor_runtime.rs`).
3. Leave `doctor.rs` as a thin re-export hub + shared helpers (≤200 LOC).
4. Register siblings in `crates/argo-cli/src/handlers/mod.rs`.

Pure mechanical extraction. Public API stable.

## Files you may touch

- `crates/argo-cli/src/handlers/doctor.rs` — shrink to ≤200 LOC
- NEW siblings: `doctor_security.rs`, `doctor_storage.rs`, `doctor_gateway.rs`, `doctor_runtime.rs` (or whatever groupings the actual content supports — adjust during the work)
- `crates/argo-cli/src/handlers/mod.rs` — register modules
- Any existing test file referencing `doctor::*` (path may change if function moved; aim for zero test edits via re-exports)

## Files you must NOT touch

- Test bodies — handler signatures unchanged
- The CLI surface (`cli_doctor_cmds.rs`) — re-exports cover the
  callsite changes

## Acceptance criteria

- [ ] `doctor.rs` < 500 LOC (target ≤ 200)
- [ ] Every new sibling < 500 LOC
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All `cargo test -p argo-cli doctor` tests pass unchanged
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-handlers-doctor-split`
- PR title: `cli: split handlers/doctor.rs under 500 LOC (claude-handlers-doctor-split)`

## Notes

- The `-split` suffix triggers `check-pre-claim-ceiling.sh` auto-skip
  so this brief itself is claimable.
- Pure mechanical work — no behavior change.
