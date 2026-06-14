# win-config-user-config-path-home-userprofile-fallback

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — user-config layer silently vanishes on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `079afccf0` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-config-user-config-path-home-userprofile-fallback`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (findings 6 + 7 of 9 — same crate, one helper).

## Problem
`crates/argo-config/src/loader.rs:97` and `crates/argo-config/src/source_tracking.rs:156`
each carried an IDENTICAL `user_config_path()` that, after the `XDG_CONFIG_HOME`
check, read `std::env::var_os("HOME")?` with NO `USERPROFILE` fallback. On native
Windows `HOME` is typically unset, so both return `None`:
- loader.rs → the user-level config layer (`<home>/.config/argo/argo.toml`) is
  silently skipped, breaking the documented precedence (env > project-local >
  user-level > defaults).
- source_tracking.rs → `argo config show --source` drops its user-config
  annotations.

#2904 fixed this exact class in `sections_core::home_dir`/`expand_tilde` +
`argo-credentials::master_key` but missed these two call sites.

## Fix
No new parallel abstraction: add ONE canonical
`sections_core::user_config_path()` (`pub(crate)`) with a pure, env-injected
core `user_config_path_from(xdg, home, userprofile)` that reuses the existing
`home_dir_from` (HOME→USERPROFILE, empty ignored). Delete BOTH duplicate
`user_config_path()` fns and call the canonical resolver at both sites. Unix is
byte-identical (HOME set ⇒ same path; XDG branch unchanged). Pure cross-platform
tests (no env mutation): XDG-wins, HOME/.config, USERPROFILE fallback, none.

## Verify
On Windows: `cargo test -p argo-config --lib` green (incl. the 4 new
`user_config_*` cases + existing source_tracking test) + clippy clean. Conductor
re-verifies unix (HOME-set path byte-identical). Branch + IN_REVIEW.
