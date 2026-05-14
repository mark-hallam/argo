# mission-marketplace-seller-e2b-demo — Marketplace-seller end-to-end via E2B (CU sandbox)

**Owner**: claude. **Depends on**: argo-executor-computer-use MERGED (existing); marketplace-seller skill-family briefs MERGED (existing).
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketplace-seller-e2b-demo`.

## Scope

MISSION_BACKLOG item 143. Demonstrates the marketplace-seller vertical end-to-end against an **E2B sandbox** (Computer-Use environment) rather than a real Facebook Marketplace test account. The real-account piece is operator-decisional (manual setup of test credentials, etc.) and out of scope here. This brief: drive the worker through list-creation → CU-driven Marketplace listing → screenshot evidence → operator review.

LOC: ~700.

## Files you may touch

- NEW `verticals/marketplace-seller/demo/e2b_e2e.sh` — shell driver script.
- NEW `verticals/marketplace-seller/demo/e2b_recording_fixture.json` — recorded CU action sequence (replay-driven).
- NEW `verticals/marketplace-seller/demo/README.md` — operator setup notes + real-account follow-up (DEFERRED-operator).

## Files you must NOT touch

- argo-executor-computer-use internals; marketplace-seller skill IRs (consume).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Shell script runs against E2B sandbox; produces 5+ screenshots showing listing creation flow.
- [ ] Worker stops at `approval_required` with draft listing for operator review.
- [ ] README documents the real-Marketplace-account setup as DEFERRED-operator follow-up.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-marketplace-seller-e2b-demo`
- PR title: `Marketplace-seller end-to-end via E2B (CU sandbox) (mission-marketplace-seller-e2b-demo)`

## Notes for the implementer

- Mission-claim alignment: claim 5 (PC tasks via CU + MCP fallback).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).
