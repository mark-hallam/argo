# win-vertical-packs-home-userprofile-fallback

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — `argo packs` HARD-ERRORS on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `b85e6ecd1` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-vertical-packs-home-userprofile-fallback`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (finding 4 of 9).

## Problem
`crates/argo-vertical-packs/src/loader.rs:51` (`PackLoaderConfig::default_from_env`)
reads `std::env::var("HOME")` and `.map_err(...)`s to a hard `ArgoError::Io("HOME
is not set; cannot resolve ~/.argo/packs")` with NO `USERPROFILE` fallback. On
native Windows `HOME` is typically unset → `argo packs list` (any packs
subcommand: `argo-cli/src/handlers/packs.rs:58` → `PackLoader::default_loader()`
→ `default_from_env()`) **hard-errors** at runtime. Unlike the silent-skip
config bugs, this one fails the command outright. #2904 fixed this class
elsewhere; argo-vertical-packs was missed.

## Fix
Mirror the #2904 pattern with a small pure helper. New `src/home_dir.rs` module
(keeps `loader.rs` under 500 LOC): `home_dir()` = `HOME` → `USERPROFILE`
(empty ignored) via a pure, env-injected `home_dir_from` with unit tests.
`default_from_env` calls it and only errors when BOTH are unset (message updated).
Unix byte-identical (HOME set ⇒ same `~/.argo/packs`).

## Verify
On Windows: `cargo test -p argo-vertical-packs --lib` green (incl. 4 new
`home_dir` cases) + clippy clean + loader.rs < 500 LOC. Conductor re-verifies
unix (HOME-set path byte-identical). Branch + IN_REVIEW.
