# fix-2791-shared-identity-null-vertical-only

Authored against: origin/main b516ce6fa907181b145f51fede1861791ae1ab58 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-2791-shared-identity-null-vertical-only

## Goal

Tighten PR #2791 so the shared-identity-file relaxation does NOT re-open a
cross-vertical read. Adversarial review found a NEW-VULN: the relaxation is too
wide.

## Why now (adversarial review of #2791, 2026-06-10 — NEW-VULN)

#2791 lets a vertical-bound teammate read shared identity files
(USER.md/SOUL.md/AGENTS.md) again (closing #2787's over-fence). The filename
gate is correct (closed enum `is_shared_identity()`, entry.rs:71) and TENANT
isolation is intact (`WHERE workspace_id = ?`). BUT the shared branch uses
`list_active` (store.rs:201-213) which returns EVERY USER.md row in the
workspace **regardless of vertical_id** — and the write path
(`append_for_vertical`, store_vertical.rs:24, reachable from CLI
handlers/memory.rs:111 `--vertical <vid>`) accepts a caller-supplied vid for
identity files. So vertical "beta" can write a USER.md tagged
`vertical_id="beta"` and vertical "alpha" reads it via
`list_for_vertical_or_shared(ws, User, "alpha")` — a cross-vertical read of a
vertical-scoped identity file. "Shared" must mean workspace-global ONLY, not
"any vertical."

## Fix

1. In the shared-identity branch, filter to NULL-vertical rows only: instead of
   `list_active` (all-vertical), query `WHERE vertical_id IS NULL` for the
   shared filenames (shared == workspace-global). MEMORY.md + non-shared files
   stay on `list_for_vertical` (`AND vertical_id = ?`) unchanged (#2787 preserved).
2. (Defense in depth) consider rejecting identity-file writes with a non-NULL
   vertical_id at `append_for_vertical`, or document that identity rows are
   always workspace-global.

## Acceptance / verifier (add the missing tests)

1. A vertical-bound teammate CAN read a workspace-global (NULL-vertical) USER.md.
2. A USER.md written with `vertical_id="beta"` is NOT returned to vertical
   "alpha" (the NEW-VULN — must be DENIED).
3. Cross-tenant: tenant A cannot read tenant B's USER.md (currently untested).
4. MEMORY.md cross-vertical still denied (#2787 regression pin).
5. `cargo test -p argo-memory` green.

## Size / owner

~20-40 LOC + 2 tests. Owner: claude. 🔴 P1 SECURITY — closes a cross-vertical
identity-file read the #2791 relaxation would otherwise introduce; WS1.
