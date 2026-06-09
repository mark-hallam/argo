# DEC-043 — Stop committing the generated `feature-matrix.html` (root-fix the per-PR merge conflict)

- **Date:** 2026-06-09
- **Status:** **active** — operator-directed ("fix stale at the root", 2026-06-09).
- **Owner:** conductor (Claude) — implemented; operator authorized.
- **Relates to:** the ledgers-in-band rule in `CLAUDE.md`; [DEC-042](DEC-042-continuous-self-improvement-loop.md) §3 B-2 (the Stop-gate's Gate 2 enforced the old rule); [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) (drain PR backlogs).
- **Supersedes:** the `CLAUDE.md` sub-rule "whenever you edit `FEATURE_MATRIX.md`, regenerate its HTML view **in the same commit** … the committed `docs/skycot/feature-matrix.html` is generated from the .md." That clause is removed; the rest of the ledgers-in-band rule stands.

---

## 1. Problem (verified 2026-06-09)
`docs/skycot/feature-matrix.html` is a **generated** artifact (rendered from
`docs/build-plan/FEATURE_MATRIX.md` by `scripts/render-feature-matrix.py`) that
was **committed** to version control, with a `CLAUDE.md` rule mandating it be
regenerated **in the same commit** as any `FEATURE_MATRIX.md` edit.

Because the HTML is a *full re-render*, any two PRs that both touch
`FEATURE_MATRIX.md` produce **non-line-mergeable** HTML and **hard-conflict** on
it — even when the underlying `.md` rows auto-merge cleanly. With ~11 merged
PRs/day, most touching the matrix, this conflicted constantly. It stalled three
otherwise-clean PRs this session (#2754 voice tenant-scope, #2752 channel
hygiene, #2751 worker attribution): their *code* merged clean; only the
generated HTML conflicted. Worse, `render-feature-matrix.py` resolved its paths
`__file__`-relative, so regenerating from a git worktree wrote into the **main
checkout**, polluting the shared tree (hit during a conductor gate attempt).

## 2. Decision
**Stop committing the generated HTML.** `FEATURE_MATRIX.md` (markdown) is the
single source-of-truth, in-band ledger. `docs/skycot/feature-matrix.html` becomes
a **gitignored, on-demand build artifact** — regenerated locally only to *view*
the matrix in a browser, never committed.

Implemented atomically:
1. `git rm --cached docs/skycot/feature-matrix.html` + `.gitignore` entry.
2. `CLAUDE.md` ledgers-in-band rule updated: the `.md` is the ledger; the HTML is
   gitignored/on-demand; do NOT commit it.
3. `scripts/stop-gate-verify.sh` Gate 2 flipped from "`.md` changed ⇒ `.html`
   must change too" to "the generated `.html` must NOT be committed" (enforces the
   new contract; still no-compile, read-only over the branch diff).
4. `scripts/render-feature-matrix.py` made **worktree-safe**: resolves the repo
   root from the git top-level of the *current working dir* (falls back to
   `__file__`), so regenerating from a worktree writes that worktree's view — and
   since the output is now gitignored, it can never pollute a tracked tree.

## 3. Why this (first-principles), and why not the alternatives
- **Generated artifacts do not belong in version control.** The `.md` is the
  source; the HTML is derived. Committing a derived full-render guarantees
  conflicts. Removing it removes the conflict class entirely.
- **The committed HTML was vestigial.** Verified: it is **not** served by GitHub
  Pages (no Pages/CNAME config), **not** linked from the SPA or any served route,
  **not** read at runtime — every reference is process-prose ("regenerate the
  HTML"). Its only use was "open the local file"; the on-demand regen still gives
  that. So nothing of value is lost.
- **A git merge driver was rejected:** merge drivers are *local-only*; GitHub's
  web/`gh pr merge` path ignores them, so the actual merge (admin-merge, GHA
  disabled) would still conflict. Gitignore is the only fix that works for the
  real merge path.
- **`merge=union` was rejected:** concatenating two HTML renders yields garbled
  duplicate markup.

## 4. Consequences
- Future `FEATURE_MATRIX.md` PRs no longer conflict on the generated HTML.
- **Existing stale PRs** (#2754/#2752/#2751) now hit a trivial *modify/delete*
  conflict on the HTML when merged onto post-DEC-043 main — resolved by dropping
  their HTML edit (`git rm` on their side); no renderer needed. (Follow-on: rebase
  + land them.)
- **Build-agent / worker prompts inherit this:** update `FEATURE_MATRIX.md` in-band
  as before; do NOT regenerate or commit `feature-matrix.html`. The Stop-gate
  (`ARGO_STOP_GATE=1`) now *fails* a commit that includes the generated HTML.
- The browsable HTML view is still available on demand:
  `python3 scripts/render-feature-matrix.py` (writes the gitignored file).

## 5. Verification (NO THEATER, DEC-029)
- `git check-ignore docs/skycot/feature-matrix.html` → ignored ✓.
- `git rm --cached` staged the untrack ✓; a subsequent `render-feature-matrix.py`
  run produces **no tracked change** (output is ignored) ✓.
- `python3 -m py_compile scripts/render-feature-matrix.py` clean; the renderer
  still produces the HTML ✓.
- Gate 2 of `stop-gate-verify.sh` now rejects a committed `feature-matrix.html`.
