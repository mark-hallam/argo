# Mission load-bearing wire — `skycot/marketplace-browser:latest` E2B sandbox image

**Owner**: claude. **Depends on**: existing E2B sandbox integration in `argo-sandbox` (verify in first 5min via `grep -rn 'e2b\|E2b\|E2B' crates/argo-sandbox/src/`); existing browser-executor wiring (MERGED via mvp-browser-action-executor); Docker (operator-installed for image build).
**Authored against**: origin/main `f24073c3` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketplace-browser-e2b-image`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #92 — "Build `skycot/marketplace-browser:latest` E2B sandbox image." Today browser executor runs against the operator's local Chrome. Mission promise: a pre-baked E2B sandbox image with Chromium + browser-MCP server pre-installed so workers can spin up isolated browser sandboxes per task (no contention on the operator's machine). Bundles the artifacts; doesn't run them — that's a separate runtime wire.

## Scope

1. **NEW `images/marketplace-browser/` directory** at repo root:
   - `Dockerfile` based on `e2bdev/code-interpreter:latest` (or whatever E2B's published base is — verify via `cat README.md | grep -i e2b` first; document the chosen base).
   - Adds: Chromium (`apt-get install -y chromium`), pinned Chromium version (write the apt-pinning file).
   - Adds: the browser MCP server binary (built from `verticals/marketplace-seller-browser/` per `mvp-demo-vertical-browser-marketplace` brief — if not landed, use the mock fixture at `tests/fixtures/browser_mock_mcp.py`).
   - Adds: a non-root user `argouser` (uid 1000), runs Chromium as that user.
   - Sets `ENTRYPOINT` to the MCP server.

2. **Build script**: NEW `images/marketplace-browser/build.sh`:
   - Tags as `skycot/marketplace-browser:<git-sha>` AND `:latest`.
   - Optional `--push-to <registry>` flag pushing to a configured registry.
   - Validates `docker --version` first; clear error if missing.

3. **CI hook**: NEW `images/marketplace-browser/ci-build.yml.disabled` (matches the existing pattern of `.disabled` workflows in `.github/workflows/`). When CI billing is restored, operator renames to `.yml` to enable. The workflow: on push touching `images/marketplace-browser/**`, run build.sh + push to `ghcr.io/skycot/marketplace-browser:<sha>`.

4. **NEW operator doc**: `docs/skycot/runbook/marketplace-browser-image.md` (≤ 200 LOC):
   - Why this image exists (mission claim 5 reference).
   - How to build locally (`./images/marketplace-browser/build.sh`).
   - How to use it (`argo.toml` config block: `[sandbox.image] marketplace_browser = "skycot/marketplace-browser:latest"`).
   - Verifying it works (a 3-step smoke: pull, run, hit a known URL through the MCP server).
   - Updating Chromium / browser-MCP versions.

5. **`argo.toml` schema**: NEW optional `[sandbox.image]` table with named-image map:
   ```toml
   [sandbox.image]
   marketplace_browser = "skycot/marketplace-browser:latest"
   ```
   Parser change in `argo-config` (NEW), default to empty map.

6. **Runtime hook**: when the browser-executor is invoked AND `marketplace_browser` is set in config, the sandbox launcher consults `image_for_executor("browser")` and passes the named image to E2B. NEW helper in `argo-sandbox` — verify the surface in first 5min.

This is NOT in scope: pushing the actual image to a public registry (operator decision, requires Docker Hub / GHCR auth); GPU-enabled variants; ARM64 cross-compile (note as follow-up); image scanning / SBOM generation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `images/marketplace-browser/Dockerfile` (NEW, ≤ 100 LOC)
- `images/marketplace-browser/build.sh` (NEW, ≤ 80 LOC, bash)
- `images/marketplace-browser/ci-build.yml.disabled` (NEW)
- `images/marketplace-browser/README.md` (NEW, ≤ 100 LOC)
- `docs/skycot/runbook/marketplace-browser-image.md` (NEW, ≤ 200 LOC)
- `crates/argo-config/src/sandbox.rs` (verify path; may be `crates/argo-config/src/lib.rs`) — narrowly scoped: ADD `[sandbox.image]` table parser. ≤ 40 LOC delta.
- `crates/argo-config/src/sandbox_tests.rs` (NEW or extend existing) — 3 unit tests: missing-table-is-empty; parse-multi-image; reject-non-string-value.
- `crates/argo-sandbox/src/image_resolver.rs` (NEW, ≤ 120 LOC) — `image_for_executor(name) -> Option<String>` helper.
- `crates/argo-sandbox/src/image_resolver_tests.rs` (NEW, sibling, ≤ 100 LOC) — 4 unit tests.

## Files you must NOT touch

- `crates/argo-sandbox/src/sandbox.rs` (or whatever the main impl is) — only consume the new resolver via dep injection; don't modify core sandbox boot
- `verticals/**` — vertical manifests stable; reuse only
- `tests/fixtures/browser_mock_mcp.py` — read-only, dockerfile copies it
- `crates/argo-store/migrations/**` — no schema change
- Any non-config/non-sandbox crate

## Acceptance criteria

- [ ] `images/marketplace-browser/Dockerfile` builds locally without errors
- [ ] `build.sh` tags both `<git-sha>` and `latest`
- [ ] `build.sh --help` works; missing Docker prints clear error
- [ ] `docs/skycot/runbook/marketplace-browser-image.md` covers: build, use, smoke-verify, update
- [ ] `argo.toml [sandbox.image] marketplace_browser = "..."` parses; missing table → empty map (back-compat)
- [ ] `image_for_executor("browser")` returns the configured image or None
- [ ] 3+4 unit tests for parser + resolver
- [ ] NO actual image push (operator decision)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-config -p argo-sandbox` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-marketplace-browser-e2b-image`
- PR title: `infra: skycot/marketplace-browser E2B image + sandbox image resolver (mission-marketplace-browser-e2b-image)`

## Notes for the implementer

- The Dockerfile is the smallest viable shape. Chromium version pinning matters — write a `chromium.pref` apt-pin file so `apt-get install` is reproducible.
- For the MCP server binary: if `mvp-demo-vertical-browser-marketplace` hasn't landed, COPY the mock fixture into the image as a fallback. Document the swap-in-real point.
- The `.yml.disabled` pattern is intentional — CI is currently disabled per ledger header. Don't try to re-enable any CI in this brief.
- E2B's exact base image name and version: verify by running `grep -rn 'e2bdev\|e2b-dev' .` against the workspace first. If E2B is not yet integrated, BLOCK and request a precursor brief — DON'T add E2B integration in this brief.
- The runbook doc is part of the brief because mission claim 5 is operator-facing — having the runbook is ship-gate quality.
- The image build runs locally; you don't need Docker to develop the Rust-side parser changes, but the integration test in step 1 of acceptance criteria requires Docker on the test machine. Note that and skip with `#[cfg(feature = "docker-integration")]` if needed.
