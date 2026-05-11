# Runbook — `skycot/marketplace-browser` E2B image

Operator runbook for the pre-baked sandbox image that backs the
`BrowserExecutor`. Wires up `MISSION_BACKLOG.md` Section 3 Claim 5 #92.

## Why this exists

The BrowserExecutor today drives the operator's local Chrome, which
means every worker contends for one browser process and pollutes the
host with cookies/profile state. Mission Claim 5 calls for a
pre-baked E2B sandbox image so each worker spins up an isolated
browser sandbox per task — no contention, no host pollution, and a
deterministic Chromium pin we can update on our own cadence.

This image bundles those artifacts (Chromium + chromium-driver +
browser MCP server) into one OCI tag. The runtime wire that makes
E2B actually launch the image lives behind the
`image_for_executor("browser")` resolver in `argo-sandbox` and is
consumed when the runtime is built.

## Build locally

```sh
./images/marketplace-browser/build.sh
```

Tags applied to every build:

- `skycot/marketplace-browser:<short-git-sha>`
- `skycot/marketplace-browser:latest`

Prerequisites: Docker (the script prints a clear error if `docker`
isn't on PATH or the daemon isn't reachable). No additional tooling
required.

Optional push to a registry — requires `docker login <registry>` to
have already happened:

```sh
./images/marketplace-browser/build.sh --push-to ghcr.io/skycot
```

The build context is the repository root so the Dockerfile can
`COPY` the browser MCP fixture from `crates/argo-cli/tests/fixtures/`
without a separate prep step.

## Configure `argo.toml`

The `[sandbox.image]` table maps executor names → image references:

```toml
[sandbox.image]
marketplace_browser = "skycot/marketplace-browser:latest"
```

Missing table is fine: the parser defaults to an empty map and the
resolver returns `None`, letting the runtime fall back to whatever
default sandbox boot path it already uses.

Pin to a specific build for reproducibility:

```toml
[sandbox.image]
marketplace_browser = "skycot/marketplace-browser:f24073c"
```

## Use from the runtime

`argo-sandbox::image_for_executor("browser")` is the single lookup
seam. The sandbox launcher consults it before opening an E2B
sandbox; when set, the named image is passed as the template. When
unset, the launcher behaves exactly as before.

Add new named images by extending the `SandboxConfig` table and
calling `image_for_executor("<executor-name>")` from the relevant
launcher. The resolver intentionally returns `Option<String>` so
callers can fall back without an extra `if let`.

## Smoke-verify the image

Three-step smoke once you have a build:

1. **Pull** (or skip if you just built it locally):

    ```sh
    docker pull skycot/marketplace-browser:latest
    ```

2. **Run** the MCP server inline and confirm it starts:

    ```sh
    docker run --rm -i skycot/marketplace-browser:latest <<<'{}'
    ```

   The fixture echoes a handshake to stdout. If the container exits
   non-zero, re-run with `docker run -it --entrypoint /bin/bash …`
   and inspect `/opt/marketplace-browser/browser_mcp.py`.

3. **Drive Chromium** with the bundled driver:

    ```sh
    docker run --rm skycot/marketplace-browser:latest \
        bash -c 'xvfb-run chromium --headless --disable-gpu \
            --no-sandbox --dump-dom https://example.com | head -5'
    ```

   Should print the leading HTML of `example.com`. Failures here
   usually mean the Chromium apt pin drifted; re-check
   `chromium.pref`.

## Updating Chromium / browser MCP versions

- **Chromium track**: edit `images/marketplace-browser/chromium.pref`
  (e.g. swap `release a=stable` for a specific Debian codename),
  rebuild, run the smoke above.
- **Browser MCP server**: today the Dockerfile copies the offline
  fixture at `crates/argo-cli/tests/fixtures/browser_mock_mcp.py`.
  When `mvp-demo-vertical-browser-marketplace` lands a real binary,
  swap the `COPY` line in the Dockerfile and bump the tag.
- **E2B base**: `FROM e2bdev/code-interpreter:latest` floats by
  default. Pin to a digest (`e2bdev/code-interpreter@sha256:…`) when
  you need bit-for-bit reproducibility — see E2B's Docker Hub page
  for the digest list.

After any update, rebuild + smoke + bump the consumer's
`argo.toml [sandbox.image]` pin if you tagged a specific SHA.

## Follow-ups (not in scope of the bundling brief)

- ARM64 / multi-arch build (today this is amd64 only — the build
  script uses the local docker default architecture).
- Image scanning + SBOM generation.
- Pushing `:latest` to a public registry (Docker Hub / GHCR auth is
  an operator decision).
