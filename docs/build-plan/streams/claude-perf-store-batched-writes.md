# claude-perf-store-batched-writes — Batch hot-path inserts (audit, message)

**Owner**: claude. **Depends on**: claude-bench-message-write-perf
(provides the baseline numbers this brief is measured against).

> The two highest-volume tables in argo's store — `audit` and
> `message` — currently insert one row per call, each in its own
> implicit transaction. Under swarm load (8 parallel runners
> writing 10s of messages/second each) this saturates the WAL.
> This brief introduces a small **batched writer** that coalesces
> inserts within a short time window, reusing one transaction per
> batch.

## Why

Sqlite WAL throughput collapses when many short transactions
contend for the same writer lock. Argo's swarm runs hit this wall
directly — message append latency under swarm load is observably
higher than under single-runner load. The fix isn't a schema
change, it's coalescing.

## Scope

- **NEW** `crates/argo-store/src/batched_writer.rs` (≤ 350 LOC):
  generic `BatchedWriter<T>` that:
  - accepts items via `tokio::sync::mpsc::UnboundedSender`
  - drains the channel in a background task
  - flushes either on `MAX_BATCH_SIZE` (default 64) or on
    `MAX_BATCH_LATENCY_MS` (default 10ms), whichever first
  - calls a configured async `flush_fn(&[T]) -> ArgoResult<()>`
- **Modify** message append hot-path (likely `argo-store::message_query`
  or wherever the gateway / agent-runner appends) to **opt-in** route
  through the BatchedWriter. The single-write path stays as a fallback
  (env knob `ARGO_STORE_BATCHED_WRITES=0` to disable).
- **Modify** audit append hot-path equivalently.

## Approach (concrete)

1. Implement `BatchedWriter<T>` with shutdown semantics (flush on drop).
2. Add two writer instances to the store crate's `Store` struct:
   `messages: Arc<BatchedWriter<MessageRow>>`,
   `audits: Arc<BatchedWriter<AuditRow>>`.
3. Initialize them on `Store::open` and ensure they flush on
   `Store::shutdown` / `Drop`.
4. The `flush_fn` for each writer opens a single transaction, runs
   prepared `INSERT` statements per row, commits.
5. Provide knobs via env: `ARGO_STORE_BATCH_SIZE`,
   `ARGO_STORE_BATCH_LATENCY_MS`.
6. Document defaults in the file's doc-comment.

## Files you may touch

- `crates/argo-store/src/batched_writer.rs` (NEW, ≤ 350 LOC)
- `crates/argo-store/src/batched_writer_tests.rs` (NEW, ≤ 200 LOC)
  — 5 tests: flush-on-size, flush-on-latency, flush-on-drop,
  env-knob respected, error-from-flush-fn-propagated
- `crates/argo-store/src/message_query.rs` — narrowly: route
  the public append fn through the writer
- `crates/argo-store/src/lib.rs` — narrowly: wire the writer into
  `Store::open`, expose shutdown hook
- one audit-append site (look in `argo-audit` or
  `argo-cli/src/handlers/audit.rs`); narrowly route through writer
- `crates/argo-store/Cargo.toml` — narrowly: `tokio` features if
  `mpsc` is not already pulled

## Files you must NOT touch

- Schema / migrations (no new columns)
- Read-side query files (this brief is **write-side only**)
- `argo-cli` clap surface (no new CLI flags)

## Acceptance criteria

- [ ] `BatchedWriter<T>` with documented thresholds + env knobs
- [ ] Message + audit append hot-paths route through the writer by
      default
- [ ] `ARGO_STORE_BATCHED_WRITES=0` reverts to per-row writes
      (proves opt-out works)
- [ ] 5 unit tests + the existing message-write bench shows
      improvement vs. baseline (target: ≥ 3× speedup at batch-100,
      record actual number in PR body)
- [ ] No data loss on shutdown — `Store::shutdown` flushes pending
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store` clean
- [ ] Every file ≤ 500 LOC; new file ≤ 350 LOC

## Branch + PR

- Branch: `claude-perf-store-batched-writes`
- PR title: `store: coalesced WAL-friendly batched writes for message + audit (claude-perf-store-batched-writes)`

## Notes for the implementer

- Keep the writer **generic** so a future caller (e.g. spend events)
  can reuse it without copy-paste.
- Beware: tests that assert "row was visible immediately" need to
  flush explicitly; expose a `pub fn flush_now(&self).await` for tests.
- The bench from `claude-bench-message-write-perf` is your scoreboard.
  Re-run it before/after and put both tables in the PR body.
