# Mission Phase 2 — Workspace-wide secrets scrubbing in tracing layer

**Owner**: claude. **Depends on**: none (independent).

## Scope

Argo's `tracing` subscriber emits structured logs across the workspace. Today,
`api_key`, `Authorization: Bearer ...`, password fields, and JWT tokens can leak
into log output if a span field or log message embeds them. This brief adds a
`SecretsScrubberLayer` to the `argo-tracing` subscriber chain that pattern-scrubs
known secret shapes before any writer sees them — replacing the value with
`<scrubbed:kind>` (e.g. `<scrubbed:bearer>`, `<scrubbed:apikey>`).

This is a defense-in-depth surface; sites that take secrets via typed config
should still avoid logging them, but this layer is the safety net.

## Approach (concrete)

1. **NEW** `crates/argo-tracing/src/secrets_scrubber.rs` (~280 LOC):
   - `pub struct SecretsScrubberLayer` implementing
     `tracing_subscriber::Layer<S>` for `S: Subscriber + for<'a> LookupSpan<'a>`
   - `on_event` intercepts the event's recorded fields, runs each string-typed
     field through `scrub(&str) -> Cow<'_, str>`, then re-emits via the inner
     subscriber chain
   - `scrub` runs a fixed set of compiled `regex::Regex`:
     - `Bearer\s+[A-Za-z0-9\-_\.=]+` → `Bearer <scrubbed:bearer>`
     - `(api[_-]?key|apikey)["':\s=]+["']?[A-Za-z0-9\-_]{16,}["']?` →
       redact value
     - `(password|passwd|pwd)["':\s=]+["']?[^\s"',}]+["']?` → redact value
     - JWT shape `eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+` →
       `<scrubbed:jwt>`
     - Generic high-entropy hex/base64 ≥ 32 chars adjacent to known key names
   - Compiled regexes live behind a `OnceLock<Vec<Regex>>` for zero-alloc reuse
2. **Modify** `crates/argo-tracing/src/lib.rs`:
   - `init_subscriber` (or whatever existing init fn) wires the scrubber into
     the layer chain BEFORE the `fmt` writer layer (so all writers see scrubbed
     output)
   - Env opt-out via `ARGO_TRACING_SCRUB=0` for local debug only (default on)
3. **Audit pass** across the workspace:
   - `rg -nP '(api_key|password|secret|token|bearer)' --type rust` and confirm
     no `tracing::info!` / `debug!` / `warn!` / `error!` emits a raw secret as a
     formatted argument (best-effort — the layer is the safety net)
   - Document findings in PR description

## Files you may touch

- `crates/argo-tracing/src/secrets_scrubber.rs` (NEW, ≤ 320 LOC)
- `crates/argo-tracing/src/secrets_scrubber_tests.rs` (NEW, ≤ 250 LOC) —
  6 unit tests:
  - bearer token in event message → scrubbed
  - api_key field value → scrubbed
  - password field value → scrubbed
  - JWT shape → scrubbed
  - innocuous string with the word "token" → NOT scrubbed
  - opt-out via `ARGO_TRACING_SCRUB=0` → not scrubbed
- `crates/argo-tracing/src/lib.rs` — narrowly: layer wiring + 1 pub mod
- `crates/argo-tracing/Cargo.toml` — narrowly: add `regex` (already a
  workspace dep)

## Files you must NOT touch

- Any other crate's `tracing::*!` call sites unless the audit pass surfaces a
  clear leak — this brief is about the layer, not call-site rewriting
- `argo-types` — no new types; the scrubber operates on tracing's value model

## Acceptance criteria

- [ ] `SecretsScrubberLayer` implements `tracing_subscriber::Layer`
- [ ] Default subscriber chain wires the scrubber by default
- [ ] `ARGO_TRACING_SCRUB=0` disables (logged once at init)
- [ ] 6 unit tests cover the patterns above
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tracing` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-security-secrets-scrubbing`
- PR title: `tracing: scrub secrets from log output (claude-security-secrets-scrubbing)`

## Notes for the implementer

- `tracing-subscriber` 0.3 layer pattern: implement `Layer::on_event` and
  build a `Visit` that records each field, scrubs string values, then
  forwards via a wrapper event. The `tracing-subscriber` examples
  (`reload`, `fmt`) show the visit/forward pattern.
- Compile regexes once via `OnceLock<Vec<Regex>>`; do NOT recompile per event.
- Keep the regex set small and tight — over-scrubbing breaks debug usefulness.
