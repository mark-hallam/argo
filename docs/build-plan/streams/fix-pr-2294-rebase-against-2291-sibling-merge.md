# fix-pr-2294-rebase-against-2291-sibling-merge

**🔴 P0 fast-mode rebase brief — Track P / demo-seed cascade**

Authored against: origin/main `0c48732b8` on 2026-05-25
Validate before claim: bash scripts/check-brief-freshness.sh fix-pr-2294-rebase-against-2291-sibling-merge

## Why this exists

PR #2294 (`demo-seed: real-LLM agent-turn driver`) is the second sequential brief in the `mission-demo-seed-data-bootstrap` split (3 children: scaffold #2272 + driver #2294 + runbook). Sibling PR #2291 (`mission-demo-seed-data-bootstrap` — the rolled-up scaffold + driver + skills + workflow + cross-tenant package) merged at T14-30Z, which conflicts file-by-file with #2294's narrower driver-only changes since both target `crates/argo-demo-seed/`.

## What this brief does

Rebase `claude-mission-demo-seed-real-llm-driver` onto current `origin/main`, resolving conflicts in:
- `crates/argo-demo-seed/src/lib.rs`
- `crates/argo-demo-seed/src/seed.rs`
- `crates/argo-demo-seed/src/skills.rs`
- `crates/argo-demo-seed/src/workflow.rs`
- `crates/argo-demo-seed/Cargo.toml`
- Possibly `crates/argo-demo-seed/tests/*.rs`

Preserve BOTH contributions:
- **#2291 contribution (already on main)**: scaffold + Pass B fixture audit rows (150 rows) + workers + cross-tenant invisibility test + skills bundle + workflow registration. Treat this as the new baseline.
- **#2294 contribution to ADD**: `drive_agent_turns(&Arc<SessionStore>, &Arc<AuditSink>, TenantId, SeedProfile) -> SeedReport` Pass A real-LLM driver. 6 scripted beats (3 approval + 3 non-approval). Fail-closed on `ARGO_GENESIS_PROVIDER_KEY` / `ARGO_DEMO_SEED_PROVIDER_KEY` unset (NO `ANTHROPIC_API_KEY` fallback — cost guardrail). 12 Pass A turns + 3 approval rows + 14-day backdate on `created_at` only (not a chain-hash input).

If #2291 already includes a `drive_agent_turns` or equivalent Pass A driver, MERGE-AS-SUPERSEDED instead of force-pushing — flip ledger row to `MERGED-AS-COVERED-BY-#2291` with a one-line note. Verify by `rg drive_agent_turns crates/argo-demo-seed/` on current main.

## Acceptance criteria

1. `git rebase origin/main` completes (or worker determines MERGED-AS-COVERED and closes #2294 with note)
2. `cargo clippy -p argo-demo-seed --all-targets -- -D warnings` clean
3. `cargo test -p argo-demo-seed` passes all (Pass A live-key test should SKIP-as-PASS if env unset)
4. `scripts/workspace-clippy-fast-fail.sh` clean
5. Force-push to existing branch (preserve PR #2294 history)
6. PR #2294 flips to MERGEABLE/CLEAN

## May-touch

- `crates/argo-demo-seed/src/**/*.rs`
- `crates/argo-demo-seed/tests/*.rs`
- `crates/argo-demo-seed/Cargo.toml`

## Must NOT touch

- Other crates
- BUILD_STATUS.md (conductor flip post-merge)
- PR description on #2294 (preserve original)

## Verification

```bash
git fetch origin main
git checkout claude-mission-demo-seed-real-llm-driver
git rebase origin/main
# resolve conflicts; check if #2291 already covers driver via:
rg "drive_agent_turns|ARGO_GENESIS_PROVIDER_KEY" crates/argo-demo-seed/
cargo clippy -p argo-demo-seed --all-targets -- -D warnings
cargo test -p argo-demo-seed
bash scripts/workspace-clippy-fast-fail.sh
git push --force-with-lease origin claude-mission-demo-seed-real-llm-driver
```

## Fast-mode candidate

Yes — rebase + possible merge-as-superseded determination. ~60-90 min.

## Reference

- Conductor rebase recipe on PR #2294: https://github.com/mark-hallam/Argo-Agent/pull/2294#issuecomment-4535295200
- Sibling PR #2291 (the merge that caused conflict): MERGED 2026-05-25T14:30Z squash be1ccd1ca
- Parent brief: `mission-demo-seed-data-bootstrap` (now MERGED via #2291)
