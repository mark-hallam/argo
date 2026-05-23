# eval-claim-5-live-mcp-and-cu-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-claim-5-live-mcp-and-cu-test`.

🟡 **P1 CLAIM-5 LIVE.** Claim 5 LIVE regression test (both halves).

## Scope

Per round-18 draft: (a) MCP fake-server workflow step routes through pool + cap-check + audit; (b) macOS cu notes_smoke with ARGO_NATIVE_MACOS_INTEGRATION=1 set. Two assertions, both green = LIVE.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-workflow/tests/, crates/argo-executor-computer-use-macos/tests/
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production source outside the may-touch list.
- Other crates not part of this audit.

## Acceptance criteria

- [ ] Per-brief acceptance (see Scope above).
- [ ] If real cross-tenant leak / broken-wire found: ship as `#[ignore]`'d FINDINGS-ONLY test + name a fix-brief.
- [ ] `cargo clippy -p <affected-crate> --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `eval-claim-5-live-mcp-and-cu-test`
- PR title: `eval-claim-5-live-mcp-and-cu-test`

## Notes for the implementer

- ~300-500 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
