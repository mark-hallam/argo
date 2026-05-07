# claude-perf-streaming-responses — Stream large gateway responses instead of buffering

**Owner**: claude. **Depends on**: none.

> Several gateway GET routes today materialise their full result set
> into memory before serialising — fine at 100 rows, problematic at
> 100k. This brief introduces a small streaming helper and wires it
> into the worst-offender read routes (session list, message
> backfill, audit list) so the gateway streams JSON-lines as rows
> arrive from sqlx.

## Why

Two related problems:
1. memory: a 100k-row response can blow several hundred MB transient
   heap during serialisation
2. time-to-first-byte: clients block until the whole list is built;
   even a paginated client doing `--all` pages serially has nothing
   to do with row 1 until all N are ready

Sqlx's `Stream` impl already gives us row-at-a-time async iteration.
We just need a thin wrapper that bridges that into a chunked HTTP
response.

## Scope

- **NEW** `crates/argo-cli/src/handlers/gateway/streaming.rs`
  (≤ 250 LOC): `stream_json_array<T, S>(stream: S) -> Response`
  where `S: Stream<Item = ArgoResult<T>>`, `T: Serialize`. Emits a
  proper JSON array (`[\n`, comma-separated rows with newlines, `]\n`)
  with `Content-Type: application/json` and
  `Transfer-Encoding: chunked`.
- Provide an alternate `stream_ndjson<T, S>(...)` variant for
  newline-delimited JSON (consumer opts in via `Accept: application/x-ndjson`).
- **Modify** at most three routes to opt-in to streaming:
  - `GET /sessions` (list)
  - `GET /sessions/{id}/messages` (backfill)
  - `GET /audit` (list)
- Each route negotiates: streaming if `Accept` requests it
  **or** if the result-set is anticipated > 1k rows; legacy
  buffered path otherwise (so existing clients are unaffected).

## Approach

1. Build the helper around `axum::body::Body::from_stream` (or
   equivalent — match whatever the existing gateway uses).
2. Pump the sqlx row stream through `try_stream!` (futures), serialise
   each row with `serde_json::to_vec`, prepend comma-separator after
   first row.
3. The three target routes already exist; modify their handler bodies
   to construct an sqlx stream instead of `fetch_all`, and pass
   through the helper.
4. Backwards compatibility: the JSON array variant produces the
   exact same bytes as the buffered version, so existing clients
   parse it identically.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/streaming.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/gateway/streaming_tests.rs` (NEW,
  ≤ 200 LOC) — unit tests for both variants + content-negotiation
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 pub
  mod line (BLANKET CLI ALLOWLIST allows mod.rs edits)
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly:
  swap three handler bodies to use streaming
- existing route files (e.g. `routes_audit.rs`) — narrowly: the
  three target handlers

## Files you must NOT touch

- Other gateway routes (no drive-by streaming conversions)
- Store-layer query implementations (use existing sqlx APIs)
- Schema / migrations
- TUI or CLI client code (the wire format is byte-compatible)

## Acceptance criteria

- [ ] `stream_json_array` + `stream_ndjson` helpers with unit tests
- [ ] Three target routes converted; default path is streaming when
      result-set predicted large, buffered otherwise
- [ ] Memory benchmark or smoke test: list 50k sessions, observe RSS
      stays well below buffered-path peak (record before/after in PR)
- [ ] Time-to-first-byte for `GET /sessions` on a 50k-row store is
      < 100ms (record observed value)
- [ ] All existing gateway integration tests pass unchanged
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] No file > 500 LOC

## Branch + PR

- Branch: `claude-perf-streaming-responses`
- PR title: `gateway: stream large list responses instead of buffering (claude-perf-streaming-responses)`

## Notes for the implementer

- The helper must emit valid JSON even on an empty stream (`[]\n`).
- Serialisation errors mid-stream are awkward — chunked transfer
  cannot rewind. Choose: either `panic!` (kills the connection,
  client retries) or emit a documented `{"_error": "..."}` final
  element. Prefer the former; it's loud and obvious.
- Don't add new clap flags — content-negotiation goes through the
  HTTP layer.
