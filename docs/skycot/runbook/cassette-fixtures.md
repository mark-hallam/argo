# VCR-style cassette fixtures

> Source crate: `crates/argo-test-cassette`. Brief:
> [`streams/mission-vcr-cassette-fixtures.md`](../../build-plan/streams/mission-vcr-cassette-fixtures.md).
> Companion docs: silent-skip nightly runner brief
> (`mission-silent-skip-nightly-integration`) — live tests still run
> nightly to catch real-API drift; cassettes catch wire-format
> regressions in milliseconds during the normal `cargo test` cycle.

## What a cassette is

A cassette is a JSON file on disk that captures one or more HTTP
exchanges (`request → response` pairs) recorded against a real API.
In CI, the test replays the cassette instead of hitting the network:

- Fast (no DNS, no TLS, no rate limits).
- Deterministic (same input → same output every run).
- Free (no live secrets needed in CI; cassettes are committed to git).

This replaces the historical "silent-skip" pattern where a test
checked `if std::env::var("ARGO_TEST_*_KEY").is_err() { return }` and
quietly returned a pass when the operator hadn't set credentials. The
silent-skip pattern produced false-confidence green CI: 65 integration
tests were silently skipping in CI as of 2026-05-12.

## Modes

The recorder reads `ARGO_TEST_CASSETTE_MODE`:

| Value         | Behavior                                                    | Network? |
| ------------- | ----------------------------------------------------------- | -------- |
| `replay` (default) | Pop next recorded exchange; method+URL must match.     | no       |
| `record`      | Hit the real API; persist the redacted exchange.            | yes      |
| `passthrough` | Hit the real API; do not persist. Debug only.               | yes      |

`replay` is the default so CI runs deterministically with no opt-in
required.

## How to record a cassette

```bash
ARGO_TEST_CASSETTE_MODE=record \
ARGO_TEST_ANTHROPIC_KEY=sk-ant-... \
cargo test -p argo-mcp-connector-anthropic --test my_cassette_test
```

The recorder hits the real provider, captures each request+response,
runs them through the redactor, and writes
`crates/argo-mcp-connector-anthropic/fixtures/my.cassette.json`. Read
the JSON file before committing — the operator is the last reviewer
of redaction correctness.

## How to replay (the CI default)

```bash
cargo test -p argo-mcp-connector-anthropic --test my_cassette_test
```

No env var needed. If the cassette is missing or the recorded
exchanges don't match the test's request sequence, the test fails
loudly — not silently. That is the entire point.

## How to refresh a cassette when the API contract changes

1. Re-run with `ARGO_TEST_CASSETTE_MODE=record` and live creds.
2. Diff the resulting cassette JSON; verify only the expected fields
   changed (e.g. response schema additions).
3. Re-read the redacted body for any newly-leaked secret-shaped keys.
4. Commit the new cassette in the same PR as the test/code change.

## Secret-redaction guarantees

**Headers.** Only headers on the allowlist
(`content-type`, `accept`, `content-length`, `user-agent`,
`authorization`, `x-api-key`, `x-auth-token`, `api-key`, `x-api-token`,
`proxy-authorization`) are persisted at all. Headers in the allowlist
that are auth-shaped are kept *for shape* but with their values
replaced by `<REDACTED>`.

**Body.** JSON keys whose lowercased name *contains* any of
`api_key`, `secret`, `password`, `token`, `bearer` have their values
replaced by the string `<REDACTED>`. Recursion descends through nested
objects and arrays.

**body_sha256.** The full request body is hashed once before redaction
and the hex digest is stored. This provides defense-in-depth: even if
the redactor misses a sensitive value embedded in body bytes, the
on-disk cassette holds only the hash.

**What is NOT scrubbed automatically.**

- Free-form text inside JSON string values that happen to contain a
  secret (e.g. a prompt that quotes an API key in narrative form).
- Headers not on the allowlist — these are dropped entirely, but if
  a vendor uses a non-standard auth header, add it to the allowlist
  *and* the sensitive-header match list in `cassette.rs`.
- Non-JSON bodies (the v1 cassette converts non-JSON bodies to JSON
  `null` for storage; the raw bytes are not retained).

**Operator review is the final gate.** Read the cassette JSON before
committing. If you spot anything that looks like a credential, the
redaction rules are incomplete for that vendor — extend
`is_sensitive_header` / `is_sensitive_body_key` in `cassette.rs`,
re-record, and only then commit.

## Sequential-match ordering

Cassettes are an ordered list of exchanges. The replayer keeps a
cursor and pops in order. On each `pop_next`, the recorded
`method + url` must match the actual request — otherwise the test
fails with `CassetteError::Mismatch`.

This means: **test order must be deterministic.** If your test
parallelizes requests with `futures::join!`, the JOIN order in your
code is not necessarily the order in which the runtime drives the
futures. Either serialize the requests (`req_a.await; req_b.await`)
or scope each request to its own cassette file.

If a future cassette upgrade needs out-of-order matching (e.g. by
request-shape hash), see the v2 notes in the brief.

## Failure modes

| Symptom                                  | Likely cause                                          | Fix                                                                 |
| ---------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------- |
| `Mismatch { expected … got … }`          | Test changed shape since last record                  | Re-record OR fix test back to recorded shape                        |
| `Exhausted at index N`                   | Test makes more requests than the cassette has        | Re-record OR shorten test                                           |
| Cassette has more exchanges than test    | Test was simplified; cassette is stale                | Re-record                                                           |
| `body_sha256` differs in failure log     | Test body changed (whitespace, key order, payload)    | Re-record (or stabilize the body)                                   |
| CI fails on a cassette test in your PR   | The cassette format silently changed in a parent PR   | Re-record on your branch                                            |

## Migration recipe (per-test follow-up)

Use this when converting one existing silent-skip integration test
to the cassette pattern:

1. **Identify** the test's HTTP calls. There must be ≤ one HTTP client
   in the test scope.
2. **Add a cassette file path** for the test:
   `crates/<crate>/fixtures/<name>.cassette.json`.
3. **Add `argo-test-cassette` as a dev-dep** of the test's crate.
4. **In the test, replace** the raw `reqwest::Client::new()` with a
   `CassetteRecorder`-backed call, e.g.

   ```rust
   let cassette = Cassette::load("fixtures/foo.cassette.json")?;
   let mut rec = CassetteRecorder::new(cassette);
   let resp = rec.execute("POST", url, headers, body).await?;
   ```

5. **Drop the silent-skip guard**:
   `if std::env::var("ARGO_TEST_*").is_err() { return; }` — remove it.
6. **Record once** with real creds:
   `ARGO_TEST_CASSETTE_MODE=record ARGO_TEST_FOO_KEY=… cargo test -p <crate> --test <name>`.
7. **Eyeball the cassette JSON**, then commit cassette + test.

The 65-test fan-out is intentionally NOT done in this brief — each
test owner converts theirs, with the test owner reviewing redaction
for their vendor.

## Known v1 limitations

- HTTP/1.1 request-response only. SSE-streaming responses are out of
  scope for v1; the cassette format will grow a `stream_events` array
  in v2 when a real streaming integration test demands it.
- WebSocket exchanges are not in scope.
- Pattern-based body redaction (regex match on string *values*) is
  not in v1. Today's redaction is key-based only.
- Cassette schema version migrations are not implemented; v1 is the
  only format. The `version: "1"` field reserves space for v2+ loaders.
