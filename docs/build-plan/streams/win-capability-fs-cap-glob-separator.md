# win-capability-fs-cap-glob-separator

**Owner**: windows-fleet — **P1 CROSS-PLATFORM CORRECTNESS (potential silent deny) — from #2911.**
**Depends on**: none.
**Authored against**: origin/main `83bce61f4` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/TEST-FAILURES.md (#2911 first Windows test inventory).

## Problem
`argo-capability` default filesystem-capability globs are built with the
OS-native path separator, so on Windows they contain backslashes
(`/tmp/ws\**`). `globset` matches with FORWARD slashes — so a backslash glob
may **silently fail to match real paths at runtime**, denying (or mis-allowing)
a Windows user's filesystem access. This is NOT just the failing test
(`workspace_defaults_scope_user_filesystem_caps`) — it's a potential runtime
authz correctness/security bug on Windows.

## Fix
Build the fs-capability globs with `/` ALWAYS (globset's separator), regardless
of OS — normalize any path→glob construction to forward slashes. Verify the
default user-filesystem caps match real Windows paths at runtime. The fix is
cross-platform (unix unaffected: it already used `/`).

## Verify
On Windows: the cap glob matches a real `C:\...`-rooted workspace path
(normalized) + the failing test passes. Conductor re-verifies argo-capability
on unix (globs unchanged, all tests green). Branch + IN_REVIEW.
