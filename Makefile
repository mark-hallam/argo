# Argo / SKYCOT operator targets.
#
# This Makefile is intentionally thin — Cargo handles build/test.
# It exists as a stable place to land safe one-command operator
# procedures that would otherwise be 5-step runbooks (see
# docs/skycot/operator/).

.PHONY: help restart-live-gateway first-run-setup test-restart-live-gateway

help:
	@echo "Operator targets:"
	@echo "  make restart-live-gateway        Safe-restart the local gateway"
	@echo "                                   (auto-bootstraps env.local on first run;"
	@echo "                                    see docs/skycot/operator/gateway-restart-policy.md)"
	@echo "  make first-run-setup             Run first-run bootstrap only — generate"
	@echo "                                   \$$ARGO_HOME/env.local with a fresh DB key"
	@echo "                                   and API key, without launching the gateway."
	@echo "  make test-restart-live-gateway   Run the restart-live-gateway smoke tests"
	@echo "                                   (restart scenarios + first-run bootstrap)"

# Flips the live gateway from a stale schema_version to current.
# Pass NO_PROMPT=1 to skip the 5s pre-restart warning (cron-friendly).
# Pass FORCE=1 to restart even when the live schema_version is already
# current (useful when switching from an inline DB key to a persistent
# env.local DB key).
# Pass PORT=NNNN to override the default 7400.
restart-live-gateway:
	@bash scripts/restart-live-gateway.sh \
		$(if $(NO_PROMPT),--no-prompt) \
		$(if $(FORCE),--force) \
		$(if $(PORT),--port $(PORT)) \
		$(if $(HOST),--host $(HOST))

# Run only the first-run bootstrap (generate $ARGO_HOME/env.local).
# No-op if env.local already exists. Useful when an operator wants to
# pre-create env.local before the first `make restart-live-gateway`.
first-run-setup:
	@bash scripts/restart-live-gateway.sh --bootstrap-only --no-prompt

# Smoke test the restart script across stopped / current / stale /
# --force scenarios plus the first-run bootstrap. Uses a fake-binary
# harness so the suite runs in seconds without depending on a real
# release build.
test-restart-live-gateway:
	@bash scripts/restart-live-gateway-tests.sh
	@bash scripts/restart-live-gateway-first-run-tests.sh
