# check-genesis-seed-user-md-vertical-id

Authored against: origin/main 62765b6cf40a55540cdca653efb03e423ff3d139 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh check-genesis-seed-user-md-vertical-id

## Goal

Verify (and fix if needed) that globally-shared seeded memory files
(USER.md / SOUL.md / AGENTS.md) remain readable by a vertical-BOUND teammate
after PR #2787 made the memory tool vertical-scoped.

## Why now (adversarial review of #2787, non-blocking observation)

#2787 (merged/pending) makes the in-agent memory tool list/read scope by
vertical_id — correct WS1 fail-closed direction. BUT the reviewer noted: with
a vertical bound, `list_action` routes USER.md/SOUL.md/AGENTS.md through
`list_for_vertical`, which EXCLUDES legacy NULL-vertical rows. So if the
provisioning/seed path writes a legitimately-SHARED USER.md WITHOUT a
vertical_id, a vertical-bound teammate may no longer read it — a functional
regression (not a leak).

## Investigate → fix-if-needed

1. Find where USER.md/SOUL.md/AGENTS.md are seeded (rg 'USER.md|SOUL.md|AGENTS.md'
   in argo-demo-seed / provisioning / memory store). Do they set a vertical_id?
2. If seeded WITHOUT vertical_id (NULL) AND meant to be globally shared: either
   (a) make `list_for_vertical`/read include the workspace-global (NULL-vertical)
   rows for these specific shared filenames (USER/SOUL/AGENTS), preserving
   tenant scope, OR (b) seed them per-vertical. Prefer (a) — shared identity
   files SHOULD be workspace-global, only MEMORY.md is vertical-scoped.
3. If they`re already vertical-scoped or per-vertical, this is a NO-OP — record
   the finding and close STALE-VERIFIED.

## Acceptance / verifier

1. A test: a vertical-bound teammate CAN read a workspace-global seeded USER.md
   (shared identity) but CANNOT read another vertical`s MEMORY.md (the #2787
   isolation still holds).
2. cargo test -p argo-tool memory green.

## Size / owner

Investigation + ~0-60 LOC. Owner: claude. 🟡 P2 — closes the #2787 review's
flagged shared-file-readability gap (functional, not a leak).
