# Argo / SKYCOT operator targets.
#
# This Makefile is intentionally thin — Cargo handles build/test.
# It exists as a stable place to land safe one-command operator
# procedures that would otherwise be 5-step runbooks (see
# docs/skycot/operator/).

.PHONY: help restart-live-gateway test-restart-live-gateway

help:
	@echo "Operator targets:"
	@echo "  make restart-live-gateway        Safe-restart the local gateway"
	@echo "                                   (see docs/skycot/operator/gateway-restart-policy.md)"
	@echo "  make test-restart-live-gateway   Run the restart-live-gateway smoke tests"

# Flips the live gateway from a stale schema_version to current.
# Pass NO_PROMPT=1 to skip the 5s pre-restart warning (cron-friendly).
# Pass PORT=NNNN to override the default 7400.
restart-live-gateway:
	@bash scripts/restart-live-gateway.sh \
		$(if $(NO_PROMPT),--no-prompt) \
		$(if $(PORT),--port $(PORT)) \
		$(if $(HOST),--host $(HOST))

# Smoke test the restart script across stopped / current / stale
# scenarios. Uses a fake-binary harness so the suite runs in
# seconds without depending on a real release build.
test-restart-live-gateway:
	@bash scripts/restart-live-gateway-tests.sh
