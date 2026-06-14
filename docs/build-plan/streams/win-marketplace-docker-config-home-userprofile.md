# win-marketplace-docker-config-home-userprofile

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — Docker registry auth silently skipped on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `c1a5a485a` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-marketplace-docker-config-home-userprofile`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (finding 5 of 9).

## Problem
`crates/argo-marketplace/src/oci_auth.rs:119` (`docker_config_path()`, after the
`DOCKER_CONFIG` check) reads `std::env::var_os("HOME")` with NO `USERPROFILE`
fallback. On native Windows `HOME` is typically unset → returns `None` → the
docker-config credential source (`~/.docker/config.json`) is silently skipped,
forcing an explicit `--token` on every `argo vertical publish`/`pull`
(`argo-cli` `vertical_publish.rs:70` + `vertical_pull.rs:56`). #2904 fixed this
class elsewhere; argo-marketplace was missed.

## Fix
Refactor `docker_config_path` to delegate to a pure, env-injected
`docker_config_path_from(docker_config, home, userprofile)` that adds the
`HOME` → `USERPROFILE` fallback (empty ignored), mirroring the #2904 pattern.
Pure cross-platform tests (no env mutation): DOCKER_CONFIG-wins, HOME/.docker,
USERPROFILE fallback, none. Unix byte-identical (HOME set ⇒ same path).

## Verify
On Windows: `cargo test -p argo-marketplace --lib` green (incl. 4 new
`docker_config_*` cases) + clippy clean + oci_auth.rs < 500 LOC. Conductor
re-verifies unix (HOME-set path byte-identical). Branch + IN_REVIEW.
