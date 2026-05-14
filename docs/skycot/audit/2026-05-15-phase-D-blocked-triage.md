# SKYCOT BLOCKED Triage Report (2026-05-15)

**Authored**: 2026-05-15 by conductor via Explore agent (aca63c5ef2f165234) + conductor verification.
**Scope**: 49 BLOCKED rows audited; categorized into stale/real/scope-mismatch/deferred/dead.
**Trigger**: operator request to maximize codex-claimable TODO surface.

## Actions taken this pass

- **6 stale-blocks → TODO**: L11, L12, mission-cross-tenant-skill-propagation, mission-forge-stubforge-cleanup, mission-gdpr-data-residency, mission-saml-xml-signature-verify
- **4 dead rows → DROPPED**: mission-delegate-task-fallback-chain, mission-roster-resolve-policy, mission-judge-dreams-output-v2, mission-anomaly-alerting-notifychannel-v2

## Remaining categories (left BLOCKED)

- **8 L-track real-blocks** waiting on L6 ghostwriter merging (currently DIRTY; rebase brief `claude-rebase-pr-1632` authored): L7, L8, L10, L13, L14, L15, L16, L17
- **3 DEFERRED** (operator-paused): claude-mcp-monday-client-split, claude-identity-policy-split, claude-delete-consumer-mcp-stubs
- **18 SCOPE-MISMATCH** (need conductor brief rewrites): claude-error-types-rewrite-callsites, claude-config-loader-migrate-callsites, claude-tui-* (4), claude-gateway-request-log-schema, codex-connector-postmark, mission-cu-sandbox-pool-reaper, mission-skill-publish-verified-ingest, mission-gateway-bind-default-harden, mission-skill-explanation-provenance, mission-operator-feedback-dreams-curation-v2, mission-spend-gate-wire-transport, mission-delete-circuit-breaker-subsystem, mission-delete-stubforge-orphan-code, mission-sso-saml-xml-dsig-verify, mission-vertical-action-executor-completion, mission-rtbf-dreams-propagation, mission-forge-spend-polling-race-fix, mission-canonical-exponential-backoff
- **~6 real-blocks** (waiting on real unmerged deps): mvp-coverage-gate, M15-vertical-pack-loader, E2-i-hire-flow-day1-wrap-duty, J17, J18, K4-deploy-first-run-setup-wizard-v2

## Conductor TODO (for fresh session 2026-05-19)

Prioritized brief rewrites that would unlock the most BLOCKED rows:
1. `claude-tui-input-globals-split` → unblocks 3 TUI briefs (approval-tray, spend-monitor, search-everywhere, keybindings-help)
2. `claude-skill-cli-enum-split` → unblocks 3 skill CLI briefs (skill-publish-verified-ingest, skill-explanation-provenance, others)
3. Refresh `claude-error-types-rewrite-callsites` with split-per-crate approach (~438 sites across 13 crates; current brief is single-PR-unmanageable)

Per agent's report, the highest single-dep impact is L6 merging (cascade-unblocks 8 L-track briefs). `claude-rebase-pr-1632` is in TODO for that.
