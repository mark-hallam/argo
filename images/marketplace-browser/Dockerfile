# skycot/marketplace-browser — E2B-compatible sandbox image with
# Chromium + browser MCP server pre-installed. Bundled per
# MISSION_BACKLOG Section 3 Claim 5 #92 so workers spin up isolated
# browser sandboxes per task rather than contending on the operator's
# local Chrome.
#
# Base: e2bdev/code-interpreter:latest — E2B's published sandbox base.
# Confirm latest tag via https://hub.docker.com/r/e2bdev/code-interpreter
# before re-baking.
FROM e2bdev/code-interpreter:latest

# Reproducibility: pin Chromium to a known-good track via apt
# preferences (see chromium.pref alongside this Dockerfile). Updating
# the pin: edit chromium.pref, rebuild, smoke-test per runbook.
COPY chromium.pref /etc/apt/preferences.d/chromium

# Chromium + minimal runtime deps. --no-install-recommends keeps the
# image tight; xvfb + dbus-x11 cover headless launch without a real
# display server.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        chromium \
        chromium-driver \
        ca-certificates \
        dbus-x11 \
        fonts-liberation \
        python3 \
        xvfb \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Non-root user. uid 1000 matches the convention used by E2B's stock
# sandbox runtime, so file ownership lines up with mounted workspaces.
RUN groupadd --system --gid 1000 argouser \
    && useradd  --system --uid 1000 --gid 1000 \
        --home-dir /home/argouser --create-home \
        --shell /bin/bash argouser

# Browser MCP server. When the marketplace-seller-browser vertical's
# real server lands (per mvp-demo-vertical-browser-marketplace), swap
# this COPY for the built binary. Today's image vendors the offline
# Python stdio fixture so the image is testable end-to-end without a
# precursor build step.
#
# NOTE: the fixture lives under crates/argo-cli/tests/fixtures/ rather
# than the tests/fixtures/ path the brief cited. The repo layout is
# crate-scoped; documented as a sensible deviation in the PR body.
COPY crates/argo-cli/tests/fixtures/browser_mock_mcp.py \
     /opt/marketplace-browser/browser_mcp.py
RUN chmod +x /opt/marketplace-browser/browser_mcp.py \
    && chown -R argouser:argouser /opt/marketplace-browser

USER argouser
WORKDIR /home/argouser

# Chromium needs a writable user-data dir; default to a sandbox-local
# path under $HOME so the runtime mount doesn't clash.
ENV CHROMIUM_USER_DATA_DIR=/home/argouser/.chromium-data

ENTRYPOINT ["python3", "/opt/marketplace-browser/browser_mcp.py"]
