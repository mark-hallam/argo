# V1.4 — Documentation site build

**Owner**: claude. **Depends on**: nothing.

## Scope

Project docs are scattered: `docs/skycot/` (mission, runbook,
investigations), `docs/build-plan/` (protocol + plan +
streams), root README, AUTHORING.md inside crates. No unified
discoverable surface for "how do I do X".

Build a static doc site using **mdBook** (Rust ecosystem
standard, fast, no Node toolchain). Builds to
`book/` directory; deploys via GitHub Pages on each main push.

Layout:
- Quickstart (5-min install → first worker → first action)
- How-to guides (per role: marketing intern / sales SDR /
  customer support — link to demo verticals)
- Reference (CLI command catalog auto-generated from
  `argo --help`)
- Architecture (PROTOCOL.md + investigation docs)
- Operations (RUNBOOK.md, troubleshooting)

Auto-update: a `docs-publish` GitHub Action runs on push to
main, builds the book, pushes to `gh-pages` branch.

## Files you may touch
- `book.toml` (NEW — mdBook config at repo root)
- `book/` (NEW — source markdown structured per layout above)
- `book/SUMMARY.md` (NEW — table of contents)
- `book/quickstart/install.md` + 4 more quickstart pages
- `book/howto/*.md` (3-5 how-to guides)
- `book/reference/cli.md` (NEW — auto-generated)
- `book/architecture/protocol.md` (NEW — mostly references PROTOCOL.md)
- `.github/workflows/docs.yml` (NEW or extend)
- `scripts/build-cli-reference.sh` (NEW — runs argo --help
  recursively + emits markdown)

## Acceptance criteria
- [ ] `mdbook build` produces clean HTML in `book/` output dir
- [ ] CLI reference auto-generates from `argo --help` walk
- [ ] All internal links resolve (mdbook test passes)
- [ ] Search bar works (mdbook default)
- [ ] gh-pages workflow deploys on push to main
- [ ] Mobile-responsive (mdbook default)
- [ ] No broken links in CI (mdbook-linkcheck plugin)

## Branch + PR
- Branch: `claude-docs-site-build`
- PR title: `docs: mdBook documentation site + GitHub Pages publish (claude-docs-site-build)`
