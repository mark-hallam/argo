# fix-check-gateway-freshness-sh-distinguish-503-from-unreachable — gateway-freshness script returns UNREACHABLE on /health 503, masking the stale-binary case

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `24cb565b8` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-check-gateway-freshness-sh-distinguish-503-from-unreachable`.

## Why this is P2 (4th-round evaluator carry — round-66/67/68/69)

Evaluator round-69 confirmed for the 4th consecutive round: `scripts/check-gateway-freshness.sh` returns `STATUS: UNREACHABLE` (exit 2) whenever `/health` returns HTTP 503 — because `curl -fsS` treats any non-2xx as an error, the script's `HEALTH_BODY` is empty, and it falls through the "no response" path. This is a **false negative** that has masked the schema-bump-merged-but-binary-stale state on tick T18-08 and on every conductor tick since.

Live impact verified 2026-05-25T19:25Z: `curl -fsS http://127.0.0.1:7400/health` returns nothing on stdout (curl fails on HTTP 503), but `curl -sS http://127.0.0.1:7400/health` returns body `{"status":"degraded","reason":"schema_version_mismatch","schema_version":82,...}` with HTTP 503. The body contains the version field — we just need to capture it.

Effect of fix: when the gateway is up-but-degraded (503 + schema_version present), the script should report a new `STATUS: STALE-503` line and exit code `1` (the "stale, operator should restart" path), not exit code `2` (unreachable). The conductor would then surface "operator restart needed" in tick summaries — instead of treating the gateway as down and skipping the freshness signal entirely.

Round-67 + round-68 + round-69 cite this as `γ`. Source: `docs/skycot/evals/scheduled/2026-05-25/round-69-pr-2331-expected-schema-version-merged-but-gateway-binary-stale.md` and the two prior rounds.

## Scope

Pure bash rewrite of the `/health` probe block in `scripts/check-gateway-freshness.sh` (lines 76-84 currently). Switch from `curl -fsS` (fail-on-error) to `curl -sS -w '\n%{http_code}'` (capture body + status code), then branch:

- HTTP 2xx with schema_version → existing FRESH/STALE path (unchanged).
- HTTP 503 with schema_version present in body → new `STATUS: STALE-503  (gateway up but reporting schema_version_mismatch — restart binary: `bash scripts/restart-live-gateway.sh`)` and exit `1`.
- HTTP 5xx without schema_version, or curl-level failure, or empty body → existing UNREACHABLE path (unchanged).

Update the script's exit-code documentation block (lines 26-38) to add the 503-stale case as a sub-case of exit 1 (STALE). Do not introduce a new exit code — the conductor already branches on `1` vs `2`, and 503-stale is semantically STALE.

Update the companion test `scripts/check-gateway-freshness-tests.sh` to add a 503-mock case (echo a 503 body via a tiny netcat or python -m http.server fixture, or — simpler — add a `FAKE_HEALTH_BODY` + `FAKE_HEALTH_STATUS` env-var override the script honors when set, with a test that exercises it).

**Not in scope**: changing the drift threshold, adding new exit codes, touching restart-live-gateway.sh.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/check-gateway-freshness.sh` — the probe block + exit-code header.
- `scripts/check-gateway-freshness-tests.sh` — add a 503 case.

## Files you must NOT touch

- Any file under `crates/` — this is pure ops tooling.
- `scripts/restart-live-gateway.sh` — out of scope.

## Acceptance criteria

- [ ] When `/health` returns HTTP 503 with `schema_version_mismatch` body, `bash scripts/check-gateway-freshness.sh` prints `STATUS: STALE-503` and exits `1`.
- [ ] When `/health` is unreachable (gateway not running), script still prints `STATUS: UNREACHABLE` and exits `2` (no regression).
- [ ] When `/health` returns HTTP 200 with schema_version, script still prints `STATUS: FRESH` / `STATUS: STALE` per drift (no regression).
- [ ] `bash scripts/check-gateway-freshness-tests.sh` PASS including the new 503 case.
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-fix-check-gateway-freshness-sh-distinguish-503-from-unreachable` or `codex-fix-check-gateway-freshness-sh-distinguish-503-from-unreachable`.
- PR title: `ops: check-gateway-freshness.sh distinguishes HTTP 503 from UNREACHABLE (fix-check-gateway-freshness-sh-distinguish-503-from-unreachable)`.

## Notes for the implementer

- This is bash-only, no Rust. Fast-mode candidate. ~10-20 LOC delta.
- Sample curl invocation that works: `curl -sS --max-time 3 -w '\n%{http_code}' http://127.0.0.1:7400/health 2>/dev/null`. The body comes first, then a newline, then the 3-digit status. Use `tail -1` for status + `head -n -1` for body, or read into two vars.
- The schema_version extraction sed (line 89-92) already works on a 503 body — just needs the body to be captured. Don't rewrite the sed.
- Reference round-69 eval report for the live-state proof; this is a 4th-round-unauthored finding (rare).
