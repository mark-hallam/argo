# fix-deploy-web-dockerfile-build-context

**Owner**: claude — **P2 DEPLOY BLOCKER (pre-existing; surfaced gating PR #2929).**
**Depends on**: none.
**Authored against**: origin/main `ef2e6be5e` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-deploy-web-dockerfile-build-context

Source: discovered 2026-06-14 while gating #2929 (non-root containers). The
non-root change is fine; this is a SEPARATE pre-existing builder-stage bug that
blocks the full web image build entirely.

## Problem (verified by real `docker build` on origin/main)

`docker build -f deploy/Dockerfile.web .` fails in the **builder** stage at
`pnpm run build`:

```
Error: ENOENT: no such file or directory, open '/docs/skycot/pricing/v1-tiers.toml'
```

The Next.js web build reads `docs/skycot/pricing/v1-tiers.toml` at build time,
but `deploy/Dockerfile.web` builds with a `web/`-scoped build context (or copies
only `web/`), so that docs path is NOT present in the image build context. The
web image therefore **cannot be built** today — a hard go-live blocker (the SPA
image is part of the deploy). Pre-existing on origin/main (NOT introduced by the
non-root change).

## Fix (pick the cleanest — do NOT just suppress the read)

Grep where the web build reads `v1-tiers.toml` (`rg -n 'v1-tiers.toml' web/`)
and choose:
- **(a) preferred** — make the build read the pricing source from inside `web/`
  (move or generate a `web/`-local copy of `v1-tiers.toml`, or import the tiers
  from a `web/lib` module that is the single source) so the build needs no
  out-of-context path; OR
- **(b)** widen the Docker build context to the repo root and `COPY
  docs/skycot/pricing/v1-tiers.toml` into the expected path before `pnpm build`
  (heavier context — only if (a) is infeasible).

Keep pricing single-sourced (a prior drift had marketing vs web pricing
disagree — do NOT fork the tiers data).

## May touch
- `deploy/Dockerfile.web` (build context / COPY).
- `web/` build config + wherever `v1-tiers.toml` is read (option a).
- `docs/build-plan/FEATURE_MATRIX.md` (deploy-web row, in-band same commit).

## Parallel-surface declaration
No new Rust pub types (deploy/web only). If option (a) adds a `web/lib` pricing
module, `rg 'pricing|tiers' web/lib/` first to extend an existing one, not fork.

## Acceptance / CHECKABLE verifier
- [ ] `docker build -f deploy/Dockerfile.web -t argo-web .` (from repo root)
      succeeds end-to-end (gets past `pnpm run build`); paste the passing tail.
- [ ] Pricing values served by the built image match `v1-tiers.toml` (single
      source preserved).
- [ ] FEATURE_MATRIX deploy-web row updated in-band.

~10-40 LOC. Deploy/infra — conductor review before merge.
