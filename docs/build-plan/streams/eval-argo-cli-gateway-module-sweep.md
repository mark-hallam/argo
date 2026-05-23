# eval-argo-cli-gateway-module-sweep

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-cli-gateway-module-sweep`.

🟡 **P1 CLAIM-6.** Complete handler-by-handler RBAC + tenant audit.

## Scope

Round-7 spot-audited workflow handlers. Round-9 audited workers handlers. Sweep the rest: cohorts, audit-export (post-fix), settings, skills, knowledge, etc. For each handler module: RBAC check site, tenant scoping, cross-tenant negative test.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-cli/src/handlers/gateway/**
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

- Branch: `eval-argo-cli-gateway-module-sweep`
- PR title: `eval-argo-cli-gateway-module-sweep`

## Notes for the implementer

- ~400-800 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
