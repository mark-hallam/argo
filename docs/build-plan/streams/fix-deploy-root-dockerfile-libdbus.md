# fix-deploy-root-dockerfile-libdbus

**Owner**: claude — **P2 DEPLOY BLOCKER (pre-existing; surfaced gating PR #2929).**
**Depends on**: none.
**Authored against**: origin/main `ef2e6be5e` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-deploy-root-dockerfile-libdbus

Source: discovered 2026-06-14 while gating #2929 (non-root containers). Separate
pre-existing builder-stage bug that blocks the root/ECS image build.

## Problem (verified by real `docker build` on origin/main)

`docker build -f Dockerfile .` (the root / ECS image) fails in the **builder**
stage compiling `libdbus-sys v0.2.7` — the root Dockerfile's builder installs
`ca-certificates clang cmake libssl-dev pkg-config` but **NOT `libdbus-1-dev`**,
which `libdbus-sys`'s build script needs (pkg-config can't find `dbus-1`). The
gateway Dockerfile already installs it (`deploy/Dockerfile.gateway:22`), so the
gateway builds but the root/ECS image does not. `git show origin/main:Dockerfile`
confirms this is pre-existing (NOT the non-root change). Hard go-live blocker for
the ECS image.

## Fix (one line)

Add `libdbus-1-dev` to the root `Dockerfile` builder-stage `apt-get install`
list (match `deploy/Dockerfile.gateway:22`). Confirm the two builder dep lists
are otherwise aligned; if other deps diverge, note them (but the deterministic
blocker is `libdbus-1-dev`).

## May touch
- `Dockerfile` (builder-stage apt install).
- `docs/build-plan/FEATURE_MATRIX.md` (deploy-root/ECS row, in-band).

## Parallel-surface declaration
No Rust pub types (Dockerfile only).

## Acceptance / CHECKABLE verifier
- [ ] `docker build -f Dockerfile -t argo-root .` gets PAST the `libdbus-sys`
      compile (full build green, or at minimum the libdbus-sys step succeeds —
      paste the tail). NB an earlier attempt also hit a transient `rust-docs`
      toolchain network error; the `libdbus-1-dev` fix is the deterministic one.
- [ ] FEATURE_MATRIX deploy-root row updated in-band.

~1-3 LOC. Deploy/infra — conductor review before merge.
