#!/usr/bin/env bash
# Run the onboarding E2E (mission-onboarding-e2e-integration-test).
#
# Modes:
#   local (default) — Playwright spins up SPA (3000) + marketing (3001);
#                     gateway calls are mocked via page.route().
#   deploy          — target live URLs (skycot.com / app / api).
#                     Requires E2E_TEST_INBOX_URL for magic-link polling.
#
# Usage:
#   bash scripts/run-e2e-onboarding.sh             # local
#   bash scripts/run-e2e-onboarding.sh deploy      # deploy-smoke
#   E2E_TENANT_PROVISION_READY=1 ... bash scripts/run-e2e-onboarding.sh
#
# Exit codes:
#   0 — passed
#   1 — failed
#   2 — prerequisites missing

set -euo pipefail

MODE="${1:-local}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT/web"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "fatal: pnpm not installed; install via 'corepack enable && corepack prepare pnpm@10 --activate'" >&2
  exit 2
fi

case "$MODE" in
  local)
    echo "[run-e2e-onboarding] mode=local — Playwright will manage SPA + marketing dev servers"
    if [ ! -d node_modules ]; then
      echo "[run-e2e-onboarding] installing web/ deps (first run)"
      pnpm install --frozen-lockfile
    fi
    if [ ! -d "$REPO_ROOT/marketing/node_modules" ]; then
      echo "[run-e2e-onboarding] installing marketing/ deps (first run)"
      (cd "$REPO_ROOT/marketing" && pnpm install --frozen-lockfile)
    fi
    # Browsers are large (~200MB); install on demand the first time.
    if [ ! -d "${PLAYWRIGHT_BROWSERS_PATH:-$HOME/Library/Caches/ms-playwright}" ]; then
      echo "[run-e2e-onboarding] installing Playwright chromium"
      pnpm exec playwright install chromium
    fi
    exec pnpm exec playwright test tests/e2e/onboarding.spec.ts --project=chromium 2>/dev/null \
      || exec pnpm exec playwright test tests/e2e/onboarding.spec.ts
    ;;
  deploy)
    echo "[run-e2e-onboarding] mode=deploy — targeting live URLs"
    : "${E2E_MARKETING_URL:=https://skycot.com}"
    : "${E2E_SPA_URL:=https://app.skycot.com}"
    : "${E2E_GATEWAY_URL:=https://api.skycot.com}"
    export ARGO_E2E_TARGET=deploy
    export SKIP_MARKETING_SERVER=1
    export E2E_MARKETING_URL E2E_SPA_URL E2E_GATEWAY_URL
    if [ -z "${E2E_TEST_INBOX_URL:-}" ]; then
      echo "[run-e2e-onboarding] warn: E2E_TEST_INBOX_URL unset; mail-catcher will time out and the test will fail unless an operator intercepts" >&2
    fi
    exec pnpm exec playwright test tests/e2e/onboarding.spec.ts
    ;;
  *)
    echo "fatal: unknown mode '$MODE' (expected: local|deploy)" >&2
    exit 2
    ;;
esac
