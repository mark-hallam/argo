# eval-genesis-output-loads-cleanly-through-argo-vertical-v2

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-genesis-output-loads-cleanly-through-argo-vertical`.

🟢 **P2 CLAIM-3.** Genesis output round-trips through argo-vertical loader.

## Scope

Drive Genesis on 5 distinct role descriptions, write the trees to tempdirs, argo_vertical::load_dir each, assert no parse errors. Cross-claim guard for claim 3.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-genesis/tests/, crates/argo-vertical/tests/
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

- Branch: `eval-genesis-output-loads-cleanly-through-argo-vertical`
- PR title: `eval-genesis-output-loads-cleanly-through-argo-vertical`

## Notes for the implementer

- ~150-250 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
