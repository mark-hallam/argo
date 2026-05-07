# claude-bench-message-write-perf — Criterion bench for message append throughput

**Owner**: claude. **Depends on**: claude-bench-session-throughput
(introduces criterion + bench harness pattern).

> Messages are the highest-volume row in the store: every agent turn
> writes ≥ 2 messages (user + assistant), often more with tool calls.
> This bench measures **append throughput** and compares
> single-write vs batched-write paths so we have a baseline before
> `claude-perf-store-batched-writes` lands.

## Why

The audit log + message log are the two writes that scale linearly
with agent activity. Without a benchmark we cannot defend a claim
like "batching messages is 5× faster" — we'd just be guessing.
This brief installs the bench so the batching brief can prove its
gain quantitatively.

## Scope

- **NEW** `crates/argo-cli/benches/message_write.rs` — Criterion
  bench with three cases on a warm store:
  1. **single**: 1000 messages appended one-at-a-time (current path)
  2. **batch-10**: 1000 messages in batches of 10
  3. **batch-100**: 1000 messages in batches of 100
- **Modify** `crates/argo-cli/Cargo.toml` — `[[bench]]` entry

## Approach

1. Build a single warm sqlite tempdir per bench-group iteration via
   `iter_batched_ref` (do not rebuild per sample).
2. Pre-create a single `Session` to receive the messages; the bench
   measures only the **append** path, not session/branch creation.
3. Use `Throughput::Elements(1000)` so Criterion reports msgs/sec.
4. The "batch" cases assume a future `append_messages_batch`
   API — for now, the bench can wrap a `tx.execute` loop in one
   transaction to simulate batch behaviour. Note this in the
   bench source comment so the future batched-writes brief can
   replace the simulated path with the real API.

## Files you may touch

- `crates/argo-cli/benches/message_write.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/Cargo.toml` (BLANKET CLI ALLOWLIST) — `[[bench]]` entry only

## Files you must NOT touch

- `crates/argo-store/src/message_query.rs` or message append code —
  this brief is **measurement only**, no product change
- Schema / migrations
- Other bench files

## Acceptance criteria

- [ ] `cargo bench -p argo-cli --bench message_write` runs clean
- [ ] Output reports `msgs/sec` for all three cases
- [ ] PR body records baseline numbers for single vs batch-10 vs
      batch-100, on author's machine, with hardware noted (e.g.
      "M2 Pro, 16GB, macOS 25.3")
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Bench file ≤ 200 LOC

## Branch + PR

- Branch: `claude-bench-message-write-perf`
- PR title: `bench: message append throughput, single vs batched (claude-bench-message-write-perf)`

## Notes for the implementer

- Reuse the helper pattern from `session_throughput.rs`. If a shared
  `bench_helpers.rs` module would help, place it under
  `crates/argo-cli/benches/common/mod.rs` and `include!` it (Criterion
  benches don't share a normal lib).
- Do **not** call into the gateway — bench at the store crate boundary
  for stable numbers.
- Note in PR body that batch-N numbers are upper-bound estimates; the
  actual batch API lands in `claude-perf-store-batched-writes`.
