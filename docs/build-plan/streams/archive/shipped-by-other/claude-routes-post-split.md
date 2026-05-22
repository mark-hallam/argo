# PRIORITY — split `routes_post.rs` under 500 LOC

**Owner**: claude. **Depends on**: nothing (pure refactor, ARCHITECTURAL PREP).

> **WHY THIS IS PRIORITY:** the file is currently **563 LOC** —
> already over the project's 500-LOC ceiling — and is blocking
> two Phase A mission items: `mission-gateway-bearer-auth`
> (PHASE A SHIP-BLOCKER #2) and any future addition to gateway
> POST handlers. This is the 7th split in the proven pattern
> (#851 worker_actions / #882 cli_subcommands / #909 skill_handlers /
> #910 routes_channels / #914 cli.rs / #915 argo-store-lib).

## Scope

`crates/argo-cli/src/handlers/gateway/routes_post.rs` is at **563 LOC**.
Each new POST endpoint adds ~30-80 LOC. The next legitimate addition
(bearer-auth Principal extractor wiring + audit-row stamping for
every successful POST) cannot land without a split first.

This is a pure refactor. Public API stable.

## Approach

Carve POST handlers into domain-grouped sibling files. Look at the
file's structure first — typical groupings:
- workers (worker action POSTs: enqueue, approve, deny, complete)
- skills (skill review POSTs: approve, reject, propose)
- audit (any audit-emit POSTs)
- sessions / handoffs (orchestrator POSTs)
- channels / approvals (notify-channel callback POSTs)
- duty / cron (duty schedule POSTs)

Naming convention: `routes_post_<domain>.rs` (mirrors
`routes_channels_*.rs` post-#910 split).

After:
- `routes_post.rs` ≤ 300 LOC — keeps the public-facing dispatcher /
  router-builder + cross-cutting helpers
- 4-6 NEW `routes_post_<domain>.rs` files, each ≤ 250 LOC
- Public API stable — handlers remain re-exported via `routes_post.rs`
  if any external caller imported them by full path

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_post.rs` — reduce to ≤ 300 LOC
- `crates/argo-cli/src/handlers/gateway/routes_post_<domain>.rs` (NEW × 4-6)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: register
  the new sibling modules via `mod routes_post_<domain>;`
- Any file that imported a handler by the full `routes_post::<fn>` path —
  narrowly: change import path if a symbol moved (mechanical; should be
  zero edits if `pub use` re-exports are right)

## Acceptance criteria

- [ ] `routes_post.rs` ≤ 300 LOC
- [ ] Each new sibling file ≤ 250 LOC
- [ ] No public-API change (every existing call site still compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as before
      (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Headroom for next 5-10 POST endpoint additions before any
      sibling approaches the 500 ceiling

## Branch + PR

- Branch: `claude-routes-post-split`
- PR title: `refactor: split routes_post.rs under 500 LOC budget (claude-routes-post-split)`
