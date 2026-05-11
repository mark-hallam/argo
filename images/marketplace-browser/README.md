# skycot/marketplace-browser

Pre-baked E2B sandbox image with Chromium + browser MCP server,
shipped per `MISSION_BACKLOG.md` Section 3 Claim 5 #92.

## What this is

A Dockerfile (and supporting build / CI scripts) that bakes the
artifacts the `BrowserExecutor` needs into a single OCI image:

- Chromium + chromium-driver (apt-pinned to Debian stable via
  `chromium.pref` for reproducible rebuilds).
- A non-root `argouser` (uid 1000) — Chromium runs as that user so the
  E2B runtime mounts and host filesystem stay isolated.
- The browser MCP server. Today this is the offline stdio fixture
  vendored from `crates/argo-cli/tests/fixtures/browser_mock_mcp.py`;
  swap it for the real `marketplace-seller-browser` binary once that
  brief lands.

This directory bundles the artifacts. It does **not** push to a
public registry — that's an operator decision.

## Files

- `Dockerfile` — image definition.
- `chromium.pref` — apt preferences pin for the Chromium track.
- `build.sh` — local build helper. `--push-to <registry>` is opt-in.
- `ci-build.yml.disabled` — GitHub Actions workflow. Operator renames
  to `.yml` when CI billing is restored.

## Build locally

```sh
./images/marketplace-browser/build.sh
```

Tags applied: `skycot/marketplace-browser:<git-sha>` and `:latest`.

## Wire into a runtime

See [`docs/skycot/runbook/marketplace-browser-image.md`][runbook] for
the `argo.toml [sandbox.image]` config block and the
`image_for_executor("browser")` resolver wiring.

[runbook]: ../../docs/skycot/runbook/marketplace-browser-image.md
