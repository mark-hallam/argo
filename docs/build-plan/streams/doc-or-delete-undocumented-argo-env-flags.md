# doc-or-delete-undocumented-argo-env-flags

Authored against: origin/main 55814da6ae602dc4859ee115f95b953bfc504949 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh doc-or-delete-undocumented-argo-env-flags

## Goal

For each ARGO_* env var read in code but documented/set nowhere, either
DOCUMENT it (operator-facing config reference) or DELETE it if dead.

## Why now (architecture audit 2026-06-10, P3-7)

Of 92 ARGO_* env vars read in crates/, 12 never appear in scripts/, Makefile,
web/, or docs/ — undiscoverable knobs, some possibly dead. Verified 2026-06-10
(read in crates, 0 docs/scripts refs): ARGO_CALIBRATION_STRICT,
ARGO_SAML_CLOCK_SKEW_SECS, ARGO_MARKETING_LLM_PROVIDER, ARGO_PITCH_FACTS_PATH,
ARGO_PRINCIPAL, ARGO_PUSH_DEVICES_PATH, ARGO_MAINTENANCE_ETA,
ARGO_TWILIO_SMS_DECISION_URL, ARGO_XDOTOOL_LOG, ARGO_EVAL_RUNS_DIR (+2 test-only ids).

## Fix

For each flag: `rg '<FLAG>' crates/` to find its reader. If it has a live,
load-bearing reader → add a one-line entry to the operator config reference
(docs/skycot/operator/ or the canonical env-var doc; create a short
ENV_VARS.md if none exists). If the reader is dead/unreachable (behind a
removed feature) → delete the read + the flag. Test-only ids: leave or move to
a test-constants module. One PR; list each flag's disposition (doc vs delete)
in the body.

## May touch

- docs/skycot/operator/ENV_VARS.md (NEW or existing config ref)
- the crate read-sites of any DEAD flag (delete only)

## Acceptance / verifier

1. Every one of the 12 flags is either in the config doc OR removed from code.
2. `rg 'ARGO_[A-Z_]+' crates/` flags cross-referenced: no read-but-undocumented
   remain (a small checker script or a documented list).
3. cargo check passes (if any dead-flag deletions touched code).

## Size / owner

~docs + small deletions. Owner: claude (codex-eligible). 🟢 P3 — discoverability
+ dead-code hygiene; low risk.
