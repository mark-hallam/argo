# fix-explicit-fs-root-tier-validate-under-home

**Owner**: claude or codex — capability fence / hire perimeter; conductor review before merge.
**Depends on**: #2826 merged (fs_root resolver + safe_access_fs_root guard).
**Authored against**: origin/main `3f360fcbe` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-explicit-fs-root-tier-validate-under-home

## Problem (found by the #2826 adversarial review, 2026-06-11)

`resolved_fs_root` (`crates/argo-identity/src/store_hire_fs_root.rs`) has an
**explicit `draft.fs_root` tier** that is returned **verbatim with no
validation** (`return Some(PathBuf::from(explicit))`). The entire `HireDraft`
is deserialized straight from the `POST /api/v1/workers` body
(`routes_workers_hire.rs`), so an owner-authed request with
`{"worker_draft":{"fs_root":"/etc"}}` binds a (Genesis-FENCED, supposedly
sandboxed) teammate's `fs_root` to `/etc`, emitting `FsRead{/etc/**}` — an
absolute outside-HOME path the new `safe_access_fs_root` guard would reject if
it came through the captured-Access tier. It is gated ONLY by owner-RBAC
(`Action::WorkerHire`). `with_fs_root` catches `..` but NOT an absolute clean
path like `/etc`. So a fenced teammate's sandbox can be escaped via the wire.

## The trust-boundary nuance (decide this first)

The explicit `fs_root` field is ALSO how the CLI `argo worker create --fs-root`
deliberately sets a root — and an operator may legitimately want a root OUTSIDE
HOME there (e.g. `/data/projects/acme`). So we cannot blanket-reject
outside-HOME on the explicit tier without breaking that legitimate operator use.

**Recommended resolution (first-principles):** distinguish the *transport*, not
the field. The CLI is an operator-trust boundary (the operator types the path
on their own machine); the HTTP hire body is a less-trusted surface (any
owner-authed client). So:
- **HTTP hire path** (`routes_workers_hire.rs` + `routes_onboarding_genesis.rs`):
  run the wire-supplied `draft.fs_root` through the same under-HOME guard
  (`safe_access_fs_root`-style), rejecting outside-HOME → fall back to the
  generated default (fail-soft). The SPA never sets `fs_root` explicitly today
  (it uses `accessFsRoot`), so this breaks no real SPA flow.
- **CLI path** (`worker create --fs-root`): unchanged — operator-trusted, may
  point outside HOME, with a clear one-line confirmation in the CLI output of
  where the teammate will be sandboxed.

Confirm whether `resolved_fs_root`'s explicit tier is reached from the CLI at all
(grep: the CLI `worker create` likely sets the `worker.fs_root` column directly
via `worker_create_store.rs`, NOT through `HireDraft`/`resolved_fs_root`). If the
explicit tier is ONLY reachable from the HTTP hire draft, then simply validating
it under-HOME in `resolved_fs_root` is safe and the CLI is unaffected — verify
before choosing.

## Acceptance

- An HTTP hire with `worker_draft.fs_root` = an outside-HOME absolute path
  (`/etc`, `/`, another user's home) does NOT bind that path; it falls back to
  the generated default (security pin).
- An HTTP hire with `fs_root` under HOME still binds it (regression pin).
- The CLI `worker create --fs-root /data/...` (if it uses this path) still works
  for a deliberate operator root, OR is confirmed to use a separate path that
  this change doesn't touch (documented in the PR).
- `with_fs_root` remains the type-boundary backstop (`..`/empty reject).

## Parallel-surface check

`rg 'resolved_fs_root|fs_root' crates/argo-identity/src/store_hire_fs_root.rs
crates/argo-cli/src/handlers/worker_create_store.rs
crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` — extend the
existing resolver; do not fork a second validator.
