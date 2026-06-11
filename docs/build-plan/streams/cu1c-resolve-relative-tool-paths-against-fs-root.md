# cu1c-resolve-relative-tool-paths-against-fs-root

**Owner**: claude — **SECURITY / capability-fence-adjacent → adversarial-review before merge, do NOT self-merge.**
**Depends on**: cu1 (#2812, merged) + cu1b (dispatch-cwd jail alignment).
**Authored against**: origin/main `31b8b56e8` on `2026-06-11`. **RE-SCOPED against `f9eb6bec3` on `2026-06-12`** (conductor: unblocked the 2026-06-11 BLOCK — see "Re-scope" below).
**Validate before claim**: bash scripts/check-brief-freshness.sh cu1c-resolve-relative-tool-paths-against-fs-root

## Re-scope 2026-06-12 (conductor — resolves the 2026-06-11 BLOCK)

The original design assumed `required_capabilities(&self, args)` had a `ctx`
param — it does not (`Tool` trait, `crates/argo-tool/src/tool.rs:210`; sole
prod caller `registry.rs:207` passes args only). Threading `ctx` into the
existing method = ~13 impls + ~34 call sites — out of scope. **Approved
design instead: an ADDITIVE trait method with a default impl.**

- Add `required_capabilities_in_ctx(&self, args, ctx) -> ...` to the `Tool`
  trait with a default body that delegates to `required_capabilities(args)` —
  every existing tool compiles untouched.
- Switch the capability-check call site in `registry.rs` (:207 region) to the
  `_in_ctx` variant (it has ctx there).
- ONLY the three file tools (`file_read` / `file_write` / `file_patch`)
  override it, applying the resolve-against-`ctx.workspace_root` logic below.

May-touch is widened accordingly: + `crates/argo-tool/src/tool.rs` (additive
trait method only) + `crates/argo-tool/src/registry.rs` (call-site swap only).
Everything else in the original brief (the resolve semantics, deny-by-default,
acceptance) stands unchanged.

## Goal (DEC-050 G1 polish)

Let a fenced teammate use **relative** file paths (`invoice-acme-4471.txt`)
instead of being forced to pass the absolute sandbox path
(`/Users/.../SkycotDemo/Inbox/invoice-acme-4471.txt`). Today only absolute
paths work; a relative path is refused by RBAC.

## Why now (live e2e 2026-06-11, evidence in docs/skycot/demos/2026-06-11-cu1b-real-work-proof.md)

With cu1+cu1b, a teammate reads/writes real files inside its `fs_root` — **but
only with absolute paths**. A relative path fails the capability check:

```
rbac denied: genesis-worker lacks FsRead { path_glob: "invoice-acme-4471.txt" }
```

Root cause: `crates/argo-tool/src/tools/file_read.rs::required_capabilities`
(and the `file_write`/`file_patch` equivalents) returns
`Capability::FsRead { path_glob: parsed.path }` using the **raw, un-normalized**
path from the tool args. The granted capability glob is absolute
(`<fs_root>/**`), so a relative requested path never matches — RBAC denies
*before* `confine_path` ever runs. The teammate has no reliable way to know its
own absolute sandbox path, so absolute-only is a real usability cliff (and a
demo footgun).

## Fix (normalize against the jail root, deny-by-default preserved)

In each file tool's `required_capabilities` (`file_read.rs`, `file_write.rs`,
`file_patch.rs`), resolve a relative requested path against the tool context's
jail root (`ctx.workspace_root`, which cu1b sets to `fs_root`) BEFORE
constructing the `Fs*` capability glob:

- If `parsed.path` is absolute → use as-is (unchanged).
- If relative → join onto `ctx.workspace_root`, then normalize (no `..`
  climb-out; reuse the same `..`-refusal that `path_guard::confine_path` uses).
- The capability check then compares the resolved absolute path against the
  absolute grant glob — match for in-fence relatives, miss for climb-outs.

This is a normalization-only change: it does NOT widen the grant. A relative
path that resolves outside `fs_root` (via `..`) must still be refused, both at
RBAC and at `confine_path` (defense in depth — keep both). NEVER infer a root
from anything but the request's own `ctx.workspace_root`.

## May touch

- `crates/argo-tool/src/tools/file_read.rs` (+ tests)
- `crates/argo-tool/src/tools/file_write.rs` (+ tests)
- `crates/argo-tool/src/tools/file_patch.rs` (+ tests)
- a shared helper if the three duplicate the resolve logic (prefer one
  `resolve_against_root(ctx, &parsed.path)` in a sibling module over three
  copies — grep first per DRIFT_PREVENTION).

## Acceptance / verifier

1. Unit: `required_capabilities` for a relative path `"a.txt"` with
   `ctx.workspace_root = /tmp/w` yields `FsRead{ path_glob }` whose path is
   `/tmp/w/a.txt` (absolute); for `"../escape"` it yields a path that does NOT
   fall under `/tmp/w/**` (so RBAC denies). Absolute input is unchanged.
2. Live: re-run the Demo-C AR-summary task but with the teammate using RELATIVE
   filenames in its tool calls → real `ar-summary.md` written; a relative
   `../escape.md` still denied (negative pin).
3. `cargo test -p argo-tool` green; clippy; no widening of any grant.

## Size / owner / priority

~80-140 LOC incl. tests. **🟡 P2 — DEC-050 usability polish** (cu1+cu1b already
deliver real fenced work with absolute paths; this removes the absolute-path
requirement). Fence-adjacent (touches capability derivation) → adversarial
review.
