# ops-gateway-freshness-check-also-detects-binary-staleness

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `02055815a` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-gateway-freshness-check-also-detects-binary-staleness`.

## Why

Evaluator round 94 (2026-05-26T21:18Z) surfaced a false-negative class
in `scripts/check-gateway-freshness.sh`:

- PR #2420 (genesis demo unblock) merged at `91215d129` with handler-only
  changes (no new migration, schema_version unchanged).
- The running gateway was 16.3h stale vs the merged change.
- `check-gateway-freshness.sh` reported FRESH because `schema_version`
  hadn't moved — even though the gateway binary was running pre-merge
  code that still serves the OLD 503 response on `POST /api/v1/genesis`.
- Investor demo recording is blocked until the operator runs
  `make restart-live-gateway`, but the conductor tick step 4.7 said
  FRESH so nothing surfaced the staleness.

The schema-only comparison is structurally insufficient: handler /
route / business-logic changes don't bump migrations.

## What

Embed the build's git commit SHA in `/health` JSON, then teach
`scripts/check-gateway-freshness.sh` to detect binary staleness as a
distinct status when the embedded SHA doesn't match `origin/main` HEAD.

### Surface contract (added field)

`/health` response gains a `build_sha` field (7-char short SHA, same
form as `git rev-parse --short HEAD`):

```json
{
  "status": "ok",
  "version": "0.x.y",
  "schema_version": 83,
  "uptime_seconds": 60028,
  "build_sha": "91215d1"
}
```

If the build env didn't capture a SHA (e.g. tarball install with no
`.git`), emit `"build_sha":"unknown"` — the script treats `unknown`
as "cannot diagnose, fall through to existing schema-only check".

### check-gateway-freshness.sh contract additions

After the existing schema-version pass, also probe `build_sha`:

- Extract `build_sha` from the same `/health` JSON body (same sed-regex
  pattern as `schema_version`).
- If `build_sha == "unknown"`: print one extra info line
  `build_sha=unknown (binary check skipped)` and continue to the
  existing schema-only exit code. (Backward-compatible default.)
- If `build_sha` parses: compare against
  `git -C "$REPO_ROOT" rev-parse --short origin/main` (already fetched
  by the caller — script does NOT shell out to `git fetch`; it's
  read-only).
- If SHAs differ AND uptime_seconds > 3600 (1h grace so a freshly
  started gateway isn't flagged before workers can race ahead): emit
  `STATUS: STALE-BINARY  (gateway built at <gw_sha>, main at <main_sha>, restart recommended: `make restart-live-gateway`)`
  and exit `4` (new exit code; preserves the existing 0/1/2/3 contract).
- If SHAs match (or schema mismatch already trumps): fall through to
  the existing FRESH/STALE/UNREACHABLE/PARSE_ERROR exit codes.

### Conductor tick wiring

`docs/build-plan/PROTOCOL_LOOP_PROMPT.md` step 4.7 surfaces drift; the
new STALE-BINARY status MUST also surface (don't silently suppress).
No tick action required — the operator runs `make restart-live-gateway`
when convenient.

## How

### 1. Add `build_sha` to `/health` JSON

`crates/argo-cli/src/handlers/gateway/routes_health.rs`:

- Read `env!("ARGO_BUILD_GIT_SHA")` at compile time (fallback to
  `"unknown"` via `option_env!`).
- Format into the JSON body in `health_response_with_cwd` (line 84+).

`crates/argo-cli/build.rs` (new file, ~15 LOC):

- Run `git rev-parse --short HEAD` at build time.
- Emit `cargo:rustc-env=ARGO_BUILD_GIT_SHA=<sha>` so `env!` finds it.
- On error (no `.git`, tarball install), emit
  `cargo:rustc-env=ARGO_BUILD_GIT_SHA=unknown` and continue.
- Emit `cargo:rerun-if-changed=.git/HEAD` and
  `cargo:rerun-if-changed=.git/refs/heads/main` so cargo rebuilds when
  HEAD moves.

### 2. Teach `check-gateway-freshness.sh` to parse + compare

`scripts/check-gateway-freshness.sh`:

- After line ~105 (where `GATEWAY_SCHEMA_N` is extracted), add the same
  pattern for `build_sha`.
- Compute `MAIN_SHA="$(git -C "$REPO_ROOT" rev-parse --short origin/main 2>/dev/null || true)"`.
- If `build_sha != unknown` and `MAIN_SHA != ""` and they differ and
  the response also carried `uptime_seconds > 3600`: emit STALE-BINARY
  and exit 4.

Update the exit-code contract block at the top of the script to
document the new `4` code.

### 3. Smoke test the script

Extend the existing `FAKE_HEALTH_BODY` test hook so the test harness
exercises:

- (a) old build SHA + matching schema → STALE-BINARY exit 4
- (b) matching build SHA → FRESH exit 0
- (c) `build_sha=unknown` → falls through to schema-only check (back-compat)
- (d) uptime < 3600 → does NOT flag STALE-BINARY (grace window)

Add the 4 cases to whatever existing smoke harness lives next to the
script (or create `scripts/check-gateway-freshness-tests.sh` paralleling
the brief pattern used by `scripts/check-demo-roster-tests.sh`).

## Acceptance

- [ ] `/health` JSON includes `build_sha` field on a freshly-built
      gateway (verify by `curl -s http://127.0.0.1:7400/health`).
- [ ] `scripts/check-gateway-freshness.sh` exits 4 when the gateway's
      build_sha is older than origin/main AND uptime > 3600s.
- [ ] `scripts/check-gateway-freshness.sh` still exits 0 / 1 / 2 / 3 in
      their respective conditions (4-code contract unchanged).
- [ ] Conductor tick step 4.7 prints a `STATUS: STALE-BINARY` line that
      includes the recommended remediation command.
- [ ] `bash scripts/check-gateway-freshness-tests.sh` (or equivalent
      smoke) — 4/4 PASS.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_health.rs` (add 2-3 LOC for build_sha field)
- `crates/argo-cli/build.rs` (new file, ~15 LOC)
- `scripts/check-gateway-freshness.sh` (~20 LOC added)
- `scripts/check-gateway-freshness-tests.sh` (new file, ~40 LOC) OR extend existing
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (1-line note that STALE-BINARY also surfaces at step 4.7)
- `docs/build-plan/streams/ops-gateway-freshness-check-also-detects-binary-staleness.md` (this file)

## Scope guard

- Do NOT change `/health` 200/503 semantics or the other fields. New
  field is additive; existing parsers (k8s probes, etc.) ignore it.
- Do NOT introduce a non-additive migration (no migration touch at all).
- Do NOT shell out to `git fetch` from the script — read-only.
- Do NOT auto-restart the gateway — operator's lever.
- Do NOT delete or rename existing exit codes 0/1/2/3.

~60 LOC total. P1 (closes a confirmed false-negative class that hid a
16.3h stale gateway from the conductor tick during the v1.0 demo gate).

🤖 Generated from evaluator round 94 P1 recommendation
(`docs/skycot/evals/scheduled/2026-05-27/round-94-pr-2420-genesis-demo-unblock-post-merge.md`).
