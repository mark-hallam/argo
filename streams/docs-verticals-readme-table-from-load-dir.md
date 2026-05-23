# docs-verticals-readme-table-from-load-dir

**Authored against:** `origin/main` 2af54930f on 2026-05-23 (conductor tick-110).
**Validate before claim:** `bash scripts/check-brief-freshness.sh docs-verticals-readme-table-from-load-dir`
**Owner:** codex (fast-mode-eligible — pure mechanical)
**Priority:** 🟢 P3 docs — `verticals/README.md` table is stale by 28 entries
**Source:** mission-vs-code-gap-manifest.md row argo-vertical-packs F-4 (audited 2026-05-23 round 7).

## The problem

`verticals/README.md` table lists 8 entries but `argo_vertical::load_dir` successfully loads 32 verticals from `verticals/*`. Operators reading the README think only 8 templates are available.

## What to do

1. Write a small Rust binary (or a Python+`taplo` script — operator picks the shorter path) at `scripts/regenerate-verticals-readme-table.sh` (or `.rs`) that:
   - Walks `verticals/*/` for `vertical.toml` files
   - Calls `argo_vertical::load_dir` on each
   - Builds a markdown table of (id, display_name, status, action_count, prompts_word_count)
   - Replaces the existing table region in `verticals/README.md` between two HTML-comment anchors (insert anchors if not present)
2. Run the script. Commit the regenerated table.
3. Add a CI lint or pre-commit hook check that runs the script in `--check` mode and fails if the table is drifting again.

Skip the 4 demo-only stubs (`ap-specialist`/`deal-desk`/`support-tier1`/`vendor-manager`) silently — let `argo_vertical::load_dir` reject them as it already does — but log them in a `## Demo stubs (not yet promoted)` section below the table.

## May-touch

- `verticals/README.md`
- New file: `scripts/regenerate-verticals-readme-table.sh` (or `.rs` under `tools/`)
- Possibly `.github/workflows/...` for the lint step (gated on operator approval — currently CI is disabled per v1.0 strategy, so a `scripts/check-verticals-readme-fresh.sh` invoked by a pre-commit hook is the cleaner ship for now)

## Acceptance

- Table re-rendered with all 32 loadable verticals.
- Script runs idempotently (running twice produces no diff).
- README still passes any existing `markdownlint` rules.
- No Rust crate code touched.

## Citations

- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` row argo-vertical-packs F-4
- `verticals/README.md:55-60`

## LOC budget

~150-250 LOC.
