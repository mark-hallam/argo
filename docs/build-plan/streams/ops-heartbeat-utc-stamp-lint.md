# ops-heartbeat-utc-stamp-lint

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-heartbeat-utc-stamp-lint`.

🟡 **P2 process-drift guard.** Source: repeated evaluator/auditor notes about local-time timestamps written with a `Z` suffix into heartbeat surfaces. The read side is defensive now; the write side still lacks a repo lint that makes UTC drift obvious in CI/manual checks.

## Scope

Add a lint helper that validates heartbeat JSON files and `docs/skycot/agents/SHARED.md` append lines use strict UTC `YYYY-MM-DDTHH:MM:SSZ` stamps, and flags obviously local/non-UTC variants before they accumulate unread-handoff noise.

## Files you may touch

- `scripts/check-heartbeat-utc-stamps.sh` (NEW, ≤220 LOC)
- `scripts/tests/check-heartbeat-utc-stamps.bats` (NEW, ≤220 LOC)
- `docs/skycot/agents/README.md` if present, otherwise `docs/skycot/agents/SHARED.md` header note — narrowly: mention the UTC contract

## Files you must NOT touch

- Scheduled-task files outside the repo
- Heartbeat JSON contents except via tests/fixtures
- `docs/build-plan/BUILD_STATUS.md` except the standard row flip

## Acceptance criteria

- [ ] Lint checks every `docs/skycot/agents/heartbeats/*.json`
- [ ] Lint checks appended `SHARED.md` lines for UTC `Z` format
- [ ] Output points to the exact offending file/line on failure
- [ ] Tests cover valid UTC, malformed timestamp, local-offset timestamp, and fake-`Z` local format
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-heartbeat-utc-stamp-lint`
- PR title: `ops: lint heartbeat UTC timestamp contracts (ops-heartbeat-utc-stamp-lint)`
