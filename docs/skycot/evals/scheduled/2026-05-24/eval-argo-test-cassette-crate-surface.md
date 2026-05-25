# eval-argo-test-cassette-crate-surface

- Date: 2026-05-24
- Brief: `eval-argo-test-cassette-crate-surface`
- Worker: codex, per operator override 2026-05-24 for `claude` rows
- Main anchor when claimed: `921746dc8`
- Verdict: PASS. `argo-test-cassette` has a compact, now-pinned public
  surface; the existing sample cassette remains load/save compatible; redaction,
  replay-default behavior, and request/response record shape are covered by
  offline tests.

## Scope Guard

This audit did not touch connector crates, production code, live network paths,
or the cassette JSON version. The new coverage is an integration test only:
`crates/argo-test-cassette/tests/surface_contract.rs`.

The pre-claim freshness check flagged only authored-SHA distance:
`ef55f5422` was 179 commits behind main. May-touch validation, path checks,
and deep stub checks all passed, so the brief remained actionable as a current
status audit.

## Public Surface Pin

The new contract snapshots the public inventory in
`crates/argo-test-cassette/src/lib.rs`:

- Public modules: `cassette`, `recorder`
- Cassette exports: `redact_body_json`, `redact_headers`, `Cassette`,
  `CassetteError`, `CassetteExchange`, `CassetteRequest`, `CassetteResponse`,
  `Result`, `DEFAULT_HEADER_ALLOWLIST`, `REDACTED_PLACEHOLDER`
- Recorder exports: `build_request_record`, `CassetteMode`, `CassetteRecorder`

The test also compiles those re-exported types/functions at their expected
public paths, so a future rename/removal fails before connector work silently
depends on a changed contract.

## Compatibility Coverage

The contract loads `crates/argo-test-cassette/fixtures/sample.cassette.json`,
checks its v1 shape, saves it back to a temp copy, and confirms runtime-only
fields (`cursor`, `path`) do not serialize. This pins the current JSON contract
without changing `version = "1"`.

Record construction coverage checks:

- method canonicalization to uppercase
- URL preservation
- allowlisted headers retained
- sensitive headers redacted
- non-allowlisted headers dropped
- SHA-256 body hash
- body JSON redaction
- response status and JSON body shape

## Redaction Coverage

Header redaction now covers:

- `authorization`
- `x-api-key`
- `x-auth-token`
- `proxy-authorization`

Body redaction now covers nested secret-like keys containing:

- `token`
- `secret`
- `password`
- `bearer`
- `api_key`

Non-sensitive body values and allowlisted non-sensitive headers are preserved.

## Replay / Default Mode

`CassetteMode::parse(None)` remains `Replay`, and the contract drives a
recorded `https://offline.invalid/no-network` exchange in replay mode. The test
returns the recorded response and exhausts the cassette cursor without opening a
network connection.

## Compatibility Risk

No compatibility risk found in this audit. The cassette format is simple and
versioned, and the current fixture round-trips cleanly. One intentional behavior
to keep in mind: replay matching is method+URL ordered; it does not validate
body hash during replay. That is existing behavior and not a new finding for
this brief.

## Validation

Commands:

```bash
CARGO_TARGET_DIR=/tmp/eval-argo-test-cassette-crate-surface-target cargo test -p argo-test-cassette --test surface_contract
CARGO_TARGET_DIR=/tmp/eval-argo-test-cassette-crate-surface-target cargo test -p argo-test-cassette
CARGO_TARGET_DIR=/tmp/eval-argo-test-cassette-crate-surface-target cargo clippy -p argo-test-cassette --all-targets -- -D warnings
git diff --check
bash scripts/check-brief-freshness.sh eval-argo-test-cassette-crate-surface
```

Result:

- Focused surface contract test: 6 passed
- `src/lib.rs` unit tests: 6 passed
- `tests/surface_contract.rs`: 6 passed
- doctests: 1 passed
- Clippy: clean with `-D warnings`
- Diff check: clean
- Freshness: one authored-against SHA advisory only; may-touch and premise
  checks passed

Total: 13 passed, 0 failed.

## Follow-ups

No follow-up brief proposed.
