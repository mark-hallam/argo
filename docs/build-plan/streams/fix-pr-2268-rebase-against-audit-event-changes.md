# fix-pr-2268-rebase-against-audit-event-changes

**🟡 P1 fast-mode rebase brief — Mission Claim 6 multi-tenant hardening**

Authored against: origin/main `0c48732b8` on 2026-05-25
Validate before claim: bash scripts/check-brief-freshness.sh fix-pr-2268-rebase-against-audit-event-changes

## Why this exists

PR #2268 (`audit(channels): emit CrossTenantInboundRejected on Slack inbound rejection`) has been DIRTY since before the wave-3 Rust merge train. The audit event surface (`crates/argo-audit/src/event.rs`, `event_kind.rs`) and Slack inbound channel routes (`crates/argo-cli/src/handlers/gateway/routes_channels_*.rs`) have moved underneath. The PR's premise (emit a tenant-mismatch audit row at the SMS+Slack inbound rejection path) remains LOAD-BEARING for Mission Claim 6.

## What this brief does

Rebase `claude-mission-slack-inbound-tenant-mismatch-audit-emission` onto current `origin/main`, resolving conflicts in:
- Likely: `crates/argo-audit/src/event_kind.rs` (last touched by #2292 — Option C doc extraction; `event_kind.rs` now ~270 LOC, was 500)
- Likely: `crates/argo-audit/src/event.rs` (#2317 audit-list tenant-scope leak fix may have touched)
- Likely: `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs`
- Possibly: `crates/argo-cli/tests/routes_channels_slack*.rs`

## Critical: AuditKind variant count

Current `AuditKind` variant count on main is at the post-#2317 baseline. The PR was authored against an earlier baseline. The rebase needs to:
1. Re-base the brief's new `CrossTenantInboundRejected` variant onto whatever count is current
2. Bump the variant-count assertion + label round-trip test to match
3. Ensure the 4-site recipe (event.rs enum + event_kind.rs label table + kind() match + AuditKind::all() listing) is fully applied

Reference: PR #2295 (Track P P-02) is ALSO blocked on similar variant-count issue and has its own rebase brief — coordinate; the two rebases will conflict if they land simultaneously.

## Acceptance criteria

1. `git rebase origin/main` completes cleanly
2. AuditKind variant count + label table + round-trip test all aligned with current main
3. `cargo clippy -p argo-cli -p argo-audit --all-targets -- -D warnings` clean
4. `cargo test -p argo-cli routes_channels_slack` passes (or the actual test name from the PR)
5. `scripts/workspace-clippy-fast-fail.sh` clean
6. Force-push; PR #2268 flips MERGEABLE/CLEAN

## May-touch

- `crates/argo-audit/src/event.rs`, `event_kind.rs`, `sink.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs` (+ siblings if SMS path also covered)
- `crates/argo-cli/tests/routes_channels_slack*.rs`

## Must NOT touch

- Other crates beyond argo-audit + argo-cli
- BUILD_STATUS.md
- PR description on #2268

## Verification

```bash
git fetch origin main
git checkout claude-mission-slack-inbound-tenant-mismatch-audit-emission
git rebase origin/main
# count AuditKind variants on main:
rg -c "^\\s+[A-Z][A-Za-z]+," crates/argo-audit/src/event_kind.rs
cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings
cargo test -p argo-cli routes_channels_slack
bash scripts/workspace-clippy-fast-fail.sh
git push --force-with-lease origin claude-mission-slack-inbound-tenant-mismatch-audit-emission
```

## Fast-mode candidate

Yes — mechanical rebase + count-alignment. ~60-90 min.

## Reference

- Conductor rebase recipe on PR #2268: https://github.com/mark-hallam/Argo-Agent/pull/2268#issuecomment-4535308623
- Mission Claim 6 substrate (Multi-tenant approvals + audit emission)
